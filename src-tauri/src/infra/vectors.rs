use std::collections::HashMap;

use anyhow::Context;
use instant_distance::{Builder, HnswMap, Point, Search};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::infra::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingPoint {
    pub values: Vec<f32>,
}

impl Point for EmbeddingPoint {
    fn distance(&self, other: &Self) -> f32 {
        cosine_distance(&self.values, &other.values)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVectorRecord {
    pub id: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
struct PersistedVectorIndex {
    persona_id: String,
    dimension: usize,
    records: Vec<MemoryVectorRecord>,
    index: HnswMap<EmbeddingPoint, String>,
    updated_at: String,
}

pub struct VectorIndex {
    inner: PersistedVectorIndex,
}

impl VectorIndex {
    pub fn new(persona_id: &str) -> Self {
        Self {
            inner: PersistedVectorIndex {
                persona_id: persona_id.to_string(),
                dimension: 0,
                records: Vec::new(),
                index: Builder::default().build(Vec::new(), Vec::new()),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        }
    }

    pub fn exists(persona_id: &str) -> bool {
        paths::vector_index_path(persona_id).exists()
    }

    pub fn load(persona_id: &str) -> Result<Option<Self>, AppError> {
        let path = paths::vector_index_path(persona_id);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = std::fs::read(&path)
            .with_context(|| format!("读取向量索引失败: {}", path.display()))?;
        let inner: PersistedVectorIndex =
            bincode::deserialize(&bytes).context("反序列化向量索引失败")?;
        Ok(Some(Self { inner }))
    }

    pub fn save(&self) -> Result<(), AppError> {
        let path = paths::vector_index_path(&self.inner.persona_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建向量目录失败: {}", parent.display()))?;
        }

        let bytes = bincode::serialize(&self.inner).context("序列化向量索引失败")?;
        std::fs::write(&path, bytes)
            .with_context(|| format!("写入向量索引失败: {}", path.display()))?;
        Ok(())
    }

    pub fn replace_records(
        persona_id: &str,
        records: Vec<MemoryVectorRecord>,
    ) -> Result<Self, AppError> {
        let mut index = Self::new(persona_id);
        for record in records {
            index.add_memory(&record.id, &record.text, record.embedding)?;
        }
        Ok(index)
    }

    pub fn add_memory(
        &mut self,
        id: &str,
        text: &str,
        embedding: Vec<f32>,
    ) -> Result<(), AppError> {
        if embedding.is_empty() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "向量为空，无法加入索引"
            )));
        }

        if self.inner.dimension == 0 {
            self.inner.dimension = embedding.len();
        } else if self.inner.dimension != embedding.len() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "向量维度不一致: expected={}, got={}",
                self.inner.dimension,
                embedding.len()
            )));
        }

        self.inner.records.retain(|record| record.id != id);
        self.inner.records.push(MemoryVectorRecord {
            id: id.to_string(),
            text: text.to_string(),
            embedding,
        });
        self.rebuild_index();
        self.inner.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn search(&self, embedding: Vec<f32>, k: usize) -> Vec<String> {
        if self.inner.dimension == 0 || embedding.len() != self.inner.dimension {
            return Vec::new();
        }

        let mut search = Search::default();
        let query = EmbeddingPoint { values: embedding };
        self.inner
            .index
            .search(&query, &mut search)
            .take(k)
            .map(|item| item.value.clone())
            .collect()
    }

    pub fn search_texts(&self, embedding: Vec<f32>, k: usize) -> Vec<String> {
        let by_id: HashMap<&str, &str> = self
            .inner
            .records
            .iter()
            .map(|record| (record.id.as_str(), record.text.as_str()))
            .collect();

        self.search(embedding, k)
            .into_iter()
            .filter_map(|id| by_id.get(id.as_str()).map(|text| (*text).to_string()))
            .collect()
    }

    pub fn record_count(&self) -> usize {
        self.inner.records.len()
    }

    fn rebuild_index(&mut self) {
        let points = self
            .inner
            .records
            .iter()
            .map(|record| EmbeddingPoint {
                values: record.embedding.clone(),
            })
            .collect::<Vec<_>>();

        let values = self
            .inner
            .records
            .iter()
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();

        self.inner.index = Builder::default().build(points, values);
    }
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 1.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (lhs, rhs) in a.iter().zip(b.iter()) {
        dot += lhs * rhs;
        norm_a += lhs * lhs;
        norm_b += rhs * rhs;
    }

    if norm_a <= f32::EPSILON || norm_b <= f32::EPSILON {
        return 1.0;
    }

    let similarity = dot / (norm_a.sqrt() * norm_b.sqrt());
    1.0 - similarity.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_distance_prefers_similar_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.9, 0.1, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!(cosine_distance(&a, &b) < cosine_distance(&a, &c));
    }

    #[test]
    fn vector_index_returns_nearest_texts() {
        let mut index = VectorIndex::new("persona-test");
        index
            .add_memory("m1", "喜欢咖啡", vec![1.0, 0.0, 0.0])
            .unwrap();
        index
            .add_memory("m2", "喜欢跑步", vec![0.0, 1.0, 0.0])
            .unwrap();

        let hits = index.search_texts(vec![0.8, 0.2, 0.0], 1);
        assert_eq!(hits, vec!["喜欢咖啡".to_string()]);
    }
}
