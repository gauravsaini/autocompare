# GATES — Wave GUI-FOLDER-H (Home View session tree + FolderCompare drill-down)

## Scope (OWNS only)

- `crates/gui/src/home.rs` (Home View session tree model, grouping, geometry-first + typo-second)
- `crates/gui/src/tree.rs` (FolderCompare drill-down scope, breadcrumbs, compare intent, geometry-first + typo-second)
- `checks/gui-folder-h/**`

MAT CHHOONA (FORBIDDEN — sibling waves parallel, read-only):
`crates/cli/**`, `app.rs`, `views.rs`, `copy.rs`, `take.rs`, `gutter.rs`, `viewmodel.rs`,
`views_hex.rs`, `views_table.rs`, `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`.
Colors FROZEN (zero new Color32 / Visuals).

## Goals (leaf-3.2 exquisite parity)

1. **Home View Session Tree**:
   - Pure headless `SessionTreeNode`, `SessionTreeGroup` hierarchy grouping sessions (from `SessionList` and `SessionHistory`) by session category or workspace.
   - Expand/collapse toggle methods (`toggle_session_group`, `expand_all_session_groups`, `collapse_all_session_groups`).
   - Geometry-first contract: `SESSION_TREE_WIDTH` (240.0), `SESSION_TREE_ROW_H` (24.0), `SESSION_TREE_INDENT_STEP` (12.0), `SESSION_TREE_PAD_X` (8.0), `SESSION_TREE_PAD_Y` (4.0), `SESSION_TREE_EXPANDER_W` (16.0), `SESSION_TREE_ICON_W` (16.0).
   - Typography-second contract: Group headers (`TypoRole::HomeButton` medium/strong), session items (`TypoRole::HomeButton` regular), meta/age badges (`TypoRole::caption()` small/weak). All single-sourced from existing `TypoRole` tokens.

2. **FolderCompare Drill-Down**:
   - Pure headless `FolderDrillDownScope` managing breadcrumb navigation (`drill_into`, `drill_up`, `reset`, `breadcrumbs`).
   - `scoped_folder_tree` generating filtered, re-indexed hierarchical nodes relative to the active drill-down scope.
   - `DrillDownCompareIntent` + `detect_tab_for_path` mapping extensions (`.csv`/`.xlsx` -> Table, `.png`/`.jpg` -> Picture, `.wav`/`.mp3` -> Audio, `.bin`/`.exe` -> Hex, else Text) for intelligent drill-down compare staging.
   - Geometry-first contract: `DRILL_DOWN_BAR_H` (28.0), `DRILL_DOWN_CRUMB_H` (24.0), `DRILL_DOWN_PAD_X` (8.0), `DRILL_DOWN_GAP` (4.0), `DRILL_DOWN_SEP_W` (12.0).
   - Typography-second contract: Ancestor crumbs (body mono regular), current crumb (body mono strong), separator (caption mono weak), hint (caption regular).

## DoD & Acceptance Gates

- [x] G1: `cargo test -p autocompare-gui` full green (696 passed, 0 failed) with all new session tree and drill-down unit tests.
- [x] G2: `cargo clippy -p autocompare-gui` on OWNS files zero warnings.
- [x] G3: `rustfmt --edition 2024 --check` clean on OWNS files (`crates/gui/src/home.rs`, `crates/gui/src/tree.rs`).
- [x] G4: Banned patterns zero new in non-test diff (`unwrap`, `expect(`, `panic!`, `todo!`, `unimplemented!`, `unsafe`).
- [x] G5: Colors-grep zero new (`Color32`, `Visuals` additions strictly zero).
- [x] G6: `tools/ac-loop/run.sh checks/gui-folder-h/round-1` passes with `RESULT: PASS`.
- [x] G7: 1+ valid PNG screenshot fixtures in `checks/gui-folder-h/round-1/` (`folder-drilldown.png`, `home-tree.png` both >8192 bytes, valid PNG magic).
- [x] G8: `git status --porcelain` verifies only OWNS files are staged/committed via pathspec.
