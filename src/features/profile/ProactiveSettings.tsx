import { useState, useEffect } from "react";
import { Bell, Calendar, Clock, MessageCircle, Plus, Trash2, ToggleLeft, ToggleRight } from "lucide-react";
import { toast } from "sonner";
import { getProactiveSettings, saveProactiveSettings } from "@/lib/tauri";

interface ProactiveConfig {
  enabled: boolean;
  importantDates: { date: string; description: string }[];
  overdueReminderDays: number;
  scheduledGreetings: { time: string; message: string }[];
}

interface ProactiveSettingsProps {
  personaId: string;
}

const defaultConfig: ProactiveConfig = {
  enabled: false,
  importantDates: [],
  overdueReminderDays: 3,
  scheduledGreetings: [],
};

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
          setConfig({ ...defaultConfig, ...parsed, enabled: result.enabled });
        } else {
          setConfig({ ...defaultConfig, enabled: result.enabled });
        }
      } catch {
        // No settings saved yet, use defaults
        setConfig(defaultConfig);
      }
    };
    loadSettings();
  }, [personaId]);

  const saveConfig = async () => {
    try {
      const rulesJson = JSON.stringify({
        importantDates: draftConfig.importantDates,
        overdueReminderDays: draftConfig.overdueReminderDays,
        scheduledGreetings: draftConfig.scheduledGreetings,
      });
      await saveProactiveSettings(personaId, draftConfig.enabled, rulesJson);
      setConfig(draftConfig);
      setIsEditing(false);
      toast.success("主动触达设置已保存");
    } catch (e) {
      toast.error(`保存失败: ${e}`);
    }
  };

  const handleToggle = () => {
    const newConfig = { ...draftConfig, enabled: !draftConfig.enabled };
    setDraftConfig(newConfig);
  };

  const addImportantDate = () => {
    setDraftConfig({
      ...draftConfig,
      importantDates: [...draftConfig.importantDates, { date: "", description: "" }],
    });
  };

  const updateImportantDate = (index: number, field: "date" | "description", value: string) => {
    const newDates = [...draftConfig.importantDates];
    newDates[index] = { ...newDates[index], [field]: value };
    setDraftConfig({ ...draftConfig, importantDates: newDates });
  };

  const removeImportantDate = (index: number) => {
    const newDates = draftConfig.importantDates.filter((_, i) => i !== index);
    setDraftConfig({ ...draftConfig, importantDates: newDates });
  };

  const addScheduledGreeting = () => {
    setDraftConfig({
      ...draftConfig,
      scheduledGreetings: [...draftConfig.scheduledGreetings, { time: "09:00", message: "" }],
    });
  };

  const updateScheduledGreeting = (index: number, field: "time" | "message", value: string) => {
    const newGreetings = [...draftConfig.scheduledGreetings];
    newGreetings[index] = { ...newGreetings[index], [field]: value };
    setDraftConfig({ ...draftConfig, scheduledGreetings: newGreetings });
  };

  const removeScheduledGreeting = (index: number) => {
    const newGreetings = draftConfig.scheduledGreetings.filter((_, i) => i !== index);
    setDraftConfig({ ...draftConfig, scheduledGreetings: newGreetings });
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
              background: config.enabled ? "var(--color-sage-400)/20" : "var(--color-earth-400)/20",
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
            {config.importantDates.length > 0 && (
              <div style={styles.summaryItem}>
                <Calendar size={14} />
                <span className="text-caption">
                  {config.importantDates.length} 个重要日期
                </span>
              </div>
            )}
            {config.overdueReminderDays > 0 && (
              <div style={styles.summaryItem}>
                <Clock size={14} />
                <span className="text-caption">
                  {config.overdueReminderDays} 天未回复提醒
                </span>
              </div>
            )}
            {config.scheduledGreetings.length > 0 && (
              <div style={styles.summaryItem}>
                <MessageCircle size={14} />
                <span className="text-caption">
                  {config.scheduledGreetings.length} 个定时问候
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
        <button
          type="button"
          onClick={handleToggle}
          style={styles.toggleBtn}
        >
          {draftConfig.enabled ? (
            <ToggleRight size={32} style={{ color: "var(--color-rose-500)" }} />
          ) : (
            <ToggleLeft size={32} style={{ color: "var(--color-earth-400)" }} />
          )}
        </button>
      </div>

      {draftConfig.enabled && (
        <div style={styles.settingsPanel} className="animate-slide-up">
          {/* Important Dates */}
          <div style={styles.section}>
            <div style={styles.sectionHeader}>
              <Calendar size={14} />
              <span className="text-caption" style={{ fontWeight: 600 }}>重要日期提醒</span>
            </div>
            <p className="text-small" style={{ color: "var(--color-earth-500)", marginBottom: 12 }}>
              纪念日、生日等特殊日期，AI 会提前发送祝福
            </p>
            
            {draftConfig.importantDates.map((date, index) => (
              <div key={index} style={styles.inputRow}>
                <input
                  type="date"
                  value={date.date}
                  onChange={(e) => updateImportantDate(index, "date", e.target.value)}
                  style={styles.dateInput}
                />
                <input
                  type="text"
                  placeholder="描述（如：生日）"
                  value={date.description}
                  onChange={(e) => updateImportantDate(index, "description", e.target.value)}
                  style={styles.textInput}
                />
                <button
                  type="button"
                  onClick={() => removeImportantDate(index)}
                  style={styles.iconBtn}
                >
                  <Trash2 size={16} style={{ color: "var(--color-coral-500)" }} />
                </button>
              </div>
            ))}
            
            <button type="button" onClick={addImportantDate} style={styles.addBtn}>
              <Plus size={14} />
              添加日期
            </button>
          </div>

          {/* Overdue Reminder */}
          <div style={styles.section}>
            <div style={styles.sectionHeader}>
              <Clock size={14} />
              <span className="text-caption" style={{ fontWeight: 600 }}>超期未回复提醒</span>
            </div>
            <div style={styles.sliderRow}>
              <span className="text-small">超过</span>
              <select
                value={draftConfig.overdueReminderDays}
                onChange={(e) => setDraftConfig({ ...draftConfig, overdueReminderDays: Number(e.target.value) })}
                style={styles.select}
              >
                <option value={1}>1 天</option>
                <option value={2}>2 天</option>
                <option value={3}>3 天</option>
                <option value={5}>5 天</option>
                <option value={7}>7 天</option>
              </select>
              <span className="text-small">未聊天，AI 主动发起对话</span>
            </div>
          </div>

          {/* Scheduled Greetings */}
          <div style={styles.section}>
            <div style={styles.sectionHeader}>
              <MessageCircle size={14} />
              <span className="text-caption" style={{ fontWeight: 600 }}>定时问候</span>
            </div>
            <p className="text-small" style={{ color: "var(--color-earth-500)", marginBottom: 12 }}>
              在特定时间发送问候消息
            </p>
            
            {draftConfig.scheduledGreetings.map((greeting, index) => (
              <div key={index} style={styles.inputRow}>
                <input
                  type="time"
                  value={greeting.time}
                  onChange={(e) => updateScheduledGreeting(index, "time", e.target.value)}
                  style={styles.timeInput}
                />
                <input
                  type="text"
                  placeholder="问候语"
                  value={greeting.message}
                  onChange={(e) => updateScheduledGreeting(index, "message", e.target.value)}
                  style={styles.textInput}
                />
                <button
                  type="button"
                  onClick={() => removeScheduledGreeting(index)}
                  style={styles.iconBtn}
                >
                  <Trash2 size={16} style={{ color: "var(--color-coral-500)" }} />
                </button>
              </div>
            ))}
            
            <button type="button" onClick={addScheduledGreeting} style={styles.addBtn}>
              <Plus size={14} />
              添加问候
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
  container: {
    padding: "16px 18px",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
  },
  title: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    fontSize: "0.9rem",
    fontWeight: 600,
    color: "var(--color-earth-700)",
  },
  statusBadge: {
    padding: "2px 8px",
    borderRadius: "var(--radius-full)",
    fontSize: "0.7rem",
    fontWeight: 500,
  },
  editBtn: {
    padding: "6px 14px",
    borderRadius: "var(--radius-sm)",
    border: "1px solid var(--color-cream-300)",
    background: "var(--color-cream-100)",
    fontSize: "0.8rem",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-600)",
    transition: "all var(--duration-fast)",
  },
  summary: {
    marginTop: 12,
    display: "flex",
    flexDirection: "column",
    gap: 8,
  },
  summaryItem: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    color: "var(--color-earth-600)",
  },
  toggleRow: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    paddingBottom: 16,
    borderBottom: "1px solid var(--color-cream-200)",
  },
  toggleInfo: {
    display: "flex",
    alignItems: "flex-start",
    gap: 12,
  },
  toggleBtn: {
    background: "none",
    border: "none",
    cursor: "pointer",
    padding: 0,
    display: "flex",
    alignItems: "center",
  },
  settingsPanel: {
    paddingTop: 16,
    display: "flex",
    flexDirection: "column",
    gap: 20,
  },
  section: {
    padding: "12px",
    background: "var(--color-cream-50)",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-cream-200)",
  },
  sectionHeader: {
    display: "flex",
    alignItems: "center",
    gap: 6,
    marginBottom: 8,
    color: "var(--color-earth-700)",
  },
  inputRow: {
    display: "flex",
    gap: 8,
    marginBottom: 8,
    alignItems: "center",
  },
  dateInput: {
    padding: "6px 10px",
    borderRadius: "var(--radius-sm)",
    border: "1.5px solid var(--color-cream-300)",
    background: "white",
    fontSize: "0.8rem",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-700)",
    outline: "none",
  },
  timeInput: {
    padding: "6px 10px",
    borderRadius: "var(--radius-sm)",
    border: "1.5px solid var(--color-cream-300)",
    background: "white",
    fontSize: "0.8rem",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-700)",
    outline: "none",
    width: 100,
  },
  textInput: {
    flex: 1,
    padding: "6px 10px",
    borderRadius: "var(--radius-sm)",
    border: "1.5px solid var(--color-cream-300)",
    background: "white",
    fontSize: "0.8rem",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-700)",
    outline: "none",
  },
  select: {
    padding: "6px 10px",
    borderRadius: "var(--radius-sm)",
    border: "1.5px solid var(--color-cream-300)",
    background: "white",
    fontSize: "0.8rem",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-700)",
    outline: "none",
  },
  sliderRow: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    flexWrap: "wrap",
  },
  iconBtn: {
    background: "none",
    border: "none",
    cursor: "pointer",
    padding: 4,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    borderRadius: "var(--radius-sm)",
  },
  addBtn: {
    display: "flex",
    alignItems: "center",
    gap: 4,
    padding: "6px 12px",
    borderRadius: "var(--radius-sm)",
    border: "1px dashed var(--color-cream-300)",
    background: "none",
    fontSize: "0.8rem",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-500)",
    marginTop: 4,
  },
  actions: {
    display: "flex",
    gap: 12,
    marginTop: 20,
    paddingTop: 16,
    borderTop: "1px solid var(--color-cream-200)",
  },
  saveBtn: {
    padding: "8px 20px",
    background: "var(--color-rose-500)",
    color: "white",
    border: "none",
    borderRadius: "var(--radius-md)",
    fontSize: "0.9rem",
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-body)",
  },
  cancelBtn: {
    padding: "8px 16px",
    background: "none",
    border: "1px solid var(--color-cream-300)",
    borderRadius: "var(--radius-md)",
    fontSize: "0.9rem",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    color: "var(--color-earth-500)",
  },
};
