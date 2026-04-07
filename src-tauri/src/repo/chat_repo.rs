//! Chat messages table operations.

use crate::error::AppError;
use crate::models::ChatMessage;
use rusqlite::{Connection, params};

pub fn save_message(
    conn: &Connection,
    persona_id: &str,
    session_id: &str,
    role: &str,
    content: &str,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO chat_messages (persona_id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![persona_id, session_id, role, content, now],
    )?;
    Ok(())
}

pub fn recent_messages(
    conn: &Connection,
    persona_id: &str,
    session_id: &str,
    limit: i32,
) -> Result<Vec<(String, String)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT role, content FROM chat_messages WHERE persona_id = ?1 AND session_id = ?2 ORDER BY id DESC LIMIT ?3",
    )?;
    let mut hist: Vec<(String, String)> = stmt
        .query_map(params![persona_id, session_id, limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();
    hist.reverse();
    Ok(hist)
}

pub fn get_history(
    conn: &Connection,
    persona_id: &str,
    session_id: &str,
    limit: i32,
) -> Result<Vec<ChatMessage>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, persona_id, session_id, role, content, created_at FROM chat_messages WHERE persona_id = ?1 AND session_id = ?2 ORDER BY id DESC LIMIT ?3",
    )?;
    let mut msgs: Vec<ChatMessage> = stmt
        .query_map(params![persona_id, session_id, limit], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                persona_id: row.get(1)?,
                session_id: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    msgs.reverse();
    Ok(msgs)
}

pub fn delete_session(
    conn: &Connection,
    persona_id: &str,
    session_id: &str,
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM chat_messages WHERE persona_id = ?1 AND session_id = ?2",
        params![persona_id, session_id],
    )?;
    Ok(())
}

/// Get uncompressed messages (id > last_compressed_msg_id).
pub fn uncompressed_messages(
    conn: &Connection,
    persona_id: &str,
    session_id: &str,
    last_id: i64,
) -> Result<Vec<(i64, String, String)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, role, content FROM chat_messages \
         WHERE persona_id = ?1 AND session_id = ?2 AND id > ?3 \
         ORDER BY id ASC",
    )?;

    let rows: Vec<(i64, String, String)> = stmt
        .query_map(params![persona_id, session_id, last_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}
