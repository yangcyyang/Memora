# TTS 语音模块 — 开发指南

Memora 的 TTS 模块采用 **provider-agnostic** 架构，通过 Rust trait 统一接口，支持同步 HTTP 和 WebSocket 流式两种合成模式。

## 架构概览

```
src-tauri/src/
├── tts/                       ← TTS 子系统
│   ├── provider.rs            ←   Trait 定义 + 配置 + 工厂函数
│   ├── minimax.rs             ←   MiniMax 实现（首个 provider）
│   ├── cache.rs               ←   LRU 音频缓存（可配 100~2000MB）
│   └── audio_utils.rs         ←   ffmpeg 检测 + 视频音轨提取
│
├── repo/
│   └── voice_repo.rs          ← Persona-Voice 绑定的 SQL 操作
│
├── commands/
│   └── tts.rs                 ← 12 个 Tauri command（薄壳）
│
└── infra/
    └── crypto.rs              ← API Key 加密（AES-256-GCM）
```

**数据流**：

```
前端 🔊 按钮 → invoke("speak_text") → voice_repo 查 persona_voices 表
  → cache 查缓存 → (miss) → TtsProvider::synthesize() → cache 写缓存 → 返回本地路径
  → convertFileSrc() → <audio> 播放
```

**克隆流程（支持音频 + 视频）**：

```
前端选择文件 → invoke("upload_and_clone_voice")
  → audio_utils::ensure_audio_format()
    → 音频文件：直接上传
    → 视频文件：ffmpeg 提取音轨为 mp3 → 上传 → 清理临时文件
  → TtsProvider::upload_audio() → clone_voice() → voice_repo 绑定
```

---

## 如何添加新的 TTS Provider

只需 **3 步**，即可接入任何新的语音服务商（如 Azure TTS、ElevenLabs、Fish Audio、腾讯云语音等）：

### 第 1 步：创建 Provider 实现

新建文件 `src-tauri/src/tts/<your_provider>.rs`：

```rust
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::mpsc;

use super::provider::{
    CloneRequest, CloneResult, SynthesizeRequest, TtsProvider, TtsProviderConfig,
};

pub struct MyProvider {
    api_key: String,
    base_url: String,
    default_model: String,
}

impl MyProvider {
    pub fn new(config: &TtsProviderConfig) -> Result<Self> {
        Ok(Self {
            api_key: config.api_key.clone(),
            base_url: if config.base_url.is_empty() {
                "https://api.example.com".to_string()
            } else {
                config.base_url.clone()
            },
            default_model: if config.default_model.is_empty() {
                "default-model".to_string()
            } else {
                config.default_model.clone()
            },
        })
    }
}

#[async_trait]
impl TtsProvider for MyProvider {
    fn id(&self) -> &'static str { "my_provider" }
    fn display_name(&self) -> &'static str { "My Provider" }

    fn supported_languages(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("zh-CN", "普通话"),
            ("en", "English"),
        ]
    }

    fn supports_voice_clone(&self) -> bool {
        false // 如果不支持克隆就返回 false
    }

    fn supports_streaming(&self) -> bool {
        false // 如果不支持流式就返回 false
    }

    async fn upload_audio(&self, _file_path: &str) -> Result<String> {
        anyhow::bail!("此 Provider 不支持音色克隆")
    }

    async fn clone_voice(&self, _req: CloneRequest) -> Result<CloneResult> {
        anyhow::bail!("此 Provider 不支持音色克隆")
    }

    async fn synthesize(&self, req: SynthesizeRequest) -> Result<Vec<u8>> {
        // 调用你的 TTS API，返回音频 bytes（mp3）
        let client = reqwest::Client::new();
        let resp = client
            .post(&format!("{}/tts", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "text": req.text,
                "voice": req.voice_id,
                "language": req.language,
            }))
            .send()
            .await?;

        let audio = resp.bytes().await?.to_vec();
        Ok(audio)
    }

    async fn synthesize_stream(
        &self,
        _req: SynthesizeRequest,
        _tx: mpsc::Sender<Vec<u8>>,
    ) -> Result<()> {
        anyhow::bail!("此 Provider 不支持流式合成")
    }
}
```

### 第 2 步：注册到工厂函数

修改 `src-tauri/src/tts/provider.rs`，在两处添加：

**1. 工厂函数 `get_provider()`**

```rust
// 在 match 中增加一个 arm：
pub fn get_provider(id: &str, config: &TtsProviderConfig) -> Result<Box<dyn TtsProvider>> {
    match id {
        "minimax" => Ok(Box::new(MiniMaxProvider::new(config)?)),
        "my_provider" => Ok(Box::new(MyProvider::new(config)?)),  // ← 新增
        _ => Err(anyhow!("Unknown TTS provider: {}", id)),
    }
}
```

