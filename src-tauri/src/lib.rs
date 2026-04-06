mod commands;
mod core;
mod parsers;

use tracing_subscriber::{fmt, EnvFilter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Structured logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("memora=debug")),
        )
        .init();

    tracing::info!("Memora starting...");

    // Initialize database on startup
    if let Err(e) = core::storage::initialize_db() {
        tracing::error!("Failed to initialize database: {}", e);
    }

    // Start background monitors
    core::clipboard::start_watcher();

    // Start WebSocket bridge for Chrome extension
    // We need a tokio runtime for the async WS server.
    // Tauri runs its own runtime, so we spawn it from a separate thread.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime for WS bridge");
        rt.block_on(async {
            core::ws_bridge::start_ws_server(core::ws_bridge::DEFAULT_WS_PORT);
            // Keep the runtime alive
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            // ── Settings ──
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::validate_api_key,
            // ── Parser & OCR ──
            commands::parser::detect_and_parse,
            commands::parser::parse_pasted_text,
            commands::ocr::capture_and_ocr,
            // ── Persona ──
            commands::persona::list_personas,
            commands::persona::get_persona,
            commands::persona::delete_persona,
            commands::persona::get_persona_versions,
            commands::persona::rollback_persona,
            commands::persona::update_persona_field,
            // ── Generator ──
            commands::generator::generate_persona,
            // ── Chat ──
            commands::chat::send_message,
            commands::chat::get_chat_history,
            commands::chat::list_chat_sessions,
            commands::chat::new_chat_session,
            commands::chat::delete_chat_session,
            // ── Correction ──
            commands::correction::submit_correction,
            // ── Bridge (Chrome Extension) ──
            commands::bridge::start_ws_bridge,
            commands::bridge::get_ws_bridge_port,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Memora");
}
