use base64::{engine::general_purpose, Engine};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

#[derive(Debug, thiserror::Error)]
enum RecordError {
    #[error("No input device available")]
    NoDevice,
    #[error("Failed to build input stream: {0}")]
    BuildStream(String),
    #[error("Recording not started")]
    NotRecording,
}

struct RecordingState {
    stop_sender: std::sync::mpsc::Sender<()>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

lazy_static::lazy_static! {
    static ref RECORDING_STATE: Arc<Mutex<Option<RecordingState>>> = Arc::new(Mutex::new(None));
}

#[tauri::command]
pub async fn start_recording(_app: AppHandle) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| RecordError::NoDevice.to_string())?;

    let config = device
        .default_input_config()
        .map_err(|e| RecordError::BuildStream(e.to_string()).to_string())?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = buffer.clone();

    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();

    let sample_format = config.sample_format();
    let config: cpal::StreamConfig = config.into();

    std::thread::spawn(move || {
        let stream_result = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = buffer_clone.lock() {
                        buf.extend_from_slice(data);
                    }
                },
                move |err| tracing::error!("Recording error: {}", err),
                None,
            ),
            cpal::SampleFormat::I16 => {
                let b = buffer_clone.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = b.lock() {
                            buf.extend(data.iter().map(|s| *s as f32 / i16::MAX as f32));
                        }
                    },
                    move |err| tracing::error!("Recording error: {}", err),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let b = buffer_clone.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = b.lock() {
                            buf.extend(data.iter().map(|s| (*s as f32 / u16::MAX as f32) * 2.0 - 1.0));
                        }
                    },
                    move |err| tracing::error!("Recording error: {}", err),
                    None,
                )
            }
            _ => Err(cpal::BuildStreamError::StreamConfigNotSupported),
        };

        match stream_result {
            Ok(stream) => {
                if let Err(e) = stream.play() {
                    tracing::error!("Failed to play stream: {}", e);
                    return;
                }
                // Keep stream alive until stop signal received
                let _ = stop_rx.recv();
            }
            Err(e) => {
                tracing::error!("Failed to build stream: {}", e);
            }
        }
    });

    let state = RecordingState {
        stop_sender: stop_tx,
        buffer,
        sample_rate,
        channels,
    };

    let mut recording_state = RECORDING_STATE.lock().map_err(|e| e.to_string())?;
    *recording_state = Some(state);

    tracing::info!("Recording started: {} Hz, {} channels", sample_rate, channels);
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(_app: AppHandle) -> Result<String, String> {
    let state = {
        let mut recording_state = RECORDING_STATE.lock().map_err(|e| e.to_string())?;
        recording_state.take().ok_or_else(|| RecordError::NotRecording.to_string())?
    };

    // Send stop signal to drop the stream
    let _ = state.stop_sender.send(());

    let samples = state.buffer.lock().map_err(|e| e.to_string())?;
    let samples = samples.clone();

    if samples.is_empty() {
        return Err("No audio data recorded".to_string());
    }

    // Convert f32 samples to i16 for WAV
    let i16_samples: Vec<i16> = samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();

    let mut wav_bytes = Vec::new();
    {
        let spec = hound::WavSpec {
            channels: state.channels,
            sample_rate: state.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(std::io::Cursor::new(&mut wav_bytes), spec)
            .map_err(|e| format!("WAV write error: {}", e))?;
        for sample in &i16_samples {
            writer.write_sample(*sample).map_err(|e| format!("WAV sample error: {}", e))?;
        }
        writer.finalize().map_err(|e| format!("WAV finalize error: {}", e))?;
    }

    let base64_audio = general_purpose::STANDARD.encode(&wav_bytes);
    tracing::info!(
        "Recording stopped: {} samples -> {} bytes WAV -> {} base64 chars",
        samples.len(),
        wav_bytes.len(),
        base64_audio.len()
    );

    Ok(base64_audio)
}
