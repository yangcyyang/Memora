use crate::core::db_pool::memora_pool;
use arboard::Clipboard;
use lazy_static::lazy_static;
use regex::Regex;
use std::time::Duration;
use tracing::{debug, info};

lazy_static! {
    // Looks for typical chat log formats, e.g., "A: hello", "Nickname: hello", "User 12:30 \n hello"
    static ref CHAT_PATTERN_1: Regex = Regex::new(r"(?m)^.{1,15}:\s+.+").unwrap();
    static ref CHAT_PATTERN_2: Regex = Regex::new(r"(?m)^\d{4}-\d{2}-\d{2}\s+\d{1,2}:\d{2}\s+.+").unwrap();
}

pub fn start_watcher() {
    std::thread::spawn(|| {
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

            if let Ok(text) = cb.get_text() {
                // Ignore identical or extremely large texts (prevent memory spikes)
                if text == last_content || text.len() > 10000 || text.trim().is_empty() {
                    continue;
                }
                last_content = text.clone();

                // Detect if it looks like a chat snippet
                let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
                if lines.len() < 2 {
                    continue; // Too short to be a meaningful chat context
                }

                if CHAT_PATTERN_1.is_match(&text) || CHAT_PATTERN_2.is_match(&text) {
                    debug!("Detected chat-like content in clipboard");
                    // In a real scenario, we might want to attribute this to a *specific*
                    // active Persona. For now, since Memora expects users to manually
                    // curate, we'll store this in a generic "background_scraps" table or
                    // temporarily append it to the most recently active Persona's memories.
                    
                    // Task: "静默收入特定 Persona 的后备记忆库"
                    // Let's implement adding to the most recently created/chatted Persona.
                    if let Err(e) = silently_stash_corpus(&text) {
                        tracing::error!("Failed to stash clipboard corpus: {}", e);
                    }
                }
            }
        }
    });
}

fn silently_stash_corpus(content: &str) -> anyhow::Result<()> {
    let pool = memora_pool();
    let conn = pool.get()?;

    // Find the most recently active persona
    let persona_id: Option<String> = conn.query_row(
        r#"SELECT p.id FROM personas p 
           LEFT JOIN chat_messages cm ON p.id = cm.persona_id
           ORDER BY COALESCE(cm.created_at, p.updated_at) DESC LIMIT 1"#,
        [],
        |row| row.get(0),
    ).ok();

    if let Some(pid) = persona_id {
        info!("Silent clipboard stash -> Persona: {}", pid);
        
        let now = chrono::Utc::now().to_rfc3339();
        // Append raw corpus info to their memories_md
        let append_md = format!(
            "\n\n### 自动捕捉的语料 ({})\n```text\n{}\n```\n",
            now, content
        );

        let current_memories: String = conn.query_row(
            "SELECT memories_md FROM personas WHERE id = ?1",
            rusqlite::params![pid],
            |row| row.get(0)
        )?;

        conn.execute(
            "UPDATE personas SET memories_md = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![current_memories + &append_md, now, pid]
        )?;
    }

    Ok(())
}
