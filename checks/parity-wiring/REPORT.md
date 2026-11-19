# Parity Wiring — Wave I Agent A Report

Scope: sirf owned files — `crates/gui/src/main.rs`, `app.rs`, `views.rs`, `checks/parity-wiring/**`.
`views_hex.rs`, `views_table.rs`, `viewmodel.rs`, `tree.rs`, `gutter.rs`, `theme.rs`, `STATUS.md` ko chhua nahi.

## 1. Counts

`cargo test -p autocompare-gui`: **172 passed, 0 failed.**

| Hissa | Tests |
|---|---|
| app | 21 (HEAD 15 + wiring 6) |
| views | 22 (HEAD 20 + wiring 2) |
| views_hex (Wave H, mods se wired) | 12 |
| views_table (Wave H, mods se wired) | 10 |
| viewmodel / tree / gutter / copy / home / theme / take / screenshot | 107 (HEAD, parallel waves) |
| **Total** | **172** |

Baseline math: wiring shuru me 151 tha (125 purana + 22 Wave H + 4 parallel gutter).
Wiring ne +8 jode (app me 6, views me 2). Baaki +13 HEAD me parallel waves se aaya,
mere commit me nahi hai. Task ka 147 target Wave H wiring se pura hua (125 + 22);
172 me HEAD ka parallel kaam bhi shamil hai.

## 2. Wiring points (file:line)

- `main.rs:24-25` — `mod views_hex; mod views_table;` (INTEGRATION.patch via `git apply`).
- `app.rs:51` — `CompareResult::Hex` ab `(Vec<String>, HexCompareView)` tuple deta hai;
  `CompareResult::Table` ab `(TableModel, TableKeySummary)` tuple deta hai.
- `app.rs:73,76-77` — state: `hex_view`, `table_view`, `table_tolerance_text`.
- `app.rs:856` `run_hex` — `compare_binary_files_aligned` + `hex_view.mode.to_alignment()`,
  `HexCompareView::from_comparison` se view, `build_hex_summary` se legacy lines.
- `app.rs:926` `run_table` — `compare_table_files` (row paint) + `compare_table_files_by_key`
  with `table_view.to_key_options()` (summary via `summarize`), positional flag ke saath.
- `app.rs:508` `hex_over_cap_view` — `HexCompareView::is_over_cap` se honest cap view.
- `app.rs:423-499` setters: `set_hex_mode`, `hex_select_next/prev`, `hex_goto_first_diff`,
  `toggle_table_key_column`, `clear_table_keys`, `set_table_tolerance/ignore_case/trim_space`.
- `app.rs:1115,1154` completion handlers tuple split karke `hex_model`/`hex_view`
  aur `table_model`/`table_view.summary` me daalte hain.
- `app.rs:688,694` `status_line`: hex → `hex_view.status_line()`, table → rows + `summary.total()` + `table_view.status_line()`.
- `app.rs:1602` Hex tab: alignment radios, `paint_hex`; `app.rs:1640` Table tab:
  key cols 0-7, tolerance presets + custom text field, case/trim checkboxes, `paint_table_keyed`.
- Tuple mapping (nudge #1): paint ko `.0` (lines/rows) milta hai paint ke liye,
  `.1` (view/summary) status line ke liye — `views::paint_hex(ui, &mut hex_view, lines)`,
  `views::paint_table_keyed(ui, &table_view, model)`.
- `views.rs:320` `format_hex_hunk_line`, `:330` `parse_tolerance_text` (custom field me real use),
  `:340` `paint_hex` (hunk grid + Prev/Next + First diff + legacy lines),
  `:389` `paint_hex_over_cap`, `:396` `paint_table_keyed`.

## 3. Gates

- `cargo test -p autocompare-gui`: green, 172/172.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: **owned files clean**
  (struct-update syntax, real-use fixes, `compare_binary_files` import hataya).
  Baaki 5 errors sirf `views_hex.rs`/`views_table.rs` me hain (Wave H test-helpers:
  `HEX_HUNK_LEN`, `from_label`, `build_from_slices`, `selected_hunk`, `chunk_run`,
  `suggest_for`, `apply_suggestion`, `to_compare_options`, `cells_equal`, `refresh`) —
  bin target me dead hain, files mere scope se bahar, isliye chhue nahi.
  `is_over_cap` aur `TableKeySummary::total` ko real use me laa ke list chhoti ki.
- `cargo fmt -p autocompare-gui`: clean; diff me sirf 3 owned files.

## 4. 6 limitations (honest)

1. Key suggestion auto nahi: `suggest_for` ko parsed `Table` chahiye jo engine wapas nahi deta;
   key cols 0-7 manual toggle + Clear hi wired hai.
2. Key column toggle cap 8 hai (UI bound); engine-side koi cap nahi.
3. Table rows abhi bhi positional `build_table_rows` se paint hote hain; keyed summary
   alag se judta hai — per-key row paint nahi.
4. Hex paint me byte grid nahi: hunk list (offset + counts) + legacy summary lines;
   offset ruler, per-byte cursor, nibble nav nahi.
5. `hex_select_next/prev` status likhte hain par paint ke Prev/Next buttons view ko
   seedha drive karte hain; footer `status_line` se live sync hota hai.
6. 16 MiB se bade inputs par aligned compare skip + over-cap view; unbounded
   positional fallback (`compare_binary_files`) wiring me nahi hai.
