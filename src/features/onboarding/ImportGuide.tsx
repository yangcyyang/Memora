import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { MessageSquare, Smartphone, FileText, ChevronRight, Check, ArrowRight } from "lucide-react";

interface ImportGuideProps {
  onComplete: () => void;
  onSkip: () => void;
}

type Platform = "wechat" | "qq";
type Step = 1 | 2 | 3;

const platformConfigs: Record<Platform, {
  name: string;
  icon: typeof MessageSquare;
  steps: { title: string; desc: string }[];
}> = {
  wechat: {
    name: "微信",
    icon: MessageSquare,
    steps: [
      { title: "打开对话", desc: "进入你想导出的聊天窗口，点击右上角「...」" },
      { title: "查找聊天记录", desc: "选择「查找聊天记录」→「图片及视频」" },
      { title: "导出到电脑", desc: "通过微信文件传输助手发送到电脑，保存为 .txt" },
    ],
  },
  qq: {
    name: "QQ",
    icon: Smartphone,
    steps: [
      { title: "打开消息管理", desc: "QQ 设置 → 安全设置 → 消息记录" },
      { title: "导出记录", desc: "选择联系人 → 导出消息记录 → 选择 TXT 格式" },
      { title: "保存文件", desc: "选择保存位置，建议创建专属文件夹" },
    ],
  },
};

