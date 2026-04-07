//! Audio utility functions: ffmpeg detection, video-to-audio extraction.

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Known audio extensions (case-insensitive check done by caller).
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "m4a", "flac", "ogg", "aac", "wma", "opus"];

/// Known video extensions.
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "avi", "webm", "flv", "wmv", "ts", "m4v"];

/// Returns `true` if the file extension indicates a video format.
pub fn is_video_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Returns `true` if the file extension indicates an audio format.
pub fn is_audio_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check whether `ffmpeg` is available on the system PATH.
pub async fn check_ffmpeg_available() -> bool {
    match tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
    {
        Ok(status) => {
            let available = status.success();
            debug!("ffmpeg available: {}", available);
            available
        }
        Err(_) => {
            debug!("ffmpeg not found on PATH");
            false
        }
    }
}

/// Extract the audio track from a video file using ffmpeg.
///
/// - Outputs an mp3 file to `~/.memora/audio/tmp_extract_<uuid>.mp3`.
/// - Returns the path to the extracted audio file.
/// - Caller is responsible for cleaning up the temp file via [`cleanup_temp_audio`].
pub async fn extract_audio_from_video(video_path: &str) -> Result<PathBuf> {
    if !check_ffmpeg_available().await {
        anyhow::bail!(
            "系统未安装 ffmpeg，无法从视频中提取音频。\n\
             请先安装 ffmpeg：\n\
             • macOS: brew install ffmpeg\n\
             • Windows: winget install ffmpeg\n\
             • Linux: sudo apt install ffmpeg"
        );
    }

    let output_dir = crate::infra::paths::audio_cache_dir();
    std::fs::create_dir_all(&output_dir).context("Failed to create audio cache directory")?;

    let tmp_name = format!("tmp_extract_{}.mp3", uuid::Uuid::new_v4());
    let output_path = output_dir.join(&tmp_name);

    info!(
        "Extracting audio from video: {} -> {}",
        video_path,
        output_path.display()
    );

    let output = tokio::process::Command::new("ffmpeg")
        .args([
            "-i",
            video_path,
            "-vn",           // no video
            "-acodec",
            "libmp3lame",    // encode to mp3
            "-ab",
            "192k",          // bitrate
            "-ar",
            "44100",         // sample rate
            "-ac",
            "1",             // mono
            "-y",            // overwrite
            output_path.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .context("Failed to execute ffmpeg")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("ffmpeg stderr: {}", stderr);
        // Clean up partial output
        let _ = std::fs::remove_file(&output_path);
        return Err(anyhow!(
            "ffmpeg 提取音频失败: {}",
            stderr.lines().last().unwrap_or("unknown error")
        ));
    }

    // Verify output exists and has content
    let metadata = std::fs::metadata(&output_path)
        .context("ffmpeg output file not found after extraction")?;
    if metadata.len() == 0 {
        let _ = std::fs::remove_file(&output_path);
        anyhow::bail!("ffmpeg 提取的音频文件为空，视频可能没有音轨");
    }

    info!(
        "Audio extraction complete: {} bytes",
        metadata.len()
    );
    Ok(output_path)
}

/// If the input is a video file, extract audio via ffmpeg and return the
/// extracted mp3 path. If it's already an audio file, return `None` (use original).
///
/// Returns `(effective_audio_path, temp_file_to_cleanup)`.
pub async fn ensure_audio_format(path: &str) -> Result<(String, Option<PathBuf>)> {
    if is_video_file(path) {
        let extracted = extract_audio_from_video(path).await?;
        let audio_path = extracted.to_string_lossy().to_string();
        Ok((audio_path, Some(extracted)))
    } else if is_audio_file(path) {
        Ok((path.to_string(), None))
    } else {
        Err(anyhow!(
            "不支持的文件格式。请上传音频（mp3/wav/m4a/flac/ogg）或视频（mp4/mov/mkv/avi/webm）文件。"
        ))
    }
}

/// Remove a temporary extracted audio file.
pub fn cleanup_temp_audio(path: &Path) {
    if path.exists() {
        match std::fs::remove_file(path) {
            Ok(()) => debug!("Cleaned up temp audio: {}", path.display()),
            Err(e) => warn!("Failed to clean up temp audio {}: {}", path.display(), e),
        }
    }
}
