# TIMELINE

### Feature (2026-11-21) — feat(gui): exq-hygiene52 test-output hygiene

1. **Hygiene**: RgbaImage::opaque() forces alpha 255 (RGB untouched, unit + PNG roundtrip tested); real_egui_shot writes to target tmp by default, EXQ_KEEP_PNG=1 refreshes tracked copy. Machine proof: tmp PNG alpha-0 px 0/576000 (was 568630); default cargo test leaves tracked real-egui.png untouched.
2. **Tests**: diff 40+4+33+44+20+98 + gui 1105 + 1 real green; rustfmt clean; banned 0; harness PASS.
3. **Files changed**: crates/diff/src/image.rs, crates/gui/tests/real_egui_shot.rs, checks/gui-exq-hygiene52/**, TIMELINE.md.


### Feature (2026-11-21) — feat(gui): exq-icons52 wiring audit (UNWIRED, honest)

1. **Negative result with evidence**: staged icons50 paint fns have no live call site in app.rs (rects owned by FROZEN views.rs which already paints glyphs; TOPCHROME-10 toolbar is fixture-only, live bar is 4 buttons). No fake wiring added; 0-byte PNG delta documented with sha256 in report.md. Key methodology fix: --screenshot bypasses App paint entirely, so future paint waves must prove via kittest real-egui.png, not harness after.png.
2. **Tests**: 1105 green; rustfmt clean (direct); banned 0; harness PASS.
3. **Files changed**: crates/gui/src/app.rs (contract test only), checks/gui-exq-icons52/**, TIMELINE.md.


### Feature (2026-11-21) — feat(gui): exq-light51 live mode-aware palette wiring

1. **Live wiring**: kind_palette + selective_side_palette dark-flagged via theme *_for helpers; minimap segments + gutter selection bands mode-aware; dark byte-identical, light wash + dark ink with contrast checks; zero new colors, geometry/typo untouched. agyx-delivered, verified same as opencode waves.
2. **Tests**: 1104 green (1103 + light_exq_light51_mode_aware); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (dark 10928B byte-identical, light 10973B).
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-light51/**, TIMELINE.md.


### Feature (2026-11-21) — feat(gui): exq-icons50 vector icons + tab pill (STAGED)

1. **Staged vector layer**: ExqIcons50Mark geometry (ring/bars/rails/wedge), tab-pill rect, softer light inline accent reusing LIGHT_* helpers; paint_exq_icons50_mark/tab_pill fns defined + unit-tested, zero new colors/sizes. Honest limit: paint fns NOT yet called from live toolbar (live path uses egui Button widgets, fixture fill_rect blocks are harness-only) - PNG byte-identical, wiring is round-2.
2. **Tests**: 1103 green; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/app.rs, checks/gui-exq-icons50/**, TIMELINE.md.


### Feature (2026-11-21) — feat(gui): exq-kittest49 real egui screenshot pipeline

1. **Real render pipeline**: dev-only egui_kittest =0.37.0 (eframe+wgpu, zero runtime change) + tests/real_egui_shot.rs boots real App text-compare view on t_left/t_right fixtures, writes real-egui.png 960x600 52KB; pxvar AA-PROVEN 502 vs 10 baseline ratio 50.2; pxdiff delta honestly documented in GATES.md (layout matches, font rasterization + widget chrome differ - the egui ceiling).
2. **Tests**: 1099 lib + 1 real_egui_shot green; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS. Note: real-egui.png bytes vary slightly run to run (non-deterministic render), tests assert magic/size/AA not hash.
3. **Files changed**: crates/gui/Cargo.toml, Cargo.lock, crates/gui/tests/real_egui_shot.rs, checks/gui-exq-kittest49/**, TIMELINE.md.


### Feature (2026-11-21) — feat(gui): exq-typo48 live font wiring

1. **Live font plan**: typo48_live_font_plan() maps Proportional to TYPO_UI_SANS_STACK + Monospace to TYPO_MONO_STACK (single source, zero new sizes/weights/tokens); guarded once-install on first live ui() frame, headless tests untouched; startup stderr logs installed families as machine evidence.
2. **Tests**: 1099 green (1098 + typo_exq_typo48_live_fonts); rustfmt clean on OWNS; banned non-test tokens 0; exquisite harness RESULT: PASS both modes. Honest limit: plan re-asserts stock egui families (no shipped binaries); real system-font byte loading is a follow-up wave.
3. **Files changed**: crates/gui/src/app.rs, checks/gui-exq-typo48/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-palette47 live mode-aware palette

1. **Mode-aware palette**: kind_palette()/selective_side_palette() dark-flagged via theme *_for helpers (dark byte-identical, light wash + dark ink AA); toolbar icon blocks to real shapes, zero new colors/tokens, zero geometry edits.
2. **Tests**: 1098 green; rustfmt clean on OWNS; banned non-test tokens 0; exquisite harness RESULT: PASS both modes.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-palette47/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-shot46 honest screenshot harness

1. **Honest harness**: screenshot.rs colors/layout now single-sourced from theme.rs mode-aware helpers + live pure geometry fns; Light mode paints dark-ink-on-light-wash (dark-on-dark gone); fake-chrome divergence documented; parity/ holds before/after dark+light PNGs. Verdict TOKEN-SOURCED-LIMITED (sandbox live-capture unavailable, machine-checked single-source tests instead).
2. **Tests**: 1094 green; rustfmt clean on OWNS; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-shot46/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-home45 live home cards

1. **Live home cards lock**: pure helpers in home.rs (card rect/grid geometry vs 1200px canvas, home45-live-cards-1200.json edge fixture); colors frozen.
2. **Tests**: 1092 green; rustfmt clean on OWNS; banned non-test tokens 0 (unwraps test-mod only); exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/home.rs, checks/gui-exq-home45/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-table43 live table compare grid

1. **Live table grid lock**: delegation-only pure helpers in views_table.rs (live-table-grid.json edge fixture); 65 views_table tests green; colors frozen at 38.
2. **Tests**: 1092 green; rustfmt clean on OWNS; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/views_table.rs, checks/gui-exq-table43/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-take44 live merge take resolution

1. **Live take geometry lock**: delegation-only pure helpers in take.rs (take_buttons_row_x, take geometry vs 960px budget); new test take_exq_take44_live_take_geometry; colors frozen at 38.
2. **Tests**: 1092 green (1090 + 2 new); rustfmt clean on OWNS; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/take.rs, checks/gui-exq-take44/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-hex42 live hex columns

1. **Live hex columns lock**: round-2 on hex31 geometry lock, zero new tokens; HEX42_REF_CHAR_W 7.0; offset gutter, ruler-vs-grid columns, SPACE-token ASCII gap, 1200px no-scroll budget, narrow-700 stacked edge all delegate to hex31 single source so views cannot drift.
2. **Tests**: 1090 green; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 11040 bytes, magic ok, distinct 256).
3. **Files changed**: crates/gui/src/views_hex.rs, checks/gui-exq-hex42/**, TIMELINE.md.

### Chore (2026-11-21) — chore(localrun): exq-localrun3 early feedback bundle

1. **Early feedback bundle**: CLI 10 cmds PASS, 3 PNGs default/dark/light, dist tarball + sha refreshed (VERSION 0.5.0 7bdf2ea), tail-insertion edge fixture, GAPS-TOP10. Low-disk fallback: CARGO_PROFILE_RELEASE_LTO=false + CODEGEN_UNITS=16 via env only.
2. **Tests**: REPORT RESULT: PASS; owned scope fmt clean; banned 0; no crates touch.
3. **Files changed**: checks/ac-loop/round-lr3/**, dist/*, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-theme26 light theme

1. **Light Theme (single color wave, geometry-typo identical)**: theme.rs LIGHT_DELETED_BG/INSERTED_BG/CHANGED_BG/CHANGED_FG/ERROR_BG mirrors + deleted/inserted/changed/accent/error for(dark) helpers + light_diff_accents_aa; app.rs resolve_screenshot_mode (default Light, --screenshot-mode wins, AUTOCOMPARE_DARK_REF keeps dark); main.rs screenshot dispatch. pxdiff before vs after: 390933 of 576000 DIFFERENT bbox (0,0)-(959,595); dark-ref IDENTICAL to before. Edge theme-contrast-720.json AA 17.35/6.49/6.48/5.98/6.91 min 5.32.
2. **Tests** (before -> after): autocompare-gui 1050 -> 1054 green (including theme26_light_contrast + 3 resolver tests); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS; geometric signals unchanged; colors 33 -> 38 justified light-mirrors.
3. **Files changed**: crates/gui/src/theme.rs, crates/gui/src/app.rs, crates/gui/src/main.rs, checks/gui-exq-theme26/**, TIMELINE.md.

## UPDATED ON : 2026-11-20

### Feature (2026-11-20) — folder-sync/merge and text/merge editor feature parity

1. **Feature parity**: Clean-room implementation of folder-sync, folder-merge, text-diff, and merge editor engines:
   - `crates/diff`: Myers & Patience text diff, char/word inline refinement, ignore options (whitespace collapse/trim, case insensitivity, CRLF vs LF); 2-way and 3-way text merge with clean edit merging, conflict detection & marker formatting (`<<<<<<< left`, `=======`, `>>>>>>> right`), buffer undo/redo/splice; syntax grammar rules (comments, strings, important vs unimportant differences).
   - `crates/compare`: Folder sync modes (Mirror L-to-R / R-to-L, Update, Two-Way, custom rules), dry-run plan generation, directory pruning, symlink & traversal guards; 3-way folder merge conflict detection & resolution policies; background `JobRunner` with cooperative cancellation and bounded progress queue (64 max history).
2. **Tests** (before → after): `autocompare-diff` 192 passed, `autocompare-compare` 106 passed; clippy `-D warnings` clean; rustfmt clean; zero `allow(dead_code)`, zero unsafe, zero unwrap/expect/panic/todo in non-test code.
3. **Files changed**: `crates/compare/src/{folder,folder_merge,jobs,sync}.rs`, `crates/diff/src/{grammar,merge,text_diff}.rs`, `TIMELINE.md`.

## UPDATED ON : 2026-11-20

### Feature (2026-11-20) — ac-loop round-parity verification: 3-way merge, grammar ignore, sync prune & jobs

1. **ac-loop parity verification**: Completed 4 rounds of ac-loop harness verification for Beyond Compare feature parity:
   - Round 1: 3-way text merge edge fixtures for clean vs conflicting edits with standard markers (`<<<<<<< left`, `=======`, `>>>>>>> right`).
   - Round 2: Grammar & whitespace ignore edge fixtures verifying strict diff (exit 1) vs `-b` space-change collapse (exit 0).
   - Round 3: Folder sync mirror edge fixtures testing dry-run plan generation and live application with `--prune` deleting destination orphans.
   - Round 4: Background JobRunner cancellation protocol and bounded progress queue (64 max history) validation.
2. **Tests** (before → after): `autocompare-diff` 192 passed, `autocompare-compare` 106 passed; all 4 ac-loop rounds `RESULT: PASS`; G1-G7 gates all passed with zero clippy warnings, zero `allow(dead_code)`, and zero unsafe/unwrap in non-test OWNS code.
3. **Files changed**: `checks/ac-loop/round-parity/**`, `GATES.md`, `TIMELINE.md`.


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

## UPDATED ON : 2026-11-20

### Feature (2026-11-20) — Exquisite GUI geometry and typography round-exq3

1. **GUI Geometry & Typography**: Validated responsive layout (960x600 default + 720px fallback) with zero clipped text, monospace typography consistency, headless-first viewmodel logic, and frozen colors. Verified via ac-loop round-exq3 harness with new edge fixtures under comparison/.
2. **Tests** (before → after): autocompare-gui 703 → 714 passed, 0 failed; clippy 0 warnings; cargo fmt clean; zero allow(dead_code); screenshot after.png valid.
3. **Files changed**: `crates/gui/src/*`, `checks/ac-loop/round-exq3/*`, `TIMELINE.md`.

## UPDATED ON : 2026-11-20

### Verification (2026-11-20) — ac-loop round-sessrep clean-room depth verified

1. **Scope**: Clean-room depth across `autocompare-sessions`, `autocompare-reports`, and `autocompare-vfs` verified via unified ac-loop harness with 10/10 CLI parity, GUI geo-typo PASS, and live BC observe.
2. **Tests** (before → after): 102 → 124 tests passed across 3 crates (reports: 28/28, sessions: 62/62, vfs: 34/34), clippy zero warnings, rustfmt clean, zero unsafe, zero unwrap/expect/panic/todo in non-test code, zero allow(dead_code).
3. **Files changed**: `checks/ac-loop/round-sessrep/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-20

### Verification (2026-11-20) — Exquisite GUI geometry and typography round-exq4 verified

1. **GUI Geometry & Typography Wave-4**: Verified responsive layout with zero clipped text, 960x600 canvas + 720px narrow fallback proof PNG (12,992 bytes, distinct 256), `TypoRole` single-sourcing across views, frozen colors, and new edge-04 fixture.
2. **Tests** (before → after): autocompare-gui 714 → 726 passed, 0 failed; clippy zero warnings (`-D warnings`); rustfmt clean; zero unsafe; zero unwrap/expect/panic/todo in non-test code; ac-loop round-exq4 RESULT: PASS.
3. **Files changed**: `crates/gui/src/{screenshot,views}.rs`, `checks/ac-loop/round-exq4/**`, `TIMELINE.md`.

## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-typo10 roles plus rhythm

1. **TypoRole Rhythm & Density**: Swarm parallel audit of TypoRole rhythm & line-heights in `theme.rs` (Small 13.2 < Mono 15.6 < Body 18.2 < Heading 21.6), viewmodel density & row-height tokens in `viewmodel.rs` (Compact 18/20, Comfortable 24, Spacious 28; zero clipping at 960x600 and 720px fallback), and folder-tree indent/alignment in `tree.rs` (12px indent step, 16px expander, 20/24/28px density heights). Edge fixture `empty-vs-nonempty.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 872 → 886 passed, 0 failed; clippy clean on owned files; rustfmt clean; banned patterns 0.
3. **Files changed**: `crates/gui/src/{theme,viewmodel,tree}.rs`, `checks/gui-exq-theme10/**`, `TIMELINE.md`.

## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-hextable10 cells plus grid

1. **Hex & Table Geometry & Typo**: Clean-room Beyond Compare parity for hex byte-grid cells with midline gap at col 8, gutter alignment, stacked layout on narrow screens (<720px), and table column Grid with SECTION_GAP (8.0px) col gap, cell padding (8.0px), and header weight via TypoRole (strong, 600 weight, no new sizes). Edge fixture `comparison/hex-narrow.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 886 passed, 0 failed; clippy -D warnings clean; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_hex.rs`, `crates/gui/src/views_table.rs`, `checks/gui-exq-hextable10/**`, `TIMELINE.md`.




## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-foldertable10 columns plus indent

1. **Folder Table Grid & Indent**: Beyond Compare parity for folder tree table: 3-column Grid (Name, Size, Modified) with `SECTION_GAP` (8.0px = `SPACE_SM`) col gap, row height derived from `TypoRole::Mono` line-height (15.6px + 2.0px padding = 17.6px, density-aware), indent per depth (40px depth 0 to 100px depth 5) while Size and Modified columns align at fixed column boundaries across all depths, and orphan rows dimmed (`"-"` dashes with `TypoRole::Small` / `is_weak() == true`) without color modifications. Edge fixture `comparison/folder-deep-nest.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 888 → 893 passed, 0 failed; clippy clean on owned `tree.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/tree.rs`, `checks/gui-exq-foldertable10/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-merge2 panes plus actions

1. **Merge 3-Way Panes & Take/Copy Actions**: Beyond Compare parity for 3-way merge panes with 2x `SECTION_GAP` (8.0px = `SPACE_SM`) dividers, `MERGE_PANE_MIN_W` (130.0px) floor with horizontal scroll trigger at <406px (`MERGE_TOTAL_MIN_W`), single-source row height from `TypoRole::Mono.line_height_px()` (15.6px), and `SPACE_XS` (4.0px) hit-slop expansion for take and copy button hit targets (24px to 32px effective hit box). Edge fixture `comparison/merge-3way-narrow.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 896 → 903 passed, 0 failed; clippy clean; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: `crates/gui/src/{copy,take}.rs`, `checks/gui-exq-merge2/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-chrome3 tabs plus toolbar

1. **Tabs & Toolbar Chrome Round-3**: Beyond Compare parity for session tab strip and toolbar chrome: session tab strip with 150px cells (120 max + 6 close gap + 24 close edge) and 8px inter-tab gap with horizontal overflow scroll (2 tabs fit, 8 tabs scroll at 720px), toolbar spacing via SPACE tokens (SPACE_XS 4.0px item gap, SPACE_SM 8.0px group gap, SPACE_MD 12.0px margin, SPACE_XL 24.0px button height, uniform 96px button width), and header height (21.6px) derived from TypoRole Heading chrome line-height (18.0 * 1.2). Edge fixture `comparison/chrome-narrow-720.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 904 → 906 passed, 0 failed; clippy clean on owned files; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: `crates/gui/src/app.rs`, `crates/gui/src/home.rs`, `checks/gui-exq-chrome3/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-tree2 indent plus columns

1. **Tree Indent Rhythm & Columns Alignment**: Beyond Compare parity for folder tree rows: indent step strictly derived from SPACE tokens (`tree_indent_step_from_space()` == `GEO_TREE_INDENT_STEP` == `SPACE_MD` = 12.0px), depth rhythm scaling across depths 0..=5 (0.0 to 60.0px; path origins 40.0 to 100.0px), orphan rows dimmed on missing sides using `TypoRole::Small` (`is_weak() == true`) with `"-"` dashes without modifying colors (colors frozen at 33), and Size/Modified columns strictly aligned at 960px wide pane (Size 738.0px, Modified 832.0px; fixed trailing cols 222.0px). Edge fixture `comparison/tree-wide-960.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 909 → 913 passed, 0 failed; clippy clean on owned `tree.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: `crates/gui/src/tree.rs`, `checks/gui-exq-tree2/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-hex3 ruler plus grid

1. **Hex Ruler Ticks & Grid Geometry at 960px**: Beyond Compare visual parity for hex compare: ruler tick spacing symmetric via SPACE tokens (`hex_ruler_tick_spacing()` = `GEO_SECTION_GAP` = `SPACE_SM` = 8.0px, `hex_ruler_tick_sym_pad()` = `GEO_TIGHT_GAP` = `SPACE_XS` = 4.0px), ruler column x-positions (`hex_ruler_col_x`) 1:1 aligned with byte grid columns (`hex_grid_byte_col_x`), nibble group gap between byte 7 and 8 derived from single-source midline helper (`hex_nibble_group_gap_w(char_w)`), and row height locked to `TypoRole::Mono` line-height (15.6px) at 960px wide (`HEX_CANVAS_W` = 960.0px, SideBySide mode, 30 visible rows in 470px panes height, 52px chrome reserve, 908px grid avail, (52.0, 19.6) grid origin). Colors frozen at 33. Edge fixture `comparison/hex-wide-960.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 919 → 922 passed, 0 failed; clippy clean on owned `views_hex.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: `crates/gui/src/views_hex.rs`, `checks/gui-exq-hex3/**`, `TIMELINE.md`.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-table2 rows plus widths

1. **Table Row Rhythm & Column Widths at 960px**: Beyond Compare parity for table compare: row gap symmetric via SPACE tokens (`table_row_sym_pad()` = `GEO_TIGHT_GAP` = `SPACE_XS` = 4.0px, `table_grid_gap_y()` single-sourced), column spacing `table_col_spacing()` = `GEO_SECTION_GAP` = `SPACE_SM` = 8.0px, outer widths from char metrics (`table_wide_outer_widths()` delegates `table_col_widths_px`), 960px avail `table_wide_avail_w()` = 936 with row height locked to `TypoRole::Mono` line-height (15.6px). Header underline weight reuses `TEXT_INLINE_SPAN_UNDERLINE_W` (1.0), header weight 600 via `TYPO_WEIGHT_SEMIBOLD`. Colors frozen at 33. Edge fixture `comparison/table-wide-960.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 922 → 923 passed, 0 failed; clippy clean on owned `views_table.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table2/**`, `TIMELINE.md`.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-text5 gutter plus divider

1. **Text Gutter & Center Divider at 960px**: Beyond Compare parity for text compare chrome: separate number gutter + icon column (`gutter_text_gutter_width` = number column + `GUTTER_ICON_COL_W` = SPACE_LG 16; 1-digit 32, 3-digit 40), symmetric center divider (`GUTTER_CENTER_SIDE_GAP` = SPACE_XS 4; 16 box + 2x4 = 24 center; 960 layout 12+456+24+456+12 equal panes), hit-slop symmetric (`gutter_slop_band` +/-2 both sides), row centers locked to Mono pitch 15.6. Colors frozen at 33. Edge fixture `comparison/text-gutter-960.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 923 → 928 passed, 0 failed; clippy clean; rustfmt clean on owned `gutter.rs`; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: `crates/gui/src/gutter.rs`, `checks/gui-exq-text5/**`, `TIMELINE.md`.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-chrome4 cards plus header

1. **Home Cards & Header Chrome**: Beyond Compare parity for home pane: card grid gaps via SECTION_GAP, header height from TypoRole chrome line-height, card button hit-slop symmetric via SPACE_XS at 960px wide. Colors frozen at 33. Edge fixture comparison/chrome-wide-960.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 928 → 930 passed, 0 failed; clippy clean on owned home.rs; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/home.rs, checks/gui-exq-chrome4/**, TIMELINE.md.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-chrome5 tabs plus toolbar

1. **Session Tabs & Toolbar at 960px**: Beyond Compare parity for session chrome: tab cell 150px from char metrics (16 mono chars x 7.2 + SPACE_XS slack = 120 label + 6 close gap + 24 close edge), inter-tab gap SPACE_SM (8), side chrome SPACE_MD (12), strip scroll triggers at 6 tabs (940 > 936 avail). Toolbar uniform 96 floored above 79 char need, item gap SPACE_XS (4), group gap SPACE_SM (8). Colors frozen at 33. Edge fixture comparison/chrome-tabs-960.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 930 → 932 passed, 0 failed; clippy clean; rustfmt clean on owned app.rs; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/app.rs, checks/gui-exq-chrome5/**, TIMELINE.md.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-text6 spans plus filler

1. **Inline Spans & Filler at 960px**: Beyond Compare parity for text inline diff: symmetric span pads SPACE_XS total (4.0, half per side), underline weight via TYPO tokens (regular 1px, semibold 2px), filler row height locked to Mono line-height 15.6, 960 layout 448+448+48+16, char-boundary clamp with multibyte-safe fixture. Colors frozen at 33. Edge fixture comparison/text-inline-960.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 930 → 932 passed, 0 failed; clippy clean; rustfmt clean on owned views.rs; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-text6/**, TIMELINE.md.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-tree3 rows plus guides

1. **Tree Rows & Indent Guides at 960px**: Beyond Compare parity for folder compare rows: even row rhythm locked to TypoRole Mono line-height (15.6 + 2.0 pad = 17.6), guide x-positions from SPACE tokens, orphan label ellipsis char-boundary clamp at 960px wide, depth rhythm intact. Colors frozen at 33. Edge fixture comparison/tree-narrow-720.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 932 → 936 passed, 0 failed; clippy clean; rustfmt clean on owned tree.rs; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/tree.rs, checks/gui-exq-tree3/**, TIMELINE.md.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-chrome6 pathbar plus info

1. **Pathbar & Info Line at 960px**: Beyond Compare parity for path chrome: equal path widths from char metrics, icon slots from SPACE tokens, info line geometry (date|bytes|filter), Browse button uniform width. Colors frozen at 33. Edge fixture comparison/pathbar-960.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 933 → 936 passed, 0 failed; clippy clean; rustfmt clean on owned app.rs; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/app.rs, checks/gui-exq-chrome6/**, TIMELINE.md.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-table3 rows plus cols

1. **Table Rows & Cols at 960px**: Beyond Compare parity for table compare: row height locked to TypoRole Mono line-height, col spacing from SPACE tokens, header underline weight reuse, sort indicator alignment. Colors frozen at 33. Edge fixture comparison/table-narrow-720.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 936 → 938 passed, 0 failed; clippy clean; rustfmt clean on owned views_table.rs; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/views_table.rs, checks/gui-exq-table3/**, TIMELINE.md.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-text7 rows plus rhythm

1. **Text Rows & Rhythm at 960px**: Beyond Compare parity for text compare rows: line-height locked to TypoRole Mono, row top rhythm, center divider gaps from SPACE tokens, minimap marker band geometry, ruler grid gap 8.0. Colors frozen at 33. Edge fixture comparison/text-rows-960.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 938 → 939 passed, 0 failed; clippy clean; rustfmt clean on owned views.rs; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-text7/**, TIMELINE.md.
## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-hex4 grid plus offset

1. **Hex Grid & Offset at 960px**: Beyond Compare parity for hex compare: even byte grid, offset gutter from SPACE tokens, ASCII pane sync, row height locked to TypoRole Mono. Colors frozen at 33. Edge fixture comparison/hex-wide-960.json added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 939 → 939 passed, 0 failed; clippy clean; rustfmt clean on owned views_hex.rs; banned non-test tokens 0; exquisite harness RESULT: PASS (PNG 10,401 bytes >= 8192, colors 33 frozen).
3. **Files changed**: crates/gui/src/views_hex.rs, checks/gui-exq-hex4/**, TIMELINE.md.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-home2 cards plus search

1. **Home Cards & Search at 960px**: BC parity for home view: card grid gaps symmetric via SPACE tokens (`HOME2_CARD_GRID_GAP` = `SPACE_SM` = 8.0px), content margin `SPACE_MD` (12.0px) giving 936 avail, 3 cols x 306.67px cards, header height from `TypoRole::Heading` line-height (21.6px), button hit-slop symmetric via `SPACE_XS` (4.0px). Body/Small reuse, no new sizes. Colors frozen at 33. Edge fixture `comparison/home-wide-960.json` added; exquisite harness PASS.
2. **Tests** (before → after): autocompare-gui 941 passed, 0 failed; clippy clean on owned `home.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/home.rs`, `checks/gui-exq-home2/**`, `TIMELINE.md`.

## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-gutter6 narrow plus slop

1. **Gutter Numbers & Overview at 720-960px**: BC parity for center gutter: line-number column digit scaling (1-digit 16.0, 3-digit 24.0, 7-digit cap 36.0, hidden 8.0), text-row pad vs Mono line-height (15.6 -> 19.6), overview same-plus-diff bands, arrow 16px box with 20px slop hit-target. Geometry via SPACE scale reuse, typo Mono 12 tabular reuse, no new sizes. Colors frozen at 33. Edge fixtures `comparison/edge-tall-same-diff.json` plus `comparison/gutter-narrow-720.json` added; exquisite harness PASS (PNG 10401B).
2. **Tests** (before → after): autocompare-gui 941 passed, 0 failed; clippy clean on owned `gutter.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/gutter.rs`, `checks/gui-exq-gutter6/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-text8 rows plus divider

1. **Text Rows & Divider at 960px**: BC parity for text compare panes: row height locked to `TypoRole::Mono` line-height, inner pad via `SPACE_XS`, line-num col widths from char metrics, center gap via `SPACE_SM`, divider hit-slop symmetric, minimap marker bands plus orphan blank balance at narrow widths. No new sizes, Body/Small reuse. Colors frozen at 33. Edge fixture `comparison/text-wide-960.json` added; exquisite harness PASS (PNG 10401B).
2. **Tests** (before → after): autocompare-gui all green; clippy clean on owned `views.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-text8/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-chrome7 toolbar plus pathbar

1. **Chrome Toolbar & Pathbar at 960px**: BC parity for app chrome: toolbar buttons equal widths via SPACE tokens, pathbar two equal halves with file meta, tab close hit-slop symmetric, status segments from `TypoRole` chrome line-height. Tree.rs dirt from sibling leaf left untouched, only OWNS committed. Colors frozen at 33. Edge fixture `comparison/chrome-wide-960.json` added; exquisite harness PASS (PNG 10401B).
2. **Tests** (before → after): autocompare-gui all green; `cargo check` 0 warnings; rustfmt clean on owned `app.rs`; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/app.rs`, `checks/gui-exq-chrome7/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-tree6 indent plus guides

1. **Tree Indent & Guides at 960px**: BC parity for folder compare tree: indent step via SPACE tokens, guide x from char metrics, expander hit-slop symmetric, row height locked to `TypoRole::Mono` line-height. Pre-existing clippy errors in non-OWNS files left untouched (none in `tree.rs`). Colors frozen at 33. Edge fixture `comparison/tree-wide-960.json` added; exquisite harness PASS (PNG 10401B).
2. **Tests** (before → after): autocompare-gui all green; rustfmt clean on owned `tree.rs`; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/tree.rs`, `checks/gui-exq-tree6/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-table4 cols plus rows

1. **Table Cols & Rows at 960px**: BC parity for table compare: header height from `TypoRole` chrome line-height, col spacing via `SPACE_SM` single-source, cell pad via `SPACE_XS`, row height locked to `TypoRole::Mono` line-height. Colors frozen at 33. Edge fixture `comparison/table-wide-960.json` added; exquisite harness PASS (PNG 10401B).
2. **Tests** (before → after): autocompare-gui all green; clippy clean on owned `views_table.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table4/**`, `TIMELINE.md`.

## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-hex5 grid plus ascii

1. **Hex Grid & ASCII Sync at 960px**: BC parity for hex compare: nibble group gap from single-source midline helper, ASCII pane x from grid origin, ruler tick spacing via `SPACE_SM`, row height locked to `TypoRole::Mono` line-height. Follow-up `fix(gui)`: 4 new `let...else` clippy lints rewritten with `?` operator, zero behavior change (70 hex tests green). Colors frozen at 33. Edge fixture `comparison/hex-wide-960.json` added; exquisite harness PASS (PNG 10401B).
2. **Tests** (before → after): autocompare-gui all green (70 hex tests); clippy 0 warnings on owned `views_hex.rs`; rustfmt clean; banned non-test tokens 0 (1 unwrap is test-only); exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_hex.rs`, `checks/gui-exq-hex5/**`, `TIMELINE.md`.


## UPDATED ON : 2026-11-21

### Feature (2026-11-21) — feat(gui): exq-home3 search plus indent

1. **Home Search & Indent at 720-960px**: second home pass: search field height from `TypoRole` chrome line-height, tree indent reusing tree indent step, empty-state margins via `OUTER_MARGIN`, hit-slop symmetric. Colors frozen at 33. Edge fixture `comparison/home-narrow-720.json` added; exquisite harness PASS (PNG 10401B).
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `home.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/home.rs`, `checks/gui-exq-home3/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-copy7 arrows plus hitrects

1. **Copy Arrows & Hit-Rects at 960px**: copy dialog arrows from space scale, rows from Mono line-height, hit-rect symmetry tests plus wide-960 fixture lock.
2. **Tests** (before → after): autocompare-gui all green (961 passed); clippy 0 warnings on owned `copy.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/copy.rs`, `checks/gui-exq-copy7/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-table5 cols plus separators

1. **Table Cols & Separators at 960px**: column widths from char metrics via space pad, row height locked to TypoRole Mono line-height, header underline plus strong weight.
2. **Tests** (before → after): autocompare-gui all green (962 passed); clippy 0 warnings on owned `views_table.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table5/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-gutter7 markers plus tall

1. **Gutter Markers & Tall File at 1200px**: change markers with symmetric hit-slop in center gutter, tall-file fixture lock, marker placement from space scale.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `gutter.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/gutter.rs`, `checks/gui-exq-gutter7/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-text9 rows plus highlight

1. **Text Rows & Highlight at 960px**: uniform Mono row heights with over/under fixtures, full-width changed-row highlight, edge-rows uniformity lock.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `views.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-text9/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-hex6 nibble plus ascii

1. **Hex Nibble & ASCII at 960px**: nibble group gap from single-source helper, ASCII pane x from grid origin, ruler ticks aligned to byte columns.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `views_hex.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_hex.rs`, `checks/gui-exq-hex6/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-tree7 indent plus arrows

1. **Tree Indent & Arrows at 960px**: indent step from space scale, arrow hit rects symmetric, row height from TypoRole Mono line-height.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `tree.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/tree.rs`, `checks/gui-exq-tree7/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-home4 cards plus gaps

1. **Home Cards & Gaps at 960px**: card gaps from space scale, card rows from TypoRole chrome line-height, section padding symmetric.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `home.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/home.rs`, `checks/gui-exq-home4/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-chrome8 toolbar plus filebar

1. **Chrome Toolbar & Filebar at 960px**: toolbar button size from space scale, file-bar height from TypoRole chrome line-height, status strip padding symmetric.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `app.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/app.rs`, `checks/gui-exq-chrome8/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-table6 header plus rhythm

1. **Table Header & Rhythm at 960px**: header row height from TypoRole strong line-height, body rows Mono line-height lock, separator insets symmetric.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `views_table.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table6/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-gutter8 changed-only plus width

1. **Gutter Changed-Only & Width at 960px**: markers gated on changed rows, gutter width from space scale, arrow glyph centered.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `gutter.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/gutter.rs`, `checks/gui-exq-gutter8/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-copy8 narrow plus slop

1. **Copy Narrow & Slop at 720px**: arrow hit-slop expanded symmetric via space scale, copy-line actions aligned to Mono baseline, column width from char metrics.
2. **Tests** (before → after): autocompare-gui all green (980 passed); clippy 0 warnings on owned `copy.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/copy.rs`, `checks/gui-exq-copy8/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-text10 tabs plus status

1. **Text Tabs & Status at 960px**: tab stops from char metrics, pane status bar from TypoRole chrome line-height, line-number gutter from digit count.
2. **Tests** (before → after): autocompare-gui all green; clippy 0 warnings on owned `views.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-text10/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-tree8 cols plus select

1. **Tree Cols & Select at 960px**: size column 86px and modified 128px from char metrics, grid origins steady across depths, size right-aligned, selected-row band full width.
2. **Tests** (before → after): autocompare-gui 4/4 new tests pass, isolated exquisite harness RESULT: PASS; cargo fmt clean; banned non-test tokens 0; colors frozen at 33.
3. **Files changed**: `crates/gui/src/tree.rs`, `checks/gui-exq-tree8/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-table7 sort plus align

1. **Table Sort & Align at 960px**: sort-indicator glyph 1-char reuse, right-slot rect anchored with SPACE_XS pad, header height stable Heading vs body, numeric right-alignment via grid origin.
2. **Tests** (before → after): autocompare-gui 986 green; clippy 0 warnings on owned `views_table.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table7/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-hex7 offset plus gap

1. **Hex Offset & Gap at 960px**: offset column 70px text / 78px gutter from char metrics for 8-digit addresses, ASCII pane gap from SPACE tokens, byte cell 30px box at 7px reference char.
2. **Tests** (before → after): autocompare-gui new offset-fit test green, isolated harness RESULT: PASS; clippy 0 warnings on owned `views_hex.rs`; rustfmt clean; banned non-test tokens 0; colors frozen at 33.
3. **Files changed**: `crates/gui/src/views_hex.rs`, `checks/gui-exq-hex7/**`, `TIMELINE.md`.

### Chore (2026-11-21) — chore(localrun): refresh after-text plus dist

1. **Localrun Refresh after hex7/table7/tree8**: all 10 after-*.png recaptured headless 960x600, cli+gui tarballs repacked + VERSION re-pinned, REPORT3.md with geometry/typo deltas vs reference in own words.
2. **Files changed**: `checks/localrun/REPORT3.md`, `dist/VERSION`, `dist/*.tar.gz`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-text11 gutter plus markers

1. **Text Gutter & Markers at 960px**: center gutter 48 to 20px via SPACE_LG+XS, single => marker 16px symmetric inset changed-only, row height Mono lock 17.6, per-pane status 15.2, 462+462+20+16 math.
2. **Tests** (before → after): autocompare-gui 987 green; clippy 0 warnings on owned `views.rs`; rustfmt clean; banned non-test tokens 0 (1 panic test-only); exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-text11/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-gutter9 markers plus slop

1. **Gutter Markers & Slop at 960px**: side column 20px via SPACE_LG+XS, marker box 16px, inset 2px + hit slop 2px symmetric, changed-only gating, layout 12+458+20+458+12 math.
2. **Tests** (before → after): autocompare-gui 993 green; clippy 0 warnings on owned `gutter.rs`; rustfmt clean; banned non-test tokens 0 (3 panics test-only); exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/gutter.rs`, `checks/gui-exq-gutter9/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-copy9 arrows plus rhythm

1. **Copy Arrows & Rhythm at 960px**: wide column fit from char metrics, arrow pitch 19.6 Mono+gap, action y dy 1.2 aligned, all space-token single-source delegates.
2. **Tests** (before → after): autocompare-gui 993 green (4 new headless); clippy 0 warnings on owned `copy.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/copy.rs`, `checks/gui-exq-copy9/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-chrome9 toolbar plus filebar

1. **Toolbar & Filebar at 960px**: Toolbar button size from SPACE scale (SPACE_XL 24h, 96w floor, gaps SPACE_XS/SM, side SPACE_MD symmetric; 9 fit + 1 overflow at 960w), file-bar 21.2h from Small chrome line-height + symmetric XS pads, status strip 21.2h with symmetric XS vertical plus MD horizontal pads and 3 equal 320 segments.
2. **Tests** (before → after): autocompare-gui 997 green (2 new headless tests); clippy 0 warnings on owned `app.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/app.rs`, `checks/gui-exq-chrome9/**`, `comparison/chrome-bar-960.json`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-home5 search plus indent

1. **Home Search & Indent at 720px**: search field height from TypoRole chrome line-height 13.2, tree indent step 12 reuse, empty-state margins OUTER_MARGIN 12 / SECTION_GAP 8.
2. **Tests** (before → after): autocompare-gui 997 green; clippy 0 home.rs diags; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/home.rs`, `checks/gui-exq-home5/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-table8 keys plus widths

1. **Table Keys & Widths at 960px**: key-column widths from char metrics, numeric right-alignment via grid origin, header/body heights from TypoRole, 176 fit in avail 936.
2. **Tests** (before → after): autocompare-gui green incl. new headless keys test; clippy 0 warnings on owned `views_table.rs`; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table8/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-hex8 ruler plus tall

1. **Hex Ruler Alignment & Tall 1200px View**: Ruler tick alignment locked to byte columns from char metrics (`hex_ruler_col_x` == `hex_grid_byte_col_x` across cols 0..15), byte cell size locked from TypoRole Mono (text_w 14.0, box_w 30.0, row_h 15.6), tall 1200px canvas lock (`HEX_TALL_CANVAS_H` 1200.0, panes_h 1070.0, 68 visible rows), and edge fixture `comparison/hex-tall-1200.json`.
2. **Tests** (before → after): autocompare-gui 1003 green (including `hex_exq_hex8_ruler_plus_tall_1200`); rustfmt clean; clippy clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_hex.rs`, `checks/gui-exq-hex8/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-copy10 narrow plus hits

1. **Copy Narrow-720 Arrows & Hits**: per-hunk arrows keep symmetric 32px slop hit rects (24px XL floor + 4px XS slop every side, centered at narrow center 360,300) over a 23px char-metric column (1 glyph x 7.0 advance + 2 x 8.0 section-gap bearings), stacked arrows ride locked 19.6px pitch (15.6 Mono row + 4.0 tight gap), copy-line action text (Small 13.2 line) sits dy 1.2 inside one Mono row (15.6px), copy button 32px hit centered, and edge fixture `comparison/copy-narrow-720.json`.
2. **Tests** (before → after): autocompare-gui 1008 green (including `copy10_narrow_*` narrow arrows/pitch/action-dy/fixture tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/copy.rs`, `checks/gui-exq-copy10/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-viewfill8 narrow plus panes

1. **Views Narrow-720 Dual Panes & Center Gutter**: dual text-compare panes keep symmetric widths from the space scale at 720px (two panes + 20px center gutter), file-bar height locked to TypoRole chrome rhythm, center gutter column x from char metrics with markers only at changed rows, and edge fixture `comparison/views-narrow-720.json`.
2. **Tests** (before → after): autocompare-gui 1009 green (including `exq_viewfill8` narrow panes tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-viewfill8/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-gutter10 markers plus viewport

1. **Gutter Markers & Viewport at Narrow 720px**: marker x from char metrics at narrow widths (`gutter_side_layout_720`, `gutter_side_column_x_720` 350.0, `gutter_side_arrow_cx_720` 360.0 canvas middle, glyph 354..366, hit band 348..372; center column 348.0, cx 360.0), padded click viewport bottom floor lock (`gutter_viewport_rect_padded` floor >= 4px for row 999 @400px), marker visibility only on changed rows (`gutter_marker_visible` and `gutter_side_marker_visible`), and edge fixture `comparison/gutter-narrow-720.json`.
2. **Tests** (before → after): autocompare-gui 1009 green (including `gutter10_narrow_720_marker_alignment_from_char_metrics`); clippy 0 warnings on owned `gutter.rs`; rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/gutter.rs`, `checks/gui-exq-gutter10/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-table9 narrow plus keys

1. **Table Narrow-720 Keys & Widths**: key-column widths from char metrics at 720px (avail 690-ish budget), numeric right-alignment via grid origin, header/body heights from TypoRole rhythm, and edge fixture `comparison/table-keys-720.json`.
2. **Tests** (before → after): autocompare-gui 1011 green (including `exq_table9` narrow keys tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table9/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-home6 narrow plus cards

1. **Home Narrow-720 Cards & CTA**: card min widths from space scale at 720px, CTA width from card inner width, toolbar button 24px symmetric hits, card grid wrap at narrow width, and edge fixture `comparison/home-narrow-720.json`.
2. **Tests** (before → after): autocompare-gui 1011 green (including `exq_home6` narrow cards tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/home.rs`, `checks/gui-exq-home6/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-copy11 tall plus pitch

1. **Copy Tall-1200 Arrows & Pitch**: stacked per-hunk arrow pitch locked over tall 1200px canvas (Mono row + tight gap), copy button hit band centered tall, action text baseline steady over long scroll, and edge fixture `comparison/copy-tall-1200.json`.
2. **Tests** (before → after): autocompare-gui 1015 green (including `exq_copy11` tall pitch tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/copy.rs`, `checks/gui-exq-copy11/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-home7 tall plus rows

1. **Home Tall-1200 Card Rows & CTA Lock**: card grid 3 cols stable at 1200px tall canvas (1176 avail, EXQ 24 gap, 376px flexed cards), CTA locked to card inner width (min 224, 360 at 1200, single-line over long scroll), toolbar 24px hit rhythm symmetric slop (96x24 + 4px -> 104x32), and edge fixture `comparison/home-tall-1200.json`.
2. **Tests** (before → after): autocompare-gui 1017 green (including `home7_*` tall rows tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/home.rs`, `checks/gui-exq-home7/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-hex9 narrow plus ruler

1. **Hex Narrow-720 Ruler & Cell Lock**: ruler tick xs aligned to byte grid columns 0..16 at 720px narrow canvas (322px ruler), byte cell lock from TypoRole Mono, 37 visible rows at 590px panes height, and edge fixture `comparison/hex-narrow-720.json` (tall-1200 fixture reused).
2. **Tests** (before → after): autocompare-gui 1017 green (including `hex_exq_hex9_narrow_plus_ruler`); focused re-verify 1/1 PASS; rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_hex.rs`, `checks/gui-exq-hex9/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-table10 tall plus rows

1. **Table Tall-1200 Rows & Header Lock**: row heights from TypoRole rhythm at 1200px tall canvas, key-column widths locked over long scroll, header/body heights stable with delta guard, and edge fixture `comparison/table-tall-1200.json`.
2. **Tests** (before → after): autocompare-gui 1019 green (including `table10_*` tall rows tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views_table.rs`, `checks/gui-exq-table10/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-tree9 narrow plus arrows

1. **Tree Narrow-720 Arrows & Truncation**: expand/collapse arrow hit rects symmetric slop from space scale at 720px narrow, truncated name width from char metrics, selected-row full width, and edge fixture `comparison/tree-narrow-720.json`.
2. **Tests** (before → after): autocompare-gui 1019 green (including `tree9_*` narrow arrows tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/tree.rs`, `checks/gui-exq-tree9/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-copy12 narrow plus hits

1. **Copy Narrow-720 Hits & Pitch Lock**: copy hit bands centered at 720px narrow canvas, stacked arrow pitch locked at narrow widths, per-hunk action baseline steady, and edge fixture `comparison/copy-narrow-720.json` (tall-1200 fixture reused).
2. **Tests** (before → after): autocompare-gui 1026 green (including `copy12_*` narrow hits tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/copy.rs`, `checks/gui-exq-copy12/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-viewfill9 tall plus panes

1. **Views Tall-1200 Dual Panes & Gutter Lock**: dual text-compare panes at 1200px tall canvas with scroll canvas height budget (1200.0 - 43.2 = 1156.8px), side-by-side equal heights (1156.8, 1156.8), over-under stacked equal heights (574.4, 574.4), file-bar rhythm locked to 15.2px over tall canvas, center gutter column over tall canvas (1156.8px height, 20px width, 2px hit slop [348..372] at 720 / [468..492] at 960), middle arrow vertical centering at 600.0px canvas / 578.4px scroll canvas, segment tracking `text_tall_segment_arrow_cy`, and edge fixtures `views-tall-1200.json` and `views-narrow-720.json`.
2. **Tests** (before → after): autocompare-gui 1019 green (including `viewfill9_tall_1200_dual_panes_plus_gutter_column`); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-viewfill9/**`.


### Feature (2026-11-21) — feat(gui): exq-take7 tall plus markers

1. **Take Tall-1200 Markers & Hit Lock**: take marker rows aligned with hunk rows at 1200px tall canvas, hit lock over long scroll, marker rhythm steady, and edge fixture `comparison/take-tall-1200.json` (narrow-240 fixture reused).
2. **Tests** (before → after): autocompare-gui 1027 green (including `take7_*` tall markers tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/take.rs`, `checks/gui-exq-take7/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-tree10 tall plus columns

1. **Tree Tall-1200 Columns & Right-Align Lock**: size/date column widths stable at 1200px tall canvas, right-alignment locked over long scroll, selected-row full-width highlight tall, and edge fixture `comparison/tree-tall-1200.json` (narrow-720 fixture reused).
2. **Tests** (before → after): autocompare-gui 1032 green (including `tree10_*` tall columns tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/tree.rs`, `checks/gui-exq-tree10/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-gutter11 tall plus center

1. **Gutter Tall-1200 Center Column & Marker Lock**: center ~20px gutter column over tall 1200px canvas, marker x locked from char metrics over long scroll, padded click viewport floor tall, changed-rows-only visibility, and edge fixture `comparison/gutter-tall-1200.json` (narrow-720 + padded-viewport fixtures reused).
2. **Tests** (before → after): autocompare-gui 1032 green (including `gutter11_*` tall center tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/gutter.rs`, `checks/gui-exq-gutter11/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-toolbar11 narrow plus buttons

1. **Toolbar Narrow-720 Buttons & Tabstrip**: icon+label button symmetric hit rects at 720px narrow canvas, tab widths from label metrics (tabstrip-8tabs reused), second-row nav-arrow rhythm, per-pane status halves even split, and edge fixture `comparison/toolbar-narrow-720.json`.
2. **Tests** (before → after): autocompare-gui 1033 green (including `toolbar11_*` narrow buttons tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-toolbar11/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-status12 tall plus strips

1. **Status Tall-1200 Strips & Legend**: file-bar metadata line rhythm under file bar, per-pane status strip heights + text baselines at 1200px tall canvas, bottom legend strip height, and edge fixture `comparison/status-tall-1200.json`.
2. **Tests** (before → after): autocompare-gui 1034 green (including `status12_*` tall strips tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-status12/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-paint13 center gutter adoption

1. **Paint Center-Gutter Adoption**: paint_text_body now renders the visible ~20px center gutter gap adopted from gutter geometry helpers, draws yellow changed-row markers via marker helpers, and fills floor below last row. Geometry from existing helpers only, no new widths. Edge fixture `comparison/paint-center-1200.json`.
2. **Tests** (before → after): autocompare-gui 1035 green (including `paint13_*` gutter adoption tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS.
3. **Files changed**: `crates/gui/src/views.rs`, `checks/gui-exq-paint13/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-shot15 fixture gutter pixels

1. **Fixture Gutter Pixels (first visual proof)**: fixture renderer now draws the ~20px center gutter gap with yellow '=>' markers on changed-row hunk heads in the default text fixture. pxdiff vs baseline: 10012 of 576000 pixels differ, bbox (460,148)-(837,559), VERDICT DIFFERENT (was IDENTICAL 0px before; also fixed the harness to screenshot the just-built binary). Edge fixture `comparison/shot-center-1200.json`.
2. **Tests** (before → after): autocompare-gui 1036 green (including `shot15_*` gutter pixel tests); rustfmt clean; banned non-test tokens 0; colors frozen at 33 touches; exquisite harness RESULT: PASS + pxdiff visual gate pasted in GATES.md.
3. **Files changed**: `crates/gui/src/screenshot.rs`, `checks/gui-exq-shot15/**`, `TIMELINE.md`.

### Feature (2026-11-21) — feat(gui): exq-paint16 egui gutter mirror

1. **Egui Gutter Mirror**: paint_text_body mirrors the shot15 fixture look in the live egui path — 20px center gap rects, yellow hunk-head markers (ink reused from frozen CHANGED_FG via text_center_marker_fg), selection bands, floor fill below last row. Geometry from existing helpers only, typography untouched, colors frozen at 33 touches. Edge fixture comparison/paint-egui-mirror-1200.json.
2. **Tests** (before -> after): autocompare-gui 1037 green (including paint16_egui_mirror_gap_plus_markers_plus_floor); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS. Live-window runtime verification pending as orchestrator follow-up.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-paint16/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-chrome17 app chrome geometry

1. **App-Chrome Geometry**: toolbar height/padding/button rhythm, tabstrip tab widths + padding, file-bar height + Browse alignment, per-pane metadata line rhythm. Existing SPACE_*/margin consts reused, TypoRole sizes only, colors frozen at 33 touches. Edge fixture comparison/chrome-narrow-720.json + headless test chrome17_narrow720_no_overlap.
2. **Tests** (before -> after): autocompare-gui 1038 green; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS. Pixel gate ABANDONED with reason (fixture renderer paints chrome rects hardcoded in screenshot.rs; no chrome helper feeds those pixels — same structural block as paint14; egui delta visible only in live app, live-window verification pending).
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-chrome17/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-shot18 fixture chrome pixels

