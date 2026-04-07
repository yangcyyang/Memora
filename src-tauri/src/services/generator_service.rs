//! Persona generation service.

use crate::ai::{completion, config as ai_config};
use crate::error::AppError;
use crate::infra::db::memora_pool;
use crate::models::{BasicInfo, GenerateProgress, GenerateResult, ParsedContent};
use crate::prompts;
use crate::repo::persona_repo;
use anyhow::Context;
use tauri::Emitter;
use tracing::info;

#[tracing::instrument(skip_all, err)]
pub async fn generate_persona(
    app: tauri::AppHandle,
    info: BasicInfo,
    parsed: Vec<ParsedContent>,
) -> Result<GenerateResult, AppError> {
    let config = ai_config::load_config();
    if !config.enabled || config.api_key.is_empty() {
        return Err(AppError::ai("请先设置 AI 服务密钥"));
    }

    let chat_text = if parsed.is_empty() {
        String::new()
    } else {
        parsed.iter().flat_map(|p| &p.messages)
            .map(|m| {
                let sender = if m.is_from_me { "我" } else { &m.sender };
                format!("{}: {}", sender, m.content)
            })
            .collect::<Vec<_>>().join("\n")
    };

    let tags_str = info.tags.join("、");
    let has_chat_data = !chat_text.is_empty();
    let total_steps: u32 = if has_chat_data { 5 } else { 3 };

    info!(name = %info.name, has_chat_data, "Starting persona generation");

    // Step 1
    emit_progress(&app, 1, total_steps, "分析性格中...")?;
    let persona_analysis = if has_chat_data {
        let prompt = prompts::render(prompts::PERSONA_ANALYZER, &[
            ("chat_text", &chat_text), ("name", &info.name),
            ("description", &info.description), ("tags", &tags_str),
        ]);
        completion::chat_completion(&config, &prompt, "请开始分析", 4096).await
            .map_err(|e| AppError::ai(e))?
    } else {
        format!("用户没有提供聊天记录。基于用户描述生成：\n名字：{}\n描述：{}\n标签：{}", info.name, info.description, tags_str)
    };

    // Step 2
    emit_progress(&app, 2, total_steps, "构建人格中...")?;
    let persona_prompt = prompts::render(prompts::PERSONA_BUILDER, &[
        ("analysis", &persona_analysis), ("tags", &tags_str),
    ]);
    let persona_md = completion::chat_completion(&config, &persona_prompt, "请构建 Persona 文档", 4096).await
        .map_err(|e| AppError::ai(e))?;

    // Step 3+
    let memories_md = if has_chat_data {
        emit_progress(&app, 3, total_steps, "提取回忆中...")?;
        let mem_analysis_prompt = prompts::render(prompts::MEMORIES_ANALYZER, &[("chat_text", &chat_text)]);
        let mem_analysis = completion::chat_completion(&config, &mem_analysis_prompt, "请分析共同记忆", 4096).await
            .map_err(|e| AppError::ai(e))?;

        emit_progress(&app, 4, total_steps, "整理回忆中...")?;
        let mem_build_prompt = prompts::render(prompts::MEMORIES_BUILDER, &[("analysis", &mem_analysis)]);
        completion::chat_completion(&config, &mem_build_prompt, "请构建 Memories 文档", 4096).await
            .map_err(|e| AppError::ai(e))?
    } else {
        "暂无共同记忆。当提供聊天记录后可以补充。".to_string()
    };

    // Save to database
    let persona_id = uuid::Uuid::new_v4().to_string();
    let slug = generate_slug(&info.name);
    let now = chrono::Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(&info.tags).unwrap_or_default();

    let pid = persona_id.clone();
    let pmd = persona_md.clone();
    let mmd = memories_md.clone();
    let info_name = info.name.clone();
    let info_emoji = info.avatar_emoji.clone();
    let info_desc = info.description.clone();

    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;
        persona_repo::insert(&conn, &pid, &slug, &info_name, &info_emoji, &info_desc, &tags_json, &pmd, &mmd, &now)?;
        persona_repo::insert_version(&conn, &pid, 1, &pmd, &mmd, &now)?;
        Ok::<(), AppError>(())
    })
    .await
    .context("spawn_blocking join error")
    .map_err(|e| AppError::Internal(e))??;

    emit_progress(&app, total_steps, total_steps, "完成！")?;

    let summary = format!(
        "已为「{}」生成 {} 人格和回忆",
        info.name,
        if has_chat_data { "基于聊天记录的" } else { "基于描述的" }
    );

    Ok(GenerateResult { persona_id, persona_md, memories_md, summary })
}

fn emit_progress(app: &tauri::AppHandle, step: u32, total: u32, label: &str) -> Result<(), AppError> {
    app.emit("generate://progress", GenerateProgress { step, total, label: label.to_string() })
        .context("Failed to emit progress")
        .map_err(|e| AppError::Internal(e))?;
    Ok(())
}

fn generate_slug(name: &str) -> String {
    use pinyin::ToPinyin;
    let pinyin_str: String = name.chars()
        .filter_map(|c| {
            c.to_pinyin().map(|p| p.plain().to_string()).or_else(|| {
                if c.is_alphanumeric() || c == '-' || c == '_' { Some(c.to_string()) } else { None }
            })
        })
        .collect::<Vec<_>>().join("-");

    if pinyin_str.is_empty() {
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    } else {
        pinyin_str.to_lowercase()
    }
}
