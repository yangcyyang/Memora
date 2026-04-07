use crate::models::{ParsedContent, RawMessage};
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

/// Finds all WeChat MM.sqlite files inside an iOS backup.
pub fn find_wechat_dbs(backup_dir: &Path) -> Result<Vec<PathBuf>> {
    let manifest_path = backup_dir.join("Manifest.db");
    if !manifest_path.exists() {
        anyhow::bail!("Manifest.db not found in backup directory");
    }

    let conn = Connection::open_with_flags(&manifest_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("Failed to open Manifest.db")?;

    let mut stmt = conn.prepare(
        r#"
        SELECT fileID, relativePath
        FROM Files
        WHERE domain = 'AppDomain-com.tencent.xin'
          AND relativePath LIKE 'Documents/%/DB/MM.sqlite'
        "#
    )?;

    let mut db_paths = Vec::new();
    let rows = stmt.query_map([], |row| {
        let file_id: String = row.get(0)?;
        let _relative_path: String = row.get(1)?;
        // iOS backup files are stored in subfolders named after the first 2 characters of the fileID
        let folder = &file_id[0..2];
        let file_path = backup_dir.join(folder).join(&file_id);
        Ok(file_path)
    })?;

    for row in rows {
        if let Ok(path) = row {
            if path.exists() {
                db_paths.push(path);
            }
        }
    }

    Ok(db_paths)
}

/// Parses an unencrypted WeChat MM.sqlite database from an iOS backup
pub fn parse_ios_wechat_db(db_path: &Path) -> Result<ParsedContent> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("Failed to open WeChat iOS MM.sqlite")?;

    // WeChat iOS typical schemas:
    // Chat tables are often prefixed with Chat_...
    // To simplify for this parser, we dynamically search for all tables matching Chat_%
    // and extract the messages.
    
    let mut table_stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'Chat_%'")?;
    
    let table_names: Vec<String> = table_stmt.query_map([], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();

    let mut all_messages = Vec::new();

    for table in table_names {
        // Find contacts format: Chat_abcdef123456...
        let counter_part = table.replace("Chat_", "");
        
        let sql = format!("SELECT Message, CreateTime, Des FROM {} WHERE Type = 1", table);
        if let Ok(mut msg_stmt) = conn.prepare(&sql) {
            let rows = msg_stmt.query_map([], |row| {
                let text: String = row.get(0)?;
                let timestamp: i64 = row.get(1)?;
                let is_sender: i32 = row.get(2)?; // Des=0 vs Des=1 depends on the WeChat version, usually 0 is me, 1 is them.

                let time_str = chrono::DateTime::from_timestamp(timestamp, 0)
                    .map(|dt| dt.to_rfc3339());

                Ok(RawMessage {
                    sender: if is_sender == 1 { counter_part.clone() } else { "我".to_string() },
                    content: text,
                    timestamp: time_str,
                    is_from_me: is_sender == 0,
                })
            });

            if let Ok(iter) = rows {
                for r in iter.filter_map(Result::ok) {
                    all_messages.push(r);
                }
            }
        }
    }

    // Sort heavily combined messages by time if timestamps exist
    all_messages.sort_by_key(|m| m.timestamp.clone().unwrap_or_default());

    let count = all_messages.len();

    Ok(ParsedContent {
        source: "ios_wechat".to_string(),
        target_name: None, // Will auto-detect
        messages: all_messages,
        message_count: count,
    })
}
