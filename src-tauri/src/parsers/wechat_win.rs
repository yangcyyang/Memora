// Windows WeChat local database parser
//
// Implements two strategies for accessing the WeChat database on Windows:
//   1. **Unencrypted path** — Some WeChat versions or configurations store
//      MicroMsg.db (or MSG*.db) in plain SQLite format.
//   2. **Encrypted path (sqlcipher-compatible)** — The database is encrypted
//      with AES-256-CBC + HMAC-SHA1.  The key is derived from the user's
//      "wxid" + a device-bound salt extracted from the Windows registry or
//      from the WeChat process's memory.  Because extracting the key from
//      a live process requires debug privileges and is highly version-
//      dependent, we expose a Tauri command that lets the user supply the
//      hex key directly (many community tools can dump it).
//
// References:
// - community repo "wechat-dump-rs" / "WeChatMsg" for DB schema details
// - WeChat desktop stores its data under:
//     %USERPROFILE%\Documents\WeChat Files\<wxid>\Msg\Multi\MSG<N>.db
//   or for older versions:
//     %APPDATA%\Tencent\WeChat\<wxid>\Msg\MicroMsg.db

use crate::core::models::{ParsedContent, RawMessage};
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

// ── Public API ──────────────────────────────────────────────────────

/// Locate all candidate WeChat message databases on a Windows system.
/// Returns a list of (db_path, wxid) pairs.
pub fn find_wechat_dbs_windows() -> Result<Vec<(PathBuf, String)>> {
    let mut results = Vec::new();

    // Primary location: Documents\WeChat Files\<wxid>\Msg\Multi\MSG*.db
    if let Some(home) = dirs::home_dir() {
        let wechat_root = home.join("Documents").join("WeChat Files");
        if wechat_root.is_dir() {
            scan_wechat_root(&wechat_root, &mut results)?;
        }

        // Fallback: some corporate installs use custom paths
        let appdata_root = home
            .join("AppData")
            .join("Roaming")
            .join("Tencent")
            .join("WeChat");
        if appdata_root.is_dir() {
            scan_wechat_root(&appdata_root, &mut results)?;
        }
    }

    Ok(results)
}

/// Try opening a database (possibly encrypted).
/// If `hex_key` is `None`, we attempt an unencrypted read.
/// For encrypted databases the user must supply the 64-character
/// hex key obtained from external tooling.
pub fn parse_wechat_windows_db(db_path: &Path, hex_key: Option<&str>) -> Result<ParsedContent> {
    if let Some(_key) = hex_key {
        // Encrypted path — requires sqlcipher.  We attempt to open
        // with pre-configured PRAGMA settings matching WeChat's scheme.
        // NOTE: rusqlite's bundled SQLite does NOT include sqlcipher.
        // A real production build would need the `sqlcipher` feature or
        // a custom build.  For now we return a clear error message so
        // the user knows what's needed.
        anyhow::bail!(
            "加密数据库需要 sqlcipher 支持。请先使用社区工具（如 WeChatMsg、wechat-dump-rs）\
             导出解密后的数据库，再拖入 Memora 解析。"
        );
    }

    // Unencrypted / pre-decrypted path
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("无法打开微信数据库，可能需要先解密")?;

    // Detect which schema variant we have
    let tables = list_tables(&conn)?;
    let mut all_messages = Vec::new();

    if tables.iter().any(|t| t.starts_with("MSG")) {
        // Newer multi-DB format: MSG0, MSG1, ...
        for table in tables.iter().filter(|t| t.starts_with("MSG")) {
            if let Ok(msgs) = extract_msg_table(&conn, table) {
                all_messages.extend(msgs);
            }
        }
    } else if tables.contains(&"message".to_string()) {
        // Alternate newer schema (MicroMsg.db merged messages)
        all_messages = extract_message_table(&conn)?;
    } else if tables.iter().any(|t| t.starts_with("Chat_")) {
        // Legacy schema (same as iOS backup)
        for table in tables.iter().filter(|t| t.starts_with("Chat_")) {
            if let Ok(msgs) = extract_chat_table(&conn, table) {
                all_messages.extend(msgs);
            }
        }
    } else {
        anyhow::bail!(
            "无法识别数据库 schema。已知的表: {:?}",
            tables
        );
    }

    all_messages.sort_by_key(|m| m.timestamp.clone().unwrap_or_default());
    let count = all_messages.len();

    Ok(ParsedContent {
        source: "wechat_windows".to_string(),
        target_name: None,
        messages: all_messages,
        message_count: count,
    })
}

