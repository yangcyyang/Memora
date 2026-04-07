//! Shared HTTP client singleton.

use anyhow::{Context, Result};
use std::sync::{LazyLock, Mutex};

static SHARED_HTTP_CLIENT: LazyLock<Mutex<Option<reqwest::Client>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn get_http_client() -> Result<reqwest::Client> {
    let mut guard = SHARED_HTTP_CLIENT
        .lock()
        .map_err(|_| anyhow::anyhow!("HTTP client lock poisoned"))?;

    if let Some(client) = guard.as_ref() {
        return Ok(client.clone());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(10))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(4)
        .build()
        .context("Failed to build HTTP client")?;

    *guard = Some(client.clone());
    Ok(client)
}
