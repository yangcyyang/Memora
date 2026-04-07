//! TTS commands.

use crate::error::AppError;
use crate::infra::db::memora_pool;
use crate::repo::voice_repo;
use crate::tts::audio_utils;
use crate::tts::cache::AudioCache;
use crate::tts::provider::{self, CloneRequest, SynthesizeRequest, TtsProviderConfig};
use serde::Serialize;
use tauri::Emitter;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize)]
pub struct TtsSettingsResponse {
    pub active_provider: String,
    pub has_api_key: bool,
    pub api_key: String,
    pub group_id: String,
    pub default_language: String,
    pub cache_limit_mb: u64,
    pub cache_stats: crate::tts::cache::CacheStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonaVoiceResponse {
    pub persona_id: String,
    pub provider: String,
    pub voice_id: String,
    pub language: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceCloneResponse {
    pub voice_id: String,
}

#[tauri::command]
pub async fn get_tts_settings() -> Result<TtsSettingsResponse, AppError> {
    let config = provider::load_tts_config();
    let provider_config = config.providers.get(&config.active_provider).cloned().unwrap_or_default();
    let cache = AudioCache::new(config.cache_limit_mb);
    let stats = cache.stats().map_err(|e| AppError::Internal(e))?;

    let masked_key = if provider_config.api_key.is_empty() {
        String::new()
    } else {
        let k = &provider_config.api_key;
        let char_count = k.chars().count();
        if char_count > 8 {
            let prefix: String = k.chars().take(4).collect();
            let suffix: String = k.chars().skip(char_count - 4).collect();
            format!("{}...{}", prefix, suffix)
        } else {
            "****".to_string()
        }
    };

    Ok(TtsSettingsResponse {
        active_provider: config.active_provider,
        has_api_key: !provider_config.api_key.is_empty(),
        api_key: masked_key,
        group_id: provider_config.group_id.clone(),
        default_language: provider_config.default_language.clone(),
        cache_limit_mb: config.cache_limit_mb,
        cache_stats: stats,
    })
}

#[tauri::command]
pub async fn save_tts_settings(
    provider: String, api_key: String, group_id: String, language: String, cache_limit_mb: u64,
) -> Result<(), AppError> {
    if provider.trim().is_empty() {
        return Err(AppError::tts("TTS 服务商不能为空"));
    }
    let mut config = crate::tts::provider::load_tts_config();
    config.active_provider = provider.clone();
    config.cache_limit_mb = cache_limit_mb;

    let entry = config.providers.entry(provider).or_insert_with(TtsProviderConfig::default);
    if !api_key.trim().is_empty() {
        // Sanity check: API keys should only contain safe chars, never Chinese or error text
        if api_key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            entry.api_key = api_key;
        } else {
            return Err(AppError::tts("API 密钥格式无效，请检查输入"));
        }
    }
    if !group_id.trim().is_empty() { entry.group_id = group_id; }
    if !language.is_empty() { entry.default_language = language; }

    provider::save_tts_config(&config).map_err(|e| AppError::Internal(e))
}

#[tauri::command]
pub async fn list_tts_providers() -> Result<Vec<provider::ProviderInfo>, AppError> {
    Ok(provider::list_providers())
}

#[tauri::command]
#[tracing::instrument(err)]
pub async fn upload_and_clone_voice(persona_id: String, audio_path: String) -> Result<VoiceCloneResponse, AppError> {
    let config = provider::load_tts_config();
    let provider_id = config.active_provider.clone();
    let provider_config = config.providers.get(&provider_id).cloned().unwrap_or_default();
    let prov = provider::get_provider(&provider_id, &provider_config).map_err(|e| AppError::tts(e))?;

    if !prov.supports_voice_clone() { return Err(AppError::tts("当前 TTS 服务商不支持音色克隆")); }

    // If input is a video file, extract audio via ffmpeg first
    let (effective_path, temp_file) = audio_utils::ensure_audio_format(&audio_path)
        .await
        .map_err(|e| AppError::tts(e))?;

    info!("Uploading audio for clone: {} (original: {})", effective_path, audio_path);
    let upload_result = prov.upload_audio(&effective_path).await;

    // Clean up temp file regardless of upload success/failure
    if let Some(ref tmp) = temp_file {
        audio_utils::cleanup_temp_audio(tmp);
    }

    let file_id = upload_result.map_err(|e| AppError::tts(format!("上传音频失败: {}", e)))?;

    let custom_voice_id = format!("memora_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let clone_result = prov.clone_voice(CloneRequest {
        file_id, custom_voice_id: custom_voice_id.clone(),
        sample_text: "你好，很高兴认识你，这是一段测试语音。".to_string(),
        model: None, prompt_file_id: None, prompt_text: None,
    }).await.map_err(|e| AppError::tts(format!("音色克隆失败: {}", e)))?;

    let language = provider_config.default_language.clone();
    let pool = memora_pool();
    let conn = pool.get()?;
    voice_repo::set_voice(&conn, &persona_id, &provider_id, &clone_result.voice_id, &language, "")?;

    if let Some(demo) = &clone_result.demo_audio {
        let cache = AudioCache::new(config.cache_limit_mb);
        let _ = cache.put(&format!("demo_{}", clone_result.voice_id), demo);
    }

    info!("Voice clone complete: persona={}, voice_id={}", persona_id, clone_result.voice_id);
    Ok(VoiceCloneResponse { voice_id: clone_result.voice_id })
}

