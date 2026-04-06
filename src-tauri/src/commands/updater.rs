//! App update commands

use std::sync::Mutex;
use serde::Serialize;
use tauri::Manager;
use tracing::info;

// ── State ──────────────────────────────────────────────────────────────

/// Holds the pending `Update` object between check → download → install steps.
pub struct PendingUpdate {
    inner: Mutex<Option<tauri_plugin_updater::Update>>,
}

impl PendingUpdate {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }
}

// ── Response types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub version: Option<String>,
    pub date: Option<String>,
    pub body: Option<String>,
}

// ── Commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_app_update(app: tauri::AppHandle) -> Result<UpdateCheckResult, String> {
    use tauri_plugin_updater::UpdaterExt;

    info!(target: "updater", "checking for update");

    let updater = app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("failed to build updater: {e}"))?;

    let update = updater
        .check()
        .await
        .map_err(|e| format!("update check failed: {e}"))?;

    match update {
        Some(update) => {
            let result = UpdateCheckResult {
                available: true,
                version: Some(update.version.clone()),
                date: update.date.map(|d| d.to_string()),
                body: update.body.clone(),
            };

            let pending = app.state::<PendingUpdate>();
            if let Ok(mut slot) = pending.inner.lock() {
                *slot = Some(update);
            }

            info!(target: "updater", "update available: v{}", result.version.as_deref().unwrap_or("?"));
            Ok(result)
        }
        None => {
            info!(target: "updater", "already up to date");
            Ok(UpdateCheckResult {
                available: false,
                version: None,
                date: None,
                body: None,
            })
        }
    }
}

#[tauri::command]
pub async fn download_and_install_update(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;

    let update = {
        let pending = app.state::<PendingUpdate>();
        let mut slot = pending
            .inner
            .lock()
            .map_err(|e| format!("lock error: {e}"))?;
        slot.take().ok_or("no pending update to download")?
    };

    let app_for_events = app.clone();

    update
        .download_and_install(
            move |chunk_length, content_length| {
                let _ = app_for_events.emit(
                    "updater://download-progress",
                    serde_json::json!({
                        "chunk_length": chunk_length,
                        "content_length": content_length,
                    }),
                );
            },
            || {},
        )
        .await
        .map_err(|e| format!("download_and_install failed: {e}"))?;

    info!(target: "updater", "update downloaded and installed, ready for restart");
    Ok(())
}

#[tauri::command]
pub async fn restart_after_update(app: tauri::AppHandle) -> Result<(), String> {
    app.restart();
}
