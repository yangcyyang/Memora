use crate::core::ai_provider;
use crate::core::compaction;
use crate::core::db_pool::memora_pool;
use crate::core::models::{ChatMessage, SessionSummary};
use crate::core::prompts;
use anyhow::Context;

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    persona_id: String,
    session_id: String,
    content: String,
) -> Result<String, String> {
    send_message_inner(app, persona_id, session_id, content)
        .await
        .map_err(|e| e.to_string())
}

async fn send_message_inner(
    app: tauri::AppHandle,
    persona_id: String,
    session_id: String,
    content: String,
) -> anyhow::Result<String> {
    let config = ai_provider::load_config();
    let now = chrono::Utc::now().to_rfc3339();

    // All DB work in a blocking closure to avoid holding r2d2 connections across .await
    let (system_prompt, history) = {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;

        // Save user message
        conn.execute(
            "INSERT INTO chat_messages (persona_id, session_id, role, content, created_at) VALUES (?1, ?2, 'user', ?3, ?4)",
            rusqlite::params![persona_id, session_id, content, now],
        )?;

        // Load persona
        let (name, persona_md, memories_md): (String, String, String) = conn.query_row(
            "SELECT name, persona_md, memories_md FROM personas WHERE id = ?1",
            [&persona_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).context("Persona not found")?;

        // ── Pull session summary (compaction) ───────────────────────
        let session_summary: String = conn
            .query_row(
                "SELECT summary_md FROM session_summaries WHERE session_id = ?1",
                [&session_id],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let summary_block = if session_summary.is_empty() {
            "\n".to_string()
        } else {
            format!(
                "\n## 故事前情提要\n{}\n\n",
                session_summary
            )
        };

        // Build system prompt (now includes session summary)
        let sys = prompts::render(prompts::SYSTEM_CHAT, &[
            ("name", &name),
            ("persona_md", &persona_md),
            ("memories_md", &memories_md),
            ("session_summary", &summary_block),
        ]);

        // ── Dynamic recent history window ───────────────────────────
        // If we have a compaction summary, use a smaller window (12).
        // Otherwise keep the full 50 messages for first-session experience.
        let recent_limit: i32 = if session_summary.is_empty() { 50 } else { 12 };

        let mut stmt = conn.prepare(
            "SELECT role, content FROM chat_messages WHERE persona_id = ?1 AND session_id = ?2 ORDER BY id DESC LIMIT ?3"
        )?;
        let mut hist: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![persona_id, session_id, recent_limit], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        hist.reverse();

        (sys, hist)
        // conn is dropped here before any .await
    };

    // Stream completion (async — no DB connection held)
    let request_id = uuid::Uuid::new_v4().to_string();
    let full_reply = ai_provider::chat_completion_stream(
        &config,
        &system_prompt,
        history,
        &app,
        &request_id,
    )
    .await?;

    // Save assistant message (new connection)
    {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;
        let now2 = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO chat_messages (persona_id, session_id, role, content, created_at) VALUES (?1, ?2, 'assistant', ?3, ?4)",
            rusqlite::params![persona_id, session_id, full_reply, now2],
        )?;
    }

    // ── Background compaction trigger ───────────────────────────────
    // Fire-and-forget: check thresholds and compress if needed.
    // Runs on a separate tokio task so the user gets their reply immediately.
    {
        let pid = persona_id.clone();
        let sid = session_id.clone();
        tokio::spawn(async move {
            match compaction::compact_session(&pid, &sid).await {
                Ok(true) => tracing::info!("Background compaction completed for session {}", sid),
                Ok(false) => {} // threshold not met, normal
                Err(e) => tracing::warn!("Background compaction failed for session {}: {}", sid, e),
            }
        });
    }

    Ok(full_reply)
}

#[tauri::command]
pub async fn get_chat_history(
    persona_id: String,
    session_id: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<ChatMessage>, String> {
    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().map_err(|e| e.to_string())?;
        let limit = limit.unwrap_or(100);

        if let Some(sid) = session_id {
            let mut stmt = conn
                .prepare(
                    "SELECT id, persona_id, session_id, role, content, created_at FROM chat_messages WHERE persona_id = ?1 AND session_id = ?2 ORDER BY id DESC LIMIT ?3",
                )
                .map_err(|e| e.to_string())?;
            let mut msgs: Vec<ChatMessage> = stmt
                .query_map(rusqlite::params![persona_id, sid, limit], |row| {
                    Ok(ChatMessage {
                        id: row.get(0)?,
                        persona_id: row.get(1)?,
                        session_id: row.get(2)?,
                        role: row.get(3)?,
                        content: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            msgs.reverse();
            Ok(msgs)
        } else {
            Ok(Vec::new())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_chat_sessions(persona_id: String) -> Result<Vec<SessionSummary>, String> {
    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                r#"SELECT session_id, COUNT(*) as msg_count, MAX(created_at) as last_at,
                   (SELECT content FROM chat_messages cm2 WHERE cm2.persona_id = ?1 AND cm2.session_id = cm.session_id ORDER BY cm2.id DESC LIMIT 1) as preview
                   FROM chat_messages cm WHERE persona_id = ?1 GROUP BY session_id ORDER BY last_at DESC"#,
            )
            .map_err(|e| e.to_string())?;

        let sessions: Vec<SessionSummary> = stmt
            .query_map(rusqlite::params![persona_id], |row| {
                Ok(SessionSummary {
                    session_id: row.get(0)?,
                    message_count: row.get(1)?,
                    last_message_at: row.get(2)?,
                    preview: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn new_chat_session(persona_id: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id FROM personas WHERE id = ?1",
            [&persona_id],
            |_| Ok(()),
        )
        .map_err(|_| format!("Persona {} not found", persona_id))?;
        Ok(uuid::Uuid::new_v4().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_chat_session(persona_id: String, session_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().map_err(|e| e.to_string())?;
        // Delete chat messages
        conn.execute(
            "DELETE FROM chat_messages WHERE persona_id = ?1 AND session_id = ?2",
            rusqlite::params![persona_id, session_id],
        )
        .map_err(|e| e.to_string())?;
        // Clean up compaction summary for this session
        conn.execute(
            "DELETE FROM session_summaries WHERE session_id = ?1",
            rusqlite::params![session_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
