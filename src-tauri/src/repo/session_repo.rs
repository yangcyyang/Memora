//! Session summaries (compaction) table operations.

use crate::error::AppError;
use crate::models::SessionSummary;
use rusqlite::{Connection, params};

pub fn get_summary(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<(String, i64)>, AppError> {
    let result = conn.query_row(
        "SELECT summary_md, last_compressed_msg_id FROM session_summaries WHERE session_id = ?1",
        [session_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn upsert_summary(
    conn: &Connection,
    session_id: &str,
    persona_id: &str,
    summary_md: &str,
    last_compressed_msg_id: i64,
    token_estimate: i64,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO session_summaries (session_id, persona_id, summary_md, last_compressed_msg_id, token_estimate, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(session_id) DO UPDATE SET
            summary_md = excluded.summary_md,
            last_compressed_msg_id = excluded.last_compressed_msg_id,
            token_estimate = excluded.token_estimate,
            updated_at = excluded.updated_at",
        params![session_id, persona_id, summary_md, last_compressed_msg_id, token_estimate, now],
    )?;
    Ok(())
}

pub fn delete_by_session(conn: &Connection, session_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM session_summaries WHERE session_id = ?1",
        params![session_id],
    )?;
    Ok(())
}

pub fn list_sessions(
    conn: &Connection,
    persona_id: &str,
) -> Result<Vec<SessionSummary>, AppError> {
    let mut stmt = conn.prepare(
        r#"SELECT session_id, COUNT(*) as msg_count, MAX(created_at) as last_at,
           (SELECT content FROM chat_messages cm2 WHERE cm2.persona_id = ?1 AND cm2.session_id = cm.session_id ORDER BY cm2.id DESC LIMIT 1) as preview
           FROM chat_messages cm WHERE persona_id = ?1 GROUP BY session_id ORDER BY last_at DESC"#,
    )?;

    let sessions: Vec<SessionSummary> = stmt
        .query_map(params![persona_id], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                message_count: row.get(1)?,
                last_message_at: row.get(2)?,
                preview: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}
