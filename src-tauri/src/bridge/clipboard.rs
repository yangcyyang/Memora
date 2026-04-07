//! Clipboard watcher — silently captures chat-like content.

use arboard::Clipboard;
use lazy_static::lazy_static;
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tracing::{debug, info};

pub static WATCHER_ENABLED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref CHAT_PATTERN_1: Regex = Regex::new(r"(?m)^.{1,15}:\s+.+").unwrap();
    static ref CHAT_PATTERN_2: Regex = Regex::new(r"(?m)^\d{4}-\d{2}-\d{2}\s+\d{1,2}:\d{2}\s+.+").unwrap();
}

pub fn start_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut cb = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to initiate clipboard watcher: {}", e);
                return;
            }
        };

        let mut last_content = String::new();

        loop {
            std::thread::sleep(Duration::from_millis(2000));

            if !WATCHER_ENABLED.load(Ordering::Relaxed) {
                continue;
            }

            if let Ok(text) = cb.get_text() {
                if text == last_content || text.len() > 10000 || text.trim().is_empty() {
                    continue;
                }
                last_content = text.clone();

                let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
                if lines.len() < 2 {
                    continue;
                }

                if CHAT_PATTERN_1.is_match(&text) || CHAT_PATTERN_2.is_match(&text) {
                    debug!("Detected chat-like content in clipboard");
                    #[derive(serde::Serialize, Clone)]
                    struct ClipboardPayload {
                        text: String,
                    }
                    if let Err(e) = app.emit("clipboard://chat-detected", ClipboardPayload { text: text.to_string() }) {
                        tracing::error!("Failed to emit clipboard event: {}", e);
                    }
                }
            }
        }
    });
}
