use crate::core::ws_bridge;

/// Start the WebSocket bridge server for Chrome extension communication.
#[tauri::command]
pub async fn start_ws_bridge(port: Option<u16>) -> Result<(), String> {
    let port = port.unwrap_or(ws_bridge::DEFAULT_WS_PORT);
    ws_bridge::start_ws_server(port);
    Ok(())
}

/// Get the current WebSocket bridge port for display in the UI.
#[tauri::command]
pub async fn get_ws_bridge_port() -> Result<u16, String> {
    Ok(ws_bridge::DEFAULT_WS_PORT)
}
