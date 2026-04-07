//! MiniMax TTS provider implementation.

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, info};

use super::provider::{
    CloneRequest, CloneResult, SynthesizeRequest, TtsProvider, TtsProviderConfig,
};

pub struct MiniMaxProvider {
    api_key: String,
    group_id: String,
    base_url: String,
    default_model: String,
}

impl MiniMaxProvider {
    pub fn new(config: &TtsProviderConfig) -> Result<Self> {
        let base_url = if config.base_url.is_empty() {
            "https://api.minimaxi.com".to_string()
        } else {
            config.base_url.trim_end_matches('/').to_string()
        };
        let default_model = if config.default_model.is_empty() {
            "speech-02-hd".to_string()
        } else {
            config.default_model.clone()
        };
        Ok(Self {
            api_key: config.api_key.clone(),
            group_id: config.group_id.clone(),
            base_url,
            default_model,
        })
    }

    fn client(&self) -> Result<Client> {
        Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to build HTTP client")
    }

    /// Build a URL with the required GroupId query parameter.
    fn url(&self, path: &str) -> String {
        if self.group_id.is_empty() {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}{}?GroupId={}", self.base_url, path, self.group_id)
        }
    }
}

#[async_trait]
impl TtsProvider for MiniMaxProvider {
    fn id(&self) -> &'static str { "minimax" }
    fn display_name(&self) -> &'static str { "MiniMax" }

