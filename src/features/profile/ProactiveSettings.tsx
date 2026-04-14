import { useState, useEffect } from "react";
import { Bell, Calendar, Clock, MessageCircle, Plus, Trash2, ToggleLeft, ToggleRight, Zap } from "lucide-react";
import { toast } from "sonner";
import { getProactiveSettings, saveProactiveSettings, triggerProactiveTest } from "@/lib/tauri";

// 对齐 PF-12 Rust 规则格式
interface IdleRule {
  type: "Idle";
  days: number;
}

interface DailyRule {
  type: "Daily";
  time: string;
}

interface DateRule {
  type: "Date";
  month_day: string;
  description?: string;
}

type ProactiveRule = IdleRule | DailyRule | DateRule;

interface ProactiveConfig {
  enabled: boolean;
  idle?: IdleRule;
  daily?: DailyRule;
  dates: DateRule[];
}

interface ProactiveSettingsProps {
  personaId: string;
}

const defaultConfig: ProactiveConfig = {
  enabled: false,
  idle: { type: "Idle", days: 7 },
  daily: { type: "Daily", time: "09:00" },
  dates: [],
};

function rulesToConfig(rules: ProactiveRule[], enabled: boolean): ProactiveConfig {
  const config: ProactiveConfig = { enabled, idle: undefined, daily: undefined, dates: [] };
  for (const rule of rules) {
    if (rule.type === "Idle") config.idle = rule;
    else if (rule.type === "Daily") config.daily = rule;
    else if (rule.type === "Date") config.dates.push(rule);
  }
  if (!config.idle) config.idle = { type: "Idle", days: 7 };
  if (!config.daily) config.daily = { type: "Daily", time: "09:00" };
  return config;
}

function configToRules(config: ProactiveConfig): ProactiveRule[] {
  const rules: ProactiveRule[] = [];
  if (config.idle && config.idle.days > 0) rules.push(config.idle);
  if (config.daily && config.daily.time) rules.push(config.daily);
  rules.push(...config.dates.filter(d => d.month_day));
  return rules;
}

// 兼容旧格式转换
function migrateLegacyRules(rules: any[]): ProactiveRule[] {
  const proactiveRules: ProactiveRule[] = [];
  for (const r of rules) {
    if (r.type === "Idle" || r.type === "Daily" || r.type === "Date") {
      proactiveRules.push(r);
    } else if (r.date && typeof r.date === "string") {
      // 旧格式 importantDates
      proactiveRules.push({ type: "Date", month_day: r.date.slice(5), description: r.description });
    }
  }
  return proactiveRules;
}

