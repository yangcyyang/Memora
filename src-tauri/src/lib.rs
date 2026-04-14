mod ai;
mod bridge;
mod commands;
mod error;
mod infra;
mod models;
mod parsers;
mod prompts;
mod repo;
mod services;
mod tts;

use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::fmt::writer::MakeWriterExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Determine log directory
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".memora")
        .join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    // Set up rolling file appender
    let file_appender = tracing_appender::rolling::daily(&log_dir, "memora.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Structured logging to stdout + file
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("memora=debug")),
        )
        .with_writer(std::io::stdout.and(non_blocking))
        .init();

    tracing::info!("Memora starting...");

    // Initialize database on startup
    if let Err(e) = infra::db::initialize_db() {
        tracing::error!("Failed to initialize database: {}", e);
    }

    // Background monitors initialized in setup

    // Start WebSocket bridge for Chrome extension
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime for WS bridge");
        rt.block_on(async {
            bridge::ws_server::start_ws_server(bridge::ws_server::DEFAULT_WS_PORT);
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    });

    tauri::Builder::default()
        .setup(|app| {
            bridge::clipboard::start_watcher(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::updater::PendingUpdate::new())
        .invoke_handler(tauri::generate_handler![
            // ── Updater ──
            commands::updater::check_app_update,
            commands::updater::download_and_install_update,
            commands::updater::restart_after_update,
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
            commands::persona::append_clipboard_corpus,
            // ── Generator ──
            commands::generator::generate_persona,
            // ── Calibration ──
            commands::calibration::generate_calibration_samples,
            commands::calibration::submit_calibration_feedback,
            // ── Chat ──
            commands::chat::send_message,
            commands::chat::get_chat_history,
            commands::chat::list_chat_sessions,
            commands::chat::new_chat_session,
            commands::chat::delete_chat_session,
            // ── Correction ──
            commands::correction::submit_correction,
            commands::correction::reinforce_memory,
            // ── Bridge (Chrome Extension) ──
            commands::bridge::start_ws_bridge,
            commands::bridge::get_ws_bridge_port,
            commands::bridge::toggle_clipboard_watcher,
            // ── TTS (Voice) ──
            commands::tts::get_tts_settings,
            commands::tts::save_tts_settings,
            commands::tts::list_tts_providers,
            commands::tts::upload_and_clone_voice,
            commands::tts::get_persona_voice,
            commands::tts::set_persona_voice,
            commands::tts::remove_persona_voice,
            commands::tts::speak_text,
            commands::tts::speak_text_stream,
            commands::tts::check_ffmpeg,
            commands::tts::get_cache_stats,
            commands::tts::clear_audio_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Memora");
}