1. **Fixture Chrome Pixels (visual-spec-first)**: screenshot.rs fixture renderer now paints the chrome17 geometry — taller toolbar band with button rhythm, padded tabstrip tabs, file-bar rows with per-pane metadata line, segmented bottom status. Shot15 center gutter + yellow markers intact. pxdiff vs baseline: 138850 of 576000 DIFFERENT, bbox (0,40)-(959,448), VERDICT DIFFERENT (independently re-verified).
2. **Tests** (before -> after): autocompare-gui 1039 green (including shot18_chrome720_no_overlap); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-shot18/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-typo17 text typography

1. **Text Typography**: body/mono/small role sizes from TYPO_* tokens, line-height-derived row heights, line-number column from char metrics, letter-spacing, baseline offsets. Touch ONLY text-region helpers + paint_text_body call sites (text_body_role, text_linenum_column_width_from_metrics, text_typo17_row_height, etc.). Colors frozen at 33 touches. Edge fixture comparison/typo-tall-1200.json + headless test typo17_tall1200_numbers_aligned.
2. **Tests** (before -> after): autocompare-gui 1040 green; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS. Pixel gate IDENTICAL (views.rs egui path does not feed fixture pixels — same structural block as chrome17; live-window verification pending). Note: leaf also drafted out-of-OWNS crates/sessions edits in its worktree — deliberately left uncommitted and excluded from landing.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-typo17/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-shot19 fixture typo pixels

