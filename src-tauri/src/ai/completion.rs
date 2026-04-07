//! Non-streaming chat completion.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::config::{AiConfig, ApiFormat};
use crate::infra::http_client::get_http_client;

#[derive(Serialize)]
struct ChatMsg {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<ChatMsg>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    system: String,
    messages: Vec<ChatMsg>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: Option<String>,
    thinking: Option<String>,
}

/// Normalize Anthropic base URL: ensure it ends with /v1 so we can append /messages
fn anthropic_messages_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{}/messages", base)
    } else {
        format!("{}/v1/messages", base)
    }
}

/// Non-streaming chat completion for persona generation
pub async fn chat_completion(
    config: &AiConfig,
    system_prompt: &str,
    user_message: &str,
    max_tokens: u32,
) -> Result<String> {
    let client = get_http_client()?;

    match config.api_format {
        ApiFormat::Anthropic => {
            let url = anthropic_messages_url(&config.base_url);
            let body = AnthropicRequest {
                model: config.model.clone(),
                system: system_prompt.to_string(),
                messages: vec![ChatMsg {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                }],
                max_tokens,
                temperature: 0.7,
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
                .context("Anthropic API request failed")?;

            let status = resp.status();
            let text = resp.text().await.context("Failed to read Anthropic response")?;
            if !status.is_success() {
                anyhow::bail!("Anthropic API error ({}): {}", status, text);
            }

            let parsed: AnthropicResponse =
                serde_json::from_str(&text).context("Failed to parse Anthropic response")?;

            let mut result = None;
            let mut fallback = None;

            for c in parsed.content {
                if let Some(t) = c.text {
                    result = Some(t);
                    break;
                } else if let Some(th) = c.thinking {
                    if fallback.is_none() {
                        fallback = Some(th);
                    }
                }
            }

            result.or(fallback).ok_or_else(|| anyhow::anyhow!("Empty Anthropic response"))
        }
        ApiFormat::Openai | ApiFormat::Local => {
            let url = format!(
                "{}/chat/completions",
                config.base_url.trim_end_matches('/')
            );
            let body = OpenAiRequest {
                model: config.model.clone(),
                messages: vec![
                    ChatMsg {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    ChatMsg {
                        role: "user".to_string(),
                        content: user_message.to_string(),
                    },
                ],
                temperature: 0.7,
                max_tokens,
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
                .context("OpenAI API request failed")?;

            let status = resp.status();
            let text = resp.text().await.context("Failed to read OpenAI response")?;
            if !status.is_success() {
                anyhow::bail!("OpenAI API error ({}): {}", status, text);
            }

            let parsed: OpenAiResponse =
                serde_json::from_str(&text).context("Failed to parse OpenAI response")?;
            parsed
                .choices
                .first()
                .and_then(|c| c.message.content.clone())
                .ok_or_else(|| anyhow::anyhow!("Empty OpenAI response"))
        }
    }
}
