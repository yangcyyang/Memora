//! TTS provider trait, configuration, and factory.

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::infra::crypto::{decrypt_api_key, encrypt_api_key};
use crate::infra::paths;
use super::minimax::MiniMaxProvider;

// ── Trait ────────────────────────────────────────────────────────────

#[async_trait]
pub trait TtsProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn supported_languages(&self) -> Vec<(&'static str, &'static str)>;
    fn supports_voice_clone(&self) -> bool;
    fn supports_streaming(&self) -> bool;
    async fn upload_audio(&self, file_path: &str) -> Result<String>;
    async fn clone_voice(&self, req: CloneRequest) -> Result<CloneResult>;
    async fn synthesize(&self, req: SynthesizeRequest) -> Result<Vec<u8>>;
    async fn synthesize_stream(
        &self,
        req: SynthesizeRequest,
        tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> Result<()>;
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRequest {
    pub file_id: String,
    pub custom_voice_id: String,
    pub sample_text: String,
    pub model: Option<String>,
    pub prompt_file_id: Option<String>,
    pub prompt_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneResult {
    pub voice_id: String,
    pub demo_audio: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeRequest {
    pub text: String,
    pub voice_id: String,
    pub language: String,
    pub speed: f32,
    pub model: Option<String>,
}

// ── Provider info (returned to frontend) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub supports_clone: bool,
    pub supports_streaming: bool,
    pub languages: Vec<LanguageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
}

// ── Configuration ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    #[serde(default = "default_provider")]
    pub active_provider: String,
    #[serde(default = "default_cache_limit")]
    pub cache_limit_mb: u64,
    #[serde(default)]
    pub providers: HashMap<String, TtsProviderConfig>,
}

fn default_provider() -> String {
    "minimax".to_string()
}
fn default_cache_limit() -> u64 {
    500
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            active_provider: default_provider(),
            cache_limit_mb: default_cache_limit(),
            providers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TtsProviderConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub group_id: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub default_language: String,
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

// ── Config Load / Save ──────────────────────────────────────────────

pub fn load_tts_config() -> TtsConfig {
    let path = paths::tts_settings_path();
    if !path.exists() {
        return TtsConfig::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<TtsConfig>(&content) {
            Ok(mut config) => {
                // Fix corrupted config: remove empty-key provider entries
                config.providers.remove("");
                // Ensure active_provider is never empty
                if config.active_provider.is_empty() {
                    warn!("TTS active_provider was empty, defaulting to minimax");
                    config.active_provider = default_provider();
                }
                for p in config.providers.values_mut() {
                    p.api_key = decrypt_api_key(&p.api_key);
                }
                config
            }
            Err(err) => {
                warn!("Failed to parse TTS config: {}", err);
                TtsConfig::default()
            }
        },
        Err(err) => {
            warn!("Failed to read TTS config: {}", err);
            TtsConfig::default()
        }
    }
}

pub fn save_tts_config(config: &TtsConfig) -> Result<()> {
    let path = paths::tts_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create TTS config directory")?;
    }
    let mut to_save = config.clone();
    for p in to_save.providers.values_mut() {
        p.api_key = encrypt_api_key(&p.api_key);
    }
    let content = serde_json::to_string_pretty(&to_save).context("Failed to serialize TTS config")?;
    std::fs::write(&path, content).context("Failed to write TTS config")?;
    info!("TTS config saved");
    Ok(())
}

// ── Factory ─────────────────────────────────────────────────────────

pub fn get_provider(id: &str, config: &TtsProviderConfig) -> Result<Box<dyn TtsProvider>> {
    match id {
        "minimax" => Ok(Box::new(MiniMaxProvider::new(config)?)),
        _ => Err(anyhow!("Unknown TTS provider: {}", id)),
    }
}

pub fn get_active_provider() -> Result<Box<dyn TtsProvider>> {
    let config = load_tts_config();
    let provider_config = config
        .providers
        .get(&config.active_provider)
        .cloned()
        .unwrap_or_default();
    get_provider(&config.active_provider, &provider_config)
}

pub fn list_providers() -> Vec<ProviderInfo> {
    vec![ProviderInfo {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        supports_clone: true,
        supports_streaming: true,
        languages: vec![
            LanguageInfo { code: "zh-CN".into(), name: "普通话".into() },
            LanguageInfo { code: "yue".into(), name: "粤语".into() },
            LanguageInfo { code: "en".into(), name: "English".into() },
            LanguageInfo { code: "ja".into(), name: "日本語".into() },
            LanguageInfo { code: "ko".into(), name: "한국어".into() },
        ],
    }]
}
