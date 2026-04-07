# Memora — Web UI Framework

> Frontend guide for Memora desktop. Backend rules are in [AGENTS.md](./AGENTS.md).

## Tech Stack
| Layer | Choice | Version |
|---|---|---|
| Runtime | Bun | latest |
| Framework | React + TypeScript | 19.x |
| Build | Vite | 8.x |
| Styling | TailwindCSS | 4.x |
| Routing | TanStack Router (hash mode) | 1.x |
| Data/State | TanStack Query | 5.x |
| Animation | Framer Motion | 12.x |
| Icons | Lucide React | 1.x |
| Desktop IPC | `@tauri-apps/api` | 2.x |
| Toasts | Sonner | 2.x |
| Markdown | react-markdown + remark-gfm | 10.x |

## Project Structure (Condensed)
```text
src/
├── main.tsx                      # bootstrap + QueryClientProvider + RouterProvider
├── App.tsx                       # root layout (Toaster + <Outlet />)
├── router.ts                     # TanStack Router route tree (hash history)
├── index.css                     # "Digital Keepsake" design system tokens + animations
├── features/                     # domain slices
│   ├── onboarding/               # WelcomeView (API key setup wizard)
│   ├── dashboard/                # DashboardView (persona grid + empty state)
│   ├── create/                   # CreateWizard (info → data → generate)
│   ├── chat/                     # ChatView, SessionSidebar, CorrectionDialog, MarkdownBubble
│   ├── profile/                  # ProfileView (persona detail, version history, voice config)
│   └── settings/                 # SettingsView (AI + TTS configuration)
├── hooks/                        # global hooks
├── lib/                          # tauri.ts (invoke wrappers), constants, utils
└── types/                        # shared TS types (index.ts)
```

## Architecture Rules
- Frontend data must flow through Tauri `invoke()` commands and Tauri events; no direct network fetches.
- State is hook-driven: `useQuery` for reads, `useMutation` for writes, `useState` for local UI state.
- Cross-route navigation uses `useNavigate()` / `useParams()` from `@tanstack/react-router`; no callback-prop navigation.
- Route params carry entity IDs (e.g. `/chat/$personaId`); components read params internally, not from parent props.
- After data mutations (create/delete persona), call `queryClient.invalidateQueries()` to trigger refetch.

## Routing

| Path | Component | Description |
|---|---|---|
| `/` | `DashboardView` | Persona grid, entry point |
| `/welcome` | `WelcomeView` | First-run API key setup |
| `/create` | `CreateWizard` | Multi-step persona creation |
| `/chat/$personaId` | `ChatView` | Chat with streaming AI reply |
| `/profile/$personaId` | `ProfileView` | Persona detail, edit, version history |
| `/settings` | `SettingsView` | AI + TTS provider configuration |

- Router uses `createHashHistory()` for Tauri compatibility (no web server).
- `App.tsx` is the root route component; it checks API key on mount and redirects to `/welcome` or `/`.
- Type-safe routing via module augmentation: `declare module "@tanstack/react-router" { interface Register { router: typeof router } }`.

## Streaming UX Rules
- Chat streaming: invoke `send_message`, listen to `chat://stream` events.
  - Events emit `{ delta, request_id }`; UI accumulates `delta` into a live typing bubble.
  - On `send_message` resolve, replace stream bubble with final message.
- Persona generation: invoke `generate_persona`, listen to `generate://progress` events.
  - Events emit `{ step, total, label }`; UI renders a progress bar with label text.
  - Navigation to chat happens after the invoke resolves, not on progress completion.
- Event listeners must be cleaned up via the returned `unlisten` function in `finally` blocks.

## Visual System — "Digital Keepsake"

Warm, intimate, paper-like. Not a tech product — a personal artifact.

### Core Tokens (OKLCH)
| Token | Value | Usage |
|---|---|---|
| `cream-50` | `oklch(98.5% 0.008 75)` | App background |
| `cream-100` | `oklch(96.8% 0.012 72)` | Card / input background |
| `cream-200` | `oklch(93% 0.018 70)` | Hover states, dividers |
| `cream-300` | `oklch(88% 0.022 68)` | Borders |
| `earth-500` | `oklch(50% 0.03 52)` | Secondary text |
| `earth-800` | `oklch(24% 0.02 45)` | Primary text |
| `rose-400` | `oklch(70% 0.12 12)` | User chat bubble |
| `rose-500` | `oklch(62% 0.15 12)` | Primary action, CTA |
| `sage-500` | `oklch(62% 0.1 148)` | Success |
| `coral-500` | `oklch(58% 0.15 25)` | Error / danger |
| `lavender-300` | `oklch(82% 0.06 290)` | Subtle accent |

### Typography
| Class | Font | Usage |
|---|---|---|
| `--font-display` | LXGW WenKai, Newsreader, serif | Headings, hero text |
| `--font-body` | LXGW WenKai, PingFang SC, system-ui | Body text |
| `.text-hero` | Display font, clamp(2–3.5rem) | Splash page title |
| `.text-heading` | Display font, clamp(1.25–1.75rem) | Section heading |
| `.text-caption` | Body font, 0.8rem, earth-500 | Labels, timestamps |

### Component Direction
- Chat bubbles: user = `rose-400` with white text; assistant = `cream-200` with dark text.
- Cards and inputs use `cream-100` background with `cream-300` border.
- Emoji avatars sit inside `cream-200` circles.
- Body background includes subtle warm radial gradients for paper-like texture.
- Motion should be purposeful: entry (`slide-up`, `fade-in`, `scale-in`), staggered children, and hover transitions.
- Respect `prefers-reduced-motion`.

### Animations
| Class | Keyframes | Easing |
|---|---|---|
| `.animate-fade-in` | opacity 0→1 | ease-out-quart, 250ms |
| `.animate-slide-up` | translateY(12→0) + opacity | ease-out-expo, 400ms |
| `.animate-scale-in` | scale(0.96→1) + opacity | ease-out-quart, 250ms |
| `.stagger-children` | slide-up with 60ms delay increments | ease-out-expo |

## Conventions
- Styling: TailwindCSS utilities only; inline `style={}` objects are forbidden.
- Types: shared types in `src/types/index.ts`.
- Icons: Lucide React.
- IPC wrappers: all `invoke()` calls live in `src/lib/tauri.ts`; components never call `invoke()` directly.
- Tauri event listeners: use `listen()` from `@tauri-apps/api/event` with proper cleanup.

## Desktop UX Conventions
- Context menu is disabled globally except in `<input>`, `<textarea>`, and `contentEditable` elements.
- External links (`http(s)://`) open in system browser via `@tauri-apps/plugin-shell`.
- Destructive actions (delete persona, clear cache) use explicit confirmation UI, not browser `confirm()`.
- Scrollbars are styled to 5px width with warm cream thumb colors.
- WeChat-style chat timestamps: show timestamp chip when gap between consecutive messages ≥ 5 minutes.

## Maintenance Rules
- Frontend structure/convention changes must update this file first.
- Backend architecture changes must update `AGENTS.md` first.
- Keep structure sections aligned with real filesystem layout.

## Do NOT
- Do not use CSS modules or styled-components.
- Do not bypass backend commands with direct network fetches for app data flows.
- Do not pass navigation callbacks as component props; use `useNavigate()` from the router.
- Do not use `useState` + manual fetch for data that should go through `useQuery`.
