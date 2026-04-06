use crate::core::ai_provider;
use crate::core::models::AppSettings;

#[tauri::command]
pub async fn get_settings(provider: Option<String>) -> Result<AppSettings, String> {
    let config = ai_provider::load_config();
    
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
        } else {
            let key = &api_key;
            if key.len() > 8 {
                format!("{}...{}", &key[..4], &key[key.len()-4..])
            } else {
                "****".to_string()
            }
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
) -> Result<(), String> {
    let api_format = match provider.as_str() {
        "anthropic" => ai_provider::ApiFormat::Anthropic,
        "local" => ai_provider::ApiFormat::Local,
        _ => ai_provider::ApiFormat::Openai,
    };

    let mut config = ai_provider::load_config();
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

    let active_provider_config = ai_provider::ProviderConfig {
        base_url: base_url.clone(),
        api_key: final_api_key.clone(),
        model: model.clone(),
    };

    config.providers.insert(provider.clone(), active_provider_config);
    
    config.base_url = base_url;
    config.model = model;
    config.api_key = final_api_key;

    ai_provider::save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_api_key(
    provider: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<bool, String> {
    let api_format = match provider.as_str() {
        "anthropic" => ai_provider::ApiFormat::Anthropic,
        "local" => ai_provider::ApiFormat::Local,
        _ => ai_provider::ApiFormat::Openai,
    };

    let config = ai_provider::AiConfig {
        enabled: true,
        api_format,
        base_url,
        api_key,
        model,
        providers: std::collections::HashMap::new(),
    };

    ai_provider::validate_key(&config)
        .await
        .map_err(|e| e.to_string())
}