1. **Fixture Typo Pixels (visual-spec-first)**: screenshot.rs fixture renderer mirrors the typo17 look — line-number column with char-metric inset, line-height row rhythm with breathing room, stable baselines. Shot15 gutter + markers and shot18 chrome intact. pxdiff vs baseline: 97239 of 576000 DIFFERENT, bbox (12,179)-(935,341), VERDICT DIFFERENT (independently re-verified).
2. **Tests** (before -> after): autocompare-gui 1041 green (including shot19_typotall_numbers_aligned); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-shot19/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-gutter20 live center gutter

1. **Live Center Gutter**: paint_text_body center gutter widened to locked 20px (text_center_gutter_width), yellow hunk-head arrows via text_center_marker_fg on changed rows only, selection bands + floor fill preserved. Fixture already had this from shot15; this is the egui mirror. Colors frozen at 33 touches. Edge fixture comparison/gutter-tall-1200.json + headless test gutter20_tall_gutter_stable.
2. **Tests** (before -> after): autocompare-gui 1042 green; rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS. Pixel gate ABANDONED honestly (fixture path cannot see egui pixels; IDENTICAL is the correct outcome; procedural proof = headless test + suite). Live-window verification pending.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-gutter20/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-status20 fixture status strips

1. **Fixture Status Strips**: screenshot.rs renderer now paints per-pane strips (cursor position, format word, result counts) plus a 4-segment bottom legend (Differences/Same/Insert/Overview). Shot15 gutter/markers, shot18 chrome, shot19 typo rows intact. pxdiff vs baseline: 17873 of 576000 DIFFERENT, bbox (0,542)-(959,584) bottom-strip region, VERDICT DIFFERENT (independently re-verified).
2. **Tests** (before -> after): autocompare-gui 1043 green (including status20_narrow720_no_overlap); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-status20/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-toolbar21 fixture toolbar icons

