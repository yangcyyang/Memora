import { useNavigate } from "@tanstack/react-router";
import { Heart, MessageCircle, Sparkles, Shield } from "lucide-react";

interface RoleTypeIntroProps {
  onNext: () => void;
}

const roleExamples = [
  {
    icon: MessageCircle,
    title: "老朋友",
    description: "多年的挚友，了解你的喜怒哀乐",
    example: "最近又熬夜了？记得多喝水",
    color: "var(--color-sage-500)",
  },
  {
    icon: Heart,
    title: "亲密伴侣",
    description: "知心爱人，细腻体贴的陪伴",
    example: "今天工作顺利吗，想听听",
    color: "var(--color-rose-500)",
  },
  {
    icon: Sparkles,
    title: "虚拟偶像",
    description: "二次元角色，活泼可爱的互动",
    example: "今天也要元气满满哦~",
    color: "var(--color-lavender-500)",
  },
];

export function RoleTypeIntro({ onNext }: RoleTypeIntroProps) {
  const navigate = useNavigate();

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        {/* Header */}
        <div style={styles.header}>
          <h1 className="text-hero" style={{ marginBottom: 12 }}>
            你想和什么样的角色对话？
          </h1>
          <p className="text-body" style={styles.subtitle}>
            从聊天记录中，Memora 会学习 TA 的语气、习惯和记忆
          </p>
        </div>

        {/* Role Examples */}
        <div style={styles.roleGrid}>
          {roleExamples.map((role, index) => (
            <div
              key={role.title}
              style={{
                ...styles.roleCard,
                animationDelay: `${index * 100}ms`,
              }}
              className="animate-slide-up"
            >
              <div
                style={{
                  ...styles.iconWrapper,
                  background: `linear-gradient(135deg, ${role.color}20, ${role.color}10)`,
                }}
              >
                <role.icon size={24} style={{ color: role.color }} />
              </div>
              <h3 className="text-heading-small" style={{ marginBottom: 4 }}>
                {role.title}
              </h3>
              <p className="text-caption" style={styles.roleDesc}>
                {role.description}
              </p>
              <div style={styles.exampleBubble}>
                <span className="text-caption" style={{ color: "var(--color-earth-500)" }}>
                  "{role.example}"
                </span>
              </div>
            </div>
          ))}
        </div>

        {/* Privacy Notice */}
        <div style={styles.privacyBox} className="animate-fade-in">
          <div style={styles.privacyHeader}>
            <Shield size={18} style={{ color: "var(--color-sage-500)" }} />
            <span className="text-caption" style={{ fontWeight: 600 }}>
              隐私承诺
            </span>
          </div>
          <p className="text-small" style={styles.privacyText}>
            所有数据仅存储在你的设备上，不会上传至任何云端服务器。
            聊天记录经过加密处理，你可以随时导出或彻底删除。
          </p>
        </div>

        {/* Actions */}
        <div style={styles.actions}>
          <button
            type="button"
            onClick={onNext}
            style={styles.primaryBtn}
          >
            开始导入聊天记录
          </button>
          <button
            type="button"
            onClick={() => navigate({ to: "/create" })}
            style={styles.ghostBtn}
          >
            跳过导入，直接创建角色 →
          </button>
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
    maxWidth: 720,
    width: "100%",
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
  },
  header: {
    textAlign: "center",
    marginBottom: 40,
  },
  subtitle: {
    color: "var(--color-earth-500)",
    maxWidth: 400,
    margin: "0 auto",
    lineHeight: 1.6,
  },
  roleGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
    gap: 16,
    width: "100%",
    marginBottom: 32,
  },
  roleCard: {
    background: "var(--color-cream-100)",
    border: "1.5px solid var(--color-cream-300)",
    borderRadius: "var(--radius-lg)",
    padding: 24,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    textAlign: "center",
    transition: "all var(--duration-normal) var(--ease-out-quart)",
  },
  iconWrapper: {
    width: 56,
    height: 56,
    borderRadius: "var(--radius-lg)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    marginBottom: 16,
  },
  roleDesc: {
    color: "var(--color-earth-500)",
    marginBottom: 12,
    lineHeight: 1.5,
  },
  exampleBubble: {
    background: "white",
    padding: "10px 14px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-cream-300)",
    fontStyle: "italic",
  },
  privacyBox: {
    background: "var(--color-sage-400)/10",
    border: "1.5px solid var(--color-sage-400)",
    borderRadius: "var(--radius-lg)",
    padding: 20,
    width: "100%",
    marginBottom: 32,
  },
  privacyHeader: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    marginBottom: 8,
    color: "var(--color-sage-500)",
  },
  privacyText: {
    color: "var(--color-earth-500)",
    lineHeight: 1.6,
  },
  actions: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 16,
  },
  primaryBtn: {
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
  ghostBtn: {
    padding: "10px 20px",
    background: "none",
    color: "var(--color-earth-500)",
    border: "none",
    fontSize: "0.9rem",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    transition: "all var(--duration-fast)",
  },
};
