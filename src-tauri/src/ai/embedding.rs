use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::infra::http_client::get_http_client;

use super::config::{AiConfig, ApiFormat, ProviderConfig};

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_OLLAMA_EMBED_MODEL: &str = "bge-m3";

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError>;
}

pub struct OpenAiEmbedding {
    base_url: String,
    api_key: String,
    model: String,
}

pub struct OllamaEmbedding {
    base_url: String,
    model: String,
}

#[derive(Serialize)]
struct OpenAiEmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingItem>,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingItem {
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError> {
        if self.api_key.trim().is_empty() {
            return Err(AppError::ai("OpenAI embedding key not configured"));
        }

        let client = get_http_client().map_err(AppError::Internal)?;
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let response = client
            .post(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&OpenAiEmbeddingRequest {
                model: self.model.clone(),
                input: text.to_string(),
            })
            .send()
            .await
            .context("OpenAI embeddings request failed")
            .map_err(AppError::Internal)?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read OpenAI embeddings response")
            .map_err(AppError::Internal)?;
        if !status.is_success() {
            return Err(AppError::ai(format!(
                "OpenAI embeddings error ({}): {}",
                status, body
            )));
        }

        let parsed: OpenAiEmbeddingResponse =
            serde_json::from_str(&body).context("Failed to parse OpenAI embeddings response")
                .map_err(AppError::Internal)?;
        parsed
            .data
            .into_iter()
            .next()
            .map(|item| item.embedding)
            .ok_or_else(|| AppError::ai("OpenAI embeddings returned empty data"))
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let client = get_http_client().map_err(AppError::Internal)?;
        let base = self.base_url.trim_end_matches('/');
        let root = base.strip_suffix("/v1").unwrap_or(base);
        let url = format!("{root}/api/embeddings");

        let response = client
            .post(&url)
            .header("content-type", "application/json")
            .json(&OllamaEmbeddingRequest {
                model: self.model.clone(),
                prompt: text.to_string(),
            })
            .send()
            .await
            .context("Ollama embeddings request failed")
            .map_err(AppError::Internal)?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read Ollama embeddings response")
            .map_err(AppError::Internal)?;
        if !status.is_success() {
            return Err(AppError::ai(format!(
                "Ollama embeddings error ({}): {}",
                status, body
            )));
        }

        let parsed: OllamaEmbeddingResponse =
            serde_json::from_str(&body).context("Failed to parse Ollama embeddings response")
                .map_err(AppError::Internal)?;
        if parsed.embedding.is_empty() {
            return Err(AppError::ai("Ollama embeddings returned empty vector"));
        }
        Ok(parsed.embedding)
    }
}

pub fn get_embedding_provider(config: &AiConfig) -> Result<Box<dyn EmbeddingProvider>, AppError> {
    match resolve_embedding_mode(config) {
        EmbeddingMode::Ollama(provider) => Ok(Box::new(OllamaEmbedding {
            base_url: provider
                .base_url
                .trim()
                .to_string()
                .if_empty_then(DEFAULT_OLLAMA_BASE_URL.to_string()),
            model: DEFAULT_OLLAMA_EMBED_MODEL.to_string(),
        })),
        EmbeddingMode::OpenAi(provider) => {
            if provider.api_key.trim().is_empty() {
                return Err(AppError::ai("OpenAI embedding key not configured"));
            }

            Ok(Box::new(OpenAiEmbedding {
                base_url: provider
                    .base_url
                    .trim()
                    .to_string()
                    .if_empty_then(DEFAULT_OPENAI_BASE_URL.to_string()),
                api_key: provider.api_key,
                model: "text-embedding-3-small".to_string(),
            }))
        }
    }
}

enum EmbeddingMode {
    OpenAi(ProviderConfig),
    Ollama(ProviderConfig),
}

fn resolve_embedding_mode(config: &AiConfig) -> EmbeddingMode {
    if config.api_format == ApiFormat::Local {
        return EmbeddingMode::Ollama(resolve_local_provider(config));
    }

    if let Some(provider) = config.providers.get("local") {
        if !provider.base_url.trim().is_empty() || !provider.model.trim().is_empty() {
            return EmbeddingMode::Ollama(provider.clone());
        }
    }

    EmbeddingMode::OpenAi(resolve_openai_provider(config))
}

fn resolve_openai_provider(config: &AiConfig) -> ProviderConfig {
    if let Some(provider) = config.providers.get("openai") {
        return ProviderConfig {
            base_url: if provider.base_url.trim().is_empty() {
                DEFAULT_OPENAI_BASE_URL.to_string()
            } else {
                provider.base_url.clone()
            },
            api_key: provider.api_key.clone(),
            model: if provider.model.trim().is_empty() {
                "gpt-4o".to_string()
            } else {
                provider.model.clone()
            },
        };
    }

    ProviderConfig {
        base_url: if config.base_url.trim().is_empty() {
            DEFAULT_OPENAI_BASE_URL.to_string()
        } else {
            config.base_url.clone()
        },
        api_key: config.api_key.clone(),
        model: if config.model.trim().is_empty() {
            "gpt-4o".to_string()
        } else {
            config.model.clone()
        },
    }
}

fn resolve_local_provider(config: &AiConfig) -> ProviderConfig {
    if let Some(provider) = config.providers.get("local") {
        return ProviderConfig {
            base_url: if provider.base_url.trim().is_empty() {
                DEFAULT_OLLAMA_BASE_URL.to_string()
            } else {
                provider.base_url.clone()
            },
            api_key: provider.api_key.clone(),
            model: if provider.model.trim().is_empty() {
                DEFAULT_OLLAMA_EMBED_MODEL.to_string()
            } else {
                provider.model.clone()
            },
        };
    }

    ProviderConfig {
        base_url: if config.base_url.trim().is_empty() {
            DEFAULT_OLLAMA_BASE_URL.to_string()
        } else {
            config.base_url.clone()
        },
        api_key: config.api_key.clone(),
        model: if config.model.trim().is_empty() {
            DEFAULT_OLLAMA_EMBED_MODEL.to_string()
        } else {
            config.model.clone()
        },
    }
}

trait StringExt {
    fn if_empty_then(self, fallback: String) -> String;
}

impl StringExt for String {
    fn if_empty_then(self, fallback: String) -> String {
        if self.is_empty() { fallback } else { self }
    }
}
