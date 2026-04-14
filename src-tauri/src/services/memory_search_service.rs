use anyhow::Context;
use sha2::{Digest, Sha256};

use crate::ai::{config as ai_config, embedding};
use crate::error::AppError;
use crate::infra::{db::memora_pool, vectors::{MemoryVectorRecord, VectorIndex}};
use crate::repo::persona_repo;

pub async fn index_memories(persona_id: &str) -> Result<usize, AppError> {
    let memories_md = {
        let persona_id = persona_id.to_string();
        tokio::task::spawn_blocking(move || {
            let pool = memora_pool();
            let conn = pool.get().context("DB connection failed")?;
            let (_, _, memories_md, _) = persona_repo::get_persona_data(&conn, &persona_id)?;
            Ok::<_, anyhow::Error>(memories_md)
        })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
        .map_err(AppError::Internal)?
    };

    let chunks = extract_memory_chunks(&memories_md);
    if chunks.is_empty() {
        let index = VectorIndex::new(persona_id);
        index.save()?;
        tracing::info!("semantic index rebuilt with 0 memories for persona {}", persona_id);
        return Ok(0);
    }

    let config = ai_config::load_config();
    let provider = embedding::get_embedding_provider(&config)?;
    let mut records = Vec::with_capacity(chunks.len());

    for (idx, text) in chunks.iter().enumerate() {
        let embedding = provider.embed(text).await?;
        records.push(MemoryVectorRecord {
            id: memory_id(idx, text),
            text: text.clone(),
            embedding,
        });
    }

    let index = VectorIndex::replace_records(persona_id, records)?;
    index.save()?;
    tracing::info!(
        "semantic index rebuilt with {} memories for persona {}",
        index.record_count(),
        persona_id
    );
    Ok(index.record_count())
}

pub async fn search_memories_texts(
    persona_id: &str,
    query: &str,
    k: usize,
) -> Result<Vec<String>, AppError> {
    let Some(index) = VectorIndex::load(persona_id)? else {
        return Ok(Vec::new());
    };
    if index.record_count() == 0 {
        return Ok(Vec::new());
    }

    let config = ai_config::load_config();
    let provider = embedding::get_embedding_provider(&config)?;
    let query_embedding = provider.embed(query).await?;
    Ok(index.search_texts(query_embedding, k))
}

pub fn has_index(persona_id: &str) -> bool {
    VectorIndex::exists(persona_id)
}

fn extract_memory_chunks(memories_md: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();

    let push_chunk = |buffer: &mut Vec<String>, out: &mut Vec<String>| {
        if buffer.is_empty() {
            return;
        }
        let text = buffer
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        buffer.clear();
        let cleaned = cleanup_markdown(&text);
        if cleaned.chars().count() >= 8 {
            out.push(cleaned);
        }
    };

    for raw_line in memories_md.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            push_chunk(&mut current, &mut chunks);
            continue;
        }

        if line.starts_with('#') && !current.is_empty() {
            push_chunk(&mut current, &mut chunks);
        }

        current.push(line.to_string());
    }

    push_chunk(&mut current, &mut chunks);
    chunks
}

fn cleanup_markdown(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            line.trim_start_matches('#')
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn memory_id(index: usize, text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("mem-{:03}-{:08x}", index + 1, u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_memory_chunks_from_markdown() {
        let memories = r#"
### 重要记忆
- 第一次一起去海边

### 日常模式
常在晚上聊天
"#;

        let chunks = extract_memory_chunks(memories);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains("第一次一起去海边"));
        assert!(chunks[1].contains("常在晚上聊天"));
    }
}
