import { useState, useEffect } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { ArrowLeft, Eye, EyeOff, Check, AlertCircle, Server, Key, Sparkles, Power, Sun, Moon, Monitor } from "lucide-react";
import { getSettings, saveAiSettings, validateKey } from "@/lib/tauri";
import { useTheme } from "@/hooks/useTheme";
// import type { AppSettings } from "@/types";

const AI_PROVIDERS = [
  { id: "openai", name: "OpenAI", defaultBaseUrl: "https://api.openai.com/v1", models: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"] },
  { id: "anthropic", name: "Anthropic", defaultBaseUrl: "https://api.anthropic.com/v1", models: ["claude-3-5-sonnet-20241022", "claude-3-opus-20240229", "claude-3-haiku-20240307"] },
  { id: "minimax", name: "MiniMax", defaultBaseUrl: "https://api.minimax.chat/v1", models: ["abab6.5s-chat", "abab6-chat"] },
  { id: "deepseek", name: "DeepSeek", defaultBaseUrl: "https://api.deepseek.com/v1", models: ["deepseek-chat", "deepseek-coder"] },
  { id: "ollama", name: "本地 Ollama", defaultBaseUrl: "http://localhost:11434", models: ["llama3.1", "qwen2.5", "gemma2", "phi4"] },
];

type Mode = "managed" | "advanced";

export function SettingsView() {
  const navigate = useNavigate();
  const { mode: themeMode, setMode: setThemeMode } = useTheme();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [autostartLoading, setAutostartLoading] = useState(true);
  const [autostartSaving, setAutostartSaving] = useState(false);
  
  // Mode selection
  const [mode, setMode] = useState<Mode>("managed");
  
  // Form state
  const [provider, setProvider] = useState("openai");
  const [apiKey, setApiKey] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  
  // Validation state
  const [keyValid, setKeyValid] = useState<boolean | null>(null);
  const [hasExistingKey, setHasExistingKey] = useState(false);

  // Load existing settings
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const settings = await getSettings();
        if (settings.provider) {
          setProvider(settings.provider);
          setBaseUrl(settings.base_url || "");
          setModel(settings.model || "");
          setHasExistingKey(settings.has_api_key);
          if (settings.has_api_key) {
            setMode("advanced");
          }
        }
      } catch (e) {
        console.error("Failed to load settings:", e);
      } finally {
        setLoading(false);
      }
    };
    loadSettings();
  }, []);

  useEffect(() => {
    const loadAutostartState = async () => {
      try {
        const enabled = await invoke<boolean>("plugin:autostart|is_enabled");
        setAutostartEnabled(enabled);
      } catch (e) {
        console.warn("Failed to load autostart state:", e);
      } finally {
        setAutostartLoading(false);
      }
    };
    loadAutostartState();
  }, []);

  // Auto-set default base URL and model when provider changes
  useEffect(() => {
    const providerConfig = AI_PROVIDERS.find(p => p.id === provider);
    if (providerConfig) {
      setBaseUrl(providerConfig.defaultBaseUrl);
      setModel(providerConfig.models[0]);
    }
  }, [provider]);

  const handleValidateKey = async () => {
    if (!apiKey.trim()) {
      toast.error("请输入 API Key");
      return;
    }
    setValidating(true);
    setKeyValid(null);
    try {
      const isValid = await validateKey(provider, apiKey, baseUrl, model);
      setKeyValid(isValid);
      if (isValid) {
        toast.success("API Key 验证成功");
      } else {
        toast.error("API Key 验证失败");
      }
    } catch (e) {
      toast.error(`验证出错: ${e}`);
      setKeyValid(false);
    } finally {
      setValidating(false);
    }
  };

  const handleSave = async () => {
    if (mode === "advanced" && !hasExistingKey && !apiKey.trim()) {
      toast.error("请输入 API Key");
      return;
    }
    
    setSaving(true);
    try {
      await saveAiSettings(provider, apiKey, baseUrl, model);
      toast.success("设置已保存");
      setHasExistingKey(true);
      setKeyValid(null);
    } catch (e) {
      toast.error(`保存失败: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleToggleAutostart = async () => {
    setAutostartSaving(true);
    try {
      if (autostartEnabled) {
        await invoke("plugin:autostart|disable");
        setAutostartEnabled(false);
        toast.success("已关闭开机自启");
      } else {
        await invoke("plugin:autostart|enable");
        setAutostartEnabled(true);
        toast.success("已开启开机自启");
      }
    } catch (e) {
      toast.error(`切换开机自启失败: ${e}`);
    } finally {
      setAutostartSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={styles.container}>
        <div style={styles.loading}>加载中...</div>
      </div>
    );
  }

  return (
    <div style={styles.container}>
      {/* Header */}
      <header style={styles.header}>
        <button type="button" onClick={() => navigate({ to: "/" })} style={styles.backBtn}>
          <ArrowLeft size={18} />
          <span>返回</span>
        </button>
        <h1 style={styles.title}>设置</h1>
        <div style={{ width: 80 }} />
      </header>

      <main style={styles.main}>
        <div style={styles.content}>
          {/* Mode Selection */}
          <section style={styles.section}>
            <h2 style={styles.sectionTitle}>使用模式</h2>
            <div style={styles.modeSelector}>
              <button
                type="button"
                onClick={() => setMode("managed")}
                style={{
                  ...styles.modeCard,
                  borderColor: mode === "managed" ? "var(--color-rose-400)" : "var(--color-cream-300)",
                  background: mode === "managed" ? "var(--color-rose-50)" : "white",
                }}
              >
                <Sparkles size={20} style={{ color: "var(--color-rose-400)", marginBottom: 8 }} />
                <div style={styles.modeName}>初级版</div>
                <div style={styles.modeDesc}>平台托管 API，开箱即用</div>
              </button>
              <button
                type="button"
                onClick={() => setMode("advanced")}
                style={{
                  ...styles.modeCard,
                  borderColor: mode === "advanced" ? "var(--color-sage-400)" : "var(--color-cream-300)",
                  background: mode === "advanced" ? "var(--color-sage-50)" : "white",
                }}
              >
                <Key size={20} style={{ color: "var(--color-sage-500)", marginBottom: 8 }} />
                <div style={styles.modeName}>高级版</div>
                <div style={styles.modeDesc}>自绑 API Key，更多控制</div>
              </button>
            </div>
          </section>

          {mode === "advanced" && (
            <>
              {/* Provider Selection */}
              <section style={styles.section}>
                <h2 style={styles.sectionTitle}>
                  <Server size={16} style={{ marginRight: 6 }} />
                  AI 提供商
                </h2>
                <div style={styles.providerGrid}>
                  {AI_PROVIDERS.map((p) => (
                    <button
                      key={p.id}
                      type="button"
                      onClick={() => setProvider(p.id)}
                      style={{
                        ...styles.providerCard,
                        borderColor: provider === p.id ? "var(--color-rose-400)" : "var(--color-cream-300)",
                        background: provider === p.id ? "var(--color-rose-50)" : "white",
                      }}
                    >
                      <div style={styles.providerName}>{p.name}</div>
                    </button>
                  ))}
                </div>
              </section>

              {/* API Key */}
              <section style={styles.section}>
                <h2 style={styles.sectionTitle}>
                  <Key size={16} style={{ marginRight: 6 }} />
                  API Key
                  {hasExistingKey && (
                    <span style={styles.badge}>已配置</span>
                  )}
                </h2>
                <div style={styles.inputGroup}>
                  <div style={styles.keyInputWrapper}>
                    <input
                      type={showApiKey ? "text" : "password"}
                      placeholder={hasExistingKey ? "••••••••••••••••" : "输入你的 API Key"}
                      value={apiKey}
                      onChange={(e) => {
                        setApiKey(e.target.value);
                        setKeyValid(null);
                      }}
                      style={styles.keyInput}
                    />
                    <button
                      type="button"
                      onClick={() => setShowApiKey(!showApiKey)}
                      style={styles.eyeBtn}
                    >
                      {showApiKey ? <EyeOff size={18} /> : <Eye size={18} />}
                    </button>
                  </div>
                  <div style={styles.keyActions}>
                    <button
                      type="button"
                      onClick={handleValidateKey}
                      disabled={validating || !apiKey.trim()}
                      style={{
                        ...styles.secondaryBtn,
                        opacity: validating || !apiKey.trim() ? 0.5 : 1,
                      }}
                    >
                      {validating ? "验证中..." : "验证 Key"}
                    </button>
                    {keyValid === true && (
                      <span style={styles.validBadge}>
                        <Check size={14} /> 有效
                      </span>
                    )}
                    {keyValid === false && (
                      <span style={styles.invalidBadge}>
                        <AlertCircle size={14} /> 无效
                      </span>
                    )}
                  </div>
                </div>
              </section>

              {/* Ollama / Base URL Config */}
              <section style={styles.section}>
                <h2 style={styles.sectionTitle}>
                  <Server size={16} style={{ marginRight: 6 }} />
                  {provider === "ollama" ? "Ollama 配置" : "高级配置"}
                </h2>
                
                <div style={styles.fieldGroup}>
                  <label style={styles.fieldLabel}>Base URL</label>
                  <input
                    type="text"
                    value={baseUrl}
                    onChange={(e) => setBaseUrl(e.target.value)}
                    placeholder="https://api.example.com/v1"
                    style={styles.input}
                  />
                  <p style={styles.fieldHint}>
                    {provider === "ollama" 
                      ? "本地 Ollama 默认地址: http://localhost:11434" 
                      : "一般不需要修改，除非使用代理或自建服务"}
                  </p>
                </div>

                <div style={styles.fieldGroup}>
                  <label style={styles.fieldLabel}>模型</label>
                  <select
                    value={model}
                    onChange={(e) => setModel(e.target.value)}
                    style={styles.select}
                  >
                    {AI_PROVIDERS.find(p => p.id === provider)?.models.map(m => (
                      <option key={m} value={m}>{m}</option>
                    ))}
                  </select>
                </div>
              </section>
            </>
          )}

          {mode === "managed" && (
            <section style={styles.section}>
              <div style={styles.managedInfo}>
                <Sparkles size={32} style={{ color: "var(--color-rose-400)", marginBottom: 12 }} />
                <h3 style={styles.managedTitle}>使用平台托管服务</h3>
                <p style={styles.managedDesc}>
                  你不需要配置 API Key，平台会为你管理 AI 服务。
                  <br />
                  适合快速开始，无需担心额度或配置。
                </p>
              </div>
            </section>
          )}

          {/* Appearance Settings */}
          <section style={styles.section}>
            <h2 style={styles.sectionTitle}>
              <Sun size={16} style={{ marginRight: 6 }} />
              外观
            </h2>
            <div style={styles.appearanceGrid}>
              <button
                type="button"
                onClick={() => setThemeMode("light")}
                style={{
                  ...styles.appearanceCard,
                  borderColor: themeMode === "light" ? "var(--color-rose-400)" : "var(--color-cream-300)",
                  background: themeMode === "light" ? "var(--color-rose-50)" : "white",
                }}
              >
                <Sun size={20} style={{ color: "var(--color-amber-500)", marginBottom: 8 }} />
                <div style={styles.appearanceName}>浅色</div>
              </button>
              <button
                type="button"
                onClick={() => setThemeMode("dark")}
                style={{
                  ...styles.appearanceCard,
                  borderColor: themeMode === "dark" ? "var(--color-rose-400)" : "var(--color-cream-300)",
                  background: themeMode === "dark" ? "var(--color-rose-50)" : "white",
                }}
              >
                <Moon size={20} style={{ color: "var(--color-lavender-500)", marginBottom: 8 }} />
                <div style={styles.appearanceName}>深色</div>
              </button>
              <button
                type="button"
                onClick={() => setThemeMode("system")}
                style={{
                  ...styles.appearanceCard,
                  borderColor: themeMode === "system" ? "var(--color-rose-400)" : "var(--color-cream-300)",
                  background: themeMode === "system" ? "var(--color-rose-50)" : "white",
                }}
              >
                <Monitor size={20} style={{ color: "var(--color-earth-500)", marginBottom: 8 }} />
                <div style={styles.appearanceName}>跟随系统</div>
              </button>
            </div>
          </section>

          {/* System Settings */}
          <section style={styles.section}>
            <h2 style={styles.sectionTitle}>
              <Power size={16} style={{ marginRight: 6 }} />
              系统设置
            </h2>
            <div style={styles.systemCard}>
              <div>
                <div style={styles.systemTitle}>开机自启</div>
                <div style={styles.systemDesc}>
                  启动系统后自动打开 Memora，便于后续常驻托盘和主动提醒。
                </div>
              </div>
              <button
                type="button"
                onClick={handleToggleAutostart}
                disabled={autostartLoading || autostartSaving}
                style={{
                  ...styles.toggleBtn,
                  background: autostartEnabled ? "var(--color-sage-500)" : "var(--color-cream-300)",
                  opacity: autostartLoading || autostartSaving ? 0.6 : 1,
                }}
              >
                <span
                  style={{
                    ...styles.toggleKnob,
                    transform: autostartEnabled ? "translateX(22px)" : "translateX(0)",
                  }}
                />
              </button>
            </div>
            <p style={styles.fieldHint}>
              {autostartLoading ? "正在读取状态..." : autostartEnabled ? "当前已开启" : "当前未开启"}
            </p>
          </section>

          {/* Save Button */}
          <section style={styles.section}>
            <button
              type="button"
              onClick={handleSave}
              disabled={saving}
              style={{
                ...styles.primaryBtn,
                width: "100%",
                opacity: saving ? 0.6 : 1,
              }}
            >
              {saving ? "保存中..." : "保存设置"}
            </button>
          </section>
        </div>
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { width: "100%", height: "100vh", display: "flex", flexDirection: "column", overflow: "hidden" },
  loading: { display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--color-earth-500)" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "16px 24px", flexShrink: 0, borderBottom: "1px solid var(--color-cream-300)" },
  backBtn: { display: "flex", alignItems: "center", gap: 6, background: "none", border: "none", color: "var(--color-earth-500)", cursor: "pointer", fontSize: "0.85rem", fontFamily: "var(--font-body)" },
  title: { fontSize: "1.2rem", fontWeight: 600, color: "var(--color-earth-800)" },
  main: { flex: 1, overflow: "auto", display: "flex", justifyContent: "center", padding: "32px 24px" },
  content: { maxWidth: 560, width: "100%" },
  section: { marginBottom: 32 },
  sectionTitle: { fontSize: "0.9rem", fontWeight: 600, color: "var(--color-earth-700)", marginBottom: 12, display: "flex", alignItems: "center" },
  badge: { marginLeft: 8, padding: "2px 8px", background: "var(--color-sage-400)", color: "white", borderRadius: "var(--radius-full)", fontSize: "0.7rem" },
  
  // Mode selector
  modeSelector: { display: "flex", gap: 12 },
  modeCard: { flex: 1, padding: "20px 16px", border: "2px solid", borderRadius: "var(--radius-lg)", cursor: "pointer", transition: "all var(--duration-fast)" },
  modeName: { fontSize: "1rem", fontWeight: 600, color: "var(--color-earth-800)", marginBottom: 4 },
  modeDesc: { fontSize: "0.8rem", color: "var(--color-earth-500)" },
  
  // Provider grid
  providerGrid: { display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8 },
  providerCard: { padding: "14px 12px", border: "2px solid", borderRadius: "var(--radius-md)", cursor: "pointer", transition: "all var(--duration-fast)", textAlign: "center" },
  providerName: { fontSize: "0.85rem", fontWeight: 500 },
  
  // Input styles
  inputGroup: { display: "flex", flexDirection: "column", gap: 12 },
  keyInputWrapper: { position: "relative", display: "flex" },
  keyInput: { flex: 1, padding: "12px 44px 12px 16px", border: "1.5px solid var(--color-cream-300)", borderRadius: "var(--radius-md)", background: "var(--color-cream-100)", fontSize: "0.9rem", color: "var(--color-earth-800)", fontFamily: "var(--font-body)", outline: "none" },
  eyeBtn: { position: "absolute", right: 12, top: "50%", transform: "translateY(-50%)", background: "none", border: "none", cursor: "pointer", color: "var(--color-earth-500)", padding: 4 },
  keyActions: { display: "flex", alignItems: "center", gap: 12 },
  secondaryBtn: { padding: "8px 16px", background: "var(--color-cream-200)", color: "var(--color-earth-700)", border: "none", borderRadius: "var(--radius-md)", fontSize: "0.85rem", cursor: "pointer", fontFamily: "var(--font-body)" },
  validBadge: { display: "flex", alignItems: "center", gap: 4, color: "var(--color-sage-500)", fontSize: "0.85rem" },
  invalidBadge: { display: "flex", alignItems: "center", gap: 4, color: "var(--color-rose-500)", fontSize: "0.85rem" },
  
  // Field styles
  fieldGroup: { marginBottom: 16 },
  fieldLabel: { display: "block", fontSize: "0.85rem", fontWeight: 500, color: "var(--color-earth-600)", marginBottom: 6 },
  input: { width: "100%", padding: "12px 16px", border: "1.5px solid var(--color-cream-300)", borderRadius: "var(--radius-md)", background: "var(--color-cream-100)", fontSize: "0.9rem", color: "var(--color-earth-800)", fontFamily: "var(--font-body)", outline: "none", boxSizing: "border-box" },
  select: { width: "100%", padding: "12px 16px", border: "1.5px solid var(--color-cream-300)", borderRadius: "var(--radius-md)", background: "var(--color-cream-100)", fontSize: "0.9rem", color: "var(--color-earth-800)", fontFamily: "var(--font-body)", outline: "none", cursor: "pointer" },
  fieldHint: { marginTop: 6, fontSize: "0.8rem", color: "var(--color-earth-500)" },
  
  // Managed mode info
  managedInfo: { padding: "40px 24px", textAlign: "center", background: "var(--color-cream-100)", borderRadius: "var(--radius-lg)", border: "2px dashed var(--color-cream-300)" },
  managedTitle: { fontSize: "1.1rem", fontWeight: 600, color: "var(--color-earth-800)", marginBottom: 8 },
  managedDesc: { fontSize: "0.9rem", color: "var(--color-earth-600)", lineHeight: 1.6 },
  systemCard: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 16, padding: "16px 18px", borderRadius: "var(--radius-lg)", border: "1.5px solid var(--color-cream-300)", background: "white" },
  systemTitle: { fontSize: "0.95rem", fontWeight: 600, color: "var(--color-earth-800)", marginBottom: 4 },
  systemDesc: { fontSize: "0.82rem", color: "var(--color-earth-500)", lineHeight: 1.5, maxWidth: 360 },
  toggleBtn: { width: 52, height: 30, borderRadius: 999, border: "none", padding: 4, position: "relative", cursor: "pointer", transition: "background var(--duration-fast)" },
  toggleKnob: { width: 22, height: 22, borderRadius: "50%", background: "white", display: "block", transition: "transform var(--duration-fast)" },
  
  // Primary button
  primaryBtn: { padding: "14px 28px", background: "var(--color-rose-500)", color: "white", border: "none", borderRadius: "var(--radius-md)", fontSize: "1rem", fontWeight: 500, cursor: "pointer", fontFamily: "var(--font-body)", transition: "all var(--duration-fast)" },
  
  // Appearance
  appearanceGrid: { display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 },
  appearanceCard: { padding: "16px 12px", border: "2px solid", borderRadius: "var(--radius-md)", cursor: "pointer", transition: "all var(--duration-fast)", textAlign: "center" },
  appearanceName: { fontSize: "0.85rem", fontWeight: 500, color: "var(--color-earth-700)" },
};
