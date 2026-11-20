# TIMELINE

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
