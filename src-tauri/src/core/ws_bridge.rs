// WebSocket bridge for Chrome extension integration.
//
// Starts a lightweight WebSocket server on a local TCP port (default 17394)
// that accepts incoming messages from the Memora Chrome extension.
//
// Protocol:
//   1. Client connects via ws://127.0.0.1:17394
//   2. Client sends JSON messages of shape:
//      { "action": "push_chat", "persona_id": "...", "messages": [...] }
//      or simpler form:
//      { "action": "push_text", "text": "...", "persona_id": "..." }
//   3. Server responds with { "ok": true } or { "ok": false, "error": "..." }

use crate::core::db_pool::memora_pool;
use crate::parsers;
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tracing::{error, info, warn};

pub const DEFAULT_WS_PORT: u16 = 17394;

static RUNNING: AtomicBool = AtomicBool::new(false);

// ── Wire Protocol ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct IncomingMessage {
    action: String,
    /// Target persona ID. If empty, use the most recently active persona.
    persona_id: Option<String>,
    /// For "push_text" action
    text: Option<String>,
    /// For "push_chat" action (pre-parsed messages)
    messages: Option<Vec<WireRawMessage>>,
}

#[derive(Debug, Deserialize)]
struct WireRawMessage {
    sender: String,
    content: String,
    timestamp: Option<String>,
    is_from_me: bool,
}

#[derive(Debug, Serialize)]
struct WsResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_count: Option<usize>,
}

impl WsResponse {
    fn success(count: usize) -> Self {
        Self {
            ok: true,
            error: None,
            message_count: Some(count),
        }
    }

    fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
            message_count: None,
        }
    }
}

// ── Server Lifecycle ────────────────────────────────────────────────

/// Start the WebSocket bridge server in the background.
/// Safe to call multiple times — only the first call actually spawns.
pub fn start_ws_server(port: u16) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        info!("WebSocket bridge already running");
        return;
    }

    tokio::spawn(async move {
        if let Err(e) = run_server(port).await {
            error!("WebSocket bridge error: {}", e);
            RUNNING.store(false, Ordering::SeqCst);
        }
    });
}

async fn run_server(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .context(format!("Cannot bind WebSocket on {}", addr))?;

    info!("WebSocket bridge listening on ws://{}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                info!("WS connection from {}", peer);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream).await {
                        warn!("WS connection handler error: {}", e);
                    }
                });
            }
            Err(e) => {
                warn!("WS accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(stream: tokio::net::TcpStream) -> Result<()> {
    let ws_stream = accept_async(stream)
        .await
        .context("WebSocket handshake failed")?;

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = msg.context("Failed to read WS message")?;

        if msg.is_text() {
            let text = msg.into_text().unwrap_or_default();
            let response = process_message(&text).await;
            let json = serde_json::to_string(&response).unwrap_or_default();
            write
                .send(tokio_tungstenite::tungstenite::Message::Text(json.into()))
                .await?;
        } else if msg.is_close() {
            break;
        }
    }

    Ok(())
}

// ── Message Processing ──────────────────────────────────────────────

async fn process_message(raw: &str) -> WsResponse {
    let incoming: IncomingMessage = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => return WsResponse::error(format!("Invalid JSON: {}", e)),
    };

    match incoming.action.as_str() {
        "push_text" => handle_push_text(incoming).await,
        "push_chat" => handle_push_chat(incoming).await,
        "ping" => WsResponse::success(0),
        other => WsResponse::error(format!("Unknown action: {}", other)),
    }
}

async fn handle_push_text(msg: IncomingMessage) -> WsResponse {
    let text = match msg.text {
        Some(t) if !t.trim().is_empty() => t,
        _ => return WsResponse::error("Missing or empty 'text' field"),
    };

    // Parse the text through existing wechat/generic parser
    let parsed = match parsers::parse_pasted_text(&text) {
        Ok(p) => p,
        Err(e) => return WsResponse::error(format!("Parse error: {}", e)),
    };

    let count = parsed.message_count;
    if count == 0 {
        return WsResponse::error("No messages extracted from text");
    }

    // Stash into the target persona
    match stash_to_persona(msg.persona_id.as_deref(), &text) {
        Ok(_) => WsResponse::success(count),
        Err(e) => WsResponse::error(format!("Database error: {}", e)),
    }
}

async fn handle_push_chat(msg: IncomingMessage) -> WsResponse {
    let wire_messages = match msg.messages {
        Some(m) if !m.is_empty() => m,
        _ => return WsResponse::error("Missing or empty 'messages' array"),
    };

    let count = wire_messages.len();

    // Format messages into appendable markdown
    let mut md = String::new();
    for m in &wire_messages {
        let ts = m.timestamp.as_deref().unwrap_or("?");
        md.push_str(&format!("{} {}\n{}\n\n", ts, m.sender, m.content));
    }

    match stash_to_persona(msg.persona_id.as_deref(), &md) {
        Ok(_) => WsResponse::success(count),
        Err(e) => WsResponse::error(format!("Database error: {}", e)),
    }
}

/// Append raw corpus text to a persona's memories_md.
fn stash_to_persona(persona_id: Option<&str>, content: &str) -> Result<()> {
    let pool = memora_pool();
    let conn = pool.get().context("DB pool exhausted")?;

    let pid = if let Some(id) = persona_id {
        // Verify it exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM personas WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            anyhow::bail!("Persona '{}' not found", id);
        }
        id.to_string()
    } else {
        // Use the most recently active persona
        conn.query_row(
            r#"SELECT p.id FROM personas p
               LEFT JOIN chat_messages cm ON p.id = cm.persona_id
               ORDER BY COALESCE(cm.created_at, p.updated_at) DESC LIMIT 1"#,
            [],
            |row| row.get(0),
        )
        .context("No persona exists yet. Create a persona first.")?
    };

    let now = chrono::Utc::now().to_rfc3339();
    let append_md = format!(
        "\n\n### Chrome 插件推送的语料 ({})\n```text\n{}\n```\n",
        now, content
    );

    let current_memories: String = conn.query_row(
        "SELECT memories_md FROM personas WHERE id = ?1",
        rusqlite::params![pid],
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE personas SET memories_md = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![current_memories + &append_md, now, pid],
    )?;

    info!("Chrome extension pushed corpus -> Persona: {}", pid);
    Ok(())
}