1. **Fixture Toolbar Icons**: screenshot.rs toolbar buttons now carry original 12px geometric glyph marks (fill_rect only, existing palette) left of labels; second nav row adds two chevron triangles on Prev/Next rhythm. Gutter/markers/chrome/typo/status intact. pxdiff vs baseline: 1995 of 576000 DIFFERENT, bbox (16,120)-(866,162) toolbar band only, VERDICT DIFFERENT (independently re-verified).
2. **Tests** (before -> after): autocompare-gui 1044 green (including toolbar21_narrow720_no_overlap); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS. Note: leaf also drafted out-of-OWNS crates/sessions edits in its worktree — deliberately left uncommitted and excluded from landing.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-toolbar21/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-mirror22 live gutter status mirror

1. **Live Gutter+Status Mirror (root-cause fix)**: diagnosis proved width was never the bug (all 6 live call-sites already 20px) — two visual bugs: (a) hunk-head marker used framed small_button pill on changed row only, (b) unselected rows painted transparent (no fill), so the 20px column was invisible. Fix: frameless gap-rect arrows via text_center_marker_fg on changed rows, continuous track fill (track_for when unselected, CHANGED_BG band when selected) on every live center cell incl. floors. Plus live bottom bar mirror of status20 (per-pane labels with honest fallback, 4-segment legend). Colors frozen at 33.
2. **Tests** (before -> after): autocompare-gui 1045 green (including mirror22_live_gutter_path); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS. Pixel gate ABANDONED honestly (fixture path cannot see egui pixels).
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-mirror22/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-gutter23 visible live gutter

