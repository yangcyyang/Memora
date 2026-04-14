use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::commands::notification;
use crate::error::AppError;
use crate::infra::{db::memora_pool, paths};
use crate::repo::persona_repo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProactiveRule {
    Idle { days: u32 },
    Daily { time: String },
    Date { month_day: String },
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HeartbeatState {
    last_triggered: HashMap<String, String>,
}

struct ProactivePersona {
    id: String,
    name: String,
    rules: Vec<ProactiveRule>,
    last_chat: Option<DateTime<Utc>>,
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(err) = check_once(&app).await {
                tracing::warn!("heartbeat_service check failed: {}", err);
            }
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

pub async fn check_once(app: &AppHandle) -> Result<(), AppError> {
    tracing::info!("heartbeat_service: starting hourly check");
    let personas = load_proactive_personas()?;
    if personas.is_empty() {
        tracing::info!("heartbeat_service: no proactive personas enabled");
        return Ok(());
    }

    let now = Utc::now();
    let mut state = load_state()?;

    for persona in personas {
        for rule in &persona.rules {
            let state_key = format!("{}:{}", persona.id, rule_state_key(rule));
            let last_triggered = state
                .last_triggered
                .get(&state_key)
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|dt| dt.with_timezone(&Utc));

            if should_trigger(rule, persona.last_chat, last_triggered, now) {
                let body = build_notification_body(&persona.name, rule);
                notification::send_proactive_notification(
                    app,
                    &persona.id,
                    format!("{} 想和你说句话", persona.name),
                    body,
                )?;
                state
                    .last_triggered
                    .insert(state_key, now.to_rfc3339());
                tracing::info!(
                    "heartbeat_service: triggered proactive notification for persona {} rule {:?}",
                    persona.id,
                    rule
                );
            }
        }
    }

    save_state(&state)?;
    tracing::info!("heartbeat_service: hourly check completed");
    Ok(())
}

pub fn should_trigger(
    rule: &ProactiveRule,
    last_chat: Option<DateTime<Utc>>,
    last_triggered: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    match rule {
        ProactiveRule::Idle { days } => {
            let Some(last_chat) = last_chat else {
                return false;
            };
            let threshold = chrono::Duration::days(*days as i64);
            if now - last_chat < threshold {
                return false;
            }

            match last_triggered {
                None => true,
                Some(triggered) => triggered < last_chat || now - triggered >= chrono::Duration::hours(24),
            }
        }
        ProactiveRule::Daily { time } => {
            let Some((hour, minute)) = parse_time(time) else {
                return false;
            };
            if now.hour() != hour || now.minute() < minute {
                return false;
            }

            match last_triggered {
                None => true,
                Some(triggered) => triggered.date_naive() != now.date_naive(),
            }
        }
        ProactiveRule::Date { month_day } => {
            let current = format!("{:02}-{:02}", now.month(), now.day());
            if &current != month_day {
                return false;
            }

            match last_triggered {
                None => true,
                Some(triggered) => triggered.year() != now.year(),
            }
        }
    }
}

fn load_proactive_personas() -> Result<Vec<ProactivePersona>, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    let rows = persona_repo::list_proactive_personas(&conn)?;

    let personas = rows
        .into_iter()
        .filter_map(|(id, name, rules_json, last_chat)| {
            let rules_json = rules_json?;
            let rules = serde_json::from_str::<Vec<ProactiveRule>>(&rules_json).ok()?;
            if rules.is_empty() {
                return None;
            }

            Some(ProactivePersona {
                id,
                name,
                rules,
                last_chat: last_chat
                    .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        })
        .collect();

    Ok(personas)
}

fn load_state() -> Result<HeartbeatState, AppError> {
    let path = paths::heartbeat_state_path();
    if !path.exists() {
        return Ok(HeartbeatState::default());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("读取 heartbeat state 失败: {}", path.display()))
        .map_err(AppError::Internal)?;
    serde_json::from_str(&content)
        .context("解析 heartbeat state 失败")
        .map_err(AppError::Internal)
}

fn save_state(state: &HeartbeatState) -> Result<(), AppError> {
    let path = paths::heartbeat_state_path();
    let content =
        serde_json::to_string_pretty(state).context("序列化 heartbeat state 失败").map_err(AppError::Internal)?;
    std::fs::write(&path, content)
        .with_context(|| format!("写入 heartbeat state 失败: {}", path.display()))
        .map_err(AppError::Internal)
}

fn build_notification_body(name: &str, rule: &ProactiveRule) -> String {
    match rule {
        ProactiveRule::Idle { days } => format!("{name} 已经 {days} 天没和你聊天了"),
        ProactiveRule::Daily { time } => format!("{name} 的定时问候已到：{time}"),
        ProactiveRule::Date { month_day } => format!("今天是 {month_day}，别忘了和 {name} 打个招呼"),
    }
}

fn rule_state_key(rule: &ProactiveRule) -> String {
    match rule {
        ProactiveRule::Idle { days } => format!("idle:{days}"),
        ProactiveRule::Daily { time } => format!("daily:{time}"),
        ProactiveRule::Date { month_day } => format!("date:{month_day}"),
    }
}

fn parse_time(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<u32>().ok()?;
    let minute = parts.next()?.parse::<u32>().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some((hour, minute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn idle_rule_triggers_after_threshold() {
        let now = Utc.with_ymd_and_hms(2026, 4, 14, 9, 0, 0).unwrap();
        let last_chat = Utc.with_ymd_and_hms(2026, 4, 1, 9, 0, 0).unwrap();
        assert!(should_trigger(
            &ProactiveRule::Idle { days: 7 },
            Some(last_chat),
            None,
            now
        ));
    }

    #[test]
    fn daily_rule_only_triggers_once_per_day() {
        let now = Utc.with_ymd_and_hms(2026, 4, 14, 9, 30, 0).unwrap();
        let fired = Utc.with_ymd_and_hms(2026, 4, 14, 9, 5, 0).unwrap();
        assert!(!should_trigger(
            &ProactiveRule::Daily {
                time: "09:00".into()
            },
            None,
            Some(fired),
            now
        ));
    }

    #[test]
    fn date_rule_matches_month_day() {
        let now = Utc.with_ymd_and_hms(2026, 4, 14, 12, 0, 0).unwrap();
        assert!(should_trigger(
            &ProactiveRule::Date {
                month_day: "04-14".into()
            },
            None,
            None,
            now
        ));
    }
}
