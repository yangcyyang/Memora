# Memora — Code Framework

## Architecture
Memora is a Tauri v2 desktop app with a React SPA frontend and Rust backend.
All user data stays local — only AI inference requests go through external APIs.

- Frontend calls backend via `invoke()` and Tauri events.
- Tauri commands are defined in `src-tauri/src/commands/` as thin wrappers that delegate to services.
- Business logic lives in `src-tauri/src/services/`.
- Data access is encapsulated in `src-tauri/src/repo/` (no raw SQL elsewhere).
- Persistence is SQLite via `r2d2` connection pool (`~/.memora/memora.db`) plus JSON config files.
- Subsystems (`ai/`, `tts/`, `bridge/`) are provider-agnostic with trait-based extension.

> Frontend-specific conventions live in [AGENTS-UI.md](./AGENTS-UI.md).

## Key Paths
| Purpose | Path |
|---|---|
| Data root | `~/.memora/` |
| SQLite database | `~/.memora/memora.db` |
| AI config | `~/.memora/ai_config.json` |
| TTS config | `~/.memora/tts_settings.json` |
| Persona files | `~/.memora/personas/` |
| Audio cache | `~/.memora/audio/` |

## Project Structure (Condensed)
```text
Memora/
├── src/                           # React app
│   ├── main.tsx                   # bootstrap + providers
│   ├── App.tsx                    # root layout + Outlet
│   ├── router.ts                  # TanStack Router (hash mode)
│   ├── features/                  # domain slices
│   │   ├── onboarding/            # WelcomeView (setup wizard)
│   │   ├── dashboard/             # DashboardView (persona grid)
│   │   ├── create/                # CreateWizard (persona generation)
│   │   ├── chat/                  # ChatView, SessionSidebar, CorrectionDialog
│   │   ├── profile/               # ProfileView (persona detail + edit)
│   │   └── settings/              # SettingsView (AI + TTS config)
│   ├── hooks/                     # global hooks
│   ├── lib/                       # tauri.ts (IPC bindings), constants, utils
│   └── types/                     # shared TS types
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                 # entry: plugin registration + command binding
│   │   ├── error.rs               # unified AppError enum
│   │   ├── models.rs              # shared data types (Persona, ChatMessage, …)
│   │   ├── prompts.rs             # prompt templates
│   │   ├── commands/              # Tauri command layer (thin shell)
│   │   │   ├── chat.rs            #   send_message, get_chat_history, sessions
│   │   │   ├── persona.rs         #   CRUD, versions, rollback
│   │   │   ├── generator.rs       #   generate_persona
│   │   │   ├── correction.rs      #   submit_correction
│   │   │   ├── settings.rs        #   get/save AI settings, validate key
│   │   │   ├── tts.rs             #   TTS settings, speak, clone, cache
│   │   │   ├── parser.rs          #   detect_and_parse, parse_pasted_text
│   │   │   ├── ocr.rs             #   capture_and_ocr (macOS Vision)
│   │   │   ├── bridge.rs          #   WebSocket bridge control
│   │   │   └── updater.rs         #   auto-update commands
│   │   ├── services/              # business logic
│   │   │   ├── chat_service.rs    #   context assembly + AI call
│   │   │   ├── generator_service.rs # persona generation pipeline
│   │   │   ├── correction_service.rs # memory correction
│   │   │   └── compaction.rs      #   session context compaction
│   │   ├── repo/                  # data access layer (all SQL here)
│   │   │   ├── persona_repo.rs
│   │   │   ├── chat_repo.rs
│   │   │   ├── session_repo.rs
│   │   │   └── voice_repo.rs
│   │   ├── ai/                    # AI provider subsystem
│   │   │   ├── config.rs          #   per-provider config load/save
│   │   │   ├── completion.rs      #   non-streaming chat completion
│   │   │   ├── streaming.rs       #   SSE streaming completion
│   │   │   └── validation.rs      #   API key validation
│   │   ├── tts/                   # TTS provider subsystem
│   │   │   ├── provider.rs        #   trait + config + factory
│   │   │   ├── minimax.rs         #   MiniMax implementation
│   │   │   ├── cache.rs           #   LRU audio cache
│   │   │   └── audio_utils.rs     #   ffmpeg detection + video audio extraction
│   │   ├── bridge/                # external bridges
│   │   │   ├── ws_server.rs       #   WebSocket server (Chrome ext)
│   │   │   └── clipboard.rs       #   clipboard watcher
│   │   ├── infra/                 # infrastructure
│   │   │   ├── db.rs              #   r2d2 pool + schema init
│   │   │   ├── paths.rs           #   path management
│   │   │   ├── crypto.rs          #   AES-256-GCM encryption
│   │   │   └── http_client.rs     #   shared reqwest client
│   │   └── parsers/               # data parsers
│   │       ├── wechat.rs          #   WeChat txt/html/csv
│   │       ├── wechat_win.rs      #   Windows WeChat DB
│   │       ├── imessage.rs        #   macOS iMessage
│   │       ├── ios_backup.rs      #   iOS backup extraction
│   │       └── detect.rs          #   format auto-detection
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/
│   ├── TTS.md                     # TTS module developer guide
│   ├── CHANGELOG.md
│   └── Error.md
├── AGENTS.md
└── AGENTS-UI.md
```

