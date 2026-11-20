# EXQ-TREE2 REPORT — indent plus columns (Hinglish-Roman)

Round: `checks/gui-exq-tree2/round-1` — harness `RESULT: PASS` (1 round, max 6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

Evidence me folder tree rows flat indent show kar rahi thi jisse BC tree depth rhythm nahi mil raha tha. Size aur Modified columns ki boundaries depth badhne par jitter na karein, aur missing sides par orphan entries proper dimming ke saath render hon:
1. Tree depth rhythm flat tha — har level par clear space token scale (`SPACE_MD` = 12.0px) hona zaroori tha taaki expander, status mark, aur path columns har depth par consistent visual hierarchy maintain karein.
2. Orphan rows (left-only ya right-only) missing side par blank hone ki jagah dimmed placeholder dashes (`"-"`) ke saath render honi chahiye, jahan typography `TypoRole::Small` (`is_weak() == true`) se dimming kare bina colors ko touch kiye (colors frozen).
3. 960px wide window par Size aur Modified columns ka horizontal origin strictly fixed rehna chahiye (`Size` at 738.0px, `Modified` at 832.0px, fixed trailing columns 222.0px total) chahe tree depth 0 ho ya 5.

## Kya kiya (geometry FIRST, typo SECOND, colors FROZEN)

Geometry first (SPACE scale tokens aur grid constants reuse):
- Indent step single source: `tree_indent_step_from_space()` == `crate::theme::GEO_TREE_INDENT_STEP` == `crate::theme::SPACE_MD` (12.0px).
- Depth rhythm: `tree_indent_rhythm_px(depth)` computes `depth * 12.0px`.
- Separated path origin: `tree_row_path_x(depth)` computes `depth * 12.0px + 40.0px` (expander 16px + status mark 16px + gap 8px).
- 960px column alignment: `tree_wide_960_column_xs()` and `tree_columns_aligned_at_960(depth)` verify Name at 0.0px, Size at 738.0px, Modified at 832.0px across all depths.
- Col gap: `FOLDERTABLE_COL_GAP` == `GEO_SECTION_GAP` == `SPACE_SM` (8.0px).
- Min widths: Size 86.0px (`FOLDER_SIZE_CHARS` * 7 + 16), Modified 128.0px (`FOLDER_TIME_CHARS` * 7 + 16). `832.0 + 128.0 == 960.0px` (exact right edge parity).

Typography second (zero new point sizes, roles reuse):
- Dimming role for orphan missing sides: `tree_cell_typo_role()` returns `TypoRole::Small` (rank 4, `is_weak() == true`).
- Content role for present cells: `TypoRole::Mono` (rank 3, `is_weak() == false`).
- Banned `.size` checks: strictly `['14.0']`, koi naya size introduce nahi kiya.

Colors frozen:
- Zero `Color32` or `Visuals` additions (report count 33 strictly maintained).

## Edge fixture + headless tests

`checks/gui-exq-tree2/round-1/comparison/tree-wide-960.json` (and `comparison/tree-wide-960.json`):
- 960px wide window geometry with 3-column table and 6-depth hierarchy.
- Headless test `exq_tree2_round2_tree_wide_960_and_orphan_dimming` in `crates/gui/src/tree.rs` proves:
  - Indent rhythm: depths 0..=5 scale monotonically from 0.0px to 60.0px (`[0.0, 12.0, 24.0, 36.0, 48.0, 60.0]`).
  - Path origins: scale from 40.0px to 100.0px (`[40.0, 52.0, 64.0, 76.0, 88.0, 100.0]`).
  - Column alignment: Size at 738.0px, Modified at 832.0px strictly fixed across all depths.
  - Orphan dimming: missing sides flagged as dimmed, formatted as `"-"`, and styled via `TypoRole::Small`.
- Additional unit tests:
  - `tree2_round2_indent_from_space_tokens`
  - `tree2_round2_orphan_dimming_via_typo_small`
  - `tree2_round2_columns_aligned_at_960`

## Swarm coordination

- Swarm with 2 specialized workers (`indent-worker` and `columns-worker`) recruited for:
  - `indent-worker`: SPACE token indent step verification, depth rhythm, orphan dimming via `TypoRole::Small`.
  - `columns-worker`: Size/Modified column alignment at 960px width, edge fixture specification.

## Natija

- PNG: 10401 bytes (min 8192: ok), magic ok, distinct 256 (non-blank).
- 913 tests passed, 0 failed.
- Clippy clean on `crates/gui/src/tree.rs` (`-D warnings`).
- rustfmt clean on `crates/gui/src/tree.rs`.
- Banned non-test code tokens (unwrap, expect, panic, todo, unreachable, unsafe, Color32, Visuals): 0.
- Harness: RESULT: PASS.
