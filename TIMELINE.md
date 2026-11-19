# TIMELINE

## UPDATED ON : 2026-11-20

### Task (2026-11-20) — UI observation sync & study merge

1. **Scope**: Merged live UI observations from `checks/ui-feedback/brief.md` into `docs/study.md` and `checks/ui-feedback/observations.md`. Captured menu facts (Session/View/Edit/Search/Tools labels as short functional facts), layout options, and GUI gaps P0-P3. No prose, strings, or assets copied.
2. **Tests** (before → after): Manual verification pass (10/10) to ensure original phrasing and no accidental verbatim strings from the observed app.
3. **Files changed**: `docs/study.md`, `TIMELINE.md`, `checks/ui-feedback/observations.md`.

## UPDATED ON : 2026-11-20

### Feature (2026-11-20) — Wave-3: CLI reports+VCS flags, GUI Home+gutter+screenshot

1. **Scope**: `leaf-3.1` extended `autocompare-cli` with `--report <fmt:path>` (html/text/xml/patch via `autocompare-reports`), bcomp VCS compat flags (`-ro`, `-automerge`, `-mergeoutput`, `-vcs1..4`), strict exit codes 0/1/2; `leaf-3.2` extended `autocompare-gui` with Home View, 9 tabs, pure `gutter_geometry`/`apply_copy`/`resolve_take`/`folder_tree`/`theme_contrast_ok` headless fns, rfd native dialogs, drag-drop, Dark/Light themes, and `--screenshot` PNG writer. Both leaves built in parallel under disjoint OWNS with Microsoft Rust guidelines enforced (Result errors, Debug+Display, no meta-design docs, hand-computed test truths).
2. **Tests** (before → after): CLI 0 → 20 integration tests (exit codes + reports on all 9 sessions + bcomp + session + error paths) with harness `RESULT: PASS` 10/10; GUI 0 → 57 headless view-model tests + valid 9KB screenshot PNG; workspace 43 suites ok, `clippy -D warnings` clean, `xtask layers/secrets` ok. All 4 leaf gates reverified `ALL MET` from exact ledgers.
3. **Files changed**: `crates/cli/src/{args,bcomp,error,lib,outcome,report,report_adapter,report_adapter_media,runner,session_cmd,render_text_unified}.rs`, `crates/cli/tests/{subcommands_text,subcommands_media,bcomp,session,errors}_test.rs`, `crates/gui/src/{copy,gutter,home,screenshot,take,theme,tree}.rs`, `crates/gui/{Cargo.toml→+rfd}`, `Cargo.lock`.

## UPDATED ON : 2026-11-20

### Task (2026-11-20) — Wave-4: full workspace check + release ready

1. **Scope**: Ran Wave-4 root gates on the Wave-3 tree: `cargo fmt --check`, workspace `clippy -D warnings`, full `cargo test --workspace`, `xtask layers/secrets`, exhaustive release-mode diff sweeps (`--ignored`), and `cargo build --release --workspace`.
2. **Tests** (before → after): workspace 43/43 suites ok (0 failed); release sweeps 5/5 ignored tests passed; release binaries `autocompare` (1.6MB) + `autocompare-gui` (10.8MB) built in 1m42s. No source changes needed; verification only.
3. **Files changed**: `TIMELINE.md` (this entry); evidence under `.unlazy/v1/evidence/wave-4/` (uncommitted).


## UPDATED ON : 2026-11-19

### Feature (2026-11-19) — Wave-2 Leaf-2.3: FolderCompare, SyncPlanner, FolderMerge & JobRunner

1. **Scope**: Extended `crates/compare` (`autocompare-compare`) with `autocompare-vfs` integration and serde support. Implemented `FolderCompare` tree over `LocalFs` with cancellable recursive walk and byte comparison via `AtomicBool`, `SyncPlanner` supporting `UpdateLeft`, `UpdateRight`, `Both`, `Mirror` with dry-run and apply operations, `FolderMerge` 3-way merge wrapper, and a new cancellable background `JobRunner` with thread management and progress callbacks.
2. **Tests** (before → after): 88 → 94 compare crate tests (+6 new tests covering FolderCompare over LocalFs, cancellation flag, all 4 Sync modes dry-run/apply, FolderMerge struct, and JobRunner spawn/progress/cancel/background compare/sync), strict workspace lints clean.
3. **Files changed**: `crates/compare/Cargo.toml`, `crates/compare/src/{lib,binary,folder,sync,folder_merge,jobs}.rs`, `crates/compare/tests/{folder,sync,folder_merge,jobs}.rs`.

