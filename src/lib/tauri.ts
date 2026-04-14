import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  BasicInfo,
  CacheStats,
  ChatMessage,
  CorrectionResult,
  GenerateResult,
  ParsedContent,
  Persona,
  PersonaSummary,
  PersonaVoice,
  SessionSummary,
  TtsProviderInfo,
  TtsSettings,
  UpdateCheckResult,
  VersionSummary,
  CalibrationSample,
  CalibrationFeedbackItem,
} from "@/types";

// ── Settings ──
export const getSettings = (provider?: string) => invoke<AppSettings>("get_settings", { provider });

// Note: saveAiSettings and validateKey are defined at the bottom of this file

// ── Parser & OCR ──
export const detectAndParse = (paths: string[]) =>
  invoke<Array<{ source: string; target_name: string | null; parsed: ParsedContent }>>(
    "detect_and_parse",
    { paths },
  );

export const parsePastedText = (text: string) =>
  invoke<ParsedContent>("parse_pasted_text", { text });

export const captureAndOcr = () => invoke<string>("capture_and_ocr");

// ── Persona ──
export const listPersonas = () => invoke<PersonaSummary[]>("list_personas");

export const getPersona = (id: string) => invoke<Persona>("get_persona", { id });

export const deletePersona = (id: string) => invoke("delete_persona", { id });
export const appendClipboardCorpus = (id: string, content: string) => invoke("append_clipboard_corpus", { id, content });

export const rollbackPersona = (id: string, version: number) =>
  invoke("rollback_persona", { id, version });

export const getPersonaVersions = (id: string) =>
  invoke<VersionSummary[]>("get_persona_versions", { id });

// ── Generator ──
export const generatePersona = (basicInfo: BasicInfo, parsedContents: ParsedContent[]) =>
  invoke<GenerateResult>("generate_persona", { basicInfo, parsedContents });

// ── Chat ──
export const sendMessage = (personaId: string, sessionId: string, content: string) =>
  invoke<string>("send_message", { personaId, sessionId, content });

export const getChatHistory = (personaId: string, sessionId?: string, limit?: number) =>
  invoke<ChatMessage[]>("get_chat_history", { personaId, sessionId, limit });

export const listChatSessions = (personaId: string) =>
  invoke<SessionSummary[]>("list_chat_sessions", { personaId });

export const newChatSession = (personaId: string) =>
  invoke<string>("new_chat_session", { personaId });

export const deleteChatSession = (personaId: string, sessionId: string) =>
  invoke("delete_chat_session", { personaId, sessionId });

// ── Correction ──
export const submitCorrection = (personaId: string, original: string, correction: string) =>
  invoke<CorrectionResult>("submit_correction", { personaId, original, correction });

export const reinforceMemory = (personaId: string, messageContent: string) =>
  invoke<{ success: boolean; version: number; rules: string[] }>("reinforce_memory", { personaId, messageContent });

// ── Calibration ──
export const generateCalibrationSamples = (personaId: string) =>
  invoke<CalibrationSample[]>("generate_calibration_samples", { personaId });

export const submitCalibrationFeedback = (personaId: string, feedbackItems: CalibrationFeedbackItem[], freeText?: string) =>
  invoke<{ success: boolean; version: number }>("submit_calibration_feedback", { personaId, feedbackItems, freeText });

// ── Bridge (Chrome Extension) ──
export const startWsBridge = (port?: number) =>
  invoke("start_ws_bridge", { port });

export const getWsBridgePort = () =>
  invoke<number>("get_ws_bridge_port");

export const toggleClipboardWatcher = (enabled: boolean) =>
  invoke("toggle_clipboard_watcher", { enabled });

// ── TTS (Voice) ──
export const getTtsSettings = () => invoke<TtsSettings>("get_tts_settings");

export const saveTtsSettings = (provider: string, apiKey: string, groupId: string, language: string, cacheLimitMb: number) =>
  invoke("save_tts_settings", { provider, apiKey, groupId, language, cacheLimitMb });

export const listTtsProviders = () => invoke<TtsProviderInfo[]>("list_tts_providers");

export const uploadAndCloneVoice = (personaId: string, audioPath: string) =>
  invoke<{ voice_id: string }>("upload_and_clone_voice", { personaId, audioPath });

export const getPersonaVoice = (personaId: string) =>
  invoke<PersonaVoice | null>("get_persona_voice", { personaId });

export const setPersonaVoice = (personaId: string, provider: string, voiceId: string, language: string) =>
  invoke("set_persona_voice", { personaId, provider, voiceId, language });

export const removePersonaVoice = (personaId: string) =>
  invoke("remove_persona_voice", { personaId });

export const speakText = (text: string, personaId: string) =>
  invoke<string>("speak_text", { text, personaId });

export const speakTextStream = (text: string, personaId: string) =>
  invoke<string>("speak_text_stream", { text, personaId });

export const getCacheStats = () => invoke<CacheStats>("get_cache_stats");

export const checkFfmpeg = () => invoke<boolean>("check_ffmpeg");

export const clearAudioCache = () => invoke("clear_audio_cache");

// ── Updater ──
export const checkAppUpdate = () => invoke<UpdateCheckResult>("check_app_update");

export const downloadAndInstallUpdate = () => invoke("download_and_install_update");

export const restartAfterUpdate = () => invoke("restart_after_update");

// ── Settings Extended ──
export const saveAiSettings = (provider: string, apiKey: string, baseUrl: string, model: string) =>
  invoke("save_settings", { provider, apiKey, baseUrl, model });

export const validateKey = (provider: string, apiKey: string, baseUrl: string, model: string) =>
  invoke<boolean>("validate_api_key", { provider, apiKey, baseUrl, model });

// ── Backup/Export ──
export const exportPersona = (personaId: string, outputPath: string) =>
  invoke<string>("export_persona", { personaId, outputPath });

// ── Proactive Settings ──
export const getProactiveSettings = (id: string) =>
  invoke<{ enabled: boolean; rules: string }>("get_proactive_settings", { id });

export const saveProactiveSettings = (id: string, enabled: boolean, rulesJson: string) =>
  invoke("save_proactive_settings", { id, enabled, rulesJson });

export const triggerProactiveTest = (personaId: string) =>
  invoke("send_notification", { title: "主动触达测试", body: "测试通知", personaId });

// ── STT ──
export const transcribeAudio = (audioBase64: string, mimeType: string) =>
  invoke<string>("transcribe_audio", { audioBase64, mimeType });
