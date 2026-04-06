# Memora

Memora 是一款基于 **Tauri + React + Rust** 构建的桌面端 AI 记忆与会话管理应用。它旨在通过自动和半自动的数据采集机制，提取并整合用户的会话、文件、以及环境中的上下文信息，为用户打造拥有专属“记忆”（Memory）的本地 AI 智能体（Persona），从而实现沉浸、个性化且极具拓展性的聊天体验。

## 🌟 核心特性 (Key Features)

### 💬 智能聊天体验
- **多模型支持**：支持配置调用多种 AI Provider（如 OpenAI, Anthropic, MiniMax, DeepSeek 等），灵活切换不同语言模型并保持各自的安全配置。
- **Session 隔离与管理**：同一个 Persona 支持创建多个独立的聊天会话（Session）。
- **Rich Markdown 渲染**：基于 `react-markdown` 支持代码块、加粗、语法高亮等标准 Markdown 内容渲染。
- **主动记忆纠错触发器 (Correction)**：基于会话中的互动一键“提取”新知识，或手动触发 AI 修改和纠正 Persona 的底层记忆设定，并进行版本迭代。
- **上下文自动压缩 (Context Compact)**：移植并优化了高级上下文压缩机制。基于动态长文本阈值触发异步后台总结，自动清理过期 Token 并动态组合记忆碎片，有效降低长会话请求延迟及 Token 消费。

### 🤖 Persona 与 Profile 管理
- **细粒度数据调优**：允许高级用户直接预览并对底层的 `persona_md` 和 `memories_md` 等源文件进行高深度的 Prompt 微调。
- **描述与标签编辑**：具备高可玩性的资料设定，随时修改并打磨自定义 AI 的性格及记忆标签。

### 🔄 自动化数据采集体系 (Data Acquisition)
打破传统“手动导出 TXT 投喂”的壁垒，构建了跨越原生系统与第三方平台的数据摄取网络（目前均已实装可用）：
1. **苹果生态本机直读**：获取 macOS Full Disk Access 等权限后，应用将直接解析本地 `iMessage` SQLite (`chat.db`) 及未加密的 iOS 本地备份目录。
2. **桌面自动化提取**：
   - 带有专门的屏幕框选 OCR 截屏能力（调用原生 `Vision.framework` 离线框架）提取临时会话。
   - 配置有实时剪贴板监控进程，将后台命中的关键片段静默收集沉淀。
3. **第三方多端系统整合**：
   - 从底层破解了对 Windows 本地 **微信数据库** 的解析机制，直接无缝入库。
   - 适配配套的 **Chrome Extension** (利用 WebSocket 桥接)，通过浏览器向 Tauri 后排直推网页内的社媒信息。

## 🛠️ 技术栈 (Tech Stack)

- **前端 UI (Frontend)**: React 19, TypeScript, Vite, Tailwind CSS v4, Framer Motion, Radix UI 组件库
- **系统底层 (Backend)**: Rust (Tauri v2), SQLite 结构化存储
- **工程化 (Toolchain)**: Biome (Linter / Formatter), Vitest (Unit Test), Bun

## 📦 本地开发指南 (Getting Started)

### 环境依赖
确保本机已正确安装以下软件及环境：
- Node.js (>= 18) 或 **Bun** (推荐用作包管理)
- Rust 编译器与工具链 (`cargo`)
- 对于 macOS 需事先安装配套命令行及原生库 (`xcode-select --install`)

### 启动项目

```bash
# 1. 克隆项目
git clone https://github.com/xxww0098/Memora.git
cd Memora

# 2. 安装依赖
bun install

# 3. 启动本地开发与联调
bun run tauri dev
```

### 构建与打包

```bash
# 编译生产用可执行文件 / Bundle (macOS .app 与 .dmg 等)
bun run tauri build
```
（注：打包好的文件将放置在 `src-tauri/target/release/bundle/macos/` 中；项目已提前配置好 macOS 版本专属的 Vibrancy 材质与权限签名要求）。


## 📄 许可证
当前属于实验性
