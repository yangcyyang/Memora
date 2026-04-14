//! Unified error type for Memora.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("连接池错误: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("AI 服务错误: {0}")]
    AiProvider(String),

    #[error("TTS 服务错误: {0}")]
    Tts(String),

    #[error("未找到: {0}")]
    NotFound(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP 错误: {0}")]
    Zip(String),

    #[error("序列化错误: {0}")]
    Serialize(String),

    #[error("{0}")]
    Internal(#[from] anyhow::Error),
}

// Allow `commands/` to return `Result<T, AppError>` directly as Tauri command results.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// Convenience conversion so services can do `.map_err(AppError::ai)?`
impl AppError {
    pub fn ai(msg: impl std::fmt::Display) -> Self {
        AppError::AiProvider(msg.to_string())
    }

    pub fn tts(msg: impl std::fmt::Display) -> Self {
        AppError::Tts(msg.to_string())
    }

    pub fn not_found(msg: impl std::fmt::Display) -> Self {
        AppError::NotFound(msg.to_string())
    }
}
