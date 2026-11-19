# Wave AD-COLS — GATES (Gap #6: folder size/time column paint, aligned columns)

OWNS: `crates/gui/src/tree.rs` + `crates/gui/src/views.rs` + `checks/gui-cols-ad/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `views_table.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only, grep-proof only).
Colors FROZEN (koi hue change nahi — sirf existing palette/tokens, koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: `tree.rs` col defs + cell text model (`folder_column_defs` + `folder_cell_text` + `pad_folder_cell`
+ `folder_header_line` + `folder_format_row`, char-width, fixed side widths, missing-side `-`)
ko `views.rs` folder paint me size/time columns ke saath paint karo
(rel_path + size + time aligned columns, mono). Single-list (`paint_folder`) aur
dual-tree (`paint_folder_dual` + `dual_left_line`/`dual_right_line`) dono me jahan trivial ho.
Back-compat single-string joins (`folder_columns`, `folder_format_row`) shape same rahenge (logs/tests);
nayi pure helpers Grid/row cells dengi. GEO tokens reuse (`SECTION_GAP`/`TIGHT_GAP`,
`row_height_for_line`/`text_row_height`, `tree_indent`), typo mono rows + small weak notes,
koi naya token/color nahi. Clean-room: Beyond Compare bahar se observe only, apne shabdon me.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `301 passed; 0 failed` (recorded 2026-11-20, `main@22f8848`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(tree|views).rs|checks/gui-cols-ad/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|views_table.rs|views_hex.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/tree.rs crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `tree.rs` + `views.rs` + `checks/gui-cols-ad/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Aligned size/time columns, pure cells + paint, unit-tested

- CHECK: `cargo test -p autocompare-gui folder_ && cargo test -p autocompare-gui dual_`
- EXPECT: green, kam se kam ye tests:
  + `folder_columns_match_the_cli_shape` (purana, same shape pass — `views.rs::folder_columns` back-compat)
  + `folder_lines_keep_side_columns_aligned` (purana, `tree.rs` header/format shape pass)
  + `dual_side_cells_keep_missing_sides_as_dash` (purana, dashes pass)
  + `dual_side_cells_are_fixed_width` (naya, `tree.rs`: `dual_left_cell`/`dual_right_cell`
    `pad_folder_cell` se `FOLDER_SIZE_CHARS`/`FOLDER_TIME_CHARS` tak padded, missing `-` bhi padded)
  + `folder_row_cells_keep_sides_fixed_width` (naya, `views.rs`: path raw + dir mark,
    sides `pad_folder_cell` widths tak, orphan `-` padded)
  + `folder_grid_columns_counts_glyph_plus_five` (naya: `1 + folder_column_defs().len()` = 6,
    khali nahi hota)
  + `folder_header_cells_match_defs` (naya: 5 cells, Path raw, sides padded header labels)
  + `dual_lines_use_padded_side_cells` (naya: `dual_left_line`/`dual_right_line` me padded
    size/time cells, dir mark `src/`, orphan `-`)
- WHAT: `tree.rs` me `dual_left_cell`/`dual_right_cell` `pad_folder_cell` reuse karte hain
  (har side fixed width tak pad, missing `-` bhi pad — `.get()`/saturating only, koi indexing nahi);
  `views.rs` me nayi pure `folder_header_cells() -> Vec<String>`
  (defs se: Path raw, sides padded), `folder_row_cells(row) -> Vec<String>`
  (path+dir-mark raw + 4 padded sides) aur `folder_grid_columns() -> usize`
  (`1 + defs.len()`, glyph column samet); `folder_columns`/`folder_format_row`
  shape unchanged (back-compat); `paint_folder` header line (`folder_header_line`, mono weak)
  ke baad har row me glyph cell + path + 4 padded mono side cells
  (`SECTION_GAP` spacing, `row_height_for_line(text_row_height)` rhythm,
  `tree_indent` offset Path cell par); `paint_folder_dual` dono panes me
  path + padded size/time mono cells (left pane L-sides, right pane R-sides,
  pane headers weak mono); widths/missing me `-` padded, orphan rows align rahen

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/tree.rs crates/gui/src/views.rs; echo fmt:$?`
  (sirf OWNS files; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/tree.rs crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/tree.rs crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-cols-ad/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-cols-ad/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-cols-ad/REPORT.md checks/gui-cols-ad/round-1/after.png checks/gui-cols-ad/round-1/geo-typo-report.md`
  + `git add crates/gui/src/tree.rs crates/gui/src/views.rs checks/gui-cols-ad/GATES.md checks/gui-cols-ad/REPORT.md checks/gui-cols-ad/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
