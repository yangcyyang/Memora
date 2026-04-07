//! Correction service.

use crate::ai::{completion, config as ai_config};
use crate::error::AppError;
use crate::infra::db::memora_pool;
use crate::models::CorrectionResult;
use crate::prompts;
use crate::repo::persona_repo;
use anyhow::Context;

#[tracing::instrument(err)]
pub async fn submit_correction(
    persona_id: String,
    original: String,
    correction: String,
) -> Result<CorrectionResult, AppError> {
    let config = ai_config::load_config();

    // Load persona in blocking context
    let (persona_md, memories_md, ver) = tokio::task::spawn_blocking({
        let pid = persona_id.clone();
        move || {
            let pool = memora_pool();
            let conn = pool.get().context("DB connection failed")?;
            let (_name, pmd, mmd, v) = persona_repo::get_persona_data(&conn, &pid)?;
            Ok::<_, AppError>((pmd, mmd, v))
        }
    })
    .await
    .context("spawn_blocking join error")
    .map_err(|e| AppError::Internal(e))??;

    let prompt = prompts::render(prompts::CORRECTION_HANDLER, &[
        ("persona_md", &persona_md),
        ("original", &original),
        ("correction", &correction),
    ]);

    let response = completion::chat_completion(&config, &prompt, "请分析修正并输出 JSON", 2048)
        .await
        .map_err(|e| AppError::ai(e))?;

    let cleaned = response.trim().trim_start_matches("```json")
        .trim_start_matches("```").trim_end_matches("```").trim();
    let cj: serde_json::Value = serde_json::from_str(cleaned).unwrap_or_else(|_| {
        serde_json::json!({"target":"persona","rule":correction})
    });

    let target = cj.get("target").and_then(|t| t.as_str()).unwrap_or("persona").to_string();
    let rule = cj.get("rule").and_then(|r| r.as_str()).unwrap_or(&correction).to_string();
    let new_ver = ver + 1;

    // Save in blocking context
    tokio::task::spawn_blocking({
        let pid = persona_id.clone();
        let pmd = persona_md.clone();
        let mmd = memories_md.clone();
        let target2 = target.clone();
        let orig = original.clone();
        let corr = correction.clone();
        let rule2 = rule.clone();
        move || {
            let pool = memora_pool();
            let conn = pool.get().context("DB connection failed")?;
            let now = chrono::Utc::now().to_rfc3339();

            persona_repo::insert_version(&conn, &pid, ver, &pmd, &mmd, &now)?;

            if target2 == "memories" {
                let new_md = format!("{}\n\n### Correction (v{})\n- {}", mmd, new_ver, rule2);
                persona_repo::update_memories_md(&conn, &pid, &new_md, new_ver, &now)?;
            } else {
                let new_md = format!("{}\n\n### Correction (v{})\n- {}", pmd, new_ver, rule2);
                persona_repo::update_persona_md(&conn, &pid, &new_md, new_ver, &now)?;
            }

            persona_repo::insert_correction(&conn, &pid, &target2, &orig, &corr, &now)?;
            Ok::<(), AppError>(())
        }
    })
    .await
    .context("spawn_blocking join error")
    .map_err(|e| AppError::Internal(e))??;

    Ok(CorrectionResult { success: true, target, version: new_ver })
}
