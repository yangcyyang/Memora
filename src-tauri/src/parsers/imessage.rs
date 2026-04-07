use crate::models::{ParsedContent, RawMessage};
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

pub fn parse_imessage(path: &Path) -> Result<ParsedContent> {
    // Open in read-only mode. For chat.db, because of WAL mode, we might need to copy it
    // if permissions are an issue, but let's try direct read first.
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("Failed to open iMessage database. Make sure you have Full Disk Access.")?;

    // Query messages joined with handles.
    // In chat.db, `date` is nanoseconds since 2001-01-01 (macOS High Sierra+)
    // or seconds (older). Usually nanoseconds now.
    // 978307200 is UNIX timestamp of 2001-01-01.
    let mut stmt = conn.prepare(
        r#"
        SELECT
            m.text,
            m.is_from_me,
            h.id as handle_id,
            m.date
        FROM message m
        LEFT JOIN handle h ON m.handle_id = h.ROWID
        WHERE m.text IS NOT NULL
        ORDER BY m.date ASC
        "#
    ).context("Invalid chat.db schema")?;

    let mut messages = Vec::new();

    let rows = stmt.query_map([], |row| {
        let text: String = row.get(0)?;
        let is_from_me: bool = row.get(1)?;
        let handle_id: Option<String> = row.get(2)?;
        let date_raw: i64 = row.get(3)?;

        // Try to parse CoreData timestamp (since 2001)
        // If it's a huge number, it's nanoseconds. Otherwise seconds.
        let unix_seconds = if date_raw > 10000000000000000 {
            (date_raw / 1_000_000_000) + 978307200
        } else {
            date_raw + 978307200
        };

        // Format to ISO8601 string roughly
        let timestamp = chrono::DateTime::from_timestamp(unix_seconds, 0)
            .map(|dt| dt.to_rfc3339());

        Ok(RawMessage {
            sender: handle_id.unwrap_or_else(|| "Unknown".to_string()),
            content: text,
            timestamp,
            is_from_me,
        })
    })?;

    for row in rows {
        if let Ok(msg) = row {
            if !msg.content.trim().is_empty() {
                messages.push(msg);
            }
        }
    }

    let count = messages.len();

    Ok(ParsedContent {
        source: "imessage".to_string(),
        target_name: None, // Will be auto-detected in mod.rs
        messages,
        message_count: count,
    })
}
