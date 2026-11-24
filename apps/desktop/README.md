# AutoCompare Desktop (Tauri v2 shell) — Wave 0 scaffold

Thin macOS-first desktop client over the headless engine. **Zero engine changes:**
`crates/gui` and all other crates are untouched; this shell owns no business
logic yet.

## What was scaffolded

Rust (`apps/desktop/src-tauri`, crate `autocompare-desktop`, edition 2024,
`#![forbid(unsafe_code)]`):

- `src/main.rs` / `src/lib.rs` — Tauri v2 builder wiring (dialog plugin,
  managed `AppState`, native menu hookup, **empty** `invoke_handler` +
  exported `COMMANDS: &[&str]` placeholder registry).
- `src/state.rs` — `AppState: Mutex<HashMap<SessionId, CachedSession>>`
  skeleton. **Paged-fetch rule (locked):** the front end must page large
  content via `(offset, limit)` windows; no command may serialize a whole file
  over IPC.
- `src/menu.rs` — native macOS menu skeleton: File, Edit, Session, View, Tools
  with stable namespaced item ids (`file.open-session`, …).
- `tauri.conf.json` — macOS native titlebar (`Visible`), `vibrancy: true`,
  locked CSP, window `1280x800`, bundle id `com.ScooterSoftware.BeyondCompare`
  (matches `craft.json` target).
- `capabilities/default.json` — `core:default` + `dialog:allow-open/save`.
- Registered as workspace member in root `Cargo.toml` and as L3 in
  `xtask/src/main.rs` (`crate_layer`).

Web (`apps/desktop`, Vite + React 19 + TypeScript + Tailwind v4):

- `src/main.tsx` + `src/App.tsx` — boots to a placeholder Home view
  (`sessions: 0`).
- `src/ipc/types.ts` — empty locked DTO placeholders (`SessionId`,
  `CachedSession`, `PageRequest`, `PageResponse`) referencing the Rust `serde`
  names they will mirror.
- `src/ipc/client.ts` — typed `invoke` wrapper + `VITE_MOCK_IPC=1`
  fixture-mock adapter (empty pages until Wave 1 registers commands).
- `src/theme/tokens.css` — macOS light/dark token skeleton (SF Pro / SF Mono
  stacks, pastel diff washes).
- `playwright.config.ts` — WebKit-only, `deviceScaleFactor: 2`, viewport
  `1280x800` (matches the Tauri window), mock-IPC webServer.
- `e2e/home.spec.ts` — placeholder boot assertion.

## Dev commands

```sh
# web frontend (from apps/desktop)
npm install
npm run dev        # Vite on http://localhost:1420 (VITE_MOCK_IPC=1 for UI-only work)

# full shell (requires Tauri system deps + Rust target)
cargo tauri dev    # run from apps/desktop (uses src-tauri/tauri.conf.json)

# checks
cargo check -p autocompare-desktop
cargo fmt --check
cargo xtask layers
npm run build      # tsc --noEmit + vite build
npx playwright test
```

## What Wave 1 owns next

- `src-tauri/src/commands/*` — real `#[tauri::command]` handlers
  (session open/list, paged content fetch, dialog pickers) + registration in
  `COMMANDS` / `invoke_handler`.
- Lock `ipc/types.ts` field lists and `serde(rename_all)` conventions.
- Replace fixture branches in `ipc/client.ts` command-by-command.
- Session tree / tabbed workspace UI on top of the Home view; per-view theme
  tokens; menu-event → command routing.
