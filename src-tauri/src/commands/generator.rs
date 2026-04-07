//! Generator commands.

use crate::error::AppError;
use crate::models::{BasicInfo, GenerateResult, ParsedContent};
use crate::services::generator_service;

#[tauri::command]
pub async fn generate_persona(
    app: tauri::AppHandle,
    basic_info: BasicInfo,
    parsed_contents: Vec<ParsedContent>,
) -> Result<GenerateResult, AppError> {
    generator_service::generate_persona(app, basic_info, parsed_contents).await
}
