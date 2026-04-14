//! Persona table operations.

use crate::error::AppError;
use crate::models::{Persona, PersonaSummary, VersionSummary};
use rusqlite::{Connection, params};

pub fn list_all(conn: &Connection) -> Result<Vec<PersonaSummary>, AppError> {
    let mut stmt = conn.prepare(
        r#"SELECT p.id, p.name, p.avatar_emoji, p.description, p.tags_json, p.version, p.created_at,
           (SELECT MAX(cm.created_at) FROM chat_messages cm WHERE cm.persona_id = p.id) as last_chat
           FROM personas p ORDER BY COALESCE(last_chat, p.created_at) DESC"#,
    )?;

    let personas: Vec<PersonaSummary> = stmt
        .query_map([], |row| {
            let tags_json: String = row.get(4)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(PersonaSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                avatar_emoji: row.get(2)?,
                description: row.get(3)?,
                tags,
                version: row.get(5)?,
                created_at: row.get(6)?,
                last_chat_at: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(personas)
}

pub fn find_by_id(conn: &Connection, id: &str) -> Result<Persona, AppError> {
    conn.query_row(
        "SELECT id, slug, name, avatar_emoji, description, tags_json, persona_md, memories_md, version, created_at, updated_at FROM personas WHERE id = ?1",
        [id],
        |row| {
            Ok(Persona {
                id: row.get(0)?,
                slug: row.get(1)?,
                name: row.get(2)?,
                avatar_emoji: row.get(3)?,
                description: row.get(4)?,
                tags_json: row.get(5)?,
                persona_md: row.get(6)?,
                memories_md: row.get(7)?,
                version: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )
    .map_err(|_| AppError::not_found(format!("Persona '{}' not found", id)))
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM personas WHERE id = ?1", [id])?;
    Ok(())
}

pub fn get_versions(conn: &Connection, persona_id: &str) -> Result<Vec<VersionSummary>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT version, created_at FROM persona_versions WHERE persona_id = ?1 ORDER BY version DESC",
    )?;

    let versions: Vec<VersionSummary> = stmt
        .query_map([persona_id], |row| {
            Ok(VersionSummary {
                version: row.get(0)?,
                created_at: row.get(1)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(versions)
}

pub fn get_version_data(
    conn: &Connection,
    persona_id: &str,
    version: i32,
) -> Result<(String, String), AppError> {
    conn.query_row(
        "SELECT persona_md, memories_md FROM persona_versions WHERE persona_id = ?1 AND version = ?2",
        params![persona_id, version],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|_| AppError::not_found(format!("Version {} not found", version)))
}

pub fn rollback(
    conn: &Connection,
    persona_id: &str,
    version: i32,
    persona_md: &str,
    memories_md: &str,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE personas SET persona_md = ?1, memories_md = ?2, version = ?3, updated_at = ?4 WHERE id = ?5",
        params![persona_md, memories_md, version, now, persona_id],
    )?;
    Ok(())
}

pub fn update_field(
    conn: &Connection,
    id: &str,
    field: &str,
    value: &str,
    now: &str,
) -> Result<(), AppError> {
    let column = match field {
        "avatar_emoji" => "avatar_emoji",
        "description" => "description",
        "tags_json" => "tags_json",
        "persona_md" => "persona_md",
        "memories_md" => "memories_md",
        _ => return Err(AppError::Internal(anyhow::anyhow!("Field '{}' is not editable", field))),
    };

    let sql = format!(
        "UPDATE personas SET {} = ?1, updated_at = ?2 WHERE id = ?3",
        column
    );
    conn.execute(&sql, params![value, now, id])?;
    Ok(())
}

pub fn insert(
    conn: &Connection,
    id: &str,
    slug: &str,
    name: &str,
    avatar_emoji: &str,
    description: &str,
    tags_json: &str,
    persona_md: &str,
    memories_md: &str,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        r#"INSERT INTO personas (id, slug, name, avatar_emoji, description, tags_json, persona_md, memories_md, version, created_at, updated_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?9)"#,
        params![id, slug, name, avatar_emoji, description, tags_json, persona_md, memories_md, now],
    )?;
    Ok(())
}

pub fn insert_version(
    conn: &Connection,
    persona_id: &str,
    version: i32,
    persona_md: &str,
    memories_md: &str,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        r#"INSERT INTO persona_versions (persona_id, version, persona_md, memories_md, created_at)
           VALUES (?1, ?2, ?3, ?4, ?5)"#,
        params![persona_id, version, persona_md, memories_md, now],
    )?;
    Ok(())
}

pub fn get_persona_data(
    conn: &Connection,
    id: &str,
) -> Result<(String, String, String, i32), AppError> {
    conn.query_row(
        "SELECT name, persona_md, memories_md, version FROM personas WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .map_err(|_| AppError::not_found(format!("Persona '{}' not found", id)))
}

pub fn update_persona_md(
    conn: &Connection,
    id: &str,
    persona_md: &str,
    version: i32,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE personas SET persona_md=?1, version=?2, updated_at=?3 WHERE id=?4",
        params![persona_md, version, now, id],
    )?;
    Ok(())
}

pub fn update_memories_md(
    conn: &Connection,
    id: &str,
    memories_md: &str,
    version: i32,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE personas SET memories_md=?1, version=?2, updated_at=?3 WHERE id=?4",
        params![memories_md, version, now, id],
    )?;
    Ok(())
}

pub fn append_memories(
    conn: &Connection,
    id: &str,
    append_md: &str,
    now: &str,
) -> Result<(), AppError> {
    let current: String = conn.query_row(
        "SELECT memories_md FROM personas WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE personas SET memories_md = ?1, updated_at = ?2 WHERE id = ?3",
        params![format!("{}{}", current, append_md), now, id],
    )?;
    Ok(())
}

/// Find the most recently active persona ID.
pub fn find_most_recent(conn: &Connection) -> Result<Option<String>, AppError> {
    let result = conn.query_row(
        r#"SELECT p.id FROM personas p
           LEFT JOIN chat_messages cm ON p.id = cm.persona_id
           ORDER BY COALESCE(cm.created_at, p.updated_at) DESC LIMIT 1"#,
        [],
        |row| row.get(0),
    );

    match result {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn insert_correction(
    conn: &Connection,
    persona_id: &str,
    target: &str,
    original: &str,
    correction: &str,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO corrections (persona_id,target,original,correction,applied_at) VALUES (?1,?2,?3,?4,?5)",
        params![persona_id, target, original, correction, now],
    )?;
    Ok(())
}

pub fn persona_exists(conn: &Connection, id: &str) -> Result<bool, AppError> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM personas WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Save proactive settings for a persona.
pub fn save_proactive_settings(
    conn: &Connection,
    id: &str,
    enabled: bool,
    rules_json: Option<&str>,
    now: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE personas SET proactive_enabled = ?1, proactive_rules = ?2, updated_at = ?3 WHERE id = ?4",
        params![if enabled { 1 } else { 0 }, rules_json, now, id],
    )?;
    Ok(())
}

/// Get proactive settings for a persona.
pub fn get_proactive_settings(
    conn: &Connection,
    id: &str,
) -> Result<(bool, Option<String>), AppError> {
    conn.query_row(
        "SELECT proactive_enabled, proactive_rules FROM personas WHERE id = ?1",
        [id],
        |row| {
            let enabled: i32 = row.get(0)?;
            let rules: Option<String> = row.get(1)?;
            Ok((enabled != 0, rules))
        },
    )
    .map_err(|_| AppError::not_found(format!("Persona '{}' not found", id)))
}