export function ProactiveSettings({ personaId }: ProactiveSettingsProps) {
  const [config, setConfig] = useState<ProactiveConfig>(defaultConfig);
  const [isEditing, setIsEditing] = useState(false);
  const [draftConfig, setDraftConfig] = useState<ProactiveConfig>(defaultConfig);

  // Load config from backend
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const result = await getProactiveSettings(personaId);
        if (result.rules) {
          const parsed = JSON.parse(result.rules);
          let rules: ProactiveRule[] = Array.isArray(parsed) ? parsed : parsed.rules || [];
          // 兼容旧格式
          rules = migrateLegacyRules(rules);
          const loadedConfig = rulesToConfig(rules, result.enabled);
          setConfig(loadedConfig);
        } else {
          setConfig({ ...defaultConfig, enabled: result.enabled });
        }
      } catch {
        setConfig(defaultConfig);
      }
    };
    loadSettings();
  }, [personaId]);

  const saveConfig = async () => {
    try {
      const rules = configToRules(draftConfig);
      const rulesJson = JSON.stringify(rules);
      await saveProactiveSettings(personaId, draftConfig.enabled, rulesJson);
      setConfig(draftConfig);
      setIsEditing(false);
      toast.success("主动触达设置已保存");
    } catch (e) {
      toast.error(`保存失败: ${e}`);
    }
  };

  const handleTestNotification = async () => {
    try {
      await triggerProactiveTest(personaId);
      toast.success("测试通知已发送");
    } catch (e) {
      toast.error(`发送失败: ${e}`);
    }
  };

  const handleToggle = () => {
    const newConfig = { ...draftConfig, enabled: !draftConfig.enabled };
    setDraftConfig(newConfig);
  };

  const toggleIdle = () => {
    setDraftConfig(prev => ({
      ...prev,
      idle: prev.idle ? undefined : { type: "Idle", days: 7 },
    }));
  };

  const updateIdleDays = (days: number) => {
    setDraftConfig(prev => ({
      ...prev,
      idle: prev.idle ? { ...prev.idle, days } : { type: "Idle", days },
    }));
  };

  const toggleDaily = () => {
    setDraftConfig(prev => ({
      ...prev,
      daily: prev.daily ? undefined : { type: "Daily", time: "09:00" },
    }));
  };

  const updateDailyTime = (time: string) => {
    setDraftConfig(prev => ({
      ...prev,
      daily: prev.daily ? { ...prev.daily, time } : { type: "Daily", time },
    }));
  };

  const addDate = () => {
    setDraftConfig(prev => ({
      ...prev,
      dates: [...prev.dates, { type: "Date", month_day: "", description: "" }],
    }));
  };

  const updateDate = (index: number, field: "month_day" | "description", value: string) => {
    const newDates = [...draftConfig.dates];
    newDates[index] = { ...newDates[index], [field]: value };
    setDraftConfig({ ...draftConfig, dates: newDates });
  };

  const removeDate = (index: number) => {
    const newDates = draftConfig.dates.filter((_, i) => i !== index);
    setDraftConfig({ ...draftConfig, dates: newDates });
  };

  if (!isEditing) {
    return (
      <div style={styles.container}>
        <div style={styles.header}>
          <div style={styles.title}>
            <Bell size={16} style={{ color: config.enabled ? "var(--color-rose-500)" : "var(--color-earth-400)" }} />
            <span>主动触达</span>
            <span style={{
              ...styles.statusBadge,
              background: config.enabled ? "rgba(148, 196, 154, 0.2)" : "rgba(156, 150, 138, 0.2)",
              color: config.enabled ? "var(--color-sage-500)" : "var(--color-earth-500)",
            }}>
              {config.enabled ? "已开启" : "已关闭"}
            </span>
          </div>
          <button
            type="button"
            onClick={() => { setDraftConfig(config); setIsEditing(true); }}
            style={styles.editBtn}
          >
            配置
          </button>
        </div>

        {config.enabled && (
          <div style={styles.summary}>
            {config.idle && (
              <div style={styles.summaryItem}>
                <Clock size={14} />
                <span className="text-caption">
                  {config.idle.days} 天未回复提醒
                </span>
              </div>
            )}
            {config.daily && (
              <div style={styles.summaryItem}>
                <MessageCircle size={14} />
                <span className="text-caption">
                  每日 {config.daily.time} 问候
                </span>
              </div>
            )}
            {config.dates.length > 0 && (
              <div style={styles.summaryItem}>
                <Calendar size={14} />
                <span className="text-caption">
                  {config.dates.length} 个重要日期
                </span>
              </div>
            )}
          </div>
        )}
      </div>
    );
  }

  return (
    <div style={styles.container}>
      {/* Master Toggle */}
      <div style={styles.toggleRow}>
        <div style={styles.toggleInfo}>
          <Bell size={18} style={{ color: draftConfig.enabled ? "var(--color-rose-500)" : "var(--color-earth-400)" }} />
          <div>
            <span style={{ fontWeight: 500 }}>主动触达总开关</span>
            <p className="text-caption" style={{ color: "var(--color-earth-500)", marginTop: 2 }}>
              开启后，AI 会在特定时机主动联系你
            </p>
          </div>
        </div>
        <button type="button" onClick={handleToggle} style={styles.toggleBtn}>
          {draftConfig.enabled ? (
            <ToggleRight size={32} style={{ color: "var(--color-rose-500)" }} />
          ) : (
            <ToggleLeft size={32} style={{ color: "var(--color-earth-400)" }} />
          )}
        </button>
      </div>

      {draftConfig.enabled && (
        <div style={styles.settingsPanel} className="animate-slide-up">
          {/* Idle Reminder */}
          <div style={styles.section}>
            <div style={styles.sectionHeader}>
              <Clock size={14} />
              <span className="text-caption" style={{ fontWeight: 600 }}>长时间未互动提醒</span>
              <label style={styles.switchLabel}>
                <input
                  type="checkbox"
                  checked={!!draftConfig.idle}
                  onChange={toggleIdle}
                  style={{ display: "none" }}
                />
                <span style={{
                  ...styles.switch,
                  background: draftConfig.idle ? "var(--color-rose-500)" : "var(--color-cream-300)",
                }}>
                  <span style={{
                    ...styles.switchThumb,
                    transform: draftConfig.idle ? "translateX(14px)" : "translateX(0)",
                  }} />
                </span>
              </label>
            </div>
            {draftConfig.idle && (
              <div style={styles.sliderRow}>
                <span className="text-small">超过</span>
                <select
                  value={draftConfig.idle.days}
                  onChange={(e) => updateIdleDays(Number(e.target.value))}
                  style={styles.select}
                >
                  {[1, 2, 3, 5, 7].map(d => (
                    <option key={d} value={d}>{d} 天</option>
                  ))}
                </select>
                <span className="text-small">未聊天，AI 主动发起对话</span>
              </div>
            )}
          </div>

          {/* Daily Greeting */}
          <div style={styles.section}>
            <div style={styles.sectionHeader}>
              <MessageCircle size={14} />
              <span className="text-caption" style={{ fontWeight: 600 }}>每日定时问候</span>
              <label style={styles.switchLabel}>
                <input
                  type="checkbox"
                  checked={!!draftConfig.daily}
                  onChange={toggleDaily}
                  style={{ display: "none" }}
                />
                <span style={{
                  ...styles.switch,
                  background: draftConfig.daily ? "var(--color-rose-500)" : "var(--color-cream-300)",
                }}>
                  <span style={{
                    ...styles.switchThumb,
                    transform: draftConfig.daily ? "translateX(14px)" : "translateX(0)",
                  }} />
                </span>
              </label>
            </div>
            {draftConfig.daily && (
              <div style={styles.sliderRow}>
                <span className="text-small">每天</span>
                <input
                  type="time"
                  value={draftConfig.daily.time}
                  onChange={(e) => updateDailyTime(e.target.value)}
                  style={styles.timeInput}
                />
                <span className="text-small">发送问候</span>
              </div>
            )}
          </div>

          {/* Important Dates */}
          <div style={styles.section}>
            <div style={styles.sectionHeader}>
              <Calendar size={14} />
              <span className="text-caption" style={{ fontWeight: 600 }}>重要日期提醒</span>
            </div>
            <p className="text-small" style={{ color: "var(--color-earth-500)", marginBottom: 12 }}>
              纪念日、生日等特殊日期，AI 会提前发送祝福（每年重复）
            </p>
            
            {draftConfig.dates.map((date, index) => (
              <div key={index} style={styles.inputRow}>
                <input
                  type="text"
                  placeholder="MM-DD"
                  value={date.month_day}
                  onChange={(e) => updateDate(index, "month_day", e.target.value)}
                  style={{ ...styles.monthDayInput, width: 80 }}
                  maxLength={5}
                />
                <input
                  type="text"
                  placeholder="描述（如：生日）"
                  value={date.description || ""}
                  onChange={(e) => updateDate(index, "description", e.target.value)}
                  style={styles.textInput}
                />
                <button
                  type="button"
                  onClick={() => removeDate(index)}
                  style={styles.iconBtn}
                >
                  <Trash2 size={16} style={{ color: "var(--color-coral-500)" }} />
                </button>
              </div>
            ))}
            
            <button type="button" onClick={addDate} style={styles.addBtn}>
              <Plus size={14} />
              添加日期
            </button>
          </div>

          {/* Test Button */}
          <div style={styles.section}>
            <div style={styles.sectionHeader}>
              <Zap size={14} />
              <span className="text-caption" style={{ fontWeight: 600 }}>测试通知</span>
            </div>
            <p className="text-small" style={{ color: "var(--color-earth-500)", marginBottom: 12 }}>
              立即发送一条测试通知，验证系统通知和跳转功能
            </p>
            <button type="button" onClick={handleTestNotification} style={styles.testBtn}>
              发送测试通知
            </button>
          </div>
        </div>
      )}

      {/* Actions */}
      <div style={styles.actions}>
        <button type="button" onClick={saveConfig} style={styles.saveBtn}>
          保存设置
        </button>
        <button
          type="button"
          onClick={() => { setIsEditing(false); setDraftConfig(config); }}
          style={styles.cancelBtn}
        >
          取消
        </button>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { padding: "16px 18px" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  title: { display: "flex", alignItems: "center", gap: 8, fontSize: "0.9rem", fontWeight: 600, color: "var(--color-earth-700)" },
  statusBadge: { padding: "2px 8px", borderRadius: "var(--radius-full)", fontSize: "0.7rem", fontWeight: 500 },
  editBtn: { padding: "6px 14px", borderRadius: "var(--radius-sm)", border: "1px solid var(--color-cream-300)", background: "var(--color-cream-100)", fontSize: "0.8rem", cursor: "pointer", fontFamily: "var(--font-body)", color: "var(--color-earth-600)", transition: "all var(--duration-fast)" },
  summary: { marginTop: 12, display: "flex", flexDirection: "column", gap: 8 },
  summaryItem: { display: "flex", alignItems: "center", gap: 8, color: "var(--color-earth-600)" },
  toggleRow: { display: "flex", justifyContent: "space-between", alignItems: "center", paddingBottom: 16, borderBottom: "1px solid var(--color-cream-200)" },
  toggleInfo: { display: "flex", alignItems: "flex-start", gap: 12 },
  toggleBtn: { background: "none", border: "none", cursor: "pointer", padding: 0, display: "flex", alignItems: "center" },
  settingsPanel: { paddingTop: 16, display: "flex", flexDirection: "column", gap: 20 },
  section: { padding: "12px", background: "var(--color-cream-50)", borderRadius: "var(--radius-md)", border: "1px solid var(--color-cream-200)" },
  sectionHeader: { display: "flex", alignItems: "center", gap: 6, marginBottom: 8, color: "var(--color-earth-700)" },
  switchLabel: { marginLeft: "auto", cursor: "pointer", display: "flex", alignItems: "center" },
  switch: { width: 32, height: 18, borderRadius: 9, background: "var(--color-cream-300)", position: "relative", transition: "background var(--duration-fast)" },
  switchThumb: { width: 14, height: 14, borderRadius: "50%", background: "white", position: "absolute", top: 2, left: 2, transition: "transform var(--duration-fast)" },
  inputRow: { display: "flex", gap: 8, marginBottom: 8, alignItems: "center" },
  monthDayInput: { padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1.5px solid var(--color-cream-300)", background: "white", fontSize: "0.8rem", fontFamily: "var(--font-body)", color: "var(--color-earth-700)", outline: "none" },
  timeInput: { padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1.5px solid var(--color-cream-300)", background: "white", fontSize: "0.8rem", fontFamily: "var(--font-body)", color: "var(--color-earth-700)", outline: "none", width: 100 },
  textInput: { flex: 1, padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1.5px solid var(--color-cream-300)", background: "white", fontSize: "0.8rem", fontFamily: "var(--font-body)", color: "var(--color-earth-700)", outline: "none" },
  select: { padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1.5px solid var(--color-cream-300)", background: "white", fontSize: "0.8rem", fontFamily: "var(--font-body)", color: "var(--color-earth-700)", outline: "none" },
  sliderRow: { display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" },
  iconBtn: { background: "none", border: "none", cursor: "pointer", padding: 4, display: "flex", alignItems: "center", justifyContent: "center", borderRadius: "var(--radius-sm)" },
  addBtn: { display: "flex", alignItems: "center", gap: 4, padding: "6px 12px", borderRadius: "var(--radius-sm)", border: "1px dashed var(--color-cream-300)", background: "none", fontSize: "0.8rem", cursor: "pointer", fontFamily: "var(--font-body)", color: "var(--color-earth-500)", marginTop: 4 },
  testBtn: { padding: "8px 16px", borderRadius: "var(--radius-md)", border: "1.5px solid var(--color-rose-400)", background: "var(--color-rose-50)", color: "var(--color-rose-500)", fontSize: "0.85rem", cursor: "pointer", fontFamily: "var(--font-body)", fontWeight: 500 },
  actions: { display: "flex", gap: 12, marginTop: 20, paddingTop: 16, borderTop: "1px solid var(--color-cream-200)" },
  saveBtn: { padding: "8px 20px", background: "var(--color-rose-500)", color: "white", border: "none", borderRadius: "var(--radius-md)", fontSize: "0.9rem", fontWeight: 500, cursor: "pointer", fontFamily: "var(--font-body)" },
  cancelBtn: { padding: "8px 16px", background: "none", border: "1px solid var(--color-cream-300)", borderRadius: "var(--radius-md)", fontSize: "0.9rem", cursor: "pointer", fontFamily: "var(--font-body)", color: "var(--color-earth-500)" },
};
