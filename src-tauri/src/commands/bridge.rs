//! Bridge commands.

use crate::bridge::ws_server;

#[tauri::command]
pub async fn start_ws_bridge(port: Option<u16>) -> Result<(), String> {
    let port = port.unwrap_or(ws_server::DEFAULT_WS_PORT);
    ws_server::start_ws_server(port);
    Ok(())
}

#[tauri::command]
pub async fn get_ws_bridge_port() -> Result<u16, String> {
    Ok(ws_server::DEFAULT_WS_PORT)
}

#[tauri::command]
pub fn toggle_clipboard_watcher(enabled: bool) {
    crate::bridge::clipboard::WATCHER_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}