    fn supported_languages(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("zh-CN", "普通话"), ("yue", "粤语"), ("en", "English"),
            ("ja", "日本語"), ("ko", "한국어"),
        ]
    }

    fn supports_voice_clone(&self) -> bool { true }
    fn supports_streaming(&self) -> bool { true }

    async fn upload_audio(&self, file_path: &str) -> Result<String> {
        let client = self.client()?;
        let url = self.url("/v1/files/upload");

        if self.api_key.is_empty() {
            anyhow::bail!("MiniMax API 密钥为空，请在设置 → 语音服务中配置");
        }
        debug!("MiniMax upload_audio: url={}", url);

        let file_bytes = tokio::fs::read(file_path).await.context("Failed to read audio file")?;
        let file_name = std::path::Path::new(file_path)
            .file_name().and_then(|n| n.to_str()).unwrap_or("audio.mp3").to_string();

        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name).mime_str("audio/mpeg")?;
        let form = reqwest::multipart::Form::new()
            .text("purpose", "voice_clone").part("file", part);

        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form).send().await.context("MiniMax upload request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await.context("Failed to parse upload response")?;
        if !status.is_success() { anyhow::bail!("MiniMax upload error ({}): {}", status, body); }
        check_base_resp(&body)?;

        let file_id_val = body.get("file").and_then(|f| f.get("file_id"))
            .ok_or_else(|| anyhow!("Missing file_id in upload response: {}", body))?;
        let file_id = match file_id_val {
            serde_json::Value::String(s) => s.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => anyhow::bail!("Invalid file_id type in upload response: {}", body),
        };

        info!("MiniMax upload success, file_id={}", file_id);
        Ok(file_id)
    }

    async fn clone_voice(&self, req: CloneRequest) -> Result<CloneResult> {
        let client = self.client()?;
        let url = self.url("/v1/voice_clone");
        let model = req.model.unwrap_or_else(|| self.default_model.clone());

        // MiniMax strongly types `file_id` as an integer. Since we pass it internally as a String,
        // we must parse it back into a number here if possible to avoid "invalid params" API errors.
        // If it's unexpectedly not a number, we just fallback to the string.
        let file_id_payload: serde_json::Value = req.file_id.parse::<i64>()
            .map(|n| json!(n)).unwrap_or_else(|_| json!(req.file_id));

        let mut payload = json!({
            "file_id": file_id_payload, 
            "voice_id": req.custom_voice_id,
            "text": req.sample_text, 
            "model": model,
        });

        if let (Some(prompt_fid), Some(prompt_txt)) = (&req.prompt_file_id, &req.prompt_text) {
            payload["clone_prompt"] = json!({ "prompt_audio": prompt_fid, "prompt_text": prompt_txt });
        }

        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload).send().await.context("MiniMax clone request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await.context("Failed to parse clone response")?;
        if !status.is_success() { anyhow::bail!("MiniMax clone error ({}): {}", status, body); }
        check_base_resp(&body)?;

        let demo_audio = body.get("data").and_then(|d| d.get("audio")).and_then(|a| a.as_str())
            .and_then(|hex_str| hex_decode(hex_str).ok());

        info!("MiniMax clone success, voice_id={}", req.custom_voice_id);
        Ok(CloneResult { voice_id: req.custom_voice_id, demo_audio })
    }

    async fn synthesize(&self, req: SynthesizeRequest) -> Result<Vec<u8>> {
        let client = self.client()?;
        let url = self.url("/v1/t2a_v2");
        let model = req.model.unwrap_or_else(|| self.default_model.clone());

        let payload = json!({
            "model": model, "text": req.text, "stream": false,
            "voice_setting": { "voice_id": req.voice_id, "speed": req.speed, "vol": 1, "pitch": 0 },
            "audio_setting": { "sample_rate": 32000, "bitrate": 128000, "format": "mp3", "channel": 1 },
            "language_boost": req.language,
        });

        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload).send().await.context("MiniMax TTS request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await.context("Failed to parse TTS response")?;
        if !status.is_success() { anyhow::bail!("MiniMax TTS error ({}): {}", status, body); }
        check_base_resp(&body)?;

        let audio_hex = body.get("data").and_then(|d| d.get("audio")).and_then(|a| a.as_str())
            .ok_or_else(|| anyhow!("Missing audio data in TTS response"))?;

        let audio_bytes = hex_decode(audio_hex)
            .map_err(|e| anyhow!("Failed to decode hex audio: {}", e))?;

        debug!("MiniMax TTS synthesized {} bytes", audio_bytes.len());
        Ok(audio_bytes)
    }

    async fn synthesize_stream(&self, req: SynthesizeRequest, tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
        use futures::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::http::Request;
        use tokio_tungstenite::tungstenite::Message;

        let model = req.model.unwrap_or_else(|| self.default_model.clone());

        let ws_base = self.base_url.replace("https://", "wss://").replace("http://", "ws://");
        let ws_url = if self.group_id.is_empty() {
            format!("{}/ws/v1/t2a_v2", ws_base)
        } else {
            format!("{}/ws/v1/t2a_v2?GroupId={}", ws_base, self.group_id)
        };

        let ws_host = self.base_url.trim_start_matches("https://").trim_start_matches("http://").to_string();
        let ws_request = Request::builder()
            .uri(&ws_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Host", &ws_host)
            .header("Connection", "Upgrade").header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
            .body(()).context("Failed to build WS request")?;

        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_request).await
            .context("MiniMax WebSocket connection failed")?;
        let (mut write, mut read) = ws_stream.split();

        // Wait for connected_success
        if let Some(Ok(msg)) = read.next().await {
            let text = msg.to_text().unwrap_or("");
            let val: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
            if val.get("event").and_then(|e| e.as_str()) != Some("connected_success") {
                anyhow::bail!("MiniMax WS connection failed: {}", text);
            }
            debug!("MiniMax WS connected");
        }

        let start_msg = json!({
            "event": "task_start", "model": model,
            "voice_setting": { "voice_id": req.voice_id, "speed": req.speed, "vol": 1, "pitch": 0 },
            "audio_setting": { "sample_rate": 32000, "bitrate": 128000, "format": "mp3", "channel": 1 },
            "language_boost": req.language,
        });
        write.send(Message::text(start_msg.to_string())).await.context("Failed to send task_start")?;

        if let Some(Ok(msg)) = read.next().await {
            let text = msg.to_text().unwrap_or("");
            let val: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
            if val.get("event").and_then(|e| e.as_str()) != Some("task_started") {
                anyhow::bail!("MiniMax WS task_start failed: {}", text);
            }
            debug!("MiniMax WS task started");
        }

        let continue_msg = json!({ "event": "task_continue", "text": req.text });
        write.send(Message::text(continue_msg.to_string())).await.context("Failed to send task_continue")?;

        while let Some(Ok(msg)) = read.next().await {
            let text = msg.to_text().unwrap_or("");
            if text.is_empty() { continue; }
            let val: serde_json::Value = match serde_json::from_str(text) { Ok(v) => v, Err(_) => continue };

            if let Some(audio_hex) = val.get("data").and_then(|d| d.get("audio")).and_then(|a| a.as_str()) {
                if !audio_hex.is_empty() {
                    if let Ok(audio_bytes) = hex_decode(audio_hex) {
                        if tx.send(audio_bytes).await.is_err() {
                            debug!("Audio channel closed, stopping stream");
                            break;
                        }
                    }
                }
            }

            if val.get("is_final").and_then(|f| f.as_bool()) == Some(true) {
                debug!("MiniMax WS stream complete");
                break;
            }
        }

        let finish_msg = json!({"event": "task_finish"});
        write.send(Message::text(finish_msg.to_string())).await.ok();
        Ok(())
    }
}

/// Check MiniMax JSON `base_resp` for API-level errors (they return HTTP 200 even on auth failure).
fn check_base_resp(body: &serde_json::Value) -> Result<()> {
    if let Some(base) = body.get("base_resp") {
        let code = base.get("status_code").and_then(|c| c.as_i64()).unwrap_or(0);
        if code != 0 {
            let msg = base.get("status_msg").and_then(|m| m.as_str()).unwrap_or("unknown error");
            anyhow::bail!("MiniMax API error (code {}): {}", code, msg);
        }
    }
    Ok(())
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 { return Err("Hex string has odd length".to_string()); }
    (0..hex.len()).step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("Invalid hex at {}: {}", i, e)))
        .collect()
}
