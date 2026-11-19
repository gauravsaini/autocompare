# Wave AD-COLS REPORT (Gap #6: folder size/time column paint, aligned columns)

Scope: `crates/gui/src/tree.rs` + `crates/gui/src/views.rs` + `checks/gui-cols-ad/**` only.
`app.rs` / `viewmodel.rs` / `views_table.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs`
(sibling waves + KEVL) untouched, `STATUS.md` untouched, no push.
Clean-room: Beyond Compare bahar se observe only, koi string/asset copy nahi.

## Item-wise before/after

### (a) `tree.rs`: dual side cells fixed-width pad
- Before: `dual_left_cell` / `dual_right_cell` ne `folder_cell_text` ko bina pad ke
  `"size | time"` me joda tha — orphan `-` aur chhoti values se columns tedhe dikhte the.
- After: dono cells `pad_folder_cell` se `FOLDER_SIZE_CHARS` / `FOLDER_TIME_CHARS` tak padded
  (missing `-` bhi padded). `.get()`/saturating only, koi indexing/slicing nahi.
- Test: `dual_side_cells_are_fixed_width` (size/time parts fixed char counts, values barkarar).

### (b) `views.rs`: tree col-def model se Grid paint
- Before: `paint_folder` har row ko `ui.horizontal` me glyph + single `folder_columns` string
  me paint karta tha — sizes/times path se pehle, path flex end me, header nahi tha.
  `paint_folder_dual` pane headers sirf `left` / `right` (small weak, mono nahi) the.
- After: pure helpers `folder_header_cells()` (defs se: Path raw, sides padded),
  `folder_row_cells(row)` (path+dir-mark raw + 4 padded sides, orphan `-` padded) aur
  `folder_grid_columns()` (`1 + defs.len()` = 6, glyph column samet).
  `folder_columns` / `folder_format_row` shape unchanged (back-compat, purane tests same values pass).
- Paint single-list: header line (`folder_header_line`, mono weak) + `-` rule
  (`indented_content` lead, table header jaisi) ke baad body `egui::Grid::new("folder_data_grid")`
  me ScrollArea window ke andar (`num_columns(folder_grid_columns())`,
  `spacing([SECTION_GAP, TIGHT_GAP])`, har row me glyph cell + Path cell
  (depth `tree_indent` offset + expand/collapse + path mono) + 4 padded mono side cells,
  `row_height_for_line(text_row_height)` rhythm). Path flex Grid column me rehta hai,
  sides Grid columns me align rehte hain chahe path lamba ho.
- Paint dual-tree (trivial): dono pane headers ab mono weak me padded side labels samet
  (`left: Path  L size | L modified`, `right: Path  R size | R modified`,
  `folder_header_cells` reuse); row lines `dual_left_line`/`dual_right_line`
  (ab padded `dual_*_cell` se, mono) — orphan `-` padded, dir mark `src/` barkarar.
- Reuse: `folder_column_defs`/`folder_cell_text`/`pad_folder_cell`, `SECTION_GAP`/`TIGHT_GAP`,
  `CONTENT_LEAD`, `HEADER_H`, `row_height_for_line`/`text_row_height`, `tree_indent`,
  glyph colors unchanged; koi naya token/color nahi.
- Tests: `folder_header_cells_match_defs` (5 cells, Path raw, sides fixed widths),
  `folder_row_cells_keep_sides_fixed_width` (path raw + dir mark, sides fixed, orphan `-`),
  `folder_grid_columns_counts_glyph_plus_five` (6), `dual_lines_use_padded_side_cells`
  (padded ` | ` separator, values + dashes).

## Gates
- `cargo test -p autocompare-gui`: 306 passed, 0 failed (naye 5 samet; pre-edit baseline 301).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0).
- `rustfmt --edition 2024 --check` (sirf OWNS files): clean (exit 0).
- Banned grep (non-test code): ZERO-BANNED (`unwrap()`/`expect()`/`panic!`/`todo!`/`unimplemented!`/`unreachable!`
  nahi; `unwrap_or`/`unwrap_or_default` allowed). `forbid(unsafe_code)` kept.
- Colors: NO-COLOR-TYPO-ADD (koi `from_rgb`/`from_rgba`/`Visuals::`/`.size()` add nahi).
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-cols-ad/round-1`: RESULT PASS
  (after.png 9624B, magic-ok `89504e470d0a1a0a`, distinct 256).
- OWNS-clean: diff sirf `tree.rs` + `views.rs` + `checks/gui-cols-ad/**`;
  KEVL-CLEAN (sibling waves ke kaam ko chhua nahi — commit pathspec sirf OWNS paths par).

Commit: local only, no push.
