# RENDER_DELTA — exq-tree62 round-1

Change is headless-only: `crates/gui/src/tree.rs` adopts existing
SPACE/TYPO tokens at 11 value-identical const definitions
(gutter59 precedent). `views.rs`/paint resolves through the same
`tree.rs` accessors, so raster output cannot move.

Adoptions (all value-identical, no new tokens/sizes/colors):

- `FOLDER_EXPANDER_W` 16.0 → `theme::SPACE_LG`
- `FOLDER_ROW_HEIGHT` 24.0 → `theme::SPACE_XL`
- `FOLDER_HEADER_HEIGHT` 24.0 → `theme::SPACE_XL`
- `FOLDER_CELL_PAD_X` 8.0 → `theme::SPACE_SM`
- `FOLDER_COL_GAP` 8.0 → `theme::SPACE_SM`
- `FOLDER_FILTER_BAR_HEIGHT` 24.0 → `theme::SPACE_XL`
- `FOLDER_FILTER_BAR_PAD_X` 8.0 → `theme::SPACE_SM`
- `FOLDER_FILTER_BAR_GAP` 8.0 → `theme::SPACE_SM`
- `FOLDER_BODY_SIZE` 13.0 → `theme::TYPO_BODY_SIZE`
- `FOLDER_SIDE_SIZE` 12.0 → `theme::TYPO_MONO_SIZE`
- `FOLDER_HEADER_SIZE` 12.0 → `theme::TYPO_MONO_SIZE`

Repo-wide call-site proof: `views.rs` consumes these only via
`tree::folder_*` accessors / `FOLDER_*` consts (grep:
`folder_filter_bar_height_px`, `folder_expander_width`,
`FOLDER_BODY_SIZE`, `FOLDER_SIDE_SIZE`); `views_table.rs` locks
`table_cell_pad_x() == FOLDER_CELL_PAD_X == SPACE_SM`;
`viewmodel.rs` locks `FOLDER_ROW_HEIGHT_COMFORTABLE == FOLDER_ROW_HEIGHT`.
Hit math untouched (`FOLDER_EXPANDER_HIT_H = FOLDER_ROW_HEIGHT`
transitively identical).

- `after.png` (harness software rasterizer, bypasses App paint — NOT live proof):
  10946 bytes baseline → 10946 bytes post-change, md5
  `3ea634d160afca6fc02567487e4e8466` both runs, magic ok, distinct 256,
  colors 35 (frozen, unchanged from baseline 35).
- Live proof: `cargo test -p autocompare-gui --test real_egui_shot` →
  `test result: ok`, 2 passed (dark + light text-compare shots through `App`).
- Unit proof: `exq_tree62_tests` (3 tests) lock every const to its token
  AND its legacy literal AND the derived layout budgets
  (`folder_row_x_offset`, `folder_expander_hit_rect`,
  `folder_tree_content_height`).
- Suite: 1126 gui tests green (baseline 1123 + 3 new), fmt clean,
  banned-scan 0 on added lines.

Verdict: render delta = zero by construction (value-identical token swap) and
confirmed by byte-identical after.png + green live kittest render.
