//! Chat commands — thin wrappers delegating to services::chat_service.

use crate::error::AppError;
use crate::models::{ChatMessage, SessionSummary};
use crate::services::chat_service;

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    persona_id: String,
    session_id: String,
    content: String,
) -> Result<String, AppError> {
    chat_service::send_message(app, persona_id, session_id, content).await
}

#[tauri::command]
pub async fn get_chat_history(
    persona_id: String,
    session_id: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<ChatMessage>, AppError> {
    tokio::task::spawn_blocking(move || {
        let limit = limit.unwrap_or(100);
        if let Some(sid) = session_id {
            chat_service::get_chat_history(&persona_id, &sid, limit)
        } else {
            Ok(Vec::new())
        }
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
}

#[tauri::command]
pub async fn list_chat_sessions(persona_id: String) -> Result<Vec<SessionSummary>, AppError> {
    tokio::task::spawn_blocking(move || chat_service::list_sessions(&persona_id))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
}

#[tauri::command]
pub async fn new_chat_session(persona_id: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || chat_service::new_session(&persona_id))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
}

#[tauri::command]
pub async fn delete_chat_session(persona_id: String, session_id: String) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || chat_service::delete_session(&persona_id, &session_id))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
}
