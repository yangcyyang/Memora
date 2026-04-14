import { useState, useEffect } from "react";
import { toast } from "sonner";
import { Clock, RotateCcw, Check, AlertCircle } from "lucide-react";
import { getPersonaVersions, rollbackPersona } from "@/lib/tauri";

interface Version {
  version: number;
  created_at: string;
  isCurrent?: boolean;
}

interface MemoryVersionListProps {
  personaId: string;
  currentVersion: number;
  onRollback?: () => void;
}

export function MemoryVersionList({ 
  personaId, 
  currentVersion,
  onRollback 
}: MemoryVersionListProps) {
  const [versions, setVersions] = useState<Version[]>([]);
  const [loading, setLoading] = useState(true);
  const [rollingBack, setRollingBack] = useState<number | null>(null);

  useEffect(() => {
    loadVersions();
  }, [personaId]);

  const loadVersions = async () => {
    try {
      setLoading(true);
      const result = await getPersonaVersions(personaId);
      // Sort by version desc, mark current
      const sorted = result
        .sort((a, b) => b.version - a.version)
        .map(v => ({
          ...v,
          isCurrent: v.version === currentVersion
        }));
      setVersions(sorted);
    } catch (e) {
      toast.error(`加载版本失败: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleRollback = async (version: number) => {
    if (version === currentVersion) {
      toast.info("已经是当前版本");
      return;
    }

    const confirmed = window.confirm(
      `确定要回退到版本 ${version} 吗？\n当前的人格定义和记忆将被替换为历史版本。`
    );
    
    if (!confirmed) return;

    setRollingBack(version);
    try {
      await rollbackPersona(personaId, version);
      toast.success(`已回退到版本 ${version}`);
      onRollback?.();
      // Reload versions
      await loadVersions();
    } catch (e) {
      toast.error(`回退失败: ${e}`);
    } finally {
      setRollingBack(null);
    }
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleDateString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit"
    });
  };

  if (loading) {
    return (
      <div style={styles.container}>
        <div style={styles.loading}>加载版本历史中...</div>
      </div>
    );
  }

  if (versions.length === 0) {
    return (
      <div style={styles.container}>
        <div style={styles.empty}>
          <AlertCircle size={24} style={{ color: "var(--color-earth-400)" }} />
          <span>暂无版本历史</span>
        </div>
      </div>
    );
  }

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <Clock size={16} />
        <span>版本历史（7天快照）</span>
      </div>
      
      <div style={styles.versionList}>
        {versions.map((version) => (
          <div 
            key={version.version}
            style={{
              ...styles.versionItem,
              ...(version.isCurrent ? styles.currentVersion : {})
            }}
          >
            <div style={styles.versionInfo}>
              <div style={styles.versionBadge}>
                {version.isCurrent ? (
                  <>
                    <Check size={12} />
                    <span>当前</span>
                  </>
                ) : (
                  <span>v{version.version}</span>
                )}
              </div>
              <span style={styles.versionDate}>
                {formatDate(version.created_at)}
              </span>
            </div>
            
            {!version.isCurrent && (
              <button
                type="button"
                onClick={() => handleRollback(version.version)}
                disabled={rollingBack === version.version}
                style={styles.rollbackBtn}
              >
                {rollingBack === version.version ? (
                  "回退中..."
                ) : (
                  <>
                    <RotateCcw size={14} />
                    回退
                  </>
                )}
              </button>
            )}
          </div>
        ))}
      </div>
      
      <div style={styles.tips}>
        <AlertCircle size={12} />
        <span>每7天自动创建快照，可一键回退到任意历史版本</span>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: "16px 18px",
  },
  header: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    fontSize: "0.9rem",
    fontWeight: 600,
    color: "var(--color-earth-700)",
    marginBottom: 12,
  },
  loading: {
    textAlign: "center",
    padding: "20px",
    color: "var(--color-earth-500)",
  },
  empty: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 8,
    padding: "20px",
    color: "var(--color-earth-500)",
  },
  versionList: {
    display: "flex",
    flexDirection: "column",
    gap: 8,
  },
  versionItem: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "10px 12px",
    background: "var(--color-cream-50)",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-cream-200)",
  },
  currentVersion: {
    background: "var(--color-sage-400)/10",
    borderColor: "var(--color-sage-400)",
  },
  versionInfo: {
    display: "flex",
    alignItems: "center",
    gap: 12,
  },
  versionBadge: {
    display: "flex",
    alignItems: "center",
    gap: 4,
    padding: "4px 10px",
    background: "var(--color-cream-200)",
    borderRadius: "var(--radius-full)",
    fontSize: "0.75rem",
    fontWeight: 500,
    color: "var(--color-earth-600)",
  },
  versionDate: {
    fontSize: "0.8rem",
    color: "var(--color-earth-500)",
  },
  rollbackBtn: {
    display: "flex",
    alignItems: "center",
    gap: 4,
    padding: "6px 12px",
    background: "none",
    border: "1px solid var(--color-cream-300)",
    borderRadius: "var(--radius-sm)",
    fontSize: "0.8rem",
    color: "var(--color-earth-600)",
    cursor: "pointer",
    transition: "all var(--duration-fast)",
  },
  tips: {
    display: "flex",
    alignItems: "center",
    gap: 6,
    marginTop: 12,
    padding: "10px 12px",
    background: "var(--color-cream-100)",
    borderRadius: "var(--radius-md)",
    fontSize: "0.75rem",
    color: "var(--color-earth-500)",
  },
};