1. **Visible Live Gutter (two half-fixes)**: (a) DARK_TRACK 32->48 in theme.rs (delta vs 27 pane bg goes 5->21/255, no new hue; shared track_for also lifts hex-minimap + merge-overview tracks), (b) both live hunk-head marker sites in paint_text_body switch Button::small().frame(false) to allocate_exact_size + painter().text in CHANGED_FG (255,235,160) with same hover + click behavior. First-ever LIVE-PIXEL GATE: PASS (unselected track median brightness 48.0, 100 yellow marker pixels, crop live-crop.png).
2. **Tests** (before -> after): autocompare-gui 1046 green (including gutter23_live_track_contrast); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS; fixture pxdiff ABANDONED honestly (egui-only paint, byte-identical before/after); colors frozen at 33.
3. **Files changed**: crates/gui/src/views.rs, crates/gui/src/theme.rs, checks/gui-exq-gutter23/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-filebar24 fixture file-bar

1. **Fixture File-Bar (extends status20 upward)**: screenshot.rs file-bar now paints per-pane rows tracking true pane widths — path field (ellipsis-clipped) + Browse refit inside each bar + metadata segment (timestamp | bytes | format word, "-" fallback). Wide bars (12,76,448,36)/(488,76,448,36), narrow-720 (12,76,340,36)/(356,76,340,36). Shot15 gutter/markers, shot18 chrome, shot19 typo, toolbar21 glyphs, status20 strips intact. pxdiff vs baseline: 12958 of 576000 DIFFERENT, bbox (12,76)-(952,111) file-bar band only.
2. **Tests** (before -> after): autocompare-gui 1047 green (including filebar24_narrow720_no_overlap); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS; colors frozen at 33.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-filebar24/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-toolbar22 live toolbar glyphs