## UPDATED ON : 2026-11-19

### Feature (2026-11-19) — v0.5.0: desktop GUI covers all nine session types

1. **Scope**: `autocompare-gui` grows from 3 tabs to 9 (Text, Folder, Hex, Merge, Table, Sync, Picture, FolderMerge, Audio), matching the nine CLI commands and the original's nine observed session types. Merge/FolderMerge take base/left/right paths; Sync/FolderMerge show the dry-run plan (apply stays CLI). No engine changes: thin GUI over `autocompare-compare` / `autocompare-diff`, pure headless view-model builders, safe Rust, never panics.
2. **Tests** (before → after): 251 → 264 workspace tests (+13 headless GUI view-model: hex, merge conflict, table cells, sync plan, folder-merge plan, audio), `clippy -D warnings` clean; live CLI sanity verified text/folder/table exit codes and rendering.
3. **Files changed**: `crates/gui/src/{app,viewmodel,views}.rs`, `Cargo.toml` (0.4.0 → 0.5.0), README, docs/study.md, docs/plan-v0.5.0.md.

## UPDATED ON : 2026-11-18

### Feature (2026-11-18) — v0.4.0: audio compare and the desktop GUI client

1. **Scope**: `autocompare audio` (WAV via a hand-written RIFF parser, MP3 via symphonia, 10ms-window PCM comparison) and `autocompare-gui` (eframe/egui thin client with Text/Folder/Picture tabs over the same engine). All 9 session types observed in the original's menu now have coverage.
2. **Tests** (before → after): 167 → 246 workspace tests (+49 audio incl. decode totality sweeps and a real ffmpeg-encoded MP3, +30 headless GUI view-model), `clippy -D warnings` clean; live runs verified audio against hand-computed ground truth (0.500s single-sample flip, 1/100 windows) and GUI startup liveness.
3. **Files changed**: `crates/diff/src/audio.rs`, `crates/compare/src/media.rs`, `crates/gui/src/*`, README, docs/study.md, CI (Linux GUI deps).

## UPDATED ON : 2026-11-18

### Feature (2026-11-18) — v0.3.0: picture compare and three-way folder merge

1. **Scope**: `autocompare picture` (PNG pixel diff, bounds, heatmap; first third-party dep: pure-Rust `png` crate) and `autocompare folder-merge` (plan/apply three-way tree merge over the existing content-merge engine); 7 of the original's 9 observed session types now rebuilt.
2. **Tests** (before → after): 126 → 167 workspace tests, `clippy -D warnings` clean; live runs verified picture stats against hand-computed ground truth (14/64 symmetric-difference pixels, bbox) and merge output file by file (clean merges, markers, deletions, empty dirs, output-precondition refusal).
3. **Files changed**: `crates/diff/src/image.rs`, `crates/compare/src/{picture,folder_merge}.rs`, `crates/cli/src/{render_picture,render_foldermerge}.rs`, README, docs/study.md.

## UPDATED ON : 2026-11-18

### Feature (2026-11-18) — v0.2.0: table compare and folder sync

1. **Scope**: `autocompare table` (RFC-4180-lite delimited parsing, cell-level `[old => new]` deltas) and `autocompare sync` (plan/apply folder sync, dry-run default, prune, symlink-safe, per-step failure tolerance); both features taken from the original's session menu (verified via accessibility API).
2. **Tests** (before → after): 72 → 126 workspace tests (+39 table, +15 sync), `clippy -D warnings` clean; live runs verified table deltas/alignment and sync dry-run, apply (copies + prune delete on disk) and re-plan convergence.
3. **Files changed**: `crates/diff/src/table.rs`, `crates/compare/src/{table,sync}.rs`, `crates/cli/src/{render_table,render_sync}.rs`, README, docs/study.md.

## UPDATED ON : 2026-11-18

### Task (2026-11-18) — v0.1.0 released; original's session types verified

