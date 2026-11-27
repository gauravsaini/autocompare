# AutoCompare Architecture v3 — Plugin-First Diff Harness (ADR)

**Status**: Accepted (Phase 1 & Phase 2 implemented on branch `v2`)
**Date**: 2026-11-27
**Inspiration**: Seam + Plugin + Multi-Profile + Projection architecture of [DeepSeek Harness (`dsh`)](https://github.com/deepseek-ai/deepseek-harness), adapted to AutoCompare's `#![forbid(unsafe_code)]` Rust engine.

---

## 1. Context & Problem Statement

AutoCompare already has a clean, memory-safe Rust core (`crates/diff`, `crates/compare`, `crates/vfs`, `crates/sessions`, `crates/reports`, `crates/cli`) and a fast React 19 + Tauri v2 desktop frontend (`apps/desktop/src/v2`).

However, scaling beyond the initial 8 comparison views exposes three structural bottlenecks:
1. **Central Wiring Tax**: Adding a new comparator (e.g., JSON/YAML AST diff, SQLite schema diff, PDF diff, S3/SFTP folder diff) previously required editing `V2App.tsx`, `V2Home.tsx`, `V2NewSessionModal.tsx`, `types.ts`, `useTabs.ts`, `lib.rs`, and `mockAdapter.ts`.
2. **Transport & Mock Duplication**: `mockAdapter.ts` duplicates ~1,000 lines of diff/folder/table logic in JS because browser/Playwright runs cannot talk directly to the Rust engine without Tauri.
3. **Desktop-Only Onboarding**: Users must download a full desktop installer before trying the UI, rather than launching a zero-install local Web UI (`autocompare web`).

## 2. Guiding Principle: Rust Microkernel + Plugin Seams

We **keep Rust as the high-speed, zero-`unsafe` core engine** (we do **not** rewrite diffing into Node/TS) and adopt four explicit 3-role seams (`Service Definition` -> `Service Provider` -> `Consumer`):

| Seam | Responsibility | Built-in Providers | Extension Target |
| :--- | :--- | :--- | :--- |
| **1. Comparator Seam** | Declares UI view, icon, shortcut, file-extension matchers, and tab lifecycle (`apps/desktop/src/v2/plugins/registry.ts`). | Text, Folder, Table, Hex, 3-Way Merge, Folder Merge, Folder Sync, Media, **JSON/YAML Structural Diff**. | Community/domain comparators (SQLite, Parquet, Protobuf, OpenAPI, Kubernetes manifests). |
| **2. VFS Seam** | URI-routed virtual filesystem (`crates/vfs`: `Vfs` trait). | `LocalFs`, `ZipFs`, `TarGzFs`, `SnapshotFs` (`.bcss`). | `git://`, `sftp://`, `s3://`, `gcs://`, container image layers. |
| **3. Format Converter Seam** | Declarative pre-compare canonicalization (`autocompare.formats.toml`). | Canonical JSON/YAML, CRLF/encoding normalization, grammar rules. | Sandboxed read-only converters (`.pdf` -> text, `.docx` -> markdown, `.xlsx` -> CSV). |
| **4. Report Seam** | Decoupled diff report rendering (`crates/reports` + `reportBuilder.ts`). | Side-by-Side HTML, Unified `.patch`, Markdown, RFC-4180 CSV, JSON, XML. | JUnit XML for CI, SARIF, PR comment summaries. |

## 3. Codebase Readiness Evidence

- **Single Frontend Transport Chokepoint**: `apps/desktop/src/ipc/client.ts` is 32 lines; all views funnel through `invoke<T>(cmd, args)`.
- **VFS Trait Already Implemented**: `crates/vfs/src/lib.rs` already exports `Vfs`, `LocalFs`, `ZipFs`, `TarGzFs`, and `SnapshotFs`.
- **Decoupled Reports Already Built**: `crates/reports/src/lib.rs` already separates `ReportInput` / `Hunk` / `LinkedReportBundle` from renderers.
- **Schema-Versioned Sessions**: `crates/sessions/src/lib.rs` already provides `atomic_write` and versioned `migrate` functions.

## 4. 6-Phase Migration Roadmap

### Phase 1: Frontend Comparator Plugin Registry & UI Slots
- **Scope**: Create `apps/desktop/src/v2/plugins/registry.ts` and `builtins.tsx`. Migrate all 8 built-in views into declarative plugin descriptors. Drive `V2App.tsx`, `V2Home.tsx`, `V2NewSessionModal.tsx`, `V2CommandPalette`, and `useTabs.ts` purely from the registry.
- **Evidence Gate**: `V2App.tsx` has zero hardcoded comparator view switch branches; `npm test` and `npm run build` pass with 0 errors/warnings.

### Phase 2: Litmus Test — Plugin #9 (`JSON / YAML Structural Diff`)
- **Scope**: Add a pure AST comparator (`utils/structuredDiff.ts`, `views/V2StructuredCompare.tsx`, `plugins/structuredDiffPlugin.tsx`) registered solely through the plugin registry — **zero edits** to `V2App.tsx`, `V2Home.tsx`, `V2NewSessionModal.tsx`, or `useTabs.ts`.
- **Evidence Gate**: Plugin appears automatically in Home, New Session modal, Command Palette, and Tab renderer; unit tests verify key-order-insensitive AST diffs and JSONPath tracking.

### Phase 3: Rust `VfsRegistry` + Declarative Format Converter Seam
- **Scope**: Add a URI-routed `VfsRegistry` (`file://`, `zip://`, `tar://`, `bcss://`) in `crates/vfs` and a declarative `autocompare.formats.toml` read-only converter pipeline in `crates/compare`.
- **Evidence Gate**: `cargo test --workspace` proves Zip-vs-Folder comparison and format-converted text diff run deterministically with `#![forbid(unsafe_code)]`.

### Phase 4: Unified Transport + `crates/server` (`autocompare web`)
- **Scope**: Add a thin local Axum HTTP/JSON-RPC server (`crates/server`) wrapping existing command handlers, add an `http` transport branch to `apps/desktop/src/ipc/client.ts`, and expose `autocompare web [--port 3080] [--no-open]` in `crates/cli`.
- **Protocol**: `POST /rpc` with `{ "cmd": string, "args": Record<string, unknown> }` bound strictly to `127.0.0.1` with a per-process random bearer token (CSRF/DNS-rebinding protection).
- **Evidence Gate**: `autocompare web --no-open` serves the v2 UI and runs Playwright `e2e/v2/` against the real Rust backend with zero `mockAdapter.ts` fallback.

### Phase 5: Event-Sourced Session Log & Cross-View Projections
- **Scope**: Add an append-only `SessionEvent` log (`EditLine`, `ApplyHunk`, `PinAnchor`, `ResolveConflict`, `EditCell`, `EditByte`, `SyncStep`) in `crates/sessions` (`schema_version: 2`, auto-migrating v1) with pure `snapshot()` projections.
- **Evidence Gate**: Round-trip Rust test proves v1 -> v2 migration and full state reconstruction from event replay; Undo/Redo and crash recovery share one projection engine.

### Phase 6: Sandboxed External Plugin SDK & Community Marketplace
- **Scope**: Lock `autocompare-plugin.json` manifest schema, implement a WASI / read-only stdin-stdout subprocess runner with strict wall-clock timeout and memory caps, and add `V2PluginManagerModal` for enable/disable/inspection.
- **Security Model**: External format converters and comparators never receive ambient filesystem write permissions; inputs are streamed over stdin or pre-opened read-only descriptors, and outputs are validated against bounded byte limits before rendering.
- **Evidence Gate**: Out-of-tree sample plugin installs into `<app_data>/plugins/`, passes sandbox write-denial tests, and mounts cleanly into Home + Tab slots.