## Frontend Dependencies
| Package | Version | Purpose |
|---|---|---|
| `react` / `react-dom` | 19.x | UI runtime |
| `vite` | 8.x | Build tool |
| `tailwindcss` | 4.x | Styling |
| `@tauri-apps/api` | 2.x | IPC bridge |
| `@tanstack/react-router` | 1.x | Hash-based routing |
| `@tanstack/react-query` | 5.x | Server state management |
| `framer-motion` | 12.x | Motion |
| `lucide-react` | 1.x | Icons |
| `sonner` | 2.x | Toasts |
| `react-markdown` | 10.x | Markdown render |
| `@biomejs/biome` | 2.x | Lint + format (dev) |
| `vitest` | 3.x | Unit testing (dev) |

## Backend Dependencies (Rust)
| Crate | Version | Purpose |
|---|---|---|
| `tauri` | 2 | Desktop framework |
| `tokio` | 1 | Async runtime |
| `reqwest` | 0.13 | HTTP |
| `serde` / `serde_json` | 1 | Serialization |
| `rusqlite` | 0.39 | SQLite driver |
| `r2d2` / `r2d2_sqlite` | 0.8 / 0.33 | Connection pool |
| `anyhow` | 1 | Error handling (services/infra) |
| `thiserror` | 2 | Error handling (AppError enum) |
| `aes-gcm` | 0.10 | API key encryption |
| `chrono` | 0.4 | Time |
| `uuid` | 1 | ID generation |
| `async-trait` | 0.1 | Async trait support |
| `tracing` | 0.1 | Structured logging |
| `scraper` / `csv` | 0.22 / 1 | Chat record parsing |
| `tokio-tungstenite` | 0.26 | WebSocket bridge |
| `filetime` | 0.2 | Audio cache LRU |

## Backend Layering Rules

### Commands (Thin Shell)
- Commands must not contain raw SQL, business logic, or direct HTTP calls.
- Command body is: validate args → call service/repo → `map_err(Into::into)`.
- Error handling uses `error::AppError`; never use `.map_err(|e| e.to_string())`.

### Services (Business Logic)
- Orchestrate `repo` and `infra` modules to fulfill a use case.
- May call `ai/` and `tts/` subsystems.
- Use `tokio::task::spawn_blocking` for DB-heavy operations.

### Repo (Data Access)
- All SQL lives here — no exceptions.
- One file per table group: `persona_repo`, `chat_repo`, `session_repo`, `voice_repo`.

### AI Subsystem
- Provider-agnostic: config, completion, streaming, validation are separate modules.
- Add new AI provider → create file in `ai/`, implement the required functions.
- API keys are encrypted at rest via `infra/crypto.rs` (AES-256-GCM, device-bound key).

### TTS Subsystem
- Provider-agnostic via `TtsProvider` trait in `tts/provider.rs`.
- Add new TTS provider → create file in `tts/`, implement the trait, register in factory.
- See `docs/TTS.md` for detailed extension guide.
- Audio cache is LRU-based, keyed by `SHA256(provider|voice_id|text|language)`.

### Bridge
- WebSocket server accepts chat-record push from Chrome extension.
- Clipboard watcher runs in background for quick paste capture.

### Parsers
- Stateless parsing functions: file path or text → `ParsedContent`.
- Format auto-detection in `detect.rs`.
- Supported: WeChat (txt/html/csv/Windows DB), iMessage, iOS backup.

## Backend Behavior Rules

### Chat and Context
- Chat completion supports SSE streaming via `chat://stream` events.
- Context assembly includes: persona markdown, shared memories, session summary, recent messages.
- Session compaction triggers when message count exceeds threshold; summarizes older messages via AI.
- Streaming events emit `{ delta, request_id }` payloads.

### Persona Generation
- Generation is multi-step: analyze → personality → memories → create.
- Progress events are emitted on `generate://progress` channel.
- Each step is tracked `{ step, total, label }` for frontend progress bar.

### Correction
- User can correct assistant responses to refine persona memory.
- Correction updates persona markdown via AI diff and creates a new version.

### Data Security
- API keys encrypted with `aes-gcm` using device-bound machine ID.
- All data stored locally; no telemetry, no cloud sync.
- External network calls are AI inference only.

## Auto-Update
Memora uses `tauri-plugin-updater`.

- Plugin setup in `src-tauri/src/lib.rs`
- Commands in `src-tauri/src/commands/updater.rs`
- CI release pipeline in `.github/workflows/release.yml`

Release checklist:
1. Bump version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
2. Commit version bump.
3. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.

## Maintenance Rules
- Document-first:
  - Backend or architecture change → update `AGENTS.md` first.
  - Frontend structure/convention change → update `AGENTS-UI.md` first.
- Keep directory/dependency sections in docs synced with reality.
- Log significant bug investigations and fixes in `docs/Error.md`.

### Commit Guidelines
Use Conventional Commits: `type(scope): description`

- `type`: `feat` / `fix` / `docs` / `style` / `refactor` / `perf` / `test` / `chore`
- `scope`: feature area such as `chat`, `persona`, `tts`, `settings`
- Commit messages must be English.

## Do NOT
- Do not manually add Rust dependencies by editing `Cargo.toml`; use `cargo add`.
- Do not write raw SQL outside `repo/`.
- Do not use `style={}` inline objects in React components; use Tailwind.

## Quality & CI
- Lint + format: `bun run lint` / `bun run lint:fix` / `bun run format` (Biome).
- Frontend tests: `bun run test` / `bun run test:watch` (Vitest).
- Backend build: `cargo build` from `src-tauri/`.