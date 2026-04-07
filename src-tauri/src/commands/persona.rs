//! Persona commands — thin wrappers delegating to repo layer.

use crate::error::AppError;
use crate::infra::db::memora_pool;
use crate::models::{Persona, PersonaSummary, VersionSummary};
use crate::repo::persona_repo;

#[tauri::command]
pub async fn list_personas() -> Result<Vec<PersonaSummary>, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    persona_repo::list_all(&conn)
}

#[tauri::command]
#[tracing::instrument(err)]
pub async fn get_persona(id: String) -> Result<Persona, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    persona_repo::find_by_id(&conn, &id)
}

#[tauri::command]
#[tracing::instrument(err)]
pub async fn delete_persona(id: String) -> Result<(), AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    persona_repo::delete(&conn, &id)
}

#[tauri::command]
pub async fn get_persona_versions(id: String) -> Result<Vec<VersionSummary>, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    persona_repo::get_versions(&conn, &id)
}

#[tauri::command]
#[tracing::instrument(err)]
pub async fn rollback_persona(id: String, version: i32) -> Result<(), AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    let (persona_md, memories_md) = persona_repo::get_version_data(&conn, &id, version)?;
    let now = chrono::Utc::now().to_rfc3339();
    persona_repo::rollback(&conn, &id, version, &persona_md, &memories_md, &now)
}

#[tauri::command]
#[tracing::instrument(skip(value), err)]
pub async fn update_persona_field(id: String, field: String, value: String) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get()?;
        let now = chrono::Utc::now().to_rfc3339();
        persona_repo::update_field(&conn, &id, &field, &value, &now)
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
}

#[tauri::command]
#[tracing::instrument(skip(content), err)]
pub async fn append_clipboard_corpus(id: String, content: String) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get()?;
        let now = chrono::Utc::now().to_rfc3339();
        let append_md = format!("\n\n### 自动捕捉的语料 ({})\n```text\n{}\n```\n", now, content);
        crate::repo::persona_repo::append_memories(&conn, &id, &append_md, &now)
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
}
