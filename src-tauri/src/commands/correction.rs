//! Correction commands.

use crate::error::AppError;
use crate::models::CorrectionResult;
use crate::services::correction_service;

#[tauri::command]
pub async fn submit_correction(
    _app: tauri::AppHandle,
    persona_id: String,
    original: String,
    correction: String,
) -> Result<CorrectionResult, AppError> {
    correction_service::submit_correction(persona_id, original, correction).await
}