1. **Live Toolbar Glyphs (egui overlay, views.rs only)**: TOOLBAR_GLYPH_W/H + toolbar_glyph_width/lead/variant/rects/slot + paint_toolbar_glyph; paint_toolbar_action_buttons + paint_action_nav_row + paint_text_find_state_bar now paint 12px geometric slots left of labels (Swap solid block, Reload twin bars, Prev/Next + FindPrev/FindNext chevrons) on frozen palette. First live-pixel gate on toolbar: PASS — 6/6 buttons with glyph slots (ink ~150 vs face ~50), 2 chevron triples in nav row ([520,530]/[548,566], [724,734]/[548,566], 12x20). Crop round-1/live-crop.png. Fixture pxdiff ABANDONED honestly (egui-only, before/after byte-identical 11254B).
2. **Tests** (before -> after): autocompare-gui 1047 -> 1048 green (including toolbar22_live_glyph_slots + 1200px edge fixture toolbar-live-1200.json: band 24+8+24 stable, no h-scroll); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS; colors frozen at 33.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-toolbar22/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-hatch25 calm trailing fill

1. **Calm Trailing Fill (screenshot.rs renderer)**: trailing fill region now ends at pane strips with zero bottom waste (content_end tracking, short content scrolls in full-height viewport). Shot15 gutter/markers, shot18 chrome, shot19 typo, toolbar21 glyphs, filebar24 bar, status20 strips intact. pxdiff vs baseline: 216832 of 576000 DIFFERENT, bbox (12,300)-(935,541) trailing fill band only.
2. **Tests** (before -> after): autocompare-gui 1048 -> 1049 green (including hatch25_tall_no_banding + hatch-tall-1200.json edge fixture); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS; colors frozen at 33.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-hatch25/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-legend25 live bottom legend

