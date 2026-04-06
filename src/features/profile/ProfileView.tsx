import { useEffect, useState } from "react";
import { getPersona, deletePersona, getPersonaVersions, rollbackPersona } from "@/lib/tauri";
import { invoke } from "@tauri-apps/api/core";
import type { Persona, VersionSummary } from "@/types";
import { ArrowLeft, Trash2, RotateCcw, ChevronDown, ChevronUp, Save, X, Pencil } from "lucide-react";
import { toast } from "sonner";

interface Props {
  personaId: string;
  onBack: () => void;
  onChat: () => void;
  onDeleted: () => void;
}

export function ProfileView({ personaId, onBack, onChat, onDeleted }: Props) {
  const [persona, setPersona] = useState<Persona | null>(null);
  const [versions, setVersions] = useState<VersionSummary[]>([]);
  const [showPersonaMd, setShowPersonaMd] = useState(true);
  const [showMemoriesMd, setShowMemoriesMd] = useState(false);
  const [showVersions, setShowVersions] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Editable states
  const [editDesc, setEditDesc] = useState(false);
  const [draftDesc, setDraftDesc] = useState("");
  const [editTags, setEditTags] = useState(false);
  const [draftTags, setDraftTags] = useState("");
  const [editPersonaMd, setEditPersonaMd] = useState(false);
  const [draftPersonaMd, setDraftPersonaMd] = useState("");
  const [editMemoriesMd, setEditMemoriesMd] = useState(false);
  const [draftMemoriesMd, setDraftMemoriesMd] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const p = await getPersona(personaId);
        setPersona(p);
        const v = await getPersonaVersions(personaId);
        setVersions(v);
      } catch (e) {
        toast.error(`加载失败: ${e}`);
      }
    })();
  }, [personaId]);

  const handleDelete = async () => {
    try {
      await deletePersona(personaId);
      toast.success("已删除");
      onDeleted();
    } catch (e) {
      toast.error(`删除失败: ${e}`);
    }
  };

  const handleRollback = async (version: number) => {
    try {
      await rollbackPersona(personaId, version);
      const p = await getPersona(personaId);
      setPersona(p);
      toast.success(`已回滚到 v${version}`);
    } catch (e) {
      toast.error(`回滚失败: ${e}`);
    }
  };

  const saveField = async (field: string, value: string) => {
    if (!persona) return;
    setSaving(true);
    try {
      await invoke("update_persona_field", { id: personaId, field, value });
      const p = await getPersona(personaId);
      setPersona(p);
      toast.success("已保存");
    } catch (e) {
      toast.error(`保存失败: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  if (!persona) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", width: "100%", height: "100vh" }}>
        <span className="text-muted">加载中...</span>
      </div>
    );
  }

  const tags: string[] = (() => {
    try { return JSON.parse(persona.tags_json); } catch { return []; }
  })();

  return (
    <div style={styles.container}>
      <header style={styles.header}>
        <button type="button" onClick={onBack} style={styles.backBtn}>
          <ArrowLeft size={18} />
          <span>返回</span>
        </button>
      </header>

      <main style={styles.main}>
        {/* Hero */}
        <div style={styles.hero}>
          <div style={styles.avatar}>{persona.avatar_emoji}</div>
          <h1 className="text-heading" style={{ marginTop: 16 }}>{persona.name}</h1>

          {/* Description — editable */}
          {editDesc ? (
            <div style={styles.editRow}>
              <textarea
                value={draftDesc}
                onChange={(e) => setDraftDesc(e.target.value)}
                style={styles.editTextarea}
                rows={2}
                autoFocus
                disabled={saving}
              />
              <div style={styles.editActions}>
                <button type="button" style={styles.saveBtn} disabled={saving} onClick={async () => {
                  await saveField("description", draftDesc);
                  setEditDesc(false);
                }}>
                  <Save size={13} /> 保存
                </button>
                <button type="button" style={styles.cancelEditBtn} onClick={() => setEditDesc(false)}>
                  <X size={13} />
                </button>
              </div>
            </div>
          ) : (
            <div style={styles.editableField} onClick={() => { setDraftDesc(persona.description); setEditDesc(true); }}>
              <p className="text-muted" style={{ marginTop: 6 }}>
                {persona.description || "点击添加描述"}
              </p>
              <Pencil size={12} style={{ color: "var(--color-earth-400)", flexShrink: 0 }} />
            </div>
          )}

          {/* Tags — editable */}
          {editTags ? (
            <div style={{ ...styles.editRow, marginTop: 10 }}>
              <input
                value={draftTags}
                onChange={(e) => setDraftTags(e.target.value)}
                style={styles.editInput}
                placeholder="用逗号分隔标签"
                autoFocus
                disabled={saving}
              />
              <div style={styles.editActions}>
                <button type="button" style={styles.saveBtn} disabled={saving} onClick={async () => {
                  const newTags = draftTags.split(/[,，]/).map(t => t.trim()).filter(Boolean);
                  await saveField("tags_json", JSON.stringify(newTags));
                  setEditTags(false);
                }}>
                  <Save size={13} /> 保存
                </button>
                <button type="button" style={styles.cancelEditBtn} onClick={() => setEditTags(false)}>
                  <X size={13} />
                </button>
              </div>
            </div>
          ) : (
            <div
              style={{ ...styles.editableField, marginTop: 8, cursor: "pointer" }}
              onClick={() => { setDraftTags(tags.join(", ")); setEditTags(true); }}
            >
              {tags.length > 0 ? (
                <div style={styles.tagRow}>
                  {tags.map((t) => (
                    <span key={t} style={styles.tag}>{t}</span>
                  ))}
                </div>
              ) : (
                <span className="text-caption">点击添加标签</span>
              )}
              <Pencil size={12} style={{ color: "var(--color-earth-400)", flexShrink: 0 }} />
            </div>
          )}

          <div style={{ display: "flex", gap: 12, marginTop: 24 }}>
            <button type="button" onClick={onChat} style={styles.primaryBtn}>
              开始聊天
            </button>
            <span className="text-caption" style={{ alignSelf: "center" }}>
              v{persona.version}
            </span>
          </div>
        </div>

        {/* Content sections grid */}
        <div style={styles.contentGrid}>
          {/* Persona Markdown — editable */}
          <section style={styles.section}>
            <button type="button" onClick={() => setShowPersonaMd(!showPersonaMd)} style={styles.sectionHeader}>
              <span style={styles.sectionTitle}>人物性格</span>
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                {showPersonaMd && !editPersonaMd && (
                  <span
                    style={styles.editToggle}
                    onClick={(e) => { e.stopPropagation(); setDraftPersonaMd(persona.persona_md); setEditPersonaMd(true); }}
                  >
                    <Pencil size={12} /> 编辑
                  </span>
                )}
                {showPersonaMd ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              </div>
            </button>
            {showPersonaMd && (
              editPersonaMd ? (
                <div style={styles.mdEditContainer}>
                  <textarea
                    value={draftPersonaMd}
                    onChange={(e) => setDraftPersonaMd(e.target.value)}
                    style={styles.mdTextarea}
                    rows={20}
                    disabled={saving}
                  />
                  <div style={styles.mdEditActions}>
                    <button type="button" style={styles.saveBtn} disabled={saving} onClick={async () => {
                      await saveField("persona_md", draftPersonaMd);
                      setEditPersonaMd(false);
                    }}>
                      <Save size={13} /> {saving ? "保存中…" : "保存"}
                    </button>
                    <button type="button" style={styles.cancelEditBtn} onClick={() => setEditPersonaMd(false)}>
                      <X size={13} /> 取消
                    </button>
                  </div>
                </div>
              ) : (
                <div style={styles.mdContent}>
                  {persona.persona_md || "暂无性格数据"}
                </div>
              )
            )}
          </section>

          {/* Memories Markdown — editable */}
          <section style={styles.section}>
            <button type="button" onClick={() => setShowMemoriesMd(!showMemoriesMd)} style={styles.sectionHeader}>
              <span style={styles.sectionTitle}>共同回忆</span>
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                {showMemoriesMd && !editMemoriesMd && (
                  <span
                    style={styles.editToggle}
                    onClick={(e) => { e.stopPropagation(); setDraftMemoriesMd(persona.memories_md); setEditMemoriesMd(true); }}
                  >
                    <Pencil size={12} /> 编辑
                  </span>
                )}
                {showMemoriesMd ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              </div>
            </button>
            {showMemoriesMd && (
              editMemoriesMd ? (
                <div style={styles.mdEditContainer}>
                  <textarea
                    value={draftMemoriesMd}
                    onChange={(e) => setDraftMemoriesMd(e.target.value)}
                    style={styles.mdTextarea}
                    rows={20}
                    disabled={saving}
                  />
                  <div style={styles.mdEditActions}>
                    <button type="button" style={styles.saveBtn} disabled={saving} onClick={async () => {
                      await saveField("memories_md", draftMemoriesMd);
                      setEditMemoriesMd(false);
                    }}>
                      <Save size={13} /> {saving ? "保存中…" : "保存"}
                    </button>
                    <button type="button" style={styles.cancelEditBtn} onClick={() => setEditMemoriesMd(false)}>
                      <X size={13} /> 取消
                    </button>
                  </div>
                </div>
              ) : (
                <div style={styles.mdContent}>
                  {persona.memories_md || "暂无回忆数据"}
                </div>
              )
            )}
          </section>
        </div>

        {/* Bottom sections - full width */}
        <div style={styles.bottomSections}>
          {/* Versions */}
          {versions.length > 1 && (
            <section style={styles.section}>
              <button type="button" onClick={() => setShowVersions(!showVersions)} style={styles.sectionHeader}>
                <span style={styles.sectionTitle}>
                  <RotateCcw size={14} style={{ marginRight: 6 }} />
                  版本历史 ({versions.length})
                </span>
                {showVersions ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              </button>
              {showVersions && (
                <div style={{ display: "flex", flexDirection: "column", gap: 8, padding: "12px 0" }}>
                  {versions.map((v) => (
                    <div key={v.version} style={styles.versionRow}>
                      <span>
                        v{v.version}
                        {v.version === persona.version && (
                          <span style={styles.currentBadge}>当前</span>
                        )}
                      </span>
                      <span className="text-caption">{new Date(v.created_at).toLocaleDateString("zh-CN")}</span>
                      {v.version !== persona.version && (
                        <button type="button" onClick={() => handleRollback(v.version)} style={styles.rollbackBtn}>
                          回滚
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </section>
          )}

          {/* Danger Zone */}
          <section style={{ ...styles.section, borderColor: "var(--color-coral-400)" }}>
            {!confirmDelete ? (
              <button type="button" onClick={() => setConfirmDelete(true)} style={styles.dangerBtn}>
                <Trash2 size={14} />
                删除这个伴侣
              </button>
            ) : (
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <span style={{ fontSize: "0.85rem", color: "var(--color-coral-500)" }}>
                  确定要删除吗？所有聊天记录也会被删除
                </span>
                <button type="button" onClick={handleDelete} style={styles.confirmDeleteBtn}>确定删除</button>
                <button type="button" onClick={() => setConfirmDelete(false)} style={styles.cancelBtn}>取消</button>
              </div>
            )}
          </section>
        </div>
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { width: "100%", height: "100vh", display: "flex", flexDirection: "column", overflow: "hidden" },
  header: { padding: "16px 24px", flexShrink: 0 },
  backBtn: { display: "flex", alignItems: "center", gap: 6, background: "none", border: "none", color: "var(--color-earth-500)", cursor: "pointer", fontSize: "0.85rem", fontFamily: "var(--font-body)" },
  main: { flex: 1, overflow: "auto", padding: "0 40px 48px" },
  hero: { display: "flex", flexDirection: "column", alignItems: "center", textAlign: "center", paddingBottom: 32, borderBottom: "1px solid var(--color-cream-200)", marginBottom: 28 },
  avatar: { width: 80, height: 80, borderRadius: "50%", background: "linear-gradient(135deg, var(--color-cream-200), var(--color-lavender-300))", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "2.5rem", boxShadow: "var(--shadow-md)" },
  editableField: { display: "flex", alignItems: "center", gap: 8, cursor: "pointer", padding: "4px 8px", borderRadius: "var(--radius-md)", transition: "background var(--duration-fast)" },
  editRow: { width: "100%", maxWidth: 360, display: "flex", flexDirection: "column", gap: 6 },
  editTextarea: { width: "100%", padding: "8px 12px", borderRadius: "var(--radius-md)", border: "1.5px solid var(--color-cream-300)", background: "var(--color-cream-100)", fontFamily: "var(--font-body)", fontSize: "0.9rem", lineHeight: 1.5, color: "var(--color-earth-800)", outline: "none", resize: "vertical" as const, textAlign: "center" as const },
  editInput: { width: "100%", padding: "8px 12px", borderRadius: "var(--radius-md)", border: "1.5px solid var(--color-cream-300)", background: "var(--color-cream-100)", fontFamily: "var(--font-body)", fontSize: "0.85rem", color: "var(--color-earth-800)", outline: "none" },
  editActions: { display: "flex", gap: 6, justifyContent: "center" },
  saveBtn: { display: "flex", alignItems: "center", gap: 4, padding: "5px 14px", borderRadius: "var(--radius-sm)", border: "none", background: "var(--color-sage-400)", color: "white", fontSize: "0.78rem", fontWeight: 500, cursor: "pointer", fontFamily: "var(--font-body)" },
  cancelEditBtn: { display: "flex", alignItems: "center", gap: 4, padding: "5px 10px", borderRadius: "var(--radius-sm)", border: "1px solid var(--color-cream-300)", background: "none", color: "var(--color-earth-500)", fontSize: "0.78rem", cursor: "pointer", fontFamily: "var(--font-body)" },
  editToggle: { display: "flex", alignItems: "center", gap: 4, fontSize: "0.75rem", color: "var(--color-rose-500)", cursor: "pointer", padding: "2px 8px", borderRadius: "var(--radius-sm)", background: "var(--color-cream-100)", border: "1px solid var(--color-cream-300)" },
  tagRow: { display: "flex", flexWrap: "wrap", gap: 6, justifyContent: "center" },
  tag: { padding: "4px 12px", borderRadius: "var(--radius-full)", background: "var(--color-cream-200)", fontSize: "0.75rem", color: "var(--color-earth-600)" },
  primaryBtn: { padding: "12px 32px", background: "var(--color-rose-500)", color: "white", border: "none", borderRadius: "var(--radius-md)", fontSize: "0.95rem", fontWeight: 500, cursor: "pointer", fontFamily: "var(--font-body)" },
  contentGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(400px, 1fr))", gap: 16, marginBottom: 16, alignItems: "start" },
  bottomSections: { maxWidth: "100%" },
  section: { marginBottom: 16, border: "1px solid var(--color-cream-200)", borderRadius: "var(--radius-lg)", overflow: "hidden" },
  sectionHeader: { width: "100%", display: "flex", justifyContent: "space-between", alignItems: "center", padding: "14px 18px", background: "var(--color-cream-100)", border: "none", cursor: "pointer", fontFamily: "var(--font-body)", color: "var(--color-earth-700)" },
  sectionTitle: { fontSize: "0.9rem", fontWeight: 600, display: "flex", alignItems: "center" },
  mdContent: { padding: "16px 18px", fontSize: "0.9rem", lineHeight: 1.8, color: "var(--color-earth-700)", whiteSpace: "pre-wrap", background: "var(--color-cream-50)" },
  mdEditContainer: { padding: "12px 14px", display: "flex", flexDirection: "column", gap: 10, background: "var(--color-cream-50)" },
  mdTextarea: { width: "100%", padding: "12px 14px", borderRadius: "var(--radius-md)", border: "1.5px solid var(--color-cream-300)", background: "var(--color-cream-100)", fontFamily: "'SF Mono', 'Fira Code', var(--font-body)", fontSize: "0.84rem", lineHeight: 1.7, color: "var(--color-earth-800)", outline: "none", resize: "vertical" as const },
  mdEditActions: { display: "flex", gap: 8 },
  versionRow: { display: "flex", alignItems: "center", gap: 12, padding: "8px 18px", fontSize: "0.85rem" },
  currentBadge: { marginLeft: 6, padding: "2px 8px", borderRadius: "var(--radius-full)", background: "var(--color-sage-400)", color: "white", fontSize: "0.7rem" },
  rollbackBtn: { marginLeft: "auto", padding: "4px 12px", borderRadius: "var(--radius-sm)", border: "1px solid var(--color-cream-300)", background: "none", fontSize: "0.75rem", cursor: "pointer", fontFamily: "var(--font-body)", color: "var(--color-earth-600)" },
  dangerBtn: { display: "flex", alignItems: "center", gap: 6, width: "100%", padding: "12px 18px", background: "none", border: "none", cursor: "pointer", fontSize: "0.85rem", color: "var(--color-coral-500)", fontFamily: "var(--font-body)" },
  confirmDeleteBtn: { padding: "6px 16px", borderRadius: "var(--radius-sm)", background: "var(--color-coral-500)", color: "white", border: "none", fontSize: "0.8rem", cursor: "pointer", fontFamily: "var(--font-body)" },
  cancelBtn: { padding: "6px 12px", background: "none", border: "none", fontSize: "0.8rem", cursor: "pointer", color: "var(--color-earth-500)", fontFamily: "var(--font-body)" },
};
