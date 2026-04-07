//! API key validation.

use anyhow::Result;
use tracing::warn;

use super::completion::chat_completion;
use super::config::AiConfig;

/// Validate API key by sending a minimal request
pub async fn validate_key(config: &AiConfig) -> Result<bool> {
    let test_config = AiConfig {
        enabled: true,
        ..config.clone()
    };

    match chat_completion(&test_config, "You are a test.", "Say hi", 5).await {
        Ok(_) => Ok(true),
        Err(e) => {
            let msg = format!("{}", e);
            warn!("API key validation failed: {}", msg);
            Err(anyhow::anyhow!("{}", msg))
        }
    }
}
