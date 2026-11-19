# Wave L Parity Report — hex-grid + table-grid + folder-column

Date: 2026-11-19 UTC. Scope: sirf owned files
(`crates/gui/src/views_hex.rs`, `views_table.rs`, `tree.rs`,
`checks/parity-l/**`). `views.rs`/`app.rs`/theme/gutter/home/viewmodel/main/screenshot,
STATUS.md ko chhua nahi. No push.

## 1. Gaps picked (sources: checks/local-run-exq/REPORT.md + parity-hex-table/REPORT.md)

- Gap 2 (local-run-exq:36-37): hex tab me byte grid nahi; sirf hunk summary lines.
- Gap 3 (local-run-exq:38): table tab me column grid nahi; har row ek string.
- Gap 6 (local-run-exq:41): folder tab me size/time side-by-side columns nahi.
- Wave H state (parity-hex-table/REPORT.md): dono models headless wired hain
  (`app.rs` me `hex_view`/`table_view`), paint abhi bhi summary-lines hai.

## 2. Per-feature status

### F1 hex-grid — FIXED (model slice; paint OPEN, orchestrator ke paas)
- Ruler + row math: `HEX_BYTES_PER_ROW` (views_hex.rs:380), `format_hex_offset`
  (views_hex.rs:505, `0x00000010` style), `hex_row_count` (:512),
  `hex_row_start` (:525). Tests: `hex_grid_ruler_and_row_math` (:763).
- Per-byte cursor + nibble: `HexCursor` (:388: offset/high_nibble/total_len),
  row/col (:416/:421), `from_row_col` (:427), left/right/up/down (:~475/:~486,
  16-step), `toggle_nibble`, `clamp_len`. Test `hex_cursor_moves_over_grid` (:778).
- Hunk-aware row flag: `hex_row_has_diff` (:533, `[offset, offset+max)` range).
  Test `hex_row_diff_flag_follows_hunks` (:818): 48-byte pair me diff offset 20
  sirf row 1 (16-31) par flag.
- WIRING (dusri wave/app owner kare): har visible row = `hex_row_start(row)` se
  16-byte slice + `format_hex_offset` ruler + `hex_row_has_diff` highlight.

### F2 table-grid — FIXED (model slice; paint OPEN, orchestrator ke paas)
- Widths: `TABLE_MAX_COL_CHARS` (views_table.rs:238, 32),
  `TABLE_MIN_COL_CHARS` (:243, 3), `table_column_widths` (:305, max(header,cells),
  floor+cap, ragged rows ignore). Test `column_widths_span_header_and_cells_with_cap` (:558).
- Cells: `pad_table_cell` (:336, char-based pad/cap), `format_table_header` (:353),
  `table_cell_display` (:366, missing cell `-`). Test `padded_cells_keep_columns_aligned` (:586).
- Nav: `TableCellCursor` (:249) + clamp/up/down/left/right, `table_cell_display`.
  Test `cell_cursor_nav_stays_inside_grid` (:600).
- WIRING: paint har visible row ko `table_column_widths` + `pad_table_cell` se
  columns me lay out kare, `TableCellCursor` se nav kare.

### F3 folder-column — FIXED (model slice; paint OPEN, agent A ke paas)
- Pehle se tha: `folder_column_defs` (tree.rs:456, Path flex + 4 fixed),
  `folder_cell_text` (:488, missing `-`). Naya is round me:
- `pad_folder_cell` (tree.rs:501, char pad/cap), `folder_header_line` (:517,
  `Path | L size | ...`), `folder_format_row` (:532, glyph prefix + padded sides).
  Test `folder_lines_keep_side_columns_aligned` (:928): header/row dono 5 parts,
  orphan row `L only-left.txt` se start.
- WIRING (views.rs owner): har visible node = `folder_format_row` + Path cell par
  `tree_indent_for_depth(node.depth)` offset.

## 3. Cases (checks/parity-l/cases/, khud ke banaye sample pairs)

- `hex-left.bin` / `hex-right.bin`: 64 bytes `0..64`, right me offset 20-21 par
  2-byte diff (`63 64`). Expect: first_diff 20, row 1 flag, baaki rows clean.
  Python sanity: first_diff 20, count 2, row 1 — PASS.
- `table-left.csv` / `table-right.csv`: header ID,Price,Qty; key ID; row 1 modified
  (9.99 vs 10.04), 3 left-only, 4 right-only, 2 matching. Sanity: keys L [1 2 3]
  R [1 2 4], modified [1] — PASS.
- `folder-left/` / `folder-right/`: same.txt (same), mod.txt (dono taraf alag),
  orphan-left.txt / orphan-right.txt (orphans), sub/nested.txt (same nested).
  Expect: counts 4 entries/side-ish, `folder_format_row` orphan par `-` columns.

## 4. Metrics + gates

- `cargo test -p autocompare-gui`: owned 56 pass (views_hex 17, views_table 14,
  tree 25 — pehle 12+10+? se +7 naye). Full suite: 187 pass + 3 fail jo OUT OF
  SCOPE hain (parallel waves ke: `views.rs` geometry x2 GEO wave, `theme.rs` typo x1
  TYPO wave — unki files ko chhua nahi, isliye fix nahi kar sakta).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (dead_code
  ke liye naye items par `#[allow(dead_code)]`, existing test-only pattern jaisa;
  koi indexing: sab `.get()`; koi unwrap/expect/panic/todo non-test me nahi).
- `cargo fmt -p autocompare-gui -- --check`: clean.
- PNG: `checks/parity-l/round-L-1/after.png`, 9914 bytes, magic ok, distinct 256,
  960x600 RGBA — manual check RESULT PASS (harness ka python step same kehta).
  Harness poora PASS nahi de sakta jab tak parallel waves ke fails hain — ye unke
  scope ka hai, mere owned gates green hain.
- Rounds used: 1/6. MS guidelines: `forbid(unsafe_code)` barkarar, saturating ops,
  let-else/`.get()` har jagah.

## 5. OPEN (agle waves ke liye)

- Paint wiring teeno ka (app.rs/views.rs owners): F1/F2/F3 ke WIRING notes upar.
- Gap 4/5/7/12-15 (picture overlay, gutter click-jump, merge per-hunk, find box,
  paging, progress) is wave ke bahar — PARITY age wale rounds ya alag waves dekhen.
- Full-suite 3 fails parallel GEO/TYPO waves fix karenge; mera commit unhe nahi chhuta.
