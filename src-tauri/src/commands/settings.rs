//! Settings commands.

use crate::ai::config as ai_config;
use crate::error::AppError;
use crate::models::AppSettings;

#[tauri::command]
pub async fn get_settings(provider: Option<String>) -> Result<AppSettings, AppError> {
    let config = ai_config::load_config();

    let target_provider = provider.unwrap_or_else(|| format!("{:?}", config.api_format).to_lowercase());

    let (api_key, base_url, model) = if let Some(p) = config.providers.get(&target_provider) {
        (p.api_key.clone(), p.base_url.clone(), p.model.clone())
    } else if format!("{:?}", config.api_format).to_lowercase() == target_provider {
        (config.api_key.clone(), config.base_url.clone(), config.model.clone())
    } else {
        (String::new(), String::new(), String::new())
    };

    Ok(AppSettings {
        provider: target_provider,
        api_key: if api_key.is_empty() {
            String::new()
        } else if api_key.len() > 8 {
            format!("{}...{}", &api_key[..4], &api_key[api_key.len()-4..])
        } else {
            "****".to_string()
        },
        base_url,
        model,
        has_api_key: !api_key.is_empty(),
    })
}

#[tauri::command]
pub async fn save_settings(
    provider: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<(), AppError> {
    let api_format = match provider.as_str() {
        "anthropic" => ai_config::ApiFormat::Anthropic,
        "local" => ai_config::ApiFormat::Local,
        _ => ai_config::ApiFormat::Openai,
    };

    let mut config = ai_config::load_config();
    config.enabled = true;
    config.api_format = api_format;

    let final_api_key = if api_key.trim().is_empty() {
        if let Some(existing) = config.providers.get(&provider) {
            existing.api_key.clone()
        } else if format!("{:?}", config.api_format).to_lowercase() == provider {
            config.api_key.clone()
        } else {
            String::new()
        }
    } else {
        api_key
    };

    let active_provider_config = ai_config::ProviderConfig {
        base_url: base_url.clone(),
        api_key: final_api_key.clone(),
        model: model.clone(),
    };

    config.providers.insert(provider, active_provider_config);
    config.base_url = base_url;
    config.model = model;
    config.api_key = final_api_key;

    ai_config::save_config(&config).map_err(|e| AppError::Internal(e))
}

#[tauri::command]
pub async fn validate_api_key(
    provider: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<bool, AppError> {
    let api_format = match provider.as_str() {
        "anthropic" => ai_config::ApiFormat::Anthropic,
        "local" => ai_config::ApiFormat::Local,
        _ => ai_config::ApiFormat::Openai,
    };

    let config = ai_config::AiConfig {
        enabled: true,
        api_format,
        base_url,
        api_key,
        model,
        providers: std::collections::HashMap::new(),
    };

    crate::ai::validation::validate_key(&config)
        .await
        .map_err(|e| AppError::ai(e))
}
