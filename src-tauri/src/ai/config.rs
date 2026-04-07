//! AI provider configuration management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::infra::crypto::{decrypt_api_key, encrypt_api_key};
use crate::infra::paths;

// ── Configuration ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiFormat {
    Openai,
    Anthropic,
    Local,
}

impl Default for ApiFormat {
    fn default() -> Self {
        Self::Openai
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    #[serde(default)]
    pub api_format: ApiFormat,
    pub base_url: String,
    pub api_key: String,
    pub model: String,

    #[serde(default)]
    pub providers: std::collections::HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_format: ApiFormat::default(),
            base_url: String::new(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
            providers: std::collections::HashMap::new(),
        }
    }
}

// ── Config Load/Save ────────────────────────────────────────────────

pub fn load_config() -> AiConfig {
    let path = paths::settings_path();
    if !path.exists() {
        return AiConfig::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<AiConfig>(&content) {
            Ok(mut config) => {
                config.api_key = decrypt_api_key(&config.api_key);
                for p in config.providers.values_mut() {
                    p.api_key = decrypt_api_key(&p.api_key);
                }
                config
            }
            Err(err) => {
                warn!("Failed to parse AI config: {}", err);
                AiConfig::default()
            }
        },
        Err(err) => {
            warn!("Failed to read AI config: {}", err);
            AiConfig::default()
        }
    }
}

pub fn save_config(config: &AiConfig) -> Result<()> {
    let path = paths::settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create config directory")?;
    }
    let mut config_to_save = config.clone();
    config_to_save.api_key = encrypt_api_key(&config_to_save.api_key);
    for p in config_to_save.providers.values_mut() {
        p.api_key = encrypt_api_key(&p.api_key);
    }

    let content =
        serde_json::to_string_pretty(&config_to_save).context("Failed to serialize config")?;
    std::fs::write(&path, content).context("Failed to write config")?;
    Ok(())
}
