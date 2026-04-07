//! LRU audio cache for TTS output.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;
use tracing::{debug, info};

use crate::infra::paths;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub file_count: u64,
    pub total_size_mb: f64,
}

pub struct AudioCache {
    cache_dir: PathBuf,
    limit_bytes: u64,
}

impl AudioCache {
    pub fn new(limit_mb: u64) -> Self {
        Self {
            cache_dir: paths::audio_cache_dir(),
            limit_bytes: limit_mb * 1024 * 1024,
        }
    }

    fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.cache_dir).context("Failed to create audio cache dir")?;
        Ok(())
    }

    pub fn cache_key(provider: &str, voice_id: &str, text: &str, language: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provider.as_bytes());
        hasher.update(b"|");
        hasher.update(voice_id.as_bytes());
        hasher.update(b"|");
        hasher.update(text.as_bytes());
        hasher.update(b"|");
        hasher.update(language.as_bytes());
        let hash = hasher.finalize();
        let mut hex = String::with_capacity(hash.len() * 2);
        for byte in hash {
            write!(hex, "{:02x}", byte).unwrap();
        }
        hex
    }

    pub fn get(&self, key: &str) -> Option<PathBuf> {
        let path = self.cache_dir.join(format!("{}.mp3", key));
        if path.exists() {
            let _ = filetime::set_file_mtime(&path, filetime::FileTime::now());
            debug!("Cache hit: {}", key);
            Some(path)
        } else {
            None
        }
    }

    pub fn put(&self, key: &str, data: &[u8]) -> Result<PathBuf> {
        self.ensure_dir()?;
        let path = self.cache_dir.join(format!("{}.mp3", key));
        std::fs::write(&path, data).context("Failed to write audio cache file")?;
        debug!("Cache put: {} ({} bytes)", key, data.len());
        if let Err(e) = self.evict() {
            tracing::warn!("Cache eviction error: {}", e);
        }
        Ok(path)
    }

    pub fn stats(&self) -> Result<CacheStats> {
        self.ensure_dir()?;
        let mut count = 0u64;
        let mut total = 0u64;
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("mp3") {
                count += 1;
                total += entry.metadata()?.len();
            }
        }
        Ok(CacheStats {
            file_count: count,
            total_size_mb: total as f64 / (1024.0 * 1024.0),
        })
    }

    pub fn clear(&self) -> Result<()> {
        self.ensure_dir()?;
        let mut removed = 0u64;
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("mp3") {
                std::fs::remove_file(entry.path())?;
                removed += 1;
            }
        }
        info!("Cleared {} cached audio files", removed);
        Ok(())
    }

    fn evict(&self) -> Result<()> {
        let mut entries: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
        let mut total_size = 0u64;

        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("mp3") { continue; }
            let meta = entry.metadata()?;
            let size = meta.len();
            let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            total_size += size;
            entries.push((path, size, mtime));
        }

        if total_size <= self.limit_bytes { return Ok(()); }

        entries.sort_by(|a, b| a.2.cmp(&b.2));
        let mut freed = 0u64;
        let target = total_size - self.limit_bytes;
        for (path, size, _) in &entries {
            if freed >= target { break; }
            std::fs::remove_file(path)?;
            freed += size;
            debug!("Evicted cache file: {:?} ({} bytes)", path, size);
        }

        info!("Cache eviction: freed {} bytes, {} -> {} bytes", freed, total_size, total_size - freed);
        Ok(())
    }
}
