//! Streaming chat completion (SSE).

use anyhow::{Context, Result};
use serde::Serialize;

use super::config::{AiConfig, ApiFormat};
use crate::infra::http_client::get_http_client;

#[derive(Serialize)]
struct ChatMsg {
    role: String,
    content: String,
}

/// Streaming chat completion for real-time chat
pub async fn chat_completion_stream(
    config: &AiConfig,
    system_prompt: &str,
    messages: Vec<(String, String)>, // (role, content) pairs
    app: &tauri::AppHandle,
    request_id: &str,
) -> Result<String> {
    let client = get_http_client()?;

    let mut chat_messages: Vec<ChatMsg> = Vec::new();

    match config.api_format {
        ApiFormat::Anthropic => {
            for (role, content) in &messages {
                chat_messages.push(ChatMsg {
                    role: role.clone(),
                    content: content.clone(),
                });
            }

            let base = config.base_url.trim_end_matches('/');
            let url = if base.ends_with("/v1") {
                format!("{}/messages", base)
            } else {
                format!("{}/v1/messages", base)
            };

            #[derive(Serialize)]
            struct StreamReq {
                model: String,
                system: String,
                messages: Vec<ChatMsg>,
                max_tokens: u32,
                temperature: f32,
                stream: bool,
            }

            let body = StreamReq {
                model: config.model.clone(),
                system: system_prompt.to_string(),
                messages: chat_messages,
                max_tokens: 4096,
                temperature: 0.8,
                stream: true,
            };

            let resp = client
                .post(&url)
                .header("x-api-key", &config.api_key)
                .header("authorization", format!("Bearer {}", config.api_key))
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .context("Anthropic stream request failed")?;

            stream_sse_response(resp, app, request_id).await
        }
        ApiFormat::Openai | ApiFormat::Local => {
            chat_messages.push(ChatMsg {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            });
            for (role, content) in &messages {
                chat_messages.push(ChatMsg {
                    role: role.clone(),
                    content: content.clone(),
                });
            }

            let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

            #[derive(Serialize)]
            struct StreamReq {
                model: String,
                messages: Vec<ChatMsg>,
                temperature: f32,
                max_tokens: u32,
                stream: bool,
            }

            let body = StreamReq {
                model: config.model.clone(),
                messages: chat_messages,
                temperature: 0.8,
                max_tokens: 4096,
                stream: true,
            };

            let mut req = client
                .post(&url)
                .header("content-type", "application/json");
            if !config.api_key.is_empty() {
                req = req.header("authorization", format!("Bearer {}", config.api_key));
            }

            let resp = req
                .json(&body)
                .send()
                .await
                .context("OpenAI stream request failed")?;

            stream_sse_response(resp, app, request_id).await
        }
    }
}

/// Parse SSE stream and emit deltas to frontend
async fn stream_sse_response(
    resp: reqwest::Response,
    app: &tauri::AppHandle,
    request_id: &str,
) -> Result<String> {
    use futures::StreamExt;
    use tauri::Emitter;

    let mut full_text = String::new();
    let mut stream = resp.bytes_stream();

    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Stream chunk error")?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    continue;
                }

                // Try to extract delta text
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    let delta_text = json
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|c| c.as_str())
                        .or_else(|| {
                            // Anthropic format
                            json.get("delta")
                                .and_then(|d| d.get("text"))
                                .and_then(|t| t.as_str())
                        });

                    if let Some(text) = delta_text {
                        full_text.push_str(text);

                        #[derive(Serialize, Clone)]
                        struct StreamPayload {
                            delta: String,
                            request_id: String,
                        }

                        app.emit(
                            "chat://stream",
                            StreamPayload {
                                delta: text.to_string(),
                                request_id: request_id.to_string(),
                            },
                        )
                        .ok();
                    }
                }
            }
        }
    }

    if full_text.is_empty() {
        anyhow::bail!("AI returned empty stream response");
    }

    Ok(full_text)
}
