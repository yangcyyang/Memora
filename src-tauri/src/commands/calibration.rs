//! Calibration commands.

use crate::error::AppError;
use crate::models::{CalibrationApplyResult, CalibrationFeedbackItem, CalibrationSample};
use crate::services::calibration_service;

#[tauri::command]
pub async fn generate_calibration_samples(
    persona_id: String,
) -> Result<Vec<CalibrationSample>, AppError> {
    calibration_service::generate_calibration_samples(persona_id).await
}

#[tauri::command]
pub async fn submit_calibration_feedback(
    persona_id: String,
    feedback_items: Vec<CalibrationFeedbackItem>,
    free_text: Option<String>,
) -> Result<CalibrationApplyResult, AppError> {
    calibration_service::submit_calibration_feedback(persona_id, feedback_items, free_text).await
}
