import { useEffect, useState } from "react";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import { AVATAR_EMOJI_PRESETS } from "@/lib/constants";
import { getPersona, deletePersona, getPersonaVersions, rollbackPersona, getPersonaVoice, uploadAndCloneVoice, removePersonaVoice, setPersonaVoice, speakText, checkFfmpeg, toggleClipboardWatcher, appendClipboardCorpus, exportPersona } from "@/lib/tauri";
import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { Persona, VersionSummary, PersonaVoice } from "@/types";
import { ArrowLeft, Trash2, RotateCcw, ChevronDown, ChevronUp, Save, X, Pencil, Mic, Volume2, Unlink, Loader2, Film, Download } from "lucide-react";
import { toast } from "sonner";
import { ProactiveSettings } from "./ProactiveSettings";

const VIDEO_EXTENSIONS = ["mp4", "mov", "mkv", "avi", "webm", "flv", "wmv"];

export function ProfileView() {
  const { personaId } = useParams({ strict: false }) as { personaId: string };
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const onBack = () => navigate({ to: "/" });
  const onChat = () => navigate({ to: "/chat/$personaId", params: { personaId } });
  const onDeleted = async () => {
    await queryClient.invalidateQueries({ queryKey: ["personas"] });
    navigate({ to: "/" });
  };
  const [persona, setPersona] = useState<Persona | null>(null);
  const [versions, setVersions] = useState<VersionSummary[]>([]);
  const [showPersonaMd, setShowPersonaMd] = useState(false);
  const [showMemoriesMd, setShowMemoriesMd] = useState(false);
  const [showVersions, setShowVersions] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Editable states
  const [editAvatar, setEditAvatar] = useState(false);
  const [draftAvatar, setDraftAvatar] = useState("");
  const [editDesc, setEditDesc] = useState(false);
  const [draftDesc, setDraftDesc] = useState("");
  const [editTags, setEditTags] = useState(false);
  const [draftTags, setDraftTags] = useState("");
  const [editPersonaMd, setEditPersonaMd] = useState(false);
  const [draftPersonaMd, setDraftPersonaMd] = useState("");
  const [editMemoriesMd, setEditMemoriesMd] = useState(false);
  const [draftMemoriesMd, setDraftMemoriesMd] = useState("");
  const [saving, setSaving] = useState(false);
  const [exporting, setExporting] = useState(false);

  // Voice state
  const [voice, setVoice] = useState<PersonaVoice | null>(null);
  const [cloning, setCloning] = useState(false);
  const [cloningStep, setCloningStep] = useState("");
  const [auditioning, setAuditioning] = useState(false);
  const [manualVoiceId, setManualVoiceId] = useState("");
  const [showManualInput, setShowManualInput] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const p = await getPersona(personaId);
        setPersona(p);
        const v = await getPersonaVersions(personaId);
        setVersions(v);
        // Load voice binding
        const pv = await getPersonaVoice(personaId);
        setVoice(pv);
      } catch (e) {
        toast.error(`加载失败: ${e}`);
      }
    })();

    const isWatcherEnabled = localStorage.getItem("memora_clipboard_watcher") === "true";
    if (isWatcherEnabled) toggleClipboardWatcher(true).catch(console.error);
    const unlisten = listen<{ text: string }>("clipboard://chat-detected", (e) => {
      toast("检测到剪贴板具有潜在聊天记录", {
        description: "是否将其作为语料追加到当前人物的回忆中？",
        action: {
          label: "追加",
          onClick: async () => {
            try {
              await appendClipboardCorpus(personaId, e.payload.text);
              toast.success("已追加到共同记忆，刷新即可查看最新回忆");
            } catch (err) {
              toast.error(`追加失败: ${err}`);
            }
          }
        },
        duration: 8000,
      });
    });

    return () => {
      if (isWatcherEnabled) toggleClipboardWatcher(false).catch(console.error);
      unlisten.then((fn) => fn());
    };
  }, [personaId]);

  const handleExport = async () => {
    setExporting(true);
    try {
      // Get home directory using Tauri path API
      const { homeDir } = await import('@tauri-apps/api/path');
      const home = await homeDir();
      const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, "-");
      const fileName = `memora-export-${persona?.slug || personaId}-${timestamp}.zip`;
      const outputPath = `${home}/Downloads/${fileName}`;
      
      await exportPersona(personaId, outputPath);
      toast.success(`已导出到 Downloads: ${fileName}`);
    } catch (e) {
      toast.error(`导出失败: ${e}`);
    } finally {
      setExporting(false);
    }
  };

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

  // Voice handlers
  const handleCloneVoice = async () => {
    try {
      const selected = await openDialog({
        title: "选择参考音频或视频",
        filters: [
          { name: "音频/视频文件", extensions: ["mp3", "wav", "m4a", "flac", "ogg", "aac", "mp4", "mov", "mkv", "avi", "webm", "flv", "wmv"] },
        ],
      });
      if (!selected) return;
      const path = typeof selected === "string" ? selected : (selected as any).path;
      if (!path) return;

      // Check if video file needs ffmpeg
      const ext = path.split(".").pop()?.toLowerCase() ?? "";
      if (VIDEO_EXTENSIONS.includes(ext)) {
        setCloningStep("正在检查 ffmpeg...");
        setCloning(true);
        const hasFfmpeg = await checkFfmpeg();
        if (!hasFfmpeg) {
          setCloning(false);
          setCloningStep("");
          toast.error("未检测到 ffmpeg", {
            description: "上传视频需要 ffmpeg 提取音频。请运行 brew install ffmpeg 安装后重试。",
            duration: 8000,
          });
          return;
        }
        setCloningStep("正在从视频提取音频...");
      } else {
        setCloning(true);
        setCloningStep("正在上传音频...");
      }

      setCloningStep("正在上传并克隆音色...");
      const result = await uploadAndCloneVoice(personaId, path);
      const pv = await getPersonaVoice(personaId);
      setVoice(pv);
      toast.success(`音色克隆成功！voice_id: ${result.voice_id}`);
    } catch (e) {
      toast.error(`克隆失败: ${e}`);
    } finally {
      setCloning(false);
      setCloningStep("");
    }
  };

  const handleAudition = async () => {
    setAuditioning(true);
    try {
      const audioPath = await speakText("你好，很高兴认识你，这是一段试听语音。", personaId);
      const url = convertFileSrc(audioPath);
      const audio = new Audio(url);
      audio.onended = () => setAuditioning(false);
      audio.onerror = () => { setAuditioning(false); toast.error("播放失败"); };
      await audio.play();
    } catch (e) {
      toast.error(`试听失败: ${e}`);
      setAuditioning(false);
    }
  };

  const handleUnbindVoice = async () => {
    try {
      await removePersonaVoice(personaId);
      setVoice(null);
      toast.success("已解除语音绑定");
    } catch (e) {
      toast.error(`解除失败: ${e}`);
    }
  };

  const handleSetManualVoice = async () => {
    if (!manualVoiceId.trim()) return;
    try {
      await setPersonaVoice(personaId, "minimax", manualVoiceId.trim(), "zh-CN");
      const pv = await getPersonaVoice(personaId);
      setVoice(pv);
      setShowManualInput(false);
      setManualVoiceId("");
      toast.success("语音已绑定");
    } catch (e) {
      toast.error(`绑定失败: ${e}`);
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
          {editAvatar ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 12, alignItems: "center" }}>
              <div style={styles.emojiRow}>
                {AVATAR_EMOJI_PRESETS.map((e) => (
                  <button
                    key={e}
                    type="button"
                    onClick={() => setDraftAvatar(e)}
                    style={{
                      ...styles.emojiBtn,
                      background: draftAvatar === e ? "var(--color-rose-300)" : "var(--color-cream-200)",
                      transform: draftAvatar === e ? "scale(1.15)" : "scale(1)",
                    }}
                  >
                    {e}
                  </button>
                ))}
              </div>
              <div style={styles.editActions}>
                <button type="button" style={styles.saveBtn} disabled={saving} onClick={async () => {
                  if (draftAvatar.trim() !== "") {
                    await saveField("avatar_emoji", draftAvatar.trim());
                  }
                  setEditAvatar(false);
                }}>
                  <Save size={13} /> 保存
                </button>
                <button type="button" style={styles.cancelEditBtn} onClick={() => setEditAvatar(false)}>
                  <X size={13} />
                </button>
              </div>
            </div>
          ) : (
            <div
              style={{ ...styles.avatar, cursor: "pointer", position: "relative" }}
              onClick={() => { setDraftAvatar(persona.avatar_emoji); setEditAvatar(true); }}
              title="点击修改头像"
            >
              {persona.avatar_emoji}
              <div style={{ position: "absolute", bottom: -4, right: -4, background: "white", borderRadius: "50%", padding: 4, boxShadow: "var(--shadow-sm)", display: "flex", alignItems: "center", justifyContent: "center" }}>
                <Pencil size={12} style={{ color: "var(--color-earth-500)" }} />
              </div>
            </div>
          )}
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
          {/* Voice Settings */}
          <section style={styles.section}>
            <div style={styles.sectionHeader}>
              <span style={styles.sectionTitle}>
                <Mic size={14} style={{ marginRight: 6 }} />
                语音设置
              </span>
            </div>
            <div style={{ padding: "14px 18px" }}>
              {voice ? (
                <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                  <div style={{ fontSize: "0.85rem", color: "var(--color-earth-600)", lineHeight: 1.8 }}>
                    <span style={{ fontWeight: 500 }}>服务商:</span> {voice.provider}
                    <br />
                    <span style={{ fontWeight: 500 }}>Voice ID:</span>{" "}
                    <code style={{ fontSize: "0.78rem", background: "var(--color-cream-200)", padding: "2px 6px", borderRadius: "var(--radius-sm)" }}>
                      {voice.voice_id}
                    </code>
                    <br />
                    <span style={{ fontWeight: 500 }}>语言:</span> {voice.language}
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <button type="button" onClick={handleAudition} disabled={auditioning} style={{ ...styles.rollbackBtn, color: "var(--color-lavender-500)", borderColor: "var(--color-lavender-300)" }}>
                      {auditioning ? <Loader2 size={12} className="animate-spin" style={{ marginRight: 4 }} /> : <Volume2 size={12} style={{ marginRight: 4 }} />}
                      试听
                    </button>
                    <button type="button" onClick={handleCloneVoice} disabled={cloning} style={styles.rollbackBtn}>
                      {cloning ? <Loader2 size={12} className="animate-spin" style={{ marginRight: 4 }} /> : <Mic size={12} style={{ marginRight: 4 }} />}
                      {cloning ? cloningStep || "处理中..." : "更换音色"}
                    </button>
                    <button type="button" onClick={handleUnbindVoice} style={{ ...styles.rollbackBtn, color: "var(--color-coral-500)", borderColor: "var(--color-coral-300)" }}>
                      <Unlink size={12} style={{ marginRight: 4 }} />
                      解除绑定
                    </button>
                  </div>
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                  <p className="text-caption" style={{ lineHeight: 1.6 }}>
                    上传伴侣的音频或视频克隆音色，聊天时可用 TA 的声音朗读消息。
                    <br />
                    <span style={{ fontSize: "0.72rem", color: "var(--color-earth-400)" }}>支持 mp3/wav/m4a/flac 音频，以及 mp4/mov/mkv 等视频（需安装 ffmpeg）</span>
                  </p>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <button type="button" onClick={handleCloneVoice} disabled={cloning} style={{ ...styles.primaryBtn, padding: "8px 20px", fontSize: "0.85rem" }}>
                      {cloning ? <Loader2 size={13} className="animate-spin" style={{ marginRight: 6 }} /> : <><Mic size={13} style={{ marginRight: 6 }} /><Film size={13} style={{ marginRight: 6 }} /></>}
                      {cloning ? cloningStep || "处理中..." : "上传音频/视频并克隆"}
                    </button>
                    <button
                      type="button"
                      onClick={() => setShowManualInput(!showManualInput)}
                      style={{ ...styles.rollbackBtn, fontSize: "0.8rem" }}
                    >
                      手动输入 Voice ID
                    </button>
                  </div>
                  {showManualInput && (
                    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                      <input
                        type="text"
                        value={manualVoiceId}
                        onChange={(e) => setManualVoiceId(e.target.value)}
                        placeholder="输入已有 voice_id"
                        style={{ ...styles.editInput, flex: 1 }}
                      />
                      <button type="button" onClick={handleSetManualVoice} style={{ ...styles.saveBtn }}>
                        <Save size={12} /> 绑定
                      </button>
                    </div>
                  )}
                </div>
              )}
            </div>
          </section>
          {/* Proactive Settings */}
          <section style={styles.section}>
            <ProactiveSettings personaId={personaId} />
          </section>

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

          {/* Export Backup */}
          <section style={styles.section}>
            <div style={styles.sectionHeader}>
              <span style={styles.sectionTitle}>
                <Download size={14} style={{ marginRight: 6 }} />
                备份导出
              </span>
            </div>
            <div style={{ padding: "14px 18px" }}>
              <p className="text-caption" style={{ marginBottom: 12, color: "var(--color-earth-500)" }}>
                导出角色的所有数据（性格定义、记忆、聊天记录）为 ZIP 文件
              </p>
              <button
                type="button"
                onClick={handleExport}
                disabled={exporting}
                style={{ ...styles.rollbackBtn, opacity: exporting ? 0.5 : 1 }}
              >
                {exporting ? (
                  <><Loader2 size={14} className="animate-spin" style={{ marginRight: 6 }} /> 导出中...</>
                ) : (
                  <><Download size={14} style={{ marginRight: 6 }} /> 导出备份</>
                )}
              </button>
            </div>
          </section>

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
  emojiRow: { display: "flex", flexWrap: "wrap", gap: 6, marginTop: 8, justifyContent: "center", maxWidth: 280 },
  emojiBtn: { width: 40, height: 40, borderRadius: "var(--radius-md)", border: "none", fontSize: "1.3rem", cursor: "pointer", transition: "all var(--duration-fast) var(--ease-out-quart)", display: "flex", alignItems: "center", justifyContent: "center" },
};
