use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use super::ai_provider;
use super::db_pool::memora_pool;
use super::prompts;

// ── Thresholds ──────────────────────────────────────────────────────

/// Message count threshold (loose — avoids triggering on short bursts)
const MSG_THRESHOLD: i64 = 40;

/// Token estimate threshold (~20-25% of a typical context window)
const TOKEN_THRESHOLD: usize = 8000;

// ── Token Estimation ────────────────────────────────────────────────

/// Rough token estimate for a text string.
/// Chinese: ~1.5 tokens/char, ASCII: ~0.3 tokens/char.
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk: usize = 0;
    let mut ascii: usize = 0;
    for ch in text.chars() {
        if ch.is_ascii() {
            ascii += 1;
        } else {
            cjk += 1;
        }
    }
    (cjk as f64 * 1.5 + ascii as f64 * 0.3).ceil() as usize
}

// ── Trigger Logic ───────────────────────────────────────────────────

/// Determines whether a compaction should fire for this session.
pub fn should_compact(uncompressed_msg_count: i64, uncompressed_token_estimate: usize) -> bool {
    uncompressed_msg_count >= MSG_THRESHOLD || uncompressed_token_estimate >= TOKEN_THRESHOLD
}

// ── Quality Guard ───────────────────────────────────────────────────

fn validate_summary(summary: &str) -> bool {
    let char_count = summary.chars().count();
    // Too short → LLM phoned it in
    if char_count < 80 {
        warn!("Compaction summary too short ({} chars), discarding", char_count);
        return false;
    }
    // Too long → no real compression happened
    if char_count > 3000 {
        warn!("Compaction summary too long ({} chars), discarding", char_count);
        return false;
    }
    true
}

// ── Core Compact Logic ──────────────────────────────────────────────

/// Run a compaction cycle for a given session.
///
/// 1. Fetch old summary + uncompressed messages from DB.
/// 2. If threshold not met, return early.
/// 3. Call LLM to generate new summary.
/// 4. Validate and persist.
pub async fn compact_session(persona_id: &str, session_id: &str) -> Result<bool> {
    let persona_id = persona_id.to_string();
    let session_id = session_id.to_string();

    // ── Step 1: Gather data from DB (blocking) ──────────────────────
    let (old_summary, last_compressed_id, uncompressed_messages) = {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;

        // Existing summary (if any)
        let summary_row: Option<(String, i64)> = conn
            .query_row(
                "SELECT summary_md, last_compressed_msg_id FROM session_summaries WHERE session_id = ?1",
                [&session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        let (old_summary, last_id) = summary_row.unwrap_or_default();

        // Uncompressed messages (id > last_compressed_msg_id)
        let mut stmt = conn.prepare(
            "SELECT id, role, content FROM chat_messages \
             WHERE persona_id = ?1 AND session_id = ?2 AND id > ?3 \
             ORDER BY id ASC",
        )?;

        let rows: Vec<(i64, String, String)> = stmt
            .query_map(rusqlite::params![persona_id, session_id, last_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        (old_summary, last_id, rows)
    };

    // ── Step 2: Check thresholds ────────────────────────────────────
    let msg_count = uncompressed_messages.len() as i64;
    let token_est: usize = uncompressed_messages
        .iter()
        .map(|(_, _, content)| estimate_tokens(content))
        .sum();

    if !should_compact(msg_count, token_est) {
        debug!(
            "Compaction skipped for session {}: {} msgs, ~{} tokens",
            session_id, msg_count, token_est
        );
        return Ok(false);
    }

    info!(
        "Compacting session {} ({} msgs, ~{} tokens, last_id={})",
        session_id, msg_count, token_est, last_compressed_id
    );

    // ── Step 3: Build chat segment text ─────────────────────────────
    let mut chat_segment = String::new();
    for (_, role, content) in &uncompressed_messages {
        let label = if role == "user" { "用户" } else { "AI" };
        chat_segment.push_str(&format!("{}：{}\n", label, content));
    }

    let old_summary_display = if old_summary.is_empty() {
        "（这是第一次压缩，没有旧的总结）".to_string()
    } else {
        old_summary.clone()
    };

    let compactor_prompt = prompts::render(
        prompts::SESSION_COMPACTOR,
        &[
            ("old_summary", &old_summary_display),
            ("new_chat_segment", &chat_segment),
        ],
    );

    // ── Step 4: Call LLM ────────────────────────────────────────────
    let config = ai_provider::load_config();
    let new_summary = ai_provider::chat_completion(
        &config,
        "你是一个对话历史压缩助手。直接输出结构化前情提要。",
        &compactor_prompt,
        2048,
    )
    .await
    .context("Compaction LLM call failed")?;

    // ── Step 5: Validate ────────────────────────────────────────────
    if !validate_summary(&new_summary) {
        warn!("Compaction quality check failed, keeping old summary");
        return Ok(false);
    }

    let new_token_est = estimate_tokens(&new_summary) as i64;

    // ── Step 6: Persist to DB ───────────────────────────────────────
    let new_last_id = uncompressed_messages
        .last()
        .map(|(id, _, _)| *id)
        .unwrap_or(last_compressed_id);

    let now = chrono::Utc::now().to_rfc3339();
    {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;
        conn.execute(
            "INSERT INTO session_summaries (session_id, persona_id, summary_md, last_compressed_msg_id, token_estimate, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(session_id) DO UPDATE SET
                summary_md = excluded.summary_md,
                last_compressed_msg_id = excluded.last_compressed_msg_id,
                token_estimate = excluded.token_estimate,
                updated_at = excluded.updated_at",
            rusqlite::params![session_id, persona_id, new_summary, new_last_id, new_token_est, now],
        )?;
    }

    info!(
        "Compaction complete for session {} → {} chars, ~{} tokens, cursor={}",
        session_id,
        new_summary.chars().count(),
        new_token_est,
        new_last_id
    );

    Ok(true)
}
