import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { PROVIDER_OPTIONS } from "@/lib/constants";
import { saveAiSettings, validateKey } from "@/lib/tauri";
import { toast } from "sonner";

export function WelcomeView() {
  const navigate = useNavigate();
  const onComplete = () => navigate({ to: "/" });
  const [phase, setPhase] = useState<"splash" | "provider" | "key">("splash");
  const [provider, setProvider] = useState(PROVIDER_OPTIONS[0]);
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState(PROVIDER_OPTIONS[0].defaultUrl);
  const [model, setModel] = useState(PROVIDER_OPTIONS[0].defaultModel);
  const [validating, setValidating] = useState(false);

  const handleProviderSelect = (id: string) => {
    const p = PROVIDER_OPTIONS.find((o) => o.id === id)!;
    setProvider(p);
    setBaseUrl(p.defaultUrl);
    setModel(p.defaultModel);
    setPhase("key");
  };

  const handleSubmit = async () => {
    if (!apiKey.trim() && provider.id !== "local") {
      toast.error("请输入密钥");
      return;
    }
    setValidating(true);
    try {
      const valid = await validateKey(provider.id, apiKey, baseUrl, model);
      if (valid) {
        await saveAiSettings(provider.id, apiKey, baseUrl, model);
        toast.success("连接成功！");
        onComplete();
      } else {
        toast.error("密钥验证失败，请检查后重试");
      }
    } catch (e) {
      toast.error(`验证出错: ${e}`);
    } finally {
      setValidating(false);
    }
  };

  const handleSkip = async () => {
    try {
      await saveAiSettings(provider.id, apiKey, baseUrl, model);
      toast.success("已保存");
      onComplete();
    } catch (e) {
      toast.error(`保存失败: ${e}`);
    }
  };

  return (
    <div style={styles.container}>
      {/* Splash → Click to continue */}
      {phase === "splash" && (
        <div className="animate-fade-in" style={styles.splashContent}>
          <div style={styles.emojiOrb}>💜</div>
          <h1 className="text-hero" style={{ textAlign: "center", marginBottom: 12 }}>
            Memora
          </h1>
          <p className="text-display" style={styles.tagline}>
            把聊天记录变成永远陪伴你的 AI 伴侣
          </p>
          <button type="button" onClick={() => setPhase("provider")} style={styles.startBtn}>
            开始设置
          </button>
        </div>
      )}

      {/* Provider selection */}
      {phase === "provider" && (
        <div className="animate-slide-up" style={styles.formContent}>
          <p style={styles.stepLabel}>第一步</p>
          <h2 className="text-heading" style={{ marginBottom: 32 }}>
            选一个你有密钥的 AI 服务
          </h2>
          <div style={styles.providerGrid}>
            {PROVIDER_OPTIONS.map((p) => (
              <button
                key={p.id}
                type="button"
                onClick={() => handleProviderSelect(p.id)}
                style={styles.providerCard}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = "var(--color-rose-400)";
                  e.currentTarget.style.transform = "translateY(-2px)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = "var(--color-cream-300)";
                  e.currentTarget.style.transform = "translateY(0)";
                }}
              >
                <span style={{ fontSize: "1.1rem", fontWeight: 500 }}>{p.name}</span>
                <span className="text-caption">{p.description}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Key input */}
      {phase === "key" && (
        <div className="animate-slide-up" style={styles.formContent}>
          <button type="button" onClick={() => setPhase("provider")} style={styles.backLink}>
            ← 换一个服务
          </button>
          <h2 className="text-heading" style={{ marginBottom: 8 }}>
            设置 {provider.name}
          </h2>
          <p className="text-caption" style={{ marginBottom: 32 }}>
            密钥用 AES-256 加密保存在本地，不会发到任何服务器
          </p>

          <div style={styles.fieldGroup}>
            <label style={styles.fieldLabel}>API 密钥</label>
            <input
              type="password"
              placeholder={provider.id === "local" ? "本地模型无需密钥" : "sk-..."}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              style={styles.input}
              autoFocus
            />
            {provider.guideUrl && (
              <a href={provider.guideUrl} target="_blank" rel="noreferrer" style={styles.guideLink}>
                不知道怎么获取？看教程 →
              </a>
            )}
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.fieldLabel}>API 地址</label>
            <input type="text" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} style={styles.input} />
          </div>

          <div style={styles.fieldGroup}>
            <label style={styles.fieldLabel}>模型</label>
            <input type="text" value={model} onChange={(e) => setModel(e.target.value)} style={styles.input} />
          </div>

          <div style={{ display: "flex", gap: 12, marginTop: 32 }}>
            <button type="button" onClick={handleSubmit} disabled={validating} style={styles.primaryBtn}>
              {validating ? "验证中..." : "验证并保存"}
            </button>
            <button type="button" onClick={handleSkip} style={styles.ghostBtn}>
              跳过验证
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    width: "100%",
    height: "100vh",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: "var(--color-cream-50)",
  },
  splashContent: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 16,
  },
  emojiOrb: {
    width: 88,
    height: 88,
    borderRadius: "50%",
    background: "linear-gradient(135deg, var(--color-cream-200), var(--color-lavender-300))",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    fontSize: "2.8rem",
    marginBottom: 8,
    boxShadow: "var(--shadow-md)",
  },
  tagline: {
    textAlign: "center" as const,
    fontSize: "1.15rem",
    color: "var(--color-earth-500)",
    maxWidth: 340,
    lineHeight: 1.8,
  },
  startBtn: {
    marginTop: 24,
    padding: "14px 40px",
    background: "var(--color-rose-500)",
    color: "white",
    border: "none",
    borderRadius: "var(--radius-lg)",
    fontSize: "1rem",
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    transition: "all var(--duration-fast)",
  },
  formContent: {
    maxWidth: 480,
    width: "100%",
    padding: "0 24px",
  },
  stepLabel: {
    fontSize: "0.75rem",
    color: "var(--color-rose-500)",
    fontWeight: 600,
    textTransform: "uppercase" as const,
    letterSpacing: "0.1em",
    marginBottom: 8,
  },
  providerGrid: {
    display: "flex",
    flexDirection: "column" as const,
    gap: 12,
  },
  providerCard: {
    display: "flex",
    flexDirection: "column" as const,
    gap: 4,
    textAlign: "left" as const,
    padding: "20px 24px",
    border: "1.5px solid var(--color-cream-300)",
    borderRadius: "var(--radius-lg)",
    background: "var(--color-cream-100)",
    cursor: "pointer",
    transition: "all var(--duration-normal) var(--ease-out-quart)",
    fontFamily: "var(--font-body)",
  },
  backLink: {
    background: "none",
    border: "none",
    color: "var(--color-earth-500)",
    cursor: "pointer",
    fontSize: "0.85rem",
    marginBottom: 24,
    padding: 0,
    fontFamily: "var(--font-body)",
  },
  fieldGroup: {
    marginBottom: 20,
    display: "flex",
    flexDirection: "column" as const,
    gap: 6,
  },
  fieldLabel: {
    fontSize: "0.85rem",
    fontWeight: 500,
    color: "var(--color-earth-600)",
  },
  input: {
    padding: "12px 16px",
    border: "1.5px solid var(--color-cream-300)",
    borderRadius: "var(--radius-md)",
    background: "var(--color-cream-100)",
    fontSize: "0.95rem",
    color: "var(--color-earth-800)",
    fontFamily: "var(--font-body)",
    outline: "none",
    transition: "border-color var(--duration-fast)",
  },
  guideLink: {
    fontSize: "0.8rem",
    color: "var(--color-rose-500)",
    textDecoration: "none",
  },
  primaryBtn: {
    padding: "12px 28px",
    background: "var(--color-rose-500)",
    color: "white",
    border: "none",
    borderRadius: "var(--radius-md)",
    fontSize: "0.95rem",
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    transition: "all var(--duration-fast)",
  },
  ghostBtn: {
    padding: "12px 20px",
    background: "none",
    color: "var(--color-earth-500)",
    border: "none",
    fontSize: "0.85rem",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
  },
};