**2. Provider 列表 `list_providers()`**

```rust
pub fn list_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo { /* MiniMax ... */ },
        ProviderInfo {                          // ← 新增
            id: "my_provider".to_string(),
            name: "My Provider".to_string(),
            supports_clone: false,
            supports_streaming: false,
            languages: vec![
                LanguageInfo { code: "zh-CN".into(), name: "普通话".into() },
                LanguageInfo { code: "en".into(), name: "English".into() },
            ],
        },
    ]
}
```

### 第 3 步：注册模块

在 `src-tauri/src/tts/mod.rs` 添加：

```rust
pub mod my_provider;
```

在 `provider.rs` 顶部 import：

```rust
use super::my_provider::MyProvider;
```

**完成！** 重新编译后前端会自动在设置页面显示新 Provider 的选项。

---

## TtsProvider Trait 详解

| 方法 | 必须实现？ | 说明 |
|------|:---------:|------|
| `id()` | ✅ | 唯一标识，用于配置持久化 |
| `display_name()` | ✅ | 前端展示名 |
| `supported_languages()` | ✅ | 支持的语言列表 |
| `supports_voice_clone()` | ✅ | 是否支持克隆（false 则隐藏克隆按钮） |
| `supports_streaming()` | ✅ | 是否支持 WS 流式 |
| `upload_audio()` | ⚡ | 克隆前上传音频，不支持可返回 Err |
| `clone_voice()` | ⚡ | 执行克隆，不支持可返回 Err |
| `synthesize()` | ✅ | 核心方法：文本 → 音频 bytes |
| `synthesize_stream()` | ⚡ | 流式合成，不支持可返回 Err |

> ⚡ = 条件实现：如果 `supports_xxx()` 返回 `false`，前端不会调用对应方法，但 trait 要求提供实现（可直接 `bail!()`）。

---

## 分层职责

| 层 | 文件 | 职责 |
|---|---|---|
| **commands** | `commands/tts.rs` | Tauri command 薄壳，参数校验 + 调用下层 |
| **tts** | `tts/provider.rs` | Trait 定义、配置加载/保存、工厂函数 |
| **tts** | `tts/minimax.rs` | MiniMax 具体实现（HTTP + WebSocket） |
| **tts** | `tts/cache.rs` | LRU 音频缓存（SHA256 key，按 mtime 淘汰） |
| **tts** | `tts/audio_utils.rs` | ffmpeg 检测、视频→音频提取、格式分类 |
| **repo** | `repo/voice_repo.rs` | `persona_voices` 表的增删查改 |
| **infra** | `infra/crypto.rs` | API Key 加密/解密（AES-256-GCM） |

---

## 配置结构

TTS 配置存储在 `~/.memora/tts_settings.json`，结构如下：

```json
{
  "active_provider": "minimax",
  "cache_limit_mb": 500,
  "providers": {
    "minimax": {
      "api_key": "<encrypted>",
      "base_url": "https://api.minimaxi.com",
      "default_model": "speech-2.8-hd",
      "default_language": "zh-CN",
      "extra": {}
    }
  }
}
```

每个 provider 的 `api_key` 使用 AES-256-GCM 加密存储（与 AI 服务共用 `infra/crypto.rs`）。

---

## Persona 语音绑定

每个 Persona 可绑定不同 provider 的 voice_id，存储在 SQLite `persona_voices` 表：

```sql
CREATE TABLE persona_voices (
    persona_id TEXT PRIMARY KEY,
    provider   TEXT NOT NULL,
    voice_id   TEXT NOT NULL,
    language   TEXT NOT NULL DEFAULT 'zh-CN',
    model      TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);
```

所有 SQL 操作封装在 `repo/voice_repo.rs`，command 层禁止写裸 SQL。

---

## 音频缓存

- 路径：`~/.memora/audio/`
- Key = `SHA256(provider|voice_id|text|language).mp3`
- 相同请求直接命中缓存，不重复调用 API
- LRU 淘汰：按文件 mtime 排序，超出限额时自动删除最旧文件
- 默认上限 500MB，可在设置中调整（100MB ~ 2000MB）

---

## 已实现的 Provider

| Provider | 克隆 | 流式 | 状态 |
|----------|:----:|:----:|------|
| MiniMax  | ✅   | ✅   | 已实现 |
| Azure TTS | -  | -    | 待接入 |
| ElevenLabs | - | -   | 待接入 |
| Fish Audio | -  | -  | 待接入 |
