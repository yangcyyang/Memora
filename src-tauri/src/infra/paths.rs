use anyhow::{Context, Result};
use std::path::PathBuf;

/// Base data directory for Memora: ~/.memora/
pub fn data_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".memora")
}

/// SQLite database path
pub fn db_path() -> PathBuf {
    data_dir().join("memora.db")
}

/// Settings file (contains encrypted API key)
pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

/// TTS settings file (contains encrypted TTS API keys)
pub fn tts_settings_path() -> PathBuf {
    data_dir().join("tts_settings.json")
}

/// Audio cache directory for TTS output
pub fn audio_cache_dir() -> PathBuf {
    data_dir().join("audio")
}

/// Vector index directory for semantic memory retrieval
pub fn vectors_dir() -> PathBuf {
    data_dir().join("vectors")
}

/// Path for a persona's persisted vector index
pub fn vector_index_path(persona_id: &str) -> PathBuf {
    vectors_dir().join(format!("{persona_id}.bin"))
}

/// Heartbeat state file to avoid duplicate proactive notifications
pub fn heartbeat_state_path() -> PathBuf {
    data_dir().join("heartbeat_state.json")
}

/// Ensure data directory exists
pub fn ensure_dirs() -> Result<()> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).context("Failed to create Memora data directory")?;
    std::fs::create_dir_all(audio_cache_dir()).context("Failed to create audio cache directory")?;
    std::fs::create_dir_all(vectors_dir()).context("Failed to create vector index directory")?;
    Ok(())
}