#[tauri::command]
pub async fn check_ffmpeg() -> Result<bool, AppError> {
    Ok(audio_utils::check_ffmpeg_available().await)
}

#[tauri::command]
pub async fn get_persona_voice(persona_id: String) -> Result<Option<PersonaVoiceResponse>, AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    let voice = voice_repo::get_voice(&conn, &persona_id)?;
    Ok(voice.map(|v| PersonaVoiceResponse {
        persona_id: v.persona_id, provider: v.provider, voice_id: v.voice_id,
        language: v.language, model: v.model,
    }))
}

#[tauri::command]
pub async fn set_persona_voice(persona_id: String, prov: String, voice_id: String, language: String) -> Result<(), AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    voice_repo::set_voice(&conn, &persona_id, &prov, &voice_id, &language, "")?;
    info!("Set persona voice: {} -> {}/{}", persona_id, prov, voice_id);
    Ok(())
}

#[tauri::command]
pub async fn remove_persona_voice(persona_id: String) -> Result<(), AppError> {
    let pool = memora_pool();
    let conn = pool.get()?;
    voice_repo::remove_voice(&conn, &persona_id)?;
    info!("Removed persona voice binding: {}", persona_id);
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip(text), err)]
pub async fn speak_text(text: String, persona_id: String) -> Result<String, AppError> {
    let config = provider::load_tts_config();
    let cache = AudioCache::new(config.cache_limit_mb);

    let pool = memora_pool();
    let conn = pool.get()?;
    let voice_row = voice_repo::get_voice_triple(&conn, &persona_id)?;
    drop(conn);

    let (provider_id, voice_id, language) = voice_row
        .ok_or_else(|| AppError::tts("该人物尚未绑定语音，请先在人物详情页设置音色"))?;

    let cache_key = AudioCache::cache_key(&provider_id, &voice_id, &text, &language);
    if let Some(cached_path) = cache.get(&cache_key) {
        debug!("TTS cache hit for persona {}", persona_id);
        return Ok(cached_path.to_string_lossy().to_string());
    }

    let provider_config = config.providers.get(&provider_id).cloned().unwrap_or_default();
    let prov = provider::get_provider(&provider_id, &provider_config).map_err(|e| AppError::tts(e))?;

    let audio_bytes = prov.synthesize(SynthesizeRequest {
        text, voice_id, language, speed: 1.0, model: None,
    }).await.map_err(|e| AppError::tts(format!("语音合成失败: {}", e)))?;

    let path = cache.put(&cache_key, &audio_bytes).map_err(|e| AppError::tts(format!("缓存音频失败: {}", e)))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
#[tracing::instrument(skip(text, app), err)]
pub async fn speak_text_stream(text: String, persona_id: String, app: tauri::AppHandle) -> Result<String, AppError> {
    let config = provider::load_tts_config();

    let pool = memora_pool();
    let conn = pool.get()?;
    let voice_row = voice_repo::get_voice_triple(&conn, &persona_id)?;
    drop(conn);

    let (provider_id, voice_id, language) = voice_row
        .ok_or_else(|| AppError::tts("该人物尚未绑定语音，请先在人物详情页设置音色"))?;

    let provider_config = config.providers.get(&provider_id).cloned().unwrap_or_default();
    let prov = provider::get_provider(&provider_id, &provider_config).map_err(|e| AppError::tts(e))?;

    if !prov.supports_streaming() {
        return Err(AppError::tts("当前 TTS 服务商不支持流式合成，请使用同步模式"));
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    let rid = request_id.clone();

    let app_handle = app.clone();
    let cache_limit = config.cache_limit_mb;
    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        let emitter = app_handle.clone();
        let rid2 = rid.clone();
        let receiver_task = tokio::spawn(async move {
            use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
            let mut all_bytes = Vec::new();

            while let Some(chunk) = rx.recv().await {
                all_bytes.extend_from_slice(&chunk);
                let b64 = BASE64.encode(&chunk);

                #[derive(Serialize, Clone)]
                struct AudioChunkPayload { request_id: String, chunk: String, is_final: bool }

                emitter.emit("tts://audio-chunk", AudioChunkPayload {
                    request_id: rid2.clone(), chunk: b64, is_final: false,
                }).ok();
            }

            #[derive(Serialize, Clone)]
            struct AudioChunkPayload { request_id: String, chunk: String, is_final: bool }
            emitter.emit("tts://audio-chunk", AudioChunkPayload {
                request_id: rid2.clone(), chunk: String::new(), is_final: true,
            }).ok();

            if !all_bytes.is_empty() {
                let cache = AudioCache::new(cache_limit);
                let _ = cache.put(&format!("stream_{}", rid2), &all_bytes);
            }
        });

        let result = prov.synthesize_stream(
            SynthesizeRequest { text, voice_id, language, speed: 1.0, model: None },
            tx,
        ).await;

        if let Err(e) = result { tracing::error!("TTS stream error: {}", e); }
        let _ = receiver_task.await;
    });

    Ok(request_id)
}

#[tauri::command]
pub async fn get_cache_stats() -> Result<crate::tts::cache::CacheStats, AppError> {
    let config = provider::load_tts_config();
    let cache = AudioCache::new(config.cache_limit_mb);
    cache.stats().map_err(|e| AppError::Internal(e))
}

#[tauri::command]
pub async fn clear_audio_cache() -> Result<(), AppError> {
    let config = provider::load_tts_config();
    let cache = AudioCache::new(config.cache_limit_mb);
    cache.clear().map_err(|e| AppError::Internal(e))
}
