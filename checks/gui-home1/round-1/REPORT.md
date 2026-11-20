# gui-home1 — Round 1 REPORT (leaf-3.2 Home View Engineer, wave 9)

OWNS: `crates/gui/src/home.rs`, `crates/gui/src/tree.rs`, `checks/gui-home1/**` only.
Other waves active nearby (guiexq8, uxkey, clivcs2, exq7 WIP) — none of their
paths touched. Colors FROZEN. No new deps. Clean-room: BC screen observed
from outside only (`/tmp/bc-screen.png` vs `/tmp/ac-current.png`); no
decompilation, no asset copy, contracts written in own words from paint/rhythm
tokens.

## 1. Outside observation (geometry first)

- BC (`/tmp/bc-screen.png`): tabbed session workspace (3 session tabs tiled in
  one strip), Home/Sessions toolbar cluster, path input rows with file info
  bar, gutter arrows + line numbers on both panes, right-edge overview ruler,
  bottom status bar (`1:1 | Default text`, `Same | Insert`).
- AC (`/tmp/ac-current.png`, 9765 bytes): 10-tab strip tiles in one row, two
  text panes with path rows, right-edge thin gutter segments
  (yellow/red/green = change map, `gutter.rs` owner wave ke paas — untouched),
  bottom status counts line. No Home launcher cards / session tree sidebar in
  this shot (Text tab active) — headless contracts below keep them honest for
  the paint-owner wave.
- Gaps found in OWNS headless scope (all paint-adoption disjoint):
  G1 session tree had NO Cmd-F find math (folder tree has
  `folder_find_next/prev`; session tree had none).
  G2 card grid fixed 3 columns clips below ~736px (narrow 720 fallback).
  G3 no tab-strip fit predicate for the 10-tab workspace at 960 vs narrow.

## 2. Change (minimal diff, $ponytail: reuse consts/tokens, stdlib only)

`crates/gui/src/home.rs` only (`tree.rs` drill-down already complete — no touch):
- Find bar geometry: `SESSION_TREE_FIND_BAR_H` (= `HOME_TOOLBAR_BUTTON_H` 24),
  `SESSION_TREE_FIND_PAD_X` (= `SESSION_TREE_PAD_X` 8). Zero new numbers.
- Pure match math: `session_tree_find_next` (inclusive, wrap),
  `session_tree_find_prev` (exclusive, wrap), private
  `session_tree_label_contains` (empty-needle None, `tree` precedent).
  Binding itself stays in theme keymap (`KeyAction::Find`, Default Cmd-F).
- Narrow wrap: `home_grid_columns_for_width` (1..=3 from
  `HOME_CARD_MIN_W`+`HOME_GRID_GAP`; non-finite -> canonical 3),
  `home_grid_rows_for_width` (9 cards: 960->3 rows, 720->5, 100->9, zero clip).
- Workspace fit: `home_tab_bar_overflows` (10 tabs = 574px: fits 960, scrolls
  tang; NaN -> false).
- Typography second: `session_tree_find_match_role` == item role (body 13
  medium); meta stays caption 11 weak. Zero new sizes, zero colors.

## 3. Edge fixture (+1 test, 741 -> 742)

`homegeo_wave9_find_narrow_tabs_edge`: empty nodes/needle -> None both dirs;
group header match + case-flag honesty; wrap both directions; no-match None;
find-bar const parity (24/8); columns 960->3 / 736->3 / 720->2 / 240->1 /
100->1 / negative->1 / NaN->3; rows 0/3/5/9; tabs fit 960, overflow 300,
NaN false, empty-titles false; match role == item role, size 13.0.

## 4. Gates

- G1 tests: `CARGO_TARGET_DIR=/tmp/target-op-guihome1 CARGO_BUILD_JOBS=2 cargo
  test -p autocompare-gui` -> 742 passed, 0 failed (baseline 741).
- G2 clippy: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  -> exit 0, zero warnings.
- G3 fmt: `rustfmt --edition 2024 --check crates/gui/src/home.rs` clean
  (`tree.rs` untouched; crate-wide remaining diff is `theme.rs`, not OWNS).
- G4 banned non-test (`unwrap(`/`expect(`/`panic!`/`todo!`/`unreachable!`/
  `unimplemented!`/`allow(dead_code)`/`Color32`/`Visuals`/`unsafe`): zero new
  hits in `home.rs`+`tree.rs` (only `forbid(unsafe_code)` baseline line).
- G5 docs: every new pub item has `///` docs; strong types reused
  (`TypoRole`, `SessionTreeNode`); no `[]` indexing (clippy
  `indexing_slicing` deny respected via `.get()`).
- G6 isolation: only OWNS paths staged/committed; sibling wave files
  (`crates/cli/**`, `crates/gui/src/{gutter,theme,views_hex}.rs`,
  `crates/sessions/**`, `checks/gui-sessionpaint-by/**`) left untouched.
  NO PUSH.

Commit: faf5464 (feat commit; hash line filled post-commit, no code change after)
Counts: tests 741 -> 742 (+1 edge fixture) | clippy warnings 0 | fmt clean (own file) | banned non-test 0

RESULT: PASS
