# Wave AZ-TABLEWIRE report (P1 only: paint_table_keyed key-bar wiring)

## Scope
- OWNS: `crates/gui/src/views.rs` (P1 wiring) + `checks/gui-tablewire-az/**`.
- P1 caller threading: `crates/gui/src/app.rs:5163` 1-line borrow change (`&self.table_view` → `&mut self.table_view`, disjoint-field borrow, hex precedent `app.rs:4999` jaisa, koi logic change nahi) — bina iske `cargo check` fail (callee ab `&mut` mangta hai); taaki HEAD green rahe, ye line same commit me hai.
- KEVL `views_table.rs` / `tree.rs` / `views_hex.rs` / `screenshot.rs` untouched (read-only: `paint_table_key_bar:291`, `table_header_cells` caller `views.rs:1352`, hex `&mut` caller precedent `app.rs:4999`, grep-proof).
- `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).
- Colors FROZEN: koi naya hue/token nahi — callee ke andar, yahan koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi, koi `.size(` nahi.
- Sirf P1 — P2/P3/P4 mat chhua (`paint_table` signature, header/data Grid, painter overlays sab untouched, OPEN neeche).
- NO push (orchestrator pushes).

## Problem
- Table body Grid paint `views.rs` me hai (`paint_table_keyed:1398` status + positional note, `paint_table:1866` header Grid + data Grid) — par key-col toggle bar ka paint-adoption missing tha: `views_table::paint_table_key_bar` (AS wave, `#[allow(dead_code)]` test-only) ka koi caller nahi tha, isliye key select/deselect affordance + badge kabhi paint nahi hota tha.
- Signature `view: &TableKeyView` tha — bar ko toggle ke liye `&mut` chahiye (`toggle_key_column(col)` click par).

## Delta (views.rs, paint-only — koi headless helper nahi, koi test nahi)
- `paint_table_keyed` (~1398): signature `view: &TableKeyView` → `view: &mut TableKeyView`.
- Status line ke baad key-bar insert (unwrap NAHI — `table_header_cells` returns `Option<Vec<String>>`):
  `if let Some(cells) = table_header_cells(model).as_ref()` gate me `views_table::paint_table_key_bar(ui, view, cells)` (bar khud `TABLE_KEY_BAR_H` reserve + badge + positional note dikhata hai).
- Existing `positional_note` block rakha (honest note single wording `POSITIONAL_NOTE`, dono `.small().weak()`; duplicate-safai agle wave ka call).
- `paint_table(ui, model, pager)` call untouched (P3 OPEN).
- Doc-comment 1-line honest update (key-bar mention).
- `app.rs:5163` caller threading: `(&self.table_view, &mut self.table_pager)` → `(&mut self.table_view, &mut self.table_pager)` (disjoint fields, hex `(&mut self.hex_view, &mut self.hex_pager)` precedent jaisa).

## Tests (G3)
- Naya test koi nahi (paint-only wiring; headless geometry `tablecell_geo_*` AY me locked hai).
- Baseline (pre-P1, `main@3e4e5e7`): 561 passed, 0 failed.
- After P1: 561 passed, 0 failed (delta 0 — expected).
- Hisaab: AY-full 559 + sibling emptyhint-merge 2 = 561 baseline; meri 0 (paint caller threading).

## Gates
- G1 baseline: 561 passed, 0 failed (pre-P1 run, `main@3e4e5e7`).
- G2 OWNS: diff sirf `views.rs` (~8 lines) + `app.rs` (1-line borrow) + `checks/gui-tablewire-az/**`; OWNS-CLEAN + KEVL-CLEAN (views_table/tree/views_hex/screenshot/STATUS diff zero); NO-COLOR-TYPO-ADD (diff me `from_rgb`/`from_rgba`/`Visuals`/`SetVisuals`/`.size(` zero).
- G3 full: `cargo test -p autocompare-gui` 561 passed, 0 failed.
- G4 MS rust: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` exit 0 (zero warnings); `rustfmt --check` clean dono touched files; added lines me ZERO unwrap/expect/panic/todo; `forbid(unsafe_code)` kept.
- G5 harness: `run-geo-typo.sh checks/gui-tablewire-az/round-1` RESULT PASS; after.png 9627 bytes, magic `89504e470d0a1a0a` ok, distinct 256.
- G6: ye REPORT + GATES.md + round-1 artifacts; conventional commit; NO push.

## Agle views.rs waves ke liye OPEN (is wave me kuch nahi chhua)
- P2 — header grid (`views.rs:1973-1990`): `egui::Grid::new("table_header_grid")` → `views_table::TABLE_CELL_HEADER_GRID_ID`; spacing → `[TABLE_GRID_COL_GAP, TABLE_GRID_ROW_GAP]`; glyph `CONTENT_LEAD` → `TABLE_CELL_GLYPH_PAD`.
- P3 — data grid + `paint_table` signature (`views.rs:1866, 2022-2052`): `paint_table(ui, view, model, pager, find_ctx, &style)` threading; indexed cell loop + `paint_table_data_cell`; find-source `table_model_text_rows` → `table_find_match_rows(&data_cells, …)`.
- P4 — painter overlays (`paint_table` end): `table_key_box_rects` + `table_find_underline_rects_for_cell` se `ui.painter()` boxes/underlines (views_table.rs font-free rehta hai).
- Note-dup safai: bar-wala vs outer `positional_note` — duplicates lage to ek rakho (agle wave ka call).
