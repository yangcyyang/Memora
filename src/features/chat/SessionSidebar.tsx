import { useCallback, useEffect, useState } from "react";
import { listChatSessions, newChatSession, deleteChatSession } from "@/lib/tauri";
import type { SessionSummary } from "@/types";
import { Plus, Trash2, MessageCircle, X } from "lucide-react";
import { toast } from "sonner";

interface Props {
  personaId: string;
  activeSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onClose: () => void;
}

export function SessionSidebar({ personaId, activeSessionId, onSelectSession, onClose }: Props) {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);

  const refresh = useCallback(async () => {
    try {
      const list = await listChatSessions(personaId);
      setSessions(list);
    } catch {
      // ignore
    }
  }, [personaId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleCreate = async () => {
    try {
      const sid = await newChatSession(personaId);
      await refresh();
      onSelectSession(sid);
    } catch (e) {
      toast.error(`创建失败: ${e}`);
    }
  };

  const handleDelete = async (e: React.MouseEvent, sessionId: string) => {
    e.stopPropagation();
    if (sessionId === activeSessionId) {
      toast.error("不能删除当前会话");
      return;
    }
    try {
      await deleteChatSession(personaId, sessionId);
      await refresh();
      toast.success("已删除");
    } catch (err) {
      toast.error(`删除失败: ${err}`);
    }
  };

  return (
    <div style={styles.overlay} onClick={onClose}>
      <aside style={styles.sidebar} onClick={(e) => e.stopPropagation()}>
        <div style={styles.sidebarHeader}>
          <span style={styles.sidebarTitle}>历史会话</span>
          <button type="button" onClick={onClose} style={styles.closeBtn}>
            <X size={16} />
          </button>
        </div>

        <button type="button" onClick={handleCreate} style={styles.newSessionBtn}>
          <Plus size={16} />
          <span>新建会话</span>
        </button>

        <div style={styles.sessionList}>
          {sessions.length === 0 ? (
            <div style={styles.emptyHint}>暂无历史会话</div>
          ) : (
            sessions.map((s) => (
              <button
                key={s.session_id}
                type="button"
                onClick={() => onSelectSession(s.session_id)}
                style={{
                  ...styles.sessionItem,
                  ...(s.session_id === activeSessionId ? styles.sessionActive : {}),
                }}
              >
                <div style={styles.sessionContent}>
                  <div style={styles.sessionTop}>
                    <MessageCircle size={13} style={{ opacity: 0.5, flexShrink: 0 }} />
                    <span style={styles.sessionPreview}>
                      {s.preview?.slice(0, 40) || "空会话"}
                    </span>
                  </div>
                  <div style={styles.sessionMeta}>
                    <span>{s.message_count} 条消息</span>
                    <span>·</span>
                    <span>{formatRelative(s.last_message_at)}</span>
                  </div>
                </div>
                {s.session_id !== activeSessionId && (
                  <button
                    type="button"
                    onClick={(e) => handleDelete(e, s.session_id)}
                    style={styles.deleteBtn}
                    title="删除会话"
                  >
                    <Trash2 size={12} />
                  </button>
                )}
              </button>
            ))
          )}
        </div>
      </aside>
    </div>
  );
}

function formatRelative(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "刚刚";
  if (mins < 60) return `${mins}分钟前`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}小时前`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}天前`;
  return `${Math.floor(days / 30)}个月前`;
}

const styles: Record<string, React.CSSProperties> = {
  overlay: {
    position: "fixed",
    inset: 0,
    background: "rgba(0,0,0,0.18)",
    zIndex: 100,
    display: "flex",
    animation: "fade-in 200ms ease both",
  },
  sidebar: {
    width: 280,
    height: "100vh",
    background: "var(--color-cream-50)",
    borderRight: "1px solid var(--color-cream-200)",
    display: "flex",
    flexDirection: "column",
    animation: "slide-in-left 300ms var(--ease-out-expo) both",
    boxShadow: "var(--shadow-lg)",
  },
  sidebarHeader: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "20px 16px 12px",
    borderBottom: "1px solid var(--color-cream-200)",
  },
  sidebarTitle: {
    fontFamily: "var(--font-display)",
    fontSize: "1.05rem",
    fontWeight: 500,
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
  newSessionBtn: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    margin: "12px 12px 0",
    padding: "10px 14px",
    borderRadius: "var(--radius-md)",
    border: "1.5px dashed var(--color-cream-300)",
    background: "none",
    color: "var(--color-earth-500)",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    fontSize: "0.85rem",
    transition: "all var(--duration-fast)",
  },
  sessionList: {
    flex: 1,
    overflow: "auto",
    padding: "8px 12px 16px",
    display: "flex",
    flexDirection: "column",
    gap: 4,
  },
  emptyHint: {
    textAlign: "center",
    padding: "32px 0",
    color: "var(--color-earth-400)",
    fontSize: "0.85rem",
  },
  sessionItem: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    padding: "10px 12px",
    borderRadius: "var(--radius-md)",
    border: "none",
    background: "none",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    textAlign: "left",
    transition: "background var(--duration-fast)",
    width: "100%",
  },
  sessionActive: {
    background: "var(--color-cream-200)",
  },
  sessionContent: {
    flex: 1,
    minWidth: 0,
    display: "flex",
    flexDirection: "column",
    gap: 3,
  },
  sessionTop: {
    display: "flex",
    alignItems: "center",
    gap: 6,
  },
  sessionPreview: {
    fontSize: "0.85rem",
    color: "var(--color-earth-700)",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  sessionMeta: {
    display: "flex",
    gap: 4,
    fontSize: "0.72rem",
    color: "var(--color-earth-400)",
    paddingLeft: 19,
  },
  deleteBtn: {
    width: 24,
    height: 24,
    borderRadius: "var(--radius-sm)",
    border: "none",
    background: "none",
    color: "var(--color-earth-400)",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    flexShrink: 0,
    opacity: 0.5,
    transition: "opacity var(--duration-fast)",
  },
};
