# 研究生-kimi 工作记忆记录

## 基本信息
- **角色**: 前端开发 + Rust 后端开发
- **项目**: Memora (二次元AI陪伴应用)
- **工作目录**: `/Users/cy/Documents/03 life/AI design/OrbitOS-CN/20_项目/龙虾-二次元/Memora`

---

## Sprint 1 完成任务

### T1: 新前端框架 + 侧边栏布局 ✅
- 文件: `src/components/Sidebar.tsx`
- 功能: 280px 可调整侧边栏，Persona 列表展示
- 状态: 通过

### T2: Chat页面 + 气泡菜单 ⚠️ 有条件通过
- 文件: `src/features/chat/ChatView.tsx`, `src/features/chat/MessageContextMenu.tsx`
- 功能: 右键菜单（记住这个/纠错）
- 状态: 代码完成，待 GUI 真实调用验证 `reinforce_memory`

### T3: 校准测试 Step 4 ✅
- 文件: `src/features/create/CreateWizard.tsx`
- 功能: AI生成样本 → 用户标记像/不像 → 提交反馈
- 状态: 通过
- 后端接口: `generate_calibration_samples`, `submit_calibration_feedback`

---

## Sprint 2 完成任务

### T6: Settings 页面 ✅
- 文件: `src/features/settings/SettingsView.tsx`, `src/features/settings/index.tsx`
- 功能: AI Provider选择、API Key配置、Ollama配置、初级/高级版切换
- 状态: 通过
- 统一接口: `getSettings()`, `saveAiSettings()`, `validateKey()`

### T10: 主动触达持久化 ✅
- **后端改动**:
  - `src/infra/db.rs`: personas 表新增 `proactive_enabled`, `proactive_rules`
  - `src/repo/persona_repo.rs`: `save_proactive_settings()`, `get_proactive_settings()`
  - `src/commands/persona.rs`: `save_proactive_settings` command, `get_proactive_settings` command
  - `src/lib.rs`: 注册新命令
- 状态: 通过

---

## 关键接口汇总

### 前端 tauri.ts
```typescript
// Settings
getSettings() => AppSettings
saveAiSettings(provider, apiKey, baseUrl, model)
validateKey(provider, apiKey, baseUrl, model) => boolean

// Calibration
generateCalibrationSamples(personaId) => CalibrationSample[]
submitCalibrationFeedback(personaId, feedbackItems, freeText?)

// Proactive (T10)
saveProactiveSettings(personaId, enabled, rulesJson)
getProactiveSettings(personaId) => { enabled, rules }
```

### 后端 Commands
```rust
// T3 Calibration
generate_calibration_samples(persona_id) -> Vec<CalibrationSample>
submit_calibration_feedback(persona_id, feedback_items, free_text) -> CalibrationApplyResult

// T10 Proactive
save_proactive_settings(id, enabled, rules_json)
get_proactive_settings(id) -> ProactiveSettings { enabled, rules }
```

---

## 遗留任务

| 任务 | 状态 | 说明 |
|------|------|------|
| T2 GUI验证 | 待验证 | Chat右键"记住这个"真实调用 `reinforce_memory` |
| T8 GUI验证 | 待验证 | 托盘图标、恢复窗口、自启开关 |

---

## 技能库

已下载并归纳:
- `/Users/cy/skills/khazix-skills/`
  - `hv-analysis/SKILL.md` - 横纵分析法深度研究
  - `khazix-writer/SKILL.md` - 卡兹克公众号写作风格
  - `prompts/横纵分析法.md` - 可直接使用的Prompt

---

## 项目技术栈
- **前端**: React + TypeScript + Vite + TailwindCSS + shadcn/ui
- **后端**: Rust + Tauri v2 + SQLite
- **包管理**: Bun
- **路由**: TanStack Router

---

## 最近提交 (Sprint 2)
- `0b23350` - Sprint 2 主要功能合并提交
- `1a1e10a` - 修复 autostart 配置
- `9a60cab` - 添加缺失的 @radix-ui/* 依赖

---

## 当前阻塞
等待 @yangcyyang GUI 冒烟测试:
1. 托盘图标出现
2. 托盘"显示 Memora"恢复窗口
3. Settings 开机自启开关切换
4. Chat 右键"记住这个"成功提示

---

*最后更新: 2026-04-14 15:59*