// ── Internal Helpers ────────────────────────────────────────────────

fn scan_wechat_root(root: &Path, results: &mut Vec<(PathBuf, String)>) -> Result<()> {
    let entries = std::fs::read_dir(root).context("读取 WeChat 目录失败")?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let wxid = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip non-wxid dirs
        if wxid.is_empty() || wxid.starts_with('.') || wxid == "All Users" {
            continue;
        }

        // Newer layout: Msg/Multi/MSG0.db, MSG1.db, ...
        let multi_dir = path.join("Msg").join("Multi");
        if multi_dir.is_dir() {
            for db_entry in std::fs::read_dir(&multi_dir).into_iter().flatten().flatten() {
                let db_path = db_entry.path();
                if db_path.extension().map(|e| e == "db").unwrap_or(false) {
                    results.push((db_path, wxid.clone()));
                }
            }
        }

        // Older layout: Msg/MicroMsg.db
        let micro_db = path.join("Msg").join("MicroMsg.db");
        if micro_db.exists() {
            results.push((micro_db, wxid.clone()));
        }
    }
    Ok(())
}

fn list_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
    let rows: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Extract from the newer MSG* table schema.
/// Typical columns: localId, TalkerId, MsgSvrID, Type, SubType,
///   IsSender, CreateTime, Sequence, StrTalker, StrContent, ...
fn extract_msg_table(conn: &Connection, table: &str) -> Result<Vec<RawMessage>> {
    let sql = format!(
        "SELECT StrContent, IsSender, StrTalker, CreateTime FROM \"{}\" WHERE Type = 1",
        table
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        let is_sender: i32 = row.get(1)?;
        let talker: String = row.get(2)?;
        let create_time: i64 = row.get(3)?;

        let timestamp = chrono::DateTime::from_timestamp(create_time, 0)
            .map(|dt| dt.to_rfc3339());

        Ok(RawMessage {
            sender: if is_sender == 1 {
                "我".to_string()
            } else {
                talker
            },
            content,
            timestamp,
            is_from_me: is_sender == 1,
        })
    })?;

    let mut messages = Vec::new();
    for r in rows.filter_map(Result::ok) {
        if !r.content.trim().is_empty() {
            messages.push(r);
        }
    }
    Ok(messages)
}

/// Extract from the older `message` table.
/// Columns: msgId, msgSvrId, type, isSend, createTime, talkerName, content, ...
fn extract_message_table(conn: &Connection) -> Result<Vec<RawMessage>> {
    let sql = "SELECT content, isSend, talkerName, createTime FROM message WHERE type = 1";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        let is_send: i32 = row.get(1)?;
        let talker: String = row.get(2)?;
        let create_time: i64 = row.get(3)?;

        let timestamp = chrono::DateTime::from_timestamp(create_time, 0)
            .map(|dt| dt.to_rfc3339());

        Ok(RawMessage {
            sender: if is_send == 1 {
                "我".to_string()
            } else {
                talker
            },
            content,
            timestamp,
            is_from_me: is_send == 1,
        })
    })?;

    let mut messages = Vec::new();
    for r in rows.filter_map(Result::ok) {
        if !r.content.trim().is_empty() {
            messages.push(r);
        }
    }
    Ok(messages)
}

/// Extract from legacy `Chat_*` tables (same schema as iOS backup).
fn extract_chat_table(conn: &Connection, table: &str) -> Result<Vec<RawMessage>> {
    let counter_part = table.replace("Chat_", "");
    let sql = format!(
        "SELECT Message, CreateTime, Des FROM \"{}\" WHERE Type = 1",
        table
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        let text: String = row.get(0)?;
        let timestamp: i64 = row.get(1)?;
        let des: i32 = row.get(2)?;

        let time_str = chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|dt| dt.to_rfc3339());

        Ok(RawMessage {
            sender: if des == 1 {
                counter_part.clone()
            } else {
                "我".to_string()
            },
            content: text,
            timestamp: time_str,
            is_from_me: des == 0,
        })
    })?;

    let mut messages = Vec::new();
    for r in rows.filter_map(Result::ok) {
        if !r.content.trim().is_empty() {
            messages.push(r);
        }
    }
    Ok(messages)
}
