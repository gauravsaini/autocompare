# FOLDERTABLE10 REPORT — columns plus indent (Hinglish-Roman)

Round: `checks/gui-exq-foldertable10/round-1` — harness `RESULT: PASS` (1 round, max 6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

Evidence (`checks/localrun/after-folder.png`) me folder compare rows full-width harsh bands me dikh rahi thi, jahan Size aur Date/Modified columns gayab the, tree indent minimal tha, aur sync preview aadha screen waste kar raha tha. BC folder compare me:
1. Har folder pane me 3 columns ka Grid rehta hai: Name (flex path/name, depth indented), Size (fixed width, right-aligned byte counts), aur Modified (fixed width, left-aligned timestamps).
2. Columns ke beech standard `SECTION_GAP` (8.0px = `SPACE_SM`) rehta hai taaki columns ek dusre se chipke bina saaf padhe ja sakein.
3. Row height `TypoRole::Mono.line_height_px()` (15.6px) par symmetric padding (+2.0px = 17.6px) se bandhi hoti hai, ad-hoc hardcoded pixel numbers se nahi.
4. Name column har depth par `tree_indent_for_depth(depth)` se indent hota hai, par Size aur Modified columns ki boundaries sabhi rows par fixed aur aligned rehti hain (depth badhne par columns aage-peeche nahi hilte).
5. Orphan rows (left-only ya right-only) missing side par dimmed (`"-"` dashes with `TypoRole::Small` / `is_weak() == true`) render hoti hain, bina kisi color modifications ke (colors frozen).

## Kya kiya (geometry FIRST, typo SECOND, colors FROZEN)

Geometry first (koi naya rhythm/token nahi, sab theme reuse):
- 3-column table definition: `FolderTableCol::Name`, `FolderTableCol::Size`, `FolderTableCol::Modified`.
- Table Grid col gap: `FOLDERTABLE_COL_GAP` == `crate::theme::GEO_SECTION_GAP` (8.0px = `SPACE_SM`).
- Cell padding: `FOLDERTABLE_CELL_PAD_X` == `crate::theme::SPACE_SM` (8.0px).
- Column min widths: Name 184.0px, Size 86.0px (`FOLDER_SIZE_CHARS` * 7 + 16), Modified 128.0px (`FOLDER_TIME_CHARS` * 7 + 16). Fixed trailing cols total 222.0px.
- Grid column offsets: `folder_table_column_xs(avail_w)` computes Name at 0.0, Size at 738.0, Modified at 832.0 (for 960px width). Size aur Modified columns ki horizontal origins har depth level par sthir rehti hain.
- Indent per depth: `folder_table_name_indent_px(depth)` computes `depth * tree_indent_step() + 40.0` (depth 0 par 40px, depth 5 par 100px).
- Row height: `folder_table_line_height_px()` returns `TypoRole::Mono.line_height_px()` (15.6px). `folder_table_row_height()` returns 17.6px (line height + 2 * `FOLDERTABLE_ROW_PAD_Y`), and density-aware options match `FolderDensity` tokens.

Typography second (koi naya size nahi):
- Column header weight: `"strong"`, weight value 600 (`TYPO_WEIGHT_SEMIBOLD`).
- Table cell content role: `TypoRole::Mono` (12.0pt, content_grid / tabular).
- Orphan dimming role: `TypoRole::Small` (rank 4, `is_weak() == true`, `contrast_dim_text_role()` parity).
- Zero new point sizes; harness checks `.size` strictly `['14.0']`.

Colors frozen:
- Zero `Color32` or `Visuals` modifications (report count 33, previous wave baseline se zero growth).

## Edge fixture + headless tests

`comparison/folder-deep-nest.json`: 5-level deep directory nesting (`root` through `root/sub1/sub2/sub3/sub4/deep-file.txt`) plus orphan-left and orphan-right entries.
- Headless test `exq_foldertable10_edge_deep_nest_and_dimmed_orphans` proves:
  - Indent monotonically scales from 40px (depth 0) to 100px (depth 5).
  - Trailing columns (Size and Modified) stay strictly aligned at 738.0px and 832.0px across all depths without horizontal jitter.
  - Orphan rows on missing sides are cleanly flagged as dimmed (`folder_table_cell_is_dimmed`), returning `"-"` with `TypoRole::Small` (`is_weak() == true`).
- 5 comprehensive headless unit tests in `crates/gui/src/tree.rs`:
  - `folder_table_grid_has_section_gap_and_headers`
  - `folder_table_row_height_derived_from_typo_mono`
  - `folder_table_column_xs_and_fixed_trailing_alignment`
  - `folder_table_orphan_rows_dimmed_without_color_edits`
  - `exq_foldertable10_edge_deep_nest_and_dimmed_orphans`

## Natija

- PNG 10401 bytes, magic ok, distinct 256 (non-blank).
- 893 tests passed, 0 failed.
- Clippy clean on `crates/gui/src/tree.rs`.
- rustfmt clean on `crates/gui/src/tree.rs`.
- Banned tokens (unwrap/expect/panic/todo/unreachable/unsafe) 0.
- Harness: RESULT: PASS.
