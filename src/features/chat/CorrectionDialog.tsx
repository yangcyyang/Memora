import { useState } from "react";
import { submitCorrection } from "@/lib/tauri";
import { X, AlertTriangle } from "lucide-react";
import { toast } from "sonner";

interface Props {
  personaId: string;
  originalContent: string;
  onClose: () => void;
  onCorrected: () => void;
}

export function CorrectionDialog({ personaId, originalContent, onClose, onCorrected }: Props) {
  const [correction, setCorrection] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    if (!correction.trim()) return;
    setLoading(true);
    try {
      const result = await submitCorrection(personaId, originalContent, correction.trim());
      if (result.success) {
        toast.success(`已修正 → ${result.target === "memories" ? "记忆层" : "性格层"} (v${result.version})`);
        onCorrected();
        onClose();
      }
    } catch (e) {
      toast.error(`修正失败: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={styles.overlay} onClick={onClose}>
      <div style={styles.dialog} onClick={(e) => e.stopPropagation()}>
        <div style={styles.header}>
          <div style={styles.headerTitle}>
            <AlertTriangle size={16} style={{ color: "var(--color-amber-500)" }} />
            <span>纠正回复</span>
          </div>
          <button type="button" onClick={onClose} style={styles.closeBtn}>
            <X size={16} />
          </button>
        </div>

        <div style={styles.body}>
          <div style={styles.originalSection}>
            <label style={styles.label}>AI 的原始回复</label>
            <div style={styles.originalBox}>{originalContent.slice(0, 200)}{originalContent.length > 200 ? "…" : ""}</div>
          </div>

          <div style={styles.correctionSection}>
            <label style={styles.label}>你的修正意见</label>
            <textarea
              value={correction}
              onChange={(e) => setCorrection(e.target.value)}
              placeholder="例如：TA 不会说这么正式的话，TA 平时会用很多 emoji…"
              style={styles.textarea}
              rows={4}
              disabled={loading}
            />
          </div>

          <p style={styles.hint}>
            Memora 会自动判断这是性格层还是记忆层的修正，并更新底层设定
          </p>
        </div>

        <div style={styles.footer}>
          <button type="button" onClick={onClose} style={styles.cancelBtn} disabled={loading}>
            取消
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            disabled={!correction.trim() || loading}
            style={{
              ...styles.submitBtn,
              opacity: correction.trim() && !loading ? 1 : 0.5,
            }}
          >
            {loading ? "分析中…" : "提交修正"}
          </button>
        </div>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed",
    inset: 0,
    background: "rgba(0,0,0,0.25)",
    zIndex: 200,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    animation: "fade-in 200ms ease both",
  },
  dialog: {
    width: "90%",
    maxWidth: 460,
    background: "var(--color-cream-50)",
    borderRadius: "var(--radius-xl)",
    boxShadow: "var(--shadow-lg)",
    overflow: "hidden",
    animation: "scale-in 250ms var(--ease-out-expo) both",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "16px 20px",
    borderBottom: "1px solid var(--color-cream-200)",
  },
  headerTitle: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    fontWeight: 600,
    fontSize: "0.95rem",
    color: "var(--color-earth-700)",
  },
  closeBtn: {
    width: 28,
    height: 28,
    borderRadius: "var(--radius-sm)",
    border: "none",
    background: "none",
    color: "var(--color-earth-500)",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  },
  body: {
    padding: "16px 20px",
    display: "flex",
    flexDirection: "column",
    gap: 14,
  },
  originalSection: {
    display: "flex",
    flexDirection: "column",
    gap: 6,
  },
  label: {
    fontSize: "0.8rem",
    fontWeight: 500,
    color: "var(--color-earth-500)",
  },
  originalBox: {
    padding: "10px 14px",
    borderRadius: "var(--radius-md)",
    background: "var(--color-cream-200)",
    fontSize: "0.85rem",
    lineHeight: 1.6,
    color: "var(--color-earth-600)",
    maxHeight: 100,
    overflow: "auto",
  },
  correctionSection: {
    display: "flex",
    flexDirection: "column",
    gap: 6,
  },
  textarea: {
    width: "100%",
    padding: "10px 14px",
    borderRadius: "var(--radius-md)",
    border: "1.5px solid var(--color-cream-300)",
    background: "var(--color-cream-100)",
    fontFamily: "var(--font-body)",
    fontSize: "0.9rem",
    lineHeight: 1.6,
    color: "var(--color-earth-800)",
    outline: "none",
    resize: "vertical" as const,
  },
  hint: {
    fontSize: "0.75rem",
    color: "var(--color-earth-400)",
    lineHeight: 1.5,
  },
  footer: {
    display: "flex",
    justifyContent: "flex-end",
    gap: 10,
    padding: "12px 20px 16px",
    borderTop: "1px solid var(--color-cream-200)",
  },
  cancelBtn: {
    padding: "8px 18px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-cream-300)",
    background: "none",
    fontSize: "0.85rem",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-600)",
  },
  submitBtn: {
    padding: "8px 22px",
    borderRadius: "var(--radius-md)",
    border: "none",
    background: "var(--color-rose-500)",
    color: "white",
    fontSize: "0.85rem",
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    transition: "opacity var(--duration-fast)",
  },
};
