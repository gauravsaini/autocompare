# Wave AA-TABLE — GATES (Gap #3: table grid + header paint, proper columns)

OWNS: `crates/gui/src/views_table.rs` + `crates/gui/src/views.rs` + `checks/gui-table-aa/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `tree.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only, grep-proof only).
Colors FROZEN (koi hue change nahi — sirf existing palette/tokens, koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: `views_table.rs` header/cell spec (`table_column_widths` + `pad_table_cell`, char-width, min-floor, max-cap,
missing-cell `-`) ko `views.rs` table window me header bar + cell borders/padding ke saath paint karo.
Single-string bordered row se proper `egui::Grid` columns tak: header bar (strong monospace + `-` rule,
`indented_content` lead) + body grid (glyph column + padded mono cells, `SECTION_GAP`/`TIGHT_GAP` spacing,
`text_row_height` + `row_height_for_line` rhythm). Back-compat single-string join
(`format_table_bordered_row` + `format_table_header_rule`) shape same rahenge (logs/tests);
nayi pure helpers Grid cells dengi. GEO tokens reuse, koi naya token/color nahi.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `278 passed; 0 failed` (recorded 2026-11-20, `main@3548ca1`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views_table|views).rs|checks/gui-table-aa/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/views_table.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views_table.rs` + `views.rs` + `checks/gui-table-aa/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Proper-column grid, pure cells + Grid paint, unit-tested

- CHECK: `cargo test -p autocompare-gui table_`
- EXPECT: green, kam se kam ye tests:
  + `table_cells_split_and_border_with_header` (purana, same shape pass)
  + `pad_table_row_pads_and_marks_missing` (naya, views_table.rs: widths tak pad, missing `-`, lamba text cap, khali widths khali)
  + `table_row_cells_pad_to_widths_for_grid` (naya, views.rs: split + pad join == `format_table_bordered_row` inner cells, ragged me `-`, mono widths barkarar)
  + `table_grid_columns_counts_glyph_plus_cells` (naya: `1 + widths.len()`, khali widths par 1)
- WHAT: `views_table.rs` me nayi pure `pad_table_row(cells, widths) -> Vec<String>`
  (har width tak `pad_table_cell`, missing cell `-`, `.get()`/saturating only, koi indexing nahi);
  `views.rs` me nayi pure `table_row_cells(text, widths) -> Vec<String>`
  (`split_table_cells` + `pad_table_row` reuse) aur `table_grid_columns(widths) -> usize`
  (`1 + len`, glyph column samet); `format_table_bordered_row`/`format_table_header_rule`
  shape unchanged (back-compat); `paint_table` header bar (strong mono + `-` rule,
  `indented_content` lead) ke baad body `egui::Grid::new("table_data_grid")`
  me paint hota hai — `num_columns(table_grid_columns(&widths))`,
  `spacing([SECTION_GAP, TIGHT_GAP])`, har row me glyph cell + padded mono cells
  (`CHANGED_FG` jab `row.changed`), `row_height_for_line(text_row_height)` rhythm,
  header Grid (`table_header_grid`, glyph placeholder + strong cells) body se pehle fixed rehta hai;
  widths khali (bina `hdr` row) par purana single-string fallback (`label | text`)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs crates/gui/src/views_table.rs; echo fmt:$?`
  (sirf OWNS files; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs crates/gui/src/views_table.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views_table.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-table-aa/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-table-aa/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-table-aa/REPORT.md checks/gui-table-aa/round-1/after.png checks/gui-table-aa/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs crates/gui/src/views_table.rs checks/gui-table-aa/GATES.md checks/gui-table-aa/REPORT.md checks/gui-table-aa/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
