import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import { AVATAR_EMOJI_PRESETS, PERSONA_TAG_PRESETS } from "@/lib/constants";
import { detectAndParse, generatePersona, parsePastedText, captureAndOcr, generateCalibrationSamples, submitCalibrationFeedback } from "@/lib/tauri";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { BasicInfo, GenerateProgress, ParsedContent, CalibrationSample, CalibrationFeedbackItem } from "@/types";
import { toast } from "sonner";
import { ArrowLeft, FileUp, ScanText, ThumbsUp, ThumbsDown, Tag, MessageSquare } from "lucide-react";

export function CreateWizard() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const onBack = () => navigate({ to: "/" });
  const onComplete = async (personaId: string) => {
    await queryClient.invalidateQueries({ queryKey: ["personas"] });
    navigate({ to: "/chat/$personaId", params: { personaId } });
  };
  const [step, setStep] = useState(1);
  // Step 1
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [emoji, setEmoji] = useState("💜");
  // Step 2
  const [pastedText, setPastedText] = useState("");
  const [parsed, setParsed] = useState<ParsedContent | null>(null);
  const [parsing, setParsing] = useState(false);
  // Step 3
  const [_generating, setGenerating] = useState(false);
  const [progress, setProgress] = useState<GenerateProgress | null>(null);
  // Step 4
  const [personaId, setPersonaId] = useState<string>("");
  const [calibrationSamples, setCalibrationSamples] = useState<CalibrationSample[]>([]);
  const [calibrationFeedback, setCalibrationFeedback] = useState<Record<string, { sample_id: string; liked: boolean | null; tags: string[] }>>({});
  const [calibrationFreeText, setCalibrationFreeText] = useState("");
  const [submittingCalibration, setSubmittingCalibration] = useState(false);

  const toggleTag = (tag: string) => {
    setTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
    );
  };

  const handleParse = async () => {
    if (!pastedText.trim()) return;
    setParsing(true);
    try {
      const result = await parsePastedText(pastedText);
      setParsed(result);
      toast.success(`识别到 ${result.message_count} 条消息`);
    } catch (e) {
      toast.error(`解析失败: ${e}`);
    } finally {
      setParsing(false);
    }
  };

  const handleFileImport = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [
          { name: "聊天记录", extensions: ["txt", "html", "htm", "csv", "json", "db"] },
          { name: "所有文件", extensions: ["*"] },
        ],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      setParsing(true);
      const results = await detectAndParse(paths);
      if (results.length > 0 && results[0].parsed.message_count > 0) {
        setParsed(results[0].parsed);
        toast.success(`从文件识别到 ${results[0].parsed.message_count} 条消息`);
      } else {
        toast.error("未能从文件中识别出聊天记录");
      }
    } catch (e) {
      toast.error(`文件导入失败: ${e}`);
    } finally {
      setParsing(false);
    }
  };

  const handleCapture = async () => {
    try {
      toast.info("请在屏幕上框选聊天记录...", { duration: 3000 });
      const text = await captureAndOcr();
      if (text.trim()) {
        setPastedText(prev => prev ? prev + "\n" + text : text);
        toast.success("已提取屏幕文字，请点击识别");
      }
    } catch (e) {
      toast.error(`OCR提取失败: ${e}`);
    }
  };

  const handleGenerate = async () => {
    if (!name.trim()) {
      toast.error("请输入 TA 的昵称");
      return;
    }
    setGenerating(true);
    setStep(3);
    setProgress(null);

    const unlisten = await listen<GenerateProgress>("generate://progress", (event) => {
      setProgress(event.payload);
    });

    try {
      const basicInfo: BasicInfo = {
        name: name.trim(),
        description: desc.trim(),
        tags,
        avatar_emoji: emoji,
      };
      const contents = parsed ? [parsed] : [];
      const result = await generatePersona(basicInfo, contents);
      toast.success(result.summary);
      setPersonaId(result.persona_id);
      
      // 生成校准样本
      try {
        const samples = await generateCalibrationSamples(result.persona_id);
        setCalibrationSamples(samples);
        // 初始化反馈状态
        const initialFeedback: Record<string, { sample_id: string; liked: boolean | null; tags: string[] }> = {};
        samples.forEach(sample => {
          initialFeedback[sample.id] = {
            sample_id: sample.id,
            liked: null,
            tags: [],
          };
        });
        setCalibrationFeedback(initialFeedback);
        setStep(4);
      } catch (calErr) {
        console.error("Calibration samples failed:", calErr);
        // 如果校准样本生成失败，直接完成
        await onComplete(result.persona_id);
      }
    } catch (e) {
      console.error("Generate failed:", e);
      toast.error(`生成失败: ${e}`);
      setStep(2);
    } finally {
      setGenerating(false);
      unlisten();
    }
  };

  const handleCalibrationLike = (sampleId: string, liked: boolean) => {
    setCalibrationFeedback(prev => ({
      ...prev,
      [sampleId]: {
        ...prev[sampleId],
        liked,
      },
    }));
  };

  const handleCalibrationTag = (sampleId: string, tag: string) => {
    setCalibrationFeedback(prev => {
      const current = prev[sampleId];
      const tags = current.tags || [];
      const newTags = tags.includes(tag)
        ? tags.filter(t => t !== tag)
        : [...tags, tag];
      return {
        ...prev,
        [sampleId]: {
          ...current,
          tags: newTags,
        },
      };
    });
  };

  const handleSubmitCalibration = async () => {
    if (!personaId) return;
    
    // 检查是否有评价
    const hasFeedback = Object.values(calibrationFeedback).some(f => f.liked !== null);
    if (!hasFeedback) {
      toast.error("请至少评价一条样本");
      return;
    }

    setSubmittingCalibration(true);
    try {
      const ratedSamples = calibrationSamples.filter(s => calibrationFeedback[s.id]?.liked !== null);
      const feedbackItems: CalibrationFeedbackItem[] = ratedSamples.map(sample => {
        const feedback = calibrationFeedback[sample.id];
        return {
          sample_id: sample.id,
          scenario: sample.scenario,
          reply: sample.reply,
          liked: feedback.liked!,
          tags: feedback.tags || [],
        };
      });
      await submitCalibrationFeedback(personaId, feedbackItems, calibrationFreeText || undefined);
      toast.success("校准完成！开始对话吧~");
      await onComplete(personaId);
    } catch (e) {
      console.error("Submit calibration failed:", e);
      toast.error(`提交失败: ${e}`);
    } finally {
      setSubmittingCalibration(false);
    }
  };

  const getCalibrationProgress = () => {
    const total = calibrationSamples.length;
    const rated = Object.values(calibrationFeedback).filter(f => f.liked !== null).length;
    return { total, rated, percent: total > 0 ? Math.round((rated / total) * 100) : 0 };
  };

  return (
    <div style={styles.container}>
      {/* Top Bar */}
      <header style={styles.topBar}>
        <button 
          type="button" 
          onClick={step === 1 ? onBack : step === 4 ? () => setStep(3) : () => setStep(step - 1)} 
          style={styles.backBtn}
          disabled={step === 3}
        >
          <ArrowLeft size={18} />
          <span>{step === 1 ? "返回" : step === 4 ? "重新生成" : "上一步"}</span>
        </button>
        <div style={styles.steps}>
          {[1, 2, 3, 4].map((s) => (
            <div
              key={s}
              style={{
                ...styles.stepDot,
                background: s <= step ? "var(--color-rose-500)" : "var(--color-cream-300)",
              }}
            />
          ))}
        </div>
        <div style={{ width: 80 }} />
      </header>

      <main style={styles.main}>
        {/* Step 1: Info */}
        {step === 1 && (
          <div className="animate-slide-up" style={styles.stepContent}>
            <p style={styles.stepLabel}>第 1 步</p>
            <h2 className="text-heading" style={{ marginBottom: 32 }}>
              告诉我关于 TA
            </h2>

            {/* Emoji picker */}
            <div style={{ marginBottom: 28 }}>
              <label style={styles.fieldLabel}>选一个代表 TA 的表情</label>
              <div style={styles.emojiRow}>
                {AVATAR_EMOJI_PRESETS.map((e) => (
                  <button
                    key={e}
                    type="button"
                    onClick={() => setEmoji(e)}
                    style={{
                      ...styles.emojiBtn,
                      background: emoji === e ? "var(--color-rose-300)" : "var(--color-cream-200)",
                      transform: emoji === e ? "scale(1.15)" : "scale(1)",
                    }}
                  >
                    {e}
                  </button>
                ))}
              </div>
            </div>

            <div style={styles.fieldGroup}>
              <label style={styles.fieldLabel}>TA 的昵称</label>
              <input
                type="text"
                placeholder="小美、老王、妈妈..."
                value={name}
                onChange={(e) => setName(e.target.value)}
                style={styles.input}
                autoFocus
              />
            </div>

            <div style={styles.fieldGroup}>
              <label style={styles.fieldLabel}>用一句话描述 TA</label>
              <input
                type="text"
                placeholder="爱撒娇的女朋友、总是担心我的妈妈..."
                value={desc}
                onChange={(e) => setDesc(e.target.value)}
                style={styles.input}
              />
            </div>

            <div style={styles.fieldGroup}>
              <label style={styles.fieldLabel}>TA 的性格标签（可多选）</label>
              <div style={styles.tagGrid}>
                {PERSONA_TAG_PRESETS.map((tag) => (
                  <button
                    key={tag}
                    type="button"
                    onClick={() => toggleTag(tag)}
                    style={{
                      ...styles.tagChip,
                      background: tags.includes(tag) ? "var(--color-rose-500)" : "var(--color-cream-200)",
                      color: tags.includes(tag) ? "white" : "var(--color-earth-600)",
                    }}
                  >
                    {tag}
                  </button>
                ))}
              </div>
            </div>

            <button
              type="button"
              onClick={() => setStep(2)}
              disabled={!name.trim()}
              style={{
                ...styles.primaryBtn,
                opacity: name.trim() ? 1 : 0.5,
                marginTop: 16,
              }}
            >
              下一步 →
            </button>
          </div>
        )}

        {/* Step 2: Data */}
        {step === 2 && (
          <div className="animate-slide-up" style={styles.stepContent}>
            <p style={styles.stepLabel}>第 2 步</p>
            <h2 className="text-heading" style={{ marginBottom: 8 }}>
              提供聊天记录（可跳过）
            </h2>
            <p className="text-caption" style={{ marginBottom: 32 }}>
              有聊天记录会让 AI 更像 TA。没有的话，仅用描述和标签生成
            </p>

            {/* Action buttons */}
            <div style={{ display: "flex", gap: 12 }}>
              <button type="button" onClick={handleFileImport} disabled={parsing} style={{...styles.fileDropBtn, flex: 1}}>
                <FileUp size={24} style={{ color: "var(--color-rose-400)" }} />
                <span style={{ fontWeight: 500 }}>选择聊天文件</span>
                <span className="text-caption">.txt / .html / .db 等</span>
              </button>

              <button type="button" onClick={handleCapture} disabled={parsing} style={{...styles.fileDropBtn, flex: 1}}>
                <ScanText size={24} style={{ color: "var(--color-sage-500)" }} />
                <span style={{ fontWeight: 500 }}>框选屏幕文字</span>
                <span className="text-caption">使用 macOS 截图 OCR</span>
              </button>
            </div>

            <div style={styles.divider}>
              <span className="text-caption">或者直接粘贴</span>
            </div>

            <div style={styles.fieldGroup}>
              <textarea
                placeholder={"2024-03-15 14:32 小美\n今天好累啊\n\n2024-03-15 14:33 我\n怎么了宝\n..."}
                value={pastedText}
                onChange={(e) => setPastedText(e.target.value)}
                style={styles.textarea}
                rows={10}
              />
              {pastedText.trim() && !parsed && (
                <button type="button" onClick={handleParse} disabled={parsing} style={styles.secondaryBtn}>
                  {parsing ? "识别中..." : "识别聊天记录"}
                </button>
              )}
              {parsed && (
                <div style={styles.parsedInfo}>
                  ✅ 识别到 {parsed.message_count} 条消息
                  {parsed.target_name && ` · 聊天对象：${parsed.target_name}`}
                </div>
              )}
            </div>

            <div style={{ display: "flex", gap: 12, marginTop: 24 }}>
              <button type="button" onClick={handleGenerate} style={styles.primaryBtn}>
                {parsed ? "开始生成 →" : "仅用描述生成 →"}
              </button>
              {!parsed && pastedText.trim() === "" && (
                <button type="button" onClick={handleGenerate} style={styles.ghostBtn}>
                  跳过，仅用描述生成
                </button>
              )}
            </div>
          </div>
        )}

        {/* Step 3: Generating */}
        {step === 3 && (
          <div className="animate-scale-in" style={styles.generateState}>
            <div style={styles.generateOrb}>
              <span style={{ fontSize: "3rem", animation: "pulse-soft 2s ease-in-out infinite" }}>
                {emoji}
              </span>
            </div>
            <h2 className="text-heading" style={{ marginTop: 24, marginBottom: 8 }}>
              {progress?.label || "准备中..."}
            </h2>
            {progress && (
              <div style={styles.progressTrack}>
                <div
                  style={{
                    ...styles.progressFill,
                    width: `${(progress.step / progress.total) * 100}%`,
                  }}
                />
              </div>
            )}
            <p className="text-caption" style={{ marginTop: 12 }}>
              正在用 AI 分析 {name} 的性格和你们的回忆
            </p>
          </div>
        )}

        {/* Step 4: Calibration */}
        {step === 4 && (
          <div className="animate-slide-up" style={{ ...styles.stepContent, maxWidth: 640 }}>
            <p style={styles.stepLabel}>第 4 步</p>
            <h2 className="text-heading" style={{ marginBottom: 8 }}>
              校准测试
            </h2>
            <p className="text-caption" style={{ marginBottom: 24 }}>
              看看这些回复像不像 {name}？标记"像"或"不像"帮助 AI 学习
            </p>

            {/* Progress */}
            {(() => {
              const { rated, total, percent } = getCalibrationProgress();
              return (
                <div style={{ marginBottom: 24 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
                    <span style={{ fontSize: "0.85rem", color: "var(--color-earth-600)" }}>
                      已评价 {rated}/{total}
                    </span>
                    <span style={{ fontSize: "0.85rem", color: "var(--color-rose-500)", fontWeight: 500 }}>
                      {percent}%
                    </span>
                  </div>
                  <div style={styles.progressTrack}>
                    <div
                      style={{
                        ...styles.progressFill,
                        width: `${percent}%`,
                        transition: "width 0.3s ease-out",
                      }}
                    />
                  </div>
                </div>
              );
            })()}

            {/* Sample Cards */}
            <div style={{ display: "flex", flexDirection: "column", gap: 16, marginBottom: 24 }}>
              {calibrationSamples.map((sample, index) => {
                const feedback = calibrationFeedback[sample.id];
                const isLiked = feedback?.liked;
                return (
                  <div
                    key={sample.id}
                    style={{
                      ...styles.sampleCard,
                      borderColor: isLiked === true 
                        ? "var(--color-sage-400)" 
                        : isLiked === false 
                          ? "var(--color-rose-300)" 
                          : "var(--color-cream-300)",
                    }}
                  >
                    <div style={{ marginBottom: 12 }}>
                      <span style={styles.sampleIndex}>样本 {index + 1}</span>
                    </div>
                    
                    {/* User message context */}
                    {sample.scenario && (
                      <div style={styles.contextText}>
                        <span style={{ color: "var(--color-earth-400)" }}>上下文：</span>
                        {sample.scenario}
                      </div>
                    )}
                    
                    {/* Sample response */}
                    <div style={styles.sampleText}>
                      {sample.reply}
                    </div>

                    {/* Like/Unlike buttons */}
                    <div style={{ display: "flex", gap: 8, marginTop: 12, marginBottom: 12 }}>
                      <button
                        type="button"
                        onClick={() => handleCalibrationLike(sample.id, true)}
                        style={{
                          ...styles.feedbackBtn,
                          background: isLiked === true ? "var(--color-sage-400)" : "var(--color-cream-200)",
                          color: isLiked === true ? "white" : "var(--color-earth-600)",
                        }}
                      >
                        <ThumbsUp size={14} />
                        <span>像 {name}</span>
                      </button>
                      <button
                        type="button"
                        onClick={() => handleCalibrationLike(sample.id, false)}
                        style={{
                          ...styles.feedbackBtn,
                          background: isLiked === false ? "var(--color-rose-400)" : "var(--color-cream-200)",
                          color: isLiked === false ? "white" : "var(--color-earth-600)",
                        }}
                      >
                        <ThumbsDown size={14} />
                        <span>不像</span>
                      </button>
                    </div>

                    {/* Tag selection (only show if rated) */}
                    {isLiked !== null && (
                      <div style={{ animation: "fadeIn 0.3s ease" }}>
                        <div style={{ fontSize: "0.8rem", color: "var(--color-earth-500)", marginBottom: 8 }}>
                          <Tag size={12} style={{ marginRight: 4, verticalAlign: "middle" }} />
                          添加标签（可选）
                        </div>
                        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                          {["语气像", "用词像", "太正式", "太随意", "太热情", "太冷淡"].map(tag => {
                            const isSelected = feedback?.tags?.includes(tag);
                            return (
                              <button
                                key={tag}
                                type="button"
                                onClick={() => handleCalibrationTag(sample.id, tag)}
                                style={{
                                  ...styles.tagChipSmall,
                                  background: isSelected ? "var(--color-rose-300)" : "var(--color-cream-200)",
                                  color: isSelected ? "white" : "var(--color-earth-600)",
                                }}
                              >
                                {tag}
                              </button>
                            );
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>

            {/* Free text feedback */}
            <div style={{ marginBottom: 24 }}>
              <label style={{ ...styles.fieldLabel, display: "flex", alignItems: "center", gap: 6 }}>
                <MessageSquare size={14} />
                其他建议（可选）
              </label>
              <textarea
                placeholder="比如：整体语气可以再温柔一点，多用一些 emoji..."
                value={calibrationFreeText}
                onChange={(e) => setCalibrationFreeText(e.target.value)}
                style={{ ...styles.textarea, minHeight: 80 }}
                rows={3}
              />
            </div>

            {/* Submit button */}
            <button
              type="button"
              onClick={handleSubmitCalibration}
              disabled={submittingCalibration || Object.values(calibrationFeedback).filter(f => f.liked !== null).length === 0}
              style={{
                ...styles.primaryBtn,
                width: "100%",
                opacity: submittingCalibration || Object.values(calibrationFeedback).filter(f => f.liked !== null).length === 0 ? 0.6 : 1,
              }}
            >
              {submittingCalibration ? "提交中..." : "完成校准，开始对话 →"}
            </button>
          </div>
        )}
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { width: "100%", height: "100vh", display: "flex", flexDirection: "column", overflow: "hidden" },
  topBar: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "16px 24px", flexShrink: 0 },
  backBtn: { display: "flex", alignItems: "center", gap: 6, background: "none", border: "none", color: "var(--color-earth-500)", cursor: "pointer", fontSize: "0.85rem", fontFamily: "var(--font-body)" },
  steps: { display: "flex", gap: 6 },
  stepDot: { width: 8, height: 8, borderRadius: "50%", transition: "background var(--duration-normal)" },
  main: { flex: 1, overflow: "auto", display: "flex", justifyContent: "center", padding: "0 24px 48px" },
  stepContent: { maxWidth: 520, width: "100%", paddingTop: 16 },
  stepLabel: { fontSize: "0.75rem", color: "var(--color-rose-500)", fontWeight: 600, textTransform: "uppercase" as const, letterSpacing: "0.1em", marginBottom: 8 },
  fieldGroup: { marginBottom: 24, display: "flex", flexDirection: "column" as const, gap: 8 },
  fieldLabel: { fontSize: "0.85rem", fontWeight: 500, color: "var(--color-earth-600)" },
  input: { padding: "12px 16px", border: "1.5px solid var(--color-cream-300)", borderRadius: "var(--radius-md)", background: "var(--color-cream-100)", fontSize: "0.95rem", color: "var(--color-earth-800)", fontFamily: "var(--font-body)", outline: "none" },
  textarea: { padding: "14px 16px", border: "1.5px solid var(--color-cream-300)", borderRadius: "var(--radius-md)", background: "var(--color-cream-100)", fontSize: "0.9rem", color: "var(--color-earth-800)", fontFamily: "var(--font-body)", outline: "none", resize: "vertical" as const, lineHeight: 1.7 },
  tagGrid: { display: "flex", flexWrap: "wrap" as const, gap: 8 },
  tagChip: { padding: "6px 14px", borderRadius: "var(--radius-full)", border: "none", fontSize: "0.8rem", cursor: "pointer", fontFamily: "var(--font-body)", transition: "all var(--duration-fast)" },
  emojiRow: { display: "flex", flexWrap: "wrap" as const, gap: 6, marginTop: 8 },
  emojiBtn: { width: 40, height: 40, borderRadius: "var(--radius-md)", border: "none", fontSize: "1.3rem", cursor: "pointer", transition: "all var(--duration-fast) var(--ease-out-quart)", display: "flex", alignItems: "center", justifyContent: "center" },
  primaryBtn: { padding: "12px 28px", background: "var(--color-rose-500)", color: "white", border: "none", borderRadius: "var(--radius-md)", fontSize: "0.95rem", fontWeight: 500, cursor: "pointer", fontFamily: "var(--font-body)", transition: "all var(--duration-fast)" },
  secondaryBtn: { padding: "10px 20px", background: "var(--color-cream-200)", color: "var(--color-earth-700)", border: "none", borderRadius: "var(--radius-md)", fontSize: "0.85rem", cursor: "pointer", fontFamily: "var(--font-body)" },
  ghostBtn: { padding: "12px 20px", background: "none", color: "var(--color-earth-500)", border: "none", fontSize: "0.85rem", cursor: "pointer", fontFamily: "var(--font-body)" },
  parsedInfo: { padding: "10px 14px", background: "oklch(94% 0.04 148 / 0.3)", borderRadius: "var(--radius-md)", fontSize: "0.85rem", color: "var(--color-sage-500)" },
  generateState: { display: "flex", flexDirection: "column" as const, alignItems: "center", justifyContent: "center", height: "100%", paddingBottom: 64 },
  generateOrb: { width: 100, height: 100, borderRadius: "50%", background: "var(--color-cream-200)", display: "flex", alignItems: "center", justifyContent: "center" },
  progressTrack: { width: "100%", height: 4, borderRadius: 2, background: "var(--color-cream-300)", overflow: "hidden" as const },
  progressFill: { height: "100%", background: "var(--color-rose-500)", borderRadius: 2, transition: "width 0.5s var(--ease-out-quart)" },
  fileDropBtn: { width: "100%", display: "flex", flexDirection: "column" as const, alignItems: "center", gap: 6, padding: "28px 20px", border: "2px dashed var(--color-cream-300)", borderRadius: "var(--radius-lg)", background: "var(--color-cream-100)", cursor: "pointer", fontFamily: "var(--font-body)", transition: "all var(--duration-normal) var(--ease-out-quart)", marginBottom: 0 },
  divider: { display: "flex", alignItems: "center", gap: 16, margin: "20px 0", width: "100%" },
  // Step 4 calibration styles
  sampleCard: { 
    padding: "16px 20px", 
    background: "var(--color-cream-100)", 
    border: "2px solid var(--color-cream-300)", 
    borderRadius: "var(--radius-lg)",
    transition: "all var(--duration-fast)",
  },
  sampleIndex: { 
    fontSize: "0.75rem", 
    fontWeight: 600, 
    color: "var(--color-rose-500)",
    textTransform: "uppercase" as const,
    letterSpacing: "0.05em",
  },
  contextText: { 
    fontSize: "0.8rem", 
    color: "var(--color-earth-500)", 
    marginBottom: 12,
    padding: "8px 12px",
    background: "var(--color-cream-200)",
    borderRadius: "var(--radius-md)",
    fontStyle: "italic" as const,
  },
  sampleText: { 
    fontSize: "0.95rem", 
    color: "var(--color-earth-800)", 
    lineHeight: 1.6,
    padding: "12px 16px",
    background: "white",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-cream-300)",
  },
  feedbackBtn: { 
    display: "flex", 
    alignItems: "center", 
    gap: 6, 
    padding: "8px 16px", 
    border: "none", 
    borderRadius: "var(--radius-md)", 
    fontSize: "0.85rem", 
    cursor: "pointer", 
    fontFamily: "var(--font-body)", 
    transition: "all var(--duration-fast)",
    flex: 1,
    justifyContent: "center" as const,
  },
  tagChipSmall: { 
    padding: "4px 10px", 
    borderRadius: "var(--radius-full)", 
    border: "none", 
    fontSize: "0.75rem", 
    cursor: "pointer", 
    fontFamily: "var(--font-body)", 
    transition: "all var(--duration-fast)",
  },
};
