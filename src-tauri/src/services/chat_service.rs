//! Chat business logic.

use crate::ai::{config as ai_config, streaming};
use crate::error::AppError;
use crate::infra::db::memora_pool;
use crate::models::{ChatMessage, SessionSummary};
use crate::prompts;
use crate::repo::{chat_repo, persona_repo, session_repo};
use crate::services::compaction;
use crate::services::memory_search_service;
use anyhow::Context;

#[tracing::instrument(skip(app), err)]
pub async fn send_message(
    app: tauri::AppHandle,
    persona_id: String,
    session_id: String,
    content: String,
) -> Result<String, AppError> {
    let config = ai_config::load_config();
    let now = chrono::Utc::now().to_rfc3339();
    let latest_user_message = content.clone();

    // All DB work in a blocking closure to avoid holding r2d2 connections across .await
    let (name, persona_md, memories_md, session_summary, history) = {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;

        // Save user message
        chat_repo::save_message(&conn, &persona_id, &session_id, "user", &content, &now)?;

        // Load persona
        let (name, persona_md, memories_md, _ver) =
            persona_repo::get_persona_data(&conn, &persona_id)?;

        // Pull session summary (compaction)
        let session_summary: String = session_repo::get_summary(&conn, &session_id)?
            .map(|(s, _)| s)
            .unwrap_or_default();

        let summary_block = if session_summary.is_empty() {
            "\n".to_string()
        } else {
            format!("\n## 故事前情提要\n{}\n\n", session_summary)
        };

        let sys = prompts::render(prompts::SYSTEM_CHAT, &[
            ("name", &name),
            ("persona_md", &persona_md),
            ("memories_md", &memories_md),
            ("session_summary", &summary_block),
        ]);

        let recent_limit: i32 = if session_summary.is_empty() { 50 } else { 12 };
        let hist = chat_repo::recent_messages(&conn, &persona_id, &session_id, recent_limit)?;

        (name, persona_md, memories_md, session_summary, hist)
    };

    let selected_memories_md = if memory_search_service::has_index(&persona_id) {
        match memory_search_service::search_memories_texts(&persona_id, &latest_user_message, 5).await {
            Ok(hits) if !hits.is_empty() => {
                tracing::info!(
                    "semantic_search: top-{} memories injected, query_len={}",
                    5,
                    latest_user_message.chars().count()
                );
                let bullet_list = hits
                    .into_iter()
                    .map(|item| format!("- {}", item))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("## 共同记忆（语义检索 Top-5）\n{}", bullet_list)
            }
            Ok(_) => {
                tracing::info!("semantic_search: index exists but returned 0 hits, fallback to full memories");
                memories_md.clone()
            }
            Err(err) => {
                tracing::warn!("semantic_search failed, fallback to full memories: {}", err);
                memories_md.clone()
            }
        }
    } else {
        memories_md.clone()
    };

    let summary_block = if session_summary.is_empty() {
        "\n".to_string()
    } else {
        format!("\n## 故事前情提要\n{}\n\n", session_summary)
    };

    let system_prompt = prompts::render(prompts::SYSTEM_CHAT, &[
        ("name", &name),
        ("persona_md", &persona_md),
        ("memories_md", &selected_memories_md),
        ("session_summary", &summary_block),
    ]);

    // Stream completion (async)
    let request_id = uuid::Uuid::new_v4().to_string();
    let full_reply = streaming::chat_completion_stream(
        &config,
        &system_prompt,
        history,
        &app,
        &request_id,
    )
    .await
    .map_err(|e| AppError::ai(e))?;

    // Save assistant message
    {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;
        let now2 = chrono::Utc::now().to_rfc3339();
        chat_repo::save_message(&conn, &persona_id, &session_id, "assistant", &full_reply, &now2)?;
    }

    // Background compaction trigger
    {
        let pid = persona_id.clone();
        let sid = session_id.clone();
        tokio::spawn(async move {
            match compaction::compact_session(&pid, &sid).await {
                Ok(true) => tracing::info!("Background compaction completed for session {}", sid),
                Ok(false) => {}
                Err(e) => tracing::warn!("Background compaction failed for session {}: {}", sid, e),
            }
        });
    }

    Ok(full_reply)
}

#[tracing::instrument(err)]
pub fn get_chat_history(
    persona_id: &str,
    session_id: &str,
    limit: i32,
) -> Result<Vec<ChatMessage>, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    chat_repo::get_history(&conn, persona_id, session_id, limit)
}

#[tracing::instrument(err)]
pub fn list_sessions(persona_id: &str) -> Result<Vec<SessionSummary>, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    session_repo::list_sessions(&conn, persona_id)
}

#[tracing::instrument(err)]
pub fn new_session(persona_id: &str) -> Result<String, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    // Verify persona exists
    conn.query_row(
        "SELECT id FROM personas WHERE id = ?1",
        [persona_id],
        |_| Ok(()),
    )
    .map_err(|_| AppError::not_found(format!("Persona {} not found", persona_id)))?;
    Ok(uuid::Uuid::new_v4().to_string())
}

#[tracing::instrument(err)]
pub fn delete_session(persona_id: &str, session_id: &str) -> Result<(), AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    chat_repo::delete_session(&conn, persona_id, session_id)?;
    session_repo::delete_by_session(&conn, session_id)?;
    Ok(())
}
