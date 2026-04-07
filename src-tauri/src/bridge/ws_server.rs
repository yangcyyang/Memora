//! WebSocket bridge server for Chrome extension integration.

use crate::infra::db::memora_pool;
use crate::parsers;
use crate::repo::persona_repo;
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tracing::{error, info, warn};

pub const DEFAULT_WS_PORT: u16 = 17394;

static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Deserialize)]
struct IncomingMessage {
    action: String,
    persona_id: Option<String>,
    text: Option<String>,
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
        Self { ok: true, error: None, message_count: Some(count) }
    }
    fn error(msg: impl Into<String>) -> Self {
        Self { ok: false, error: Some(msg.into()), message_count: None }
    }
}

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
    let listener = TcpListener::bind(&addr).await
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
            Err(e) => { warn!("WS accept error: {}", e); }
        }
    }
}

async fn handle_connection(stream: tokio::net::TcpStream) -> Result<()> {
    let ws_stream = accept_async(stream).await.context("WebSocket handshake failed")?;
    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = msg.context("Failed to read WS message")?;
        if msg.is_text() {
            let text = msg.into_text().unwrap_or_default();
            let response = process_message(&text).await;
            let json = serde_json::to_string(&response).unwrap_or_default();
            write.send(tokio_tungstenite::tungstenite::Message::Text(json.into())).await?;
        } else if msg.is_close() {
            break;
        }
    }
    Ok(())
}

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

    let parsed = match parsers::parse_pasted_text(&text) {
        Ok(p) => p,
        Err(e) => return WsResponse::error(format!("Parse error: {}", e)),
    };

    let count = parsed.message_count;
    if count == 0 {
        return WsResponse::error("No messages extracted from text");
    }

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

fn stash_to_persona(persona_id: Option<&str>, content: &str) -> Result<()> {
    let pool = memora_pool();
    let conn = pool.get().context("DB pool exhausted")?;

    let pid = if let Some(id) = persona_id {
        if !persona_repo::persona_exists(&conn, id)? {
            anyhow::bail!("Persona '{}' not found", id);
        }
        id.to_string()
    } else {
        persona_repo::find_most_recent(&conn)?
            .ok_or_else(|| anyhow::anyhow!("No persona exists yet. Create a persona first."))?
    };

    let now = chrono::Utc::now().to_rfc3339();
    let append_md = format!(
        "\n\n### Chrome 插件推送的语料 ({})\n```text\n{}\n```\n",
        now, content
    );

    persona_repo::append_memories(&conn, &pid, &append_md, &now)?;
    info!("Chrome extension pushed corpus -> Persona: {}", pid);
    Ok(())
}
