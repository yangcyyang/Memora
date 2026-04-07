//! Persona voice binding table operations.

use crate::error::AppError;
use rusqlite::{Connection, params};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VoiceBinding {
    pub persona_id: String,
    pub provider: String,
    pub voice_id: String,
    pub language: String,
    pub model: String,
}

pub fn get_voice(
    conn: &Connection,
    persona_id: &str,
) -> Result<Option<VoiceBinding>, AppError> {
    let result = conn.query_row(
        "SELECT persona_id, provider, voice_id, language, model FROM persona_voices WHERE persona_id = ?1",
        params![persona_id],
        |row| {
            Ok(VoiceBinding {
                persona_id: row.get(0)?,
                provider: row.get(1)?,
                voice_id: row.get(2)?,
                language: row.get(3)?,
                model: row.get(4)?,
            })
        },
    );

    match result {
        Ok(voice) => Ok(Some(voice)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_voice_triple(
    conn: &Connection,
    persona_id: &str,
) -> Result<Option<(String, String, String)>, AppError> {
    let result = conn.query_row(
        "SELECT provider, voice_id, language FROM persona_voices WHERE persona_id = ?1",
        params![persona_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    );

    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn set_voice(
    conn: &Connection,
    persona_id: &str,
    provider: &str,
    voice_id: &str,
    language: &str,
    model: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO persona_voices (persona_id, provider, voice_id, language, model, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
        params![persona_id, provider, voice_id, language, model],
    )?;
    Ok(())
}

pub fn remove_voice(conn: &Connection, persona_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM persona_voices WHERE persona_id = ?1",
        params![persona_id],
    )?;
    Ok(())
}
