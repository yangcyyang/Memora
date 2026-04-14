//! Calibration service.

use crate::ai::{completion, config as ai_config};
use crate::error::AppError;
use crate::models::{CalibrationApplyResult, CalibrationFeedbackItem, CalibrationSample};
use crate::prompts;
use crate::services::correction_service;

#[tracing::instrument(err)]
pub async fn generate_calibration_samples(
    persona_id: String,
) -> Result<Vec<CalibrationSample>, AppError> {
    let config = ai_config::load_config();
    if !config.enabled {
        return Err(AppError::ai("请先启用 AI 服务"));
    }

    let (persona_md, memories_md, ..) =
        correction_service::load_persona_context(persona_id).await?;

    let prompt = prompts::render(prompts::CALIBRATION_SAMPLE_GENERATOR, &[
        ("persona_md", &persona_md),
        ("memories_md", &memories_md),
    ]);

    let response = completion::chat_completion(&config, &prompt, "请输出校准样本 JSON", 2048)
        .await
        .map_err(AppError::ai)?;

    let cleaned = correction_service::clean_json_response(&response);
    let samples: Vec<CalibrationSample> = serde_json::from_str(cleaned)
        .map_err(|e| AppError::ai(format!("校准样本解析失败: {}", e)))?;

    if samples.is_empty() {
        return Err(AppError::ai("校准样本为空"));
    }

    Ok(samples)
}

#[tracing::instrument(skip(feedback_items), err)]
pub async fn submit_calibration_feedback(
    persona_id: String,
    feedback_items: Vec<CalibrationFeedbackItem>,
    free_text: Option<String>,
) -> Result<CalibrationApplyResult, AppError> {
    if feedback_items.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!("至少需要一条校准反馈")));
    }

    let config = ai_config::load_config();
    if !config.enabled {
        return Err(AppError::ai("请先启用 AI 服务"));
    }

    let feedback_json = serde_json::to_string_pretty(&feedback_items)
        .map_err(|e| AppError::Internal(e.into()))?;
    let free_text_value = free_text.unwrap_or_default();

    let (persona_md, memories_md, version, ..) =
        correction_service::load_persona_context(persona_id.clone()).await?;

    let prompt = prompts::render(prompts::CALIBRATION_FEEDBACK_APPLIER, &[
        ("persona_md", &persona_md),
        ("memories_md", &memories_md),
        ("feedback_json", &feedback_json),
        ("free_text", &free_text_value),
    ]);

    let response = completion::chat_completion(&config, &prompt, "请输出校准规则 JSON", 2048)
        .await
        .map_err(AppError::ai)?;

    let cleaned = correction_service::clean_json_response(&response);
    let payload: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|e| AppError::ai(format!("校准反馈解析失败: {}", e)))?;

    let rules = correction_service::extract_rules(&payload, "请按用户校准反馈调整 Persona 表达风格");
    let original = feedback_items
        .iter()
        .map(|item| format!("{} => {}", item.scenario, item.reply))
        .collect::<Vec<_>>()
        .join("\n");
    let correction = payload
        .get("summary")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("提交了一轮 Persona 校准反馈")
        .to_string();

    let new_version = correction_service::persist_memories_rules(
        persona_id,
        persona_md,
        memories_md,
        version,
        "Calibration",
        &rules,
        &original,
        &correction,
    )
    .await?;

    Ok(CalibrationApplyResult {
        success: true,
        version: new_version,
        rules,
    })
}
