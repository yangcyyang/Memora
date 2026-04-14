//! Correction service.

use crate::ai::{completion, config as ai_config};
use crate::error::AppError;
use crate::infra::db::memora_pool;
use crate::models::{CorrectionResult, ReinforcementResult};
use crate::prompts;
use crate::repo::persona_repo;
use anyhow::Context;
use serde_json::Value;

#[tracing::instrument(err)]
pub async fn submit_correction(
    persona_id: String,
    original: String,
    correction: String,
) -> Result<CorrectionResult, AppError> {
    let config = ai_config::load_config();

    let (persona_md, memories_md, ver, ..) = load_persona_context(persona_id.clone()).await?;

    let prompt = prompts::render(prompts::CORRECTION_HANDLER, &[
        ("persona_md", &persona_md),
        ("original", &original),
        ("correction", &correction),
    ]);

    let response = completion::chat_completion(&config, &prompt, "请分析修正并输出 JSON", 2048)
        .await
        .map_err(|e| AppError::ai(e))?;

    let cleaned = clean_json_response(&response);
    let cj: Value = serde_json::from_str(cleaned).unwrap_or_else(|_| {
        serde_json::json!({"target":"persona","rule":correction})
    });

    let target = cj.get("target").and_then(|t| t.as_str()).unwrap_or("persona").to_string();
    let rules = extract_rules(&cj, &correction);
    let new_ver = if target == "memories" {
        persist_memories_rules(
            persona_id,
            persona_md,
            memories_md,
            ver,
            "Correction",
            &rules,
            &original,
            &correction,
        )
        .await?
    } else {
        persist_persona_rules(
            persona_id,
            persona_md,
            memories_md,
            ver,
            "Correction",
            &rules,
            &original,
            &correction,
        )
        .await?
    };

    Ok(CorrectionResult {
        success: true,
        target,
        version: new_ver,
    })
}

#[tracing::instrument(err)]
pub async fn reinforce_memory(
    persona_id: String,
    message_content: String,
) -> Result<ReinforcementResult, AppError> {
    let config = ai_config::load_config();
    if !config.enabled {
        return Err(AppError::ai("请先启用 AI 服务"));
    }

    let (persona_md, memories_md, ver, ..) = load_persona_context(persona_id.clone()).await?;

    let prompt = prompts::render(prompts::MEMORY_REINFORCER, &[
        ("persona_md", &persona_md),
        ("memories_md", &memories_md),
        ("message_content", &message_content),
    ]);

    let response = completion::chat_completion(&config, &prompt, "请输出可写入 memories 的 JSON", 2048)
        .await
        .map_err(AppError::ai)?;
    let cleaned = clean_json_response(&response);
    let payload: Value = serde_json::from_str(cleaned).unwrap_or_else(|_| {
        serde_json::json!({"rules":[format!("以后延续这种互动方式：{}", message_content)]})
    });
    let rules = extract_rules(&payload, &format!("以后延续这种互动方式：{}", message_content));

    let new_version = persist_memories_rules(
        persona_id,
        persona_md,
        memories_md,
        ver,
        "Reinforcement",
        &rules,
        &message_content,
        "用户点击了“记住这个”",
    )
    .await?;

    Ok(ReinforcementResult {
        success: true,
        version: new_version,
        rules,
    })
}

pub(crate) async fn load_persona_context(
    persona_id: String,
) -> Result<(String, String, i32, String), AppError> {
    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;
        let (name, persona_md, memories_md, version) = persona_repo::get_persona_data(&conn, &persona_id)?;
        Ok::<_, AppError>((persona_md, memories_md, version, name))
    })
    .await
    .context("spawn_blocking join error")
    .map_err(|e| AppError::Internal(e))?
}

pub(crate) async fn persist_memories_rules(
    persona_id: String,
    persona_md: String,
    memories_md: String,
    version: i32,
    section_title: &str,
    rules: &[String],
    original: &str,
    correction: &str,
) -> Result<i32, AppError> {
    let new_version = version + 1;
    let next_memories_md = append_rules_block(&memories_md, section_title, new_version, rules);
    let target = "memories".to_string();
    let section = section_title.to_string();
    let original_text = original.to_string();
    let correction_text = correction.to_string();

    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;
        let now = chrono::Utc::now().to_rfc3339();

        persona_repo::insert_version(&conn, &persona_id, version, &persona_md, &memories_md, &now)?;
        persona_repo::update_memories_md(&conn, &persona_id, &next_memories_md, new_version, &now)?;
        persona_repo::insert_correction(
            &conn,
            &persona_id,
            &format!("{}:{}", target, section),
            &original_text,
            &correction_text,
            &now,
        )?;
        Ok::<_, AppError>(new_version)
    })
    .await
    .context("spawn_blocking join error")
    .map_err(|e| AppError::Internal(e))?
}

async fn persist_persona_rules(
    persona_id: String,
    persona_md: String,
    memories_md: String,
    version: i32,
    section_title: &str,
    rules: &[String],
    original: &str,
    correction: &str,
) -> Result<i32, AppError> {
    let new_version = version + 1;
    let next_persona_md = append_rules_block(&persona_md, section_title, new_version, rules);
    let target = "persona".to_string();
    let section = section_title.to_string();
    let original_text = original.to_string();
    let correction_text = correction.to_string();

    tokio::task::spawn_blocking(move || {
        let pool = memora_pool();
        let conn = pool.get().context("DB connection failed")?;
        let now = chrono::Utc::now().to_rfc3339();

        persona_repo::insert_version(&conn, &persona_id, version, &persona_md, &memories_md, &now)?;
        persona_repo::update_persona_md(&conn, &persona_id, &next_persona_md, new_version, &now)?;
        persona_repo::insert_correction(
            &conn,
            &persona_id,
            &format!("{}:{}", target, section),
            &original_text,
            &correction_text,
            &now,
        )?;
        Ok::<_, AppError>(new_version)
    })
    .await
    .context("spawn_blocking join error")
    .map_err(|e| AppError::Internal(e))?
}

pub(crate) fn clean_json_response(response: &str) -> &str {
    response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
}

pub(crate) fn extract_rules(payload: &Value, fallback_rule: &str) -> Vec<String> {
    let from_array = payload
        .get("rules")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if !from_array.is_empty() {
        return from_array;
    }

    payload
        .get("rule")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| vec![value.to_string()])
        .unwrap_or_else(|| vec![fallback_rule.to_string()])
}

fn append_rules_block(existing_md: &str, section_title: &str, version: i32, rules: &[String]) -> String {
    let block = rules
        .iter()
        .map(|rule| format!("- {}", rule))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{}\n\n### {} (v{})\n{}",
        existing_md.trim_end(),
        section_title,
        version,
        block
    )
}

#[cfg(test)]
mod tests {
    use super::{append_rules_block, clean_json_response, extract_rules};

    #[test]
    fn strips_json_code_fence() {
        let cleaned = clean_json_response("```json\n{\"rules\":[\"a\"]}\n```");
        assert_eq!(cleaned, "{\"rules\":[\"a\"]}");
    }

    #[test]
    fn extracts_rules_array_first() {
        let payload = serde_json::json!({"rules":["规则一","规则二"]});
        assert_eq!(extract_rules(&payload, "fallback"), vec!["规则一".to_string(), "规则二".to_string()]);
    }

    #[test]
    fn appends_markdown_block() {
        let updated = append_rules_block("已有记忆", "Reinforcement", 3, &["保持温柔".to_string()]);
        assert!(updated.contains("### Reinforcement (v3)"));
        assert!(updated.contains("- 保持温柔"));
    }
}