1. **Release**: tagged v0.1.0 (text, folder, hex, merge comparison) and published the GitHub release; CI green on all pushes.
2. **Verification**: enumerated the running original's Session → New Session menu via the macOS accessibility API (Folder Compare, Folder Merge, Folder Sync, Text Compare, Text Merge, Hex Compare, Media Compare, Picture Compare, Table Compare) and corrected docs/study.md; the visual screenshot pass is blocked by the screen-recording permission (remediation recorded).
3. **Files changed**: docs/study.md, TIMELINE.md.

## UPDATED ON : 2026-11-18

### Feature (2026-11-18) — Clean-room comparison tool rebuilt from a study of the original

1. **Scope**: text (side-by-side + unified), recursive folder, hex and three-way merge comparison as a pure-Rust workspace (`crates/diff` → `crates/compare` → `crates/cli`), built clean-room from a user-facing study of the original app (`docs/study.md`); no code, assets or wording copied.
2. **Tests** (before → after): none → 72/72 workspace tests, `clippy -D warnings` clean; 5 exhaustive release-mode sweeps (~430k pairs) verify diff replay, canonical form and minimality; live CLI sanity verified all six folder statuses, `--content` reclassification, exit codes 0/1/2 and conflict markers.
3. **Files changed**: `crates/*`, `docs/study.md`, `README.md`, `.github/workflows/ci.yml`, `LICENSE`.

## UPDATED ON : 2026-11-19

### Feature (2026-11-19) — Wave 2 Leaf-2.2: autocompare-reports crate delivered

1. **Reports Engine**: Created `crates/reports` with decoupled std-only inputs (`ReportInput`, `Hunk`, `DiffStats`, `ChildReportAnchor`). Built HTML color/mono renderers with linked child report anchors, plain text, XML, and unified/context patch renderers with hostile input character escaping.
2. **Tests** (before → after): 0 → 9 snapshot tests passing (all green); strict workspace lints passed (`forbid(unsafe_code)`, `unwrap_used=deny`, `indexing_slicing=deny`).
3. **Files changed**: `crates/reports/Cargo.toml`, `crates/reports/src/{lib,html,text,xml,patch}.rs`, `crates/reports/tests/snapshot_tests.rs`.

## UPDATED ON : 2026-11-19

### Feature (2026-11-19) — Wave 2 Leaf-2.1: autocompare-sessions crate delivered

1. **Sessions & Config Engine**: Created `crates/sessions` with `SessionDocument`, `SessionTab`, `WorkspaceDocument`, `AppSettings`, and schema migration logic. Supports v0->v1 migrations (legacy fields, unversioned, bare arrays), forward-compatibility rejection, hostile inputs handling without panics, and file/string JSON serialization.
2. **Tests** (before → after): 0 → 29 tests passing (11 unit tests, 7 corrupt-input tests, 8 migration tests, 3 round-trip tests, 0 failed); strict workspace lints passed (`forbid(unsafe_code)`, `unwrap_used=deny`, `expect_used=deny`, `panic=deny`, `indexing_slicing=deny`).
3. **Files changed**: `crates/sessions/Cargo.toml`, `crates/sessions/src/{lib,session,workspace,settings,error}.rs`, `crates/sessions/tests/{round_trip,migration,corrupt_input}.rs`.


## UPDATED ON : 2026-11-20

### Feature (2026-11-20) — GUI-SHELL P0 shell polish

1. **Shell**: session toolbar (Swap Sides + auto-recompare, Reload/Recompare), window/tab session identity (`left <--> right - Text`), view toggles (Word Wrap / Line Numbers / Syntax Highlighting) persisted in `App` state and applied to text panes via `view_text_for_paint()` clone-hook, copy-back + selection/filter propagation so sibling TEXT agent's nav/filter keep working, honest per-tab counts + action status bar, empty-result guidance on every tab.
2. **Tests** (before → after): gui crate 90 → 92 passed, 0 failed; `cargo clippy -p autocompare-gui --all-targets -- -D warnings` exit 0; `rustfmt --check` clean on owned files.
3. **Files changed**: `crates/gui/src/app.rs`, `crates/gui/src/main.rs`, `crates/gui/src/screenshot.rs`, `checks/gui-shell/{before,after}.png`, `checks/gui-shell/REPORT.md`.
