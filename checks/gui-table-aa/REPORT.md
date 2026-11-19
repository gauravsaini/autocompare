# Wave AA-TABLE REPORT (Gap #3: table grid + header paint, proper columns)

Scope: `crates/gui/src/views_table.rs` + `crates/gui/src/views.rs` + `checks/gui-table-aa/**` only.
`app.rs` / `viewmodel.rs` / `tree.rs` / `theme.rs` / `screenshot.rs` (sibling AA-MERGE + KEVL) untouched,
`STATUS.md` untouched, no push. Clean-room: bahar se observation only, koi string/asset copy nahi.

## Item-wise before/after

### (a) `views_table.rs`: nayi pure `pad_table_row` spec helper
- Before: `table_column_widths` + `pad_table_cell` spec maujood, par row-level pad helper nahi tha.
- After: `pad_table_row(cells, widths) -> Vec<String>` — har width tak `pad_table_cell`,
  missing cell `-` (ragged rows align rahen), lamba text cap par kat-ta hai.
  `.get()`/saturating only, koi indexing/slicing nahi.
- Test: `pad_table_row_pads_and_marks_missing` (pad, ragged `-`, cap, khali widths).

### (b) `views.rs`: single-string row se proper `egui::Grid` columns
- Before: `paint_table` header ko single bordered string (`| a | b |` + `-` rule) aur
  har data row ko `ui.horizontal` me glyph + single bordered string me paint karta tha —
  columns char-padding se judte the, Grid cells nahi the.
- After: pure helpers `table_row_cells(text, widths)` (`split_table_cells` +
  `pad_table_row` reuse — bordered line ki padded cells, bina `|` borders ke;
  separation Grid spacing deta hai) aur `table_grid_columns(widths)` (`1 + len`,
  glyph column samet). `format_table_bordered_row` / `format_table_header_rule`
  shape unchanged (back-compat, purana test same values pass).
- Paint: header bar `egui::Grid::new("table_header_grid")` me (glyph placeholder +
  strong mono cells, phir `-` rule `indented_content` lead ke saath) — body se pehle
  fixed; body `egui::Grid::new("table_data_grid")` me ScrollArea window ke andar
  (`num_columns(table_grid_columns(&widths))`, `spacing([SECTION_GAP, TIGHT_GAP])`,
  har row me glyph cell + padded mono cells, `row.changed` par `CHANGED_FG`).
  Widths khali (bina `hdr` row) par purana single-string fallback (`label | text`).
- Reuse: `table_column_widths`, `SECTION_GAP`/`TIGHT_GAP`, `CONTENT_LEAD`,
  `CHANGED_FG`, glyph colors unchanged; koi naya token/color nahi.
- Tests: `table_row_cells_pad_to_widths_for_grid` (widths barkarar, spec-barabar,
  ragged `-`), `table_grid_columns_counts_glyph_plus_cells` (0→1, 1→2, 2→3).

## Gates
- `cargo test -p autocompare-gui`: 289 passed, 0 failed (naye 3 samet; pre-edit baseline 278).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0).
- `rustfmt --edition 2024 --check` (sirf OWNS files): clean (exit 0).
- Banned grep (non-test code): ZERO-BANNED (`unwrap()`/`expect()`/`panic!`/`todo!`/`unimplemented!`/`unreachable!`
  nahi; `unwrap_or`/`unwrap_or_default` allowed). `forbid(unsafe_code)` kept.
- Colors: NO-COLOR-TYPO-ADD (koi `from_rgb`/`from_rgba`/`Visuals::`/`.size()` add nahi).
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-table-aa/round-1`: RESULT PASS
  (after.png 9624B, magic-ok `89504e470d0a1a0a`, distinct 256).
- OWNS-clean: diff sirf `views.rs` + `views_table.rs` + `checks/gui-table-aa/**`;
  KEVL-CLEAN (sibling AA-MERGE ke staged `tree.rs`/`viewmodel.rs` ko chhua nahi —
  commit pathspec sirf OWNS paths par, taaki sibling ka staged kaam usi ka rahe).

Commit: local only, no push.