export function ImportGuide({ onComplete, onSkip }: ImportGuideProps) {
  const navigate = useNavigate();
  const [platform, setPlatform] = useState<Platform | null>(null);
  const [currentStep, setCurrentStep] = useState<Step>(1);

  const handlePlatformSelect = (p: Platform) => {
    setPlatform(p);
    setCurrentStep(1);
  };

  const handleNextStep = () => {
    if (currentStep < 3) {
      setCurrentStep((prev) => (prev + 1) as Step);
    } else {
      onComplete();
    }
  };

  const handleBack = () => {
    if (currentStep > 1) {
      setCurrentStep((prev) => (prev - 1) as Step);
    } else {
      setPlatform(null);
    }
  };

  // Platform selection view
  if (!platform) {
    return (
      <div style={styles.container}>
        <div style={styles.content}>
          <button type="button" onClick={onSkip} style={styles.backLink}>
            ← 返回
          </button>

          <h1 className="text-hero" style={{ marginBottom: 8 }}>
            选择导入来源
          </h1>
          <p className="text-body" style={styles.subtitle}>
            我们支持微信和 QQ 的聊天记录导入
          </p>

          <div style={styles.platformGrid}>
            {(Object.keys(platformConfigs) as Platform[]).map((p) => {
              const config = platformConfigs[p];
              const Icon = config.icon;
              return (
                <button
                  key={p}
                  type="button"
                  onClick={() => handlePlatformSelect(p)}
                  style={styles.platformCard}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = "var(--color-rose-400)";
                    e.currentTarget.style.transform = "translateY(-2px)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = "var(--color-cream-300)";
                    e.currentTarget.style.transform = "translateY(0)";
                  }}
                >
                  <div style={styles.platformIcon}>
                    <Icon size={32} style={{ color: "var(--color-rose-500)" }} />
                  </div>
                  <span className="text-heading-small">{config.name}</span>
                  <span className="text-caption" style={{ color: "var(--color-earth-500)" }}>
                    3 步导出教程
                  </span>
                </button>
              );
            })}
          </div>

          <button type="button" onClick={onSkip} style={styles.skipLink}>
            已有导出文件，直接导入 →
          </button>
        </div>
      </div>
    );
  }

  // Step guide view
  const config = platformConfigs[platform];
  const Icon = config.icon;

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        {/* Header */}
        <div style={styles.guideHeader}>
          <button type="button" onClick={handleBack} style={styles.backLink}>
            ← 上一步
          </button>
          <div style={styles.platformBadge}>
            <Icon size={16} />
            <span className="text-caption">{config.name}导出教程</span>
          </div>
        </div>

        {/* Progress */}
        <div style={styles.progressBar}>
          {[1, 2, 3].map((step) => (
            <div
              key={step}
              style={{
                ...styles.progressSegment,
                background: step <= currentStep
                  ? "var(--color-rose-500)"
                  : "var(--color-cream-300)",
              }}
            />
          ))}
        </div>

        {/* Step Content */}
        <div style={styles.stepContent} className="animate-slide-up">
          <div style={styles.stepNumber}>
            <span className="text-caption" style={{ color: "white", fontWeight: 700 }}>
              {currentStep}
            </span>
          </div>

          <h2 className="text-heading" style={{ marginBottom: 12 }}>
            {config.steps[currentStep - 1].title}
          </h2>
          <p className="text-body" style={styles.stepDesc}>
            {config.steps[currentStep - 1].desc}
          </p>

          {/* Illustration placeholder */}
          <div style={styles.illustration}>
            <FileText size={48} style={{ color: "var(--color-cream-300)" }} />
            <span className="text-caption" style={{ color: "var(--color-earth-400)" }}>
              步骤示意图
            </span>
          </div>
        </div>

        {/* Navigation */}
        <div style={styles.navigation}>
          <button
            type="button"
            onClick={handleNextStep}
            style={styles.primaryBtn}
          >
            {currentStep === 3 ? "完成教程" : "下一步"}
            <ArrowRight size={18} />
          </button>

          {currentStep === 1 && (
            <button type="button" onClick={onSkip} style={styles.skipLinkSmall}>
              跳过教程，直接导入
            </button>
          )}
        </div>

        {/* Tips */}
        <div style={styles.tipsBox}>
          <div style={styles.tipsHeader}>
            <Check size={14} style={{ color: "var(--color-sage-500)" }} />
            <span className="text-caption" style={{ fontWeight: 500 }}>
              小提示
            </span>
          </div>
          <p className="text-small" style={styles.tipsText}>
            导出后的 .txt 文件可以直接拖拽到 Memora 中导入，
            我们支持自动识别格式并提取对话内容。
          </p>
        </div>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    width: "100%",
    minHeight: "100vh",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: "var(--color-cream-50)",
    padding: "40px 24px",
  },
  content: {
    maxWidth: 560,
    width: "100%",
  },
  backLink: {
    background: "none",
    border: "none",
    color: "var(--color-earth-500)",
    cursor: "pointer",
    fontSize: "0.9rem",
    marginBottom: 24,
    padding: 0,
    fontFamily: "var(--font-body)",
    display: "flex",
    alignItems: "center",
    gap: 4,
  },
  subtitle: {
    color: "var(--color-earth-500)",
    marginBottom: 32,
  },
  platformGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(2, 1fr)",
    gap: 16,
    marginBottom: 24,
  },
  platformCard: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 12,
    padding: 32,
    background: "var(--color-cream-100)",
    border: "1.5px solid var(--color-cream-300)",
    borderRadius: "var(--radius-lg)",
    cursor: "pointer",
    transition: "all var(--duration-normal) var(--ease-out-quart)",
    fontFamily: "var(--font-body)",
  },
  platformIcon: {
    width: 64,
    height: 64,
    borderRadius: "var(--radius-lg)",
    background: "var(--color-cream-50)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  },
  skipLink: {
    background: "none",
    border: "none",
    color: "var(--color-rose-500)",
    cursor: "pointer",
    fontSize: "0.9rem",
    fontFamily: "var(--font-body)",
    textDecoration: "underline",
    textUnderlineOffset: 4,
  },
  guideHeader: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    marginBottom: 24,
  },
  platformBadge: {
    display: "flex",
    alignItems: "center",
    gap: 6,
    padding: "6px 12px",
    background: "var(--color-cream-100)",
    borderRadius: "var(--radius-md)",
    color: "var(--color-earth-600)",
  },
  progressBar: {
    display: "flex",
    gap: 8,
    marginBottom: 32,
  },
  progressSegment: {
    flex: 1,
    height: 4,
    borderRadius: 2,
    transition: "all var(--duration-normal)",
  },
  stepContent: {
    textAlign: "center",
    marginBottom: 32,
  },
  stepNumber: {
    width: 40,
    height: 40,
    borderRadius: "50%",
    background: "var(--color-rose-500)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    margin: "0 auto 20px",
  },
  stepDesc: {
    color: "var(--color-earth-500)",
    maxWidth: 400,
    margin: "0 auto",
  },
  illustration: {
    width: "100%",
    height: 200,
    background: "var(--color-cream-100)",
    border: "1.5px dashed var(--color-cream-300)",
    borderRadius: "var(--radius-lg)",
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    gap: 12,
    marginTop: 24,
  },
  navigation: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 16,
    marginBottom: 24,
  },
  primaryBtn: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    padding: "14px 32px",
    background: "var(--color-rose-500)",
    color: "white",
    border: "none",
    borderRadius: "var(--radius-lg)",
    fontSize: "1rem",
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    transition: "all var(--duration-fast)",
    boxShadow: "var(--shadow-md)",
  },
  skipLinkSmall: {
    background: "none",
    border: "none",
    color: "var(--color-earth-500)",
    cursor: "pointer",
    fontSize: "0.85rem",
    fontFamily: "var(--font-body)",
  },
  tipsBox: {
    background: "var(--color-sage-400)/10",
    border: "1.5px solid var(--color-sage-400)",
    borderRadius: "var(--radius-lg)",
    padding: 16,
  },
  tipsHeader: {
    display: "flex",
    alignItems: "center",
    gap: 6,
    marginBottom: 6,
    color: "var(--color-sage-500)",
  },
  tipsText: {
    color: "var(--color-earth-500)",
    lineHeight: 1.6,
  },
};
