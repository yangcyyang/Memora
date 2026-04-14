//! Speech-to-text command using OpenAI Whisper API.
//! Model: whisper-1 ($0.006/min, no strict token limit, 25MB file max ~5min)

use crate::error::AppError;
use crate::ai::config::load_config;
use base64::{engine::general_purpose, Engine};
use reqwest::multipart;
use serde::Deserialize;
use tauri::AppHandle;

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

#[tauri::command]
pub async fn transcribe_audio(
    _app: AppHandle,
    audio_base64: String,
    mime_type: String,
) -> Result<String, AppError> {
    let config = load_config();
    if config.api_key.is_empty() {
        return Err(AppError::ai("OpenAI API key not configured"));
    }
    let api_key = config.api_key;

    let audio_bytes = general_purpose::STANDARD
        .decode(&audio_base64)
        .map_err(|e| AppError::ai(format!("Invalid base64 audio: {}", e)))?;

    // Extension from mime type: audio/webm -> webm
    let ext = mime_type
        .split('/')
        .nth(1)
        .unwrap_or("webm")
        .split(';')
        .next()
        .unwrap_or("webm");
    let filename = format!("audio.{}", ext);

    let part = multipart::Part::bytes(audio_bytes)
        .file_name(filename)
        .mime_str(&mime_type)
        .map_err(|e| anyhow::anyhow!(e))?;

    let form = multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1");

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(&api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(format!(
            "Whisper API error {}: {}",
            status, body
        )).into());
    }

    let result: WhisperResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    tracing::info!(
        "STT transcribe: {} bytes audio -> {} chars text",
        audio_base64.len(),
        result.text.len()
    );

    Ok(result.text)
}
