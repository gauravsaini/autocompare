# Wave Z-HEX — GATES (Gap #2: hex byte-grid paint, teen-column grid)

OWNS: `crates/gui/src/views_hex.rs` + `crates/gui/src/views.rs` + `checks/gui-hex-z/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `theme.rs` / `screenshot.rs` / `tree.rs` mat chhoona (read-only, grep-proof only).
Colors FROZEN (koi hue change nahi — sirf existing palette/tokens, koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: `views_hex.rs` grid math (address + 16-byte hex + ASCII, midline gap — `HEX_BYTES_PER_ROW`,
`format_hex_offset`, `hex_row_start`, `hunk_covers_offset` path) ko `views.rs` hex painter window
(`paint_hex` byte-grid section) me teen-column `egui::Grid` me paint karo. Monospace alignment,
GEO tokens reuse (`SECTION_GAP` / `TIGHT_GAP` spacing, `text_row_height` + `row_height_for_line` rhythm).
Single-string `format_hex_grid_row` shape same rahega (back-compat wrapper), nayi pure helper
`format_hex_grid_cells` (addr, hex, ascii) Grid cells degi.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `275 passed; 0 failed` (recorded 2026-11-20, `main@3e33d72`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views_hex|views).rs|checks/gui-hex-z/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|theme.rs|screenshot.rs|tree.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/views_hex.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views_hex.rs` + `views.rs` + `checks/gui-hex-z/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Teen-column grid, pure cells + Grid paint, unit-tested

- CHECK: `cargo test -p autocompare-gui hex_grid`
- EXPECT: green, kam se kam ye tests:
  + `hex_grid_row_marks_covered_bytes_with_midline_gap` (purana, same shape pass)
  + `hex_grid_rows_for_hunk_align_and_cap` (purana, same values pass)
  + `hex_grid_cells_split_address_hex_ascii` (naya: addr = `format_hex_offset`,
    hex me 2 `??` + midline gap double-space, ascii `|    ??          |`,
    empty row same width, `format_hex_grid_row` == cells join)
- WHAT: `views.rs` me nayi pure `format_hex_grid_cells(row_start, hunks) -> (String, String, String)`
  (address + hex-part + ascii-part, `HEX_BYTES_PER_ROW`/`format_hex_offset`/`hunk_covers_offset` reuse,
  midline gap 8 ke baad extra space); `format_hex_grid_row` wahi cells ko `"  "` + `" |"` se jodta hai
  (koi shape change nahi); `paint_hex` byte-grid section `for row { ui.label }` se
  `egui::Grid::new("hex_byte_grid").num_columns(3).spacing([SECTION_GAP, TIGHT_GAP])` par jata hai —
  har row me 3 monospace cells (addr, hex, ascii), GEO tokens reuse, koi naya token/color nahi

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs crates/gui/src/views_hex.rs; echo fmt:$?`
  (sirf OWNS files; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs crates/gui/src/views_hex.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views_hex.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-hex-z/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-hex-z/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-hex-z/REPORT.md checks/gui-hex-z/round-1/after.png checks/gui-hex-z/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs crates/gui/src/views_hex.rs checks/gui-hex-z/GATES.md checks/gui-hex-z/REPORT.md checks/gui-hex-z/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