1. **Live Bottom Legend (views.rs egui overlay)**: per-pane status labels (cursor/format/counts, honest fallback) + thin 4-segment legend row mirroring status20 fixture geometry. Toolbar22 glyphs, gutter23 track/markers intact. Fixture pxdiff IDENTICAL (egui-only, before/after byte-identical 11213B) — ABANDONED with reason. Live-pixel gate PASS: per-pane label ink in both halves + 4/4 legend cells marker-distinct, crop round-1/live-crop.png.
2. **Tests** (before -> after): autocompare-gui 1049 -> 1050 green (including legend25_narrow720_no_overlap + legend-narrow-720.json edge fixture); rustfmt clean; banned non-test tokens 0; exquisite harness RESULT: PASS; colors frozen at 33.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-legend25/**, TIMELINE.md.

### Chore (2026-11-21) — chore(localrun): exq-localrun4 early feedback bundle

1. **Early feedback bundle**: CLI 10 cmds PASS, 3 PNGs default 11098B / dark 11200B / light 11098B magic ok, dist tarball 5.5M + sha refreshed (VERSION 0.5.0 3d33e80), tail-insertion edge fixture, GAPS-TOP10. Release LTO link ok (13Gi free).
2. **Tests**: REPORT RESULT: PASS; owned scope fmt clean; banned 0; no crates touch.
3. **Files changed**: checks/ac-loop/round-lr4/**, dist/VERSION + tarball + sha, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-chrome27b live toolbar geometry

1. **Toolbar geometry 12px -> 16px**: TOOLBAR_GLYPH_W/H 16, slot lead (24-16)/2=4 centered, glyph rects rescaled 16x16 origin, band 24+8+24=56 stable, 1200px no h-scroll + narrow700 edge. pxdiff IDENTICAL (screenshot path glyph nahi paint karta, headless proved).
2. **Tests**: chrome27_toolbar16_band1200 + toolbar22_live_glyph_slots green; exquisite harness RESULT: PASS; views.rs fmt clean; banned 0; colors 38 frozen.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-chrome27/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-gutter28 live overview strip (agyx swarm)

1. **Overview minimap fix**: screenshot.rs unclosed delimiter fix + gutter_padded_y_for_row + gutter_overview_geometry_padded reuse, center gutter + right overview strip live mapping, 720px narrow intact. Agyx swarm single-task trial PASS — mix consistent, opencode default + agyx swarm parallel ok.
2. **Tests**: exquisite harness RESULT: PASS (11059B magic ok); headless gutter mapping green; fmt clean; banned 0; colors 38 frozen.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-gutter28/**, TIMELINE.md.

### Feature (2026-11-21) — feat(compare): folder30 sync prune + jobs

1. **Folder parity**: engine already correct, minimal 78-line sync prune test + nested-orphan 4-deep fixture, parity harness RESULT: PASS 10/10.
2. **Tests**: autocompare-compare sync green; clippy -D warnings clean; fmt clean; banned 0.
3. **Files changed**: crates/compare/tests/sync.rs, checks/ac-loop/round-folder30/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-text30 live text typo

1. **Typography second**: TEXT_ROW_HEIGHT_COMPACT 18.0 -> 20.0 SPACE_LG+SPACE_XS reuse, mono 15.6 line-height 4.4px pad, MAX 16.9 3.1px pad zero clip, narrow700 wrap fixture.
2. **Tests**: 1061 green (text30_wrap700_zero_clip new); exquisite harness RESULT: PASS 11059B; fmt clean; banned 0; colors 38 frozen.
3. **Files changed**: crates/gui/src/viewmodel.rs, checks/gui-exq-text30/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-table30 live table geometry (agyx swarm, orchestrator recovery)

1. **Table geometry**: 1200px canvas fit `needs_hscroll:false`, key-cols stable, sort markers aligned, tolerance sanitized, xlsx sheet selector 1/3 badge. Geometry first SPACE_* reuse, typo second 600 weight lock.
2. **Tests**: 1061 green, exquisite harness RESULT: PASS 11059B, fmt clean, banned 0, colors 38 frozen. Agyx did full work but timeout before commit (0-byte log, no DONE) — orchestrator verified OWNS and committed, mix still consistent for work but commit step inconsistent.
3. **Files changed**: crates/gui/src/views_table.rs, checks/gui-exq-table30/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-tree31 live folder tree (opencode default)

1. **Folder tree geometry**: indent step 8.0 SPACE_SM, row 23.6 mono 15.6 + SM, expander 16x16 centered, guides at centers, 1200px no h-scroll + 700px narrow edge.
2. **Tests**: 1063 green (+1 tree31_deepnest), exquisite harness RESULT: PASS 11059B, fmt clean, banned 0, colors 38 frozen.
3. **Files changed**: crates/gui/src/tree.rs, checks/gui-exq-tree31/**, TIMELINE.md.

### Chore (2026-11-21) — chore(localrun): exq-localrun5 early feedback bundle (opencode default)

1. **Early feedback**: CLI 10/10 PASS, 3 PNGs default 11059B / dark 11262B / light 11059B magic ok, dist tarball 5816822B + sha, REPORT RESULT: PASS, LTO release ok.
2. **Tests**: owned scope no Rust files, fmt clean, banned 0.
3. **Files changed**: checks/ac-loop/round-lr5/**, dist/VERSION + tarball + sha, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-home32 live home sessions (agyx swarm, orchestrator recovery)

1. **Home sessions geometry**: session cards grid, search field 28px, workspace rows, 1200px no h-scroll + 8tabs edge. SPACE_* reuse, TypoRole reuse.
2. **Tests**: 1063 green, exquisite harness RESULT: PASS, fmt clean, banned 0, colors 38 frozen. Agyx did work but timeout before commit (0-byte log, no DONE) — orchestrator verified OWNS and committed, same pattern as table30.
3. **Files changed**: crates/gui/src/home.rs, checks/gui-exq-home32/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-hex31 live hex geometry (opencode default)

1. **Hex geometry**: offset gutter SPACE_* reuse, row TypoRole Mono, hex dump columns alignment, ascii pane gap, 1200px no h-scroll + narrow700 edge.
2. **Tests**: 1065 green (+1 hex), exquisite harness RESULT: PASS, fmt clean, banned 0, colors 38 frozen. Leaf committed correct OWNS, sessions edits left uncommitted and reverted, no contamination landed.
3. **Files changed**: crates/gui/src/views_hex.rs, checks/gui-exq-hex31/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-take33 live merge take (opencode default)

1. **Merge geometry**: Take buttons SPACE_* reuse, 3-way center alignment, conflict badge, 1200px no h-scroll + narrow700 edge.
2. **Tests**: 1070 green (+5 take), exquisite harness RESULT: PASS, fmt clean, banned 0, colors 38 frozen. Sessions edits left uncommitted and reverted, no contamination.
3. **Files changed**: crates/gui/src/take.rs, checks/gui-exq-take33/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-gutter36 live overview polish (opencode default)

1. **Overview strip geometry**: paint_overview_strip rounding fix matching overview_row_y, 1px drift gone, MIN_H floor + frozen hues unchanged, wide 960 + narrow 720 share single source.
2. **Tests**: 1068 green (+3 gutter36 alignment/narrow/edge), exquisite harness RESULT: PASS 11040B, fmt clean, banned 0, colors +0 frozen, pixel gate 32px bbox exactly overview strip.
3. **Files changed**: crates/gui/src/screenshot.rs, checks/gui-exq-gutter36/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-app35 live shell tabs (opencode default)

1. **Shell tabs geometry**: app shell tab bar SPACE_* reuse, session tabs alignment, 1200px no h-scroll + shell-1200 edge fixture, TypoRole reuse.
2. **Tests**: 1067 green, exquisite harness RESULT: PASS 11059B, fmt clean, banned 0, colors 38 frozen. Sessions edits left uncommitted and reverted in worktree, no contamination landed.
3. **Files changed**: crates/gui/src/app.rs, checks/gui-exq-app35/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-copy34 live copy arrows (agyx swarm, orchestrator recovery)

1. **Copy geometry**: left/right arrows SPACE_XL column + XS pad, 16px hitbox centered, inline preview TypoRole Mono 15.6 rhythm, 1200px no h-scroll + 700px narrow edge.
2. **Tests**: 1070 green, exquisite harness RESULT: PASS 11059B, fmt clean, banned 0, colors 38 frozen. Agyx did full work, orchestrator verified OWNS and committed (same timeout-recovery pattern as table30/home32).
3. **Files changed**: crates/gui/src/copy.rs, checks/gui-exq-copy34/**, TIMELINE.md.

### Reports (2026-11-21) — feat(reports): rep37 html text patch (opencode default)

1. **Reports parity evidence**: ac-loop round-rep37 CLI parity RESULT: PASS 10/10, GUI geo-typo PASS, no reports source changes required this round, edge fixtures own-help.txt + bc-process.txt added.
2. **Tests**: autocompare-cli parity green, gui tests green, harness side-effect sessionpaint PNGs reverted in worktree, none landed.
3. **Files changed**: checks/ac-loop/round-rep37/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-toolbar37 live toolbar spacing (opencode default)

1. **Toolbar geometry**: TOOLBAR_H 40px SPACE_XL+SPACE_LG, button spacing SPACE_SM, 16px glyph centered in 24px button, overflow >> affordance, 1200px no h-scroll + narrow700 edge.
2. **Tests**: 1082 green (+2 toolbar), exquisite harness RESULT: PASS 11040B, fmt clean, banned 0, colors 38 frozen. Leaf self-cleaned harness side-effect PNGs, OWNS-only landed.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-toolbar37/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-text38 live center gutter (agyx swarm, clean commit)

1. **Center gutter geometry**: 20px lane at canvas midpoint, 16px arrows centered with 2px margins, TypoRole Mono 15.6 rhythm, 1200px no h-scroll + narrow700 edge.
2. **Tests**: 1080 green, exquisite harness RESULT: PASS 11040B, fmt clean, banned clean non-test, colors 38 frozen. Agyx committed cleanly itself this time, no recovery needed.
3. **Files changed**: crates/gui/src/gutter.rs, checks/gui-exq-text38/**, TIMELINE.md.

### Chore (2026-11-21) — chore(localrun): exq-localrun6 early feedback bundle (opencode default)

1. **Early feedback**: CLI 10/10 PASS, 3 PNGs default 11040B / dark 11236B / light 11040B magic ok, dist tarball 5816820B + sha, REPORT RESULT: PASS, exquisite PASS colors 38 frozen.
2. **Tests**: owned scope no Rust files, harness side-effect PNGs reverted, no contamination.
3. **Files changed**: checks/ac-loop/round-lr6/**, dist/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-viewmodel39 live text row rhythm (opencode default)

1. **Text row rhythm round-2**: TypoRole line-height reuse, padding XS/SM, narrow700 zero-clip, 1200px no h-scroll.
2. **Tests**: 1088 green (+1 viewmodel), exquisite harness RESULT: PASS 11040B, fmt clean, banned 0, colors 38 frozen. Sessions + sessionpaint strays reverted in worktree, none landed.
3. **Files changed**: crates/gui/src/viewmodel.rs, checks/gui-exq-viewmodel39/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-views40 live text panes (opencode default)

1. **Text panes round-2**: twin panes padding SPACE_*, line-number gutter alignment, hunk header spacing, 1200px no h-scroll + narrow700 edge.
2. **Tests**: 1089 green, exquisite harness RESULT: PASS 11040B, fmt clean, banned 0, colors 38 frozen. Leaf self-cleaned side-effect PNGs, OWNS-only landed.
3. **Files changed**: crates/gui/src/views.rs, checks/gui-exq-views40/**, TIMELINE.md.

### Feature (2026-11-21) — feat(gui): exq-tree41 live folder tree (agyx swarm, clean commit)

1. **Folder tree round-2**: indent rhythm SPACE_SM, 16px expander centered, guide alignment, 1200px no h-scroll + deepnest edge.
2. **Tests**: 1088 green, exquisite harness RESULT: PASS 11040B, fmt clean, banned 0, colors 38 frozen. Agyx committed cleanly itself, no recovery.
3. **Files changed**: crates/gui/src/tree.rs, checks/gui-exq-tree41/**, TIMELINE.md.
