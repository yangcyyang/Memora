//! Database connection pool and schema initialisation.

use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;
use std::sync::LazyLock;
use tracing::info;

use super::paths;

/// Pool type alias used throughout the codebase.
pub type DbPool = Pool<SqliteConnectionManager>;

/// Create a connection pool for a SQLite database file.
pub fn create_pool(db_path: &Path, max_size: u32) -> Result<DbPool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create DB directory")?;
    }

    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA foreign_keys=ON;",
        )
    });

    Pool::builder()
        .max_size(max_size)
        .build(manager)
        .context("Failed to build r2d2 connection pool")
}

/// Run a blocking closure on a pooled connection via `spawn_blocking`.
pub async fn run_blocking<F, T>(pool: &'static DbPool, f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool
            .get()
            .context("Failed to get DB connection from pool")?;
        f(&conn)
    })
    .await
    .context("spawn_blocking join error")?
}

// ── Main Memora Pool ────────────────────────────────────────────────

static MEMORA_POOL: LazyLock<DbPool> = LazyLock::new(|| {
    create_pool(&paths::db_path(), 4).expect("Memora DB pool init failed")
});

pub fn memora_pool() -> &'static DbPool {
    &MEMORA_POOL
}

/// Initialize the database with all required tables
pub fn initialize_db() -> Result<()> {
    paths::ensure_dirs()?;

    let pool = memora_pool();
    let conn = pool.get().context("Failed to get DB connection for init")?;

    conn.execute_batch(
        r#"
        -- Persona 主表
        CREATE TABLE IF NOT EXISTS personas (
            id                TEXT PRIMARY KEY,
            slug              TEXT UNIQUE NOT NULL,
            name              TEXT NOT NULL,
            avatar_emoji      TEXT DEFAULT '💜',
            description       TEXT DEFAULT '',
            tags_json         TEXT NOT NULL DEFAULT '[]',
            persona_md        TEXT NOT NULL DEFAULT '',
            memories_md       TEXT NOT NULL DEFAULT '',
            version           INTEGER NOT NULL DEFAULT 1,
            proactive_enabled INTEGER NOT NULL DEFAULT 0,
            proactive_rules   TEXT,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL
        );

        -- 版本快照（回滚用）
        CREATE TABLE IF NOT EXISTS persona_versions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            persona_id  TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
            version     INTEGER NOT NULL,
            persona_md  TEXT NOT NULL,
            memories_md TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );

        -- 聊天消息
        CREATE TABLE IF NOT EXISTS chat_messages (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            persona_id  TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
            session_id  TEXT NOT NULL,
            role        TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
            content     TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_chat_persona_session
            ON chat_messages(persona_id, session_id);

        -- 纠正记录
        CREATE TABLE IF NOT EXISTS corrections (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            persona_id  TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
            target      TEXT NOT NULL CHECK (target IN ('persona', 'memories')),
            original    TEXT,
            correction  TEXT NOT NULL,
            applied_at  TEXT NOT NULL
        );

        -- 会话上下文压缩摘要
        CREATE TABLE IF NOT EXISTS session_summaries (
            session_id              TEXT PRIMARY KEY,
            persona_id              TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
            summary_md              TEXT NOT NULL DEFAULT '',
            last_compressed_msg_id  INTEGER NOT NULL DEFAULT 0,
            token_estimate          INTEGER NOT NULL DEFAULT 0,
            updated_at              TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_session_summaries_persona
            ON session_summaries(persona_id);

        -- 应用设置
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Persona 语音绑定（每个 Persona 可绑不同 provider 的 voice_id）
        CREATE TABLE IF NOT EXISTS persona_voices (
            persona_id TEXT PRIMARY KEY REFERENCES personas(id) ON DELETE CASCADE,
            provider   TEXT NOT NULL,
            voice_id   TEXT NOT NULL,
            language   TEXT NOT NULL DEFAULT 'zh-CN',
            model      TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        );

        -- 主动触达配置迁移（兼容旧表）
        -- 注：SQLite 不支持 IF NOT EXISTS，通过 user_version 控制迁移
        PRAGMA user_version = 1;
        "#,
    )
    .context("Failed to create database tables")?;

    // ── Migrations for existing databases ───────────────────────────────
    // SQLite does not support ALTER TABLE IF NOT EXISTS, so we attempt
    // the column additions and ignore the error if they already exist.
    let _ = conn.execute("ALTER TABLE personas ADD COLUMN proactive_enabled INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE personas ADD COLUMN proactive_rules TEXT", []);

    info!("Database initialized successfully");
    Ok(())
}
