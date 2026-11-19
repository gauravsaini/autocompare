# Wave BA-TABLEP234 report (P2 header-grid: Grid ID + spacing + glyph single-source)

## Scope
- OWNS: `crates/gui/src/views.rs` + `checks/gui-tablep234-ba/**` ONLY.
- KEVL: `views_table.rs` read-only (tokens wahi se liye — koi edit nahi), `views_hex.rs` / `app.rs` / `viewmodel.rs` / `screenshot.rs` untouched.
- `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).
- Sibling wave diff (`crates/gui/src/theme.rs` typotokens + `checks/gui-typotokens-ba/`) pre-existing tha — haath nahi lagaya, stage nahi kiya.
- Colors FROZEN: koi naya hue/token nahi, koi `.size(` nahi. NO push (orchestrator pushes).

## Problem
- AY wave ne cell-grid geometry + paint helpers `views_table.rs` me single-source kar diye (`TABLE_CELL_HEADER_GRID_ID`, `TABLE_GRID_COL_GAP`/`TABLE_GRID_ROW_GAP`, `TABLE_CELL_GLYPH_PAD` — `SECTION_GAP`/`TIGHT_GAP`/`CONTENT_LEAD` mirrors, same values), par `views.rs` header Grid purane literals par tha (`"table_header_grid"`, `[SECTION_GAP, TIGHT_GAP]`, `CONTENT_LEAD`) — drift risk tha.
- AY REPORT §P2 ne exact wiring patch spec diya tha (Grid ID + spacing + glyph swap; cells loop + rule line untouched; P1/P3/P4 OPEN).

## Delta (views.rs, P2-only — values same, single-source)
- Header Grid (`paint_table`, `if let Some(cells)` block):
  + `egui::Grid::new("table_header_grid")` → `egui::Grid::new(views_table::TABLE_CELL_HEADER_GRID_ID)` (`"table_cell_header_grid"`).
  + `.spacing([SECTION_GAP, TIGHT_GAP])` → `.spacing([views_table::TABLE_GRID_COL_GAP, views_table::TABLE_GRID_ROW_GAP])` (8.0/4.0, same).
  + glyph `RichText::new(CONTENT_LEAD)` → `RichText::new(views_table::TABLE_CELL_GLYPH_PAD)` (same `"  "`).
  + Untouched: `.num_columns(table_grid_columns(&widths))` (shape `table_cell_grid_columns` jaisa), cells loop (`pad_table_row` + `.monospace().strong()`), rule line (`format_table_header_rule` + `.monospace().weak()`).
- Data Grid (`"table_data_grid"` + uska spacing/glyph/cells loop) untouched — P3 ke liye OPEN.
- `paint_table_keyed` signature/status-note untouched — P1 ke liye OPEN. Painter overlays nahi — P4 ke liye OPEN.
- Naya test (G3): `tablep234_header_grid_tokens_single_source` (Grid ID literal + COL_GAP==SECTION_GAP + ROW_GAP==TIGHT_GAP + GLYPH_PAD==CONTENT_LEAD==`"  "`).

## Tests (G3)
- Naya 1 (`cargo test -p autocompare-gui tablep234` = 1 passed): `tablep234_header_grid_tokens_single_source`.
- Full: `cargo test -p autocompare-gui` 566 passed, 0 failed. Hisaab: baseline 565 + meri 1 (baseline me sibling waves ke tests samet the).

## Gates
- G1 baseline: 565 passed, 0 failed (pre-edit run, `round-1/baseline-tests.out`).
- G2 OWNS: mera diff sirf `views.rs` (header-grid 3-token swap + 1 test) + `checks/gui-tablep234-ba/**`; `views_table.rs`/`views_hex.rs`/`screenshot`/`app` diff zero; `STATUS.md` untouched; NO-COLOR-TYPO-ADD (diff me `from_rgb`/`from_rgba`/`Visuals`/`SetVisuals`/`.size(` zero); sibling `theme.rs` diff stage se bahar.
- G4 MS rust: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` exit 0 (zero warnings); `rustfmt --edition 2024 --check crates/gui/src/views.rs` clean (sirf OWNS file); non-test me ZERO unwrap/expect/panic/todo; `forbid(unsafe_code)` kept.
- G5 harness: `run-geo-typo.sh checks/gui-tablep234-ba/round-1` RESULT PASS; after.png magic `89504e470d0a1a0a` ok (size/distinct report me).
- G6: ye REPORT + GATES.md + round-1 artifacts; conventional commit, NO push.

## P1/P3/P4 OPEN (is wave me nahi — orchestrator ke paas)
- P1 `paint_table_keyed` signature + key-bar wiring, P3 data-grid cells `paint_table_data_cell` loop + find-source switch, P4 painter overlays — sab AY REPORT §P2-P4 spec ke mutabiq OPEN hain.
