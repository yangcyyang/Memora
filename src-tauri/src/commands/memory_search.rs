use crate::error::AppError;
use crate::services::memory_search_service;

#[tauri::command]
#[tracing::instrument(err)]
pub async fn index_memories(persona_id: String) -> Result<usize, AppError> {
    memory_search_service::index_memories(&persona_id).await
}

#[tauri::command]
#[tracing::instrument(err)]
pub async fn search_memories(
    persona_id: String,
    query: String,
    k: u32,
) -> Result<Vec<String>, AppError> {
    memory_search_service::search_memories_texts(&persona_id, &query, k as usize).await
}
