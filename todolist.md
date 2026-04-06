# Memora 剩余功能与优化待办清单 (To-Do List)

## 📌 Phase 1: 核心体验补全与优化 (UI/UX & Core)

**客户端框架与构建**
- [x] 🎨 **自定义图标替换**: 完成替换 macOS 和 Windows 的所有默认 tauri 图标为空白占位符或真实图标。
- [x] 📦 **MacOS 打包签名**: 配置 `tauri.conf.json` 中的 bundle identifier 和 bundle 构建项，确保能正确打出 `.app` 和 `.dmg`。
- [x] 🪟 **窗口外观优化**: (如果需要) 将 Tauri 窗口配置为 macOS 原生半透明材质 (Vibrancy) 或无边框 (Titlebar-less) 沉浸式设计。

**聊天体验 (ChatView)**
- [x] 📜 **聊天历史记录加载**: 当前 ChatView 的 `getChatHistory` 还未通过 UI 触发，需实现向下/向上滚动加载历史消息的能力。
- [x] 📚 **历史会话侧边栏 (Session Sidebar)**:
  - 允许在同一个 Persona 下创建多个不同的聊天 Session。
  - 能切换历史 Session。
- [x] 📝 **Markdown 渲染**: 聊天气泡内当前是不支持 Markdown 的（仅换行），应当引入 `react-markdown` 来支持代码块、加粗等标准格式。
- [x] 📤 **主动记忆纠错触发器 (Correction)**: 为聊天气泡添加“纠正”按钮，可手动调用 `correction.rs`，让 AI 修改当前 Persona 的底层记忆并更新版本。

**Persona/Profile 管理**
- [x] ✏️ **描述与标签可编辑**: 在 ProfileView 中，不仅要展示，还要允许用户手动直接修改 AI 的 tags 和 description。
- [x] 🛠️ **底层 Markdown 强编**: 在 ProfileView 允许资深用户直接修改底层的 `persona_md` 和 `memories_md` 本文，以微调设定。

## 🚀 Phase 2: 自动化数据采集与生态整合 (Data Acquisition)

这是后续增强应用核心竞争力的计划：由于纯靠“导出 TXT”门槛太高，自动/半自动摄取数据才是未来。

**1. 苹果生态本地直读 (MacOS Native Integration)**
- [x] 📱 **iMessage 全磁盘直读 (macOS)**: 
  - 要求应用获取 Full Disk Access (FDA) 权限。
  - 直读 `~/Library/Messages/chat.db` SQLite 数据库。
  - 提取指定联系人的双向聊天记录。
- [x] 🍏 **iOS 本地备份解析**:
  - 定位 `~/Library/Application Support/MobileSync/Backup`.
  - 解析未加密的 iOS 备份 SQLite (用于提取微信记录等)。

**2. 桌面自动化提取**
- [x] 👁️ **基于视觉的屏幕文本提取 (OCR)**:
  - 提供一个类似 Snipaste 的选区截图工具能力 (Rust 层封装 MacOS 底层截屏 API)。
  - 调用 MacOS 自带的 Vision.framework (离线 OCR) 读取屏幕上的聊天。
- [x] 📋 **实时剪贴板监控 (Clipboard Watcher)**:
  - 在后台监控剪贴板的复制动作。
  - 用正则或预设格式判断该文本是否属于有效“语料”，静默收入特定 Persona 的后备记忆库。

**3. 第三方平台抓取方案补全**
- [x] 🖥️ **Windows 微信内存/本地 DB 解析**: 基于当前技术社区已公开的获取 windows 端 wx 数据库密钥的方案，实现在同电脑上的直读 (高难度，作为可选进阶)。
- [x] 🌐 **Chrome 插件联动**: 创建一个简单的浏览器扩展，在用户浏览网页版聊天应用、微博日志时，通过 WebSocket 将数据一键推送到后台的 Tauri TCP 端口。


---
*Note: Phase 1 中除了图标外，所有已实装的 IPC 和组件均为生产可用状态，没有技术阻塞。Phase 2 的 MacOS 数据库直读可能会遇到较严格的沙盒和签名权限问题，需要额外查阅 Apple 查档。*
