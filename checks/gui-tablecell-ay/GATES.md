# Wave AY-TABLECELL — GATES (table cell geometry: padded-cell rects + key-col boxes + find underlines)

OWNS: `crates/gui/src/views_table.rs` + `checks/gui-tablecell-ay/**` ONLY.
KEVL: `views.rs` / `views_hex.rs` / `app.rs` / `viewmodel.rs` / `screenshot.rs` KO HAATH MAT LAGANA (read bhi minimal rakhi — sirf `paint_table_keyed:1398` + `paint_table:1866` grid region + precedent signatures, grep-proof).
`checks/opencode-waves/STATUS.md` KO HAATH MAT LAGANA (orchestrator-owned).
Colors FROZEN (koi naya hue/token nahi — sirf caller ke apne `fg/bg` reuse + `TABLE_FIND_UNDERLINE_W` underline motai reuse, koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi, koi `.size(` nahi).
Geometry first, typography second (rect math pehle — koi font measure nahi, `char_w` caller deta hai).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. Repo ke bahar koi copy/sandbox nahi. `git stash/push` kabhi nahi.

Scope: table body Grid paint `views.rs` me hai (`paint_table_keyed:1398` status + positional note, `paint_table:1866` header Grid + data Grid `table_grid_columns`/`table_row_cells` se). Is wave ka delta `views_table.rs` me hai: PURE cell-geometry helpers (egui ctx nahi) + uncommitted cell-grid paint section (adhura kaam continue) — padded-cell rect math, key-col box rect, find-underline rects. Grid paint wiring OPEN hai (orchestrator agle `views.rs` wave me thread karega — naye paint + geometry helpers `#[allow(dead_code)]` test-only precedent par, caller signature stable).

## G1 — Baseline green (before new geometry)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `556 passed; 0 failed` (recorded 2026-11-20, `main@2449f1f`, pre-geometry run — cell-grid paint section already uncommitted tha)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + STATUS + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views_table.rs|checks/gui-tablecell-ay/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|views_hex.rs|screenshot|app.rs|STATUS.md" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_table.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: working tree me sirf `views_table.rs` mera diff hai (cell-grid paint continue + nayi geometry); `views.rs`/`views_hex.rs`/`screenshot`/`app` diff zero (read-only); `STATUS.md` untouched; stage sirf OWNS paths; KEVL-CLEAN; NO-COLOR-TYPO-ADD
- EVIDENCE: `round-1/owns-diff.out`

## G3 — Pure cell geometry, unit-tested (egui ctx nahi)

- CHECK: `cargo test -p autocompare-gui tablecell_geo`
- EXPECT: green, 3 naye tests (sab `tablecell_geo_` prefix):
  + `tablecell_geo_cell_rect_maps_col_row_with_gaps` (origin + widths + gaps se x/y, range-bahar/kharab rhythm par `None`, panic nahi)
  + `tablecell_geo_key_box_covers_key_cols_only` (3-row box `h = 3*row_h + 2*row_gap`, khali/range-bahar `None`, view order barkarar)
  + `tablecell_geo_find_underline_clamps_to_cell` (bottom strip `TABLE_FIND_UNDERLINE_W`, cap clamp, bahar/ulta/kharab `None`, list order + skip)
- WHAT: `views_table.rs` me pure fns (koi egui ctx/import nahi, koi indexing nahi `.get()` only, saturating arithmetic, non-finite guards):
  + `TableCellRect { x, y, w, h }` (px box, `Copy`, `PartialEq`)
  + `TableCellGrid { origin_x, origin_y, row_h, col_gap, row_gap }` (rhythm bundle — call-sites 4 args, clippy `too_many_arguments` bina `allow` ke clean)
  + `table_cell_rect(grid, col, row, col_widths_px) -> Option<TableCellRect>` (x = sum(widths[0..col]) + col*gap, y = origin + row*(row_h+row_gap))
  + `table_key_col_box_rect(grid, col, col_widths_px, total_rows) -> Option<TableCellRect>` (x/w head cell se, h = rows*row_h + (rows-1)*row_gap)
  + `table_key_box_rects(view, grid, col_widths_px, total_rows) -> Vec<TableCellRect>` (key order me, range-bahar skip)
  + `table_find_underline_rect(cell, start, end, width_chars, char_w) -> Option<TableCellRect>` (`table_find_span_in_padded` clamp reuse, cell-right clamp, y = bottom - `TABLE_FIND_UNDERLINE_W`)
  + `table_find_underline_rects_for_cell(cell, spans, width_chars, char_w) -> Vec<TableCellRect>` (span order, bahar spans skip, kharab `char_w` par empty)
- EVIDENCE: `round-1/tablecell_geo-tests.out`

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views_table.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_table.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views_table.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code (`unwrap_or` allowed — banned sirf `unwrap()`/`expect()` hain), `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()` only; saturating arithmetic; f32 non-finite guards)
- NOTE: naye pub geometry + paint helpers `#[allow(dead_code)]` test-only precedent par (file me `table_column_widths`/`TABLE_MAX_COL_CHARS` wahi pattern) — Grid paint wiring orchestrator ke paas OPEN hai
- EVIDENCE: `round-1/clippy.out`, `round-1/fmt.out`

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablecell-ay/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tablecell-ay/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- EVIDENCE: `round-1/geo-typo-report.md`, `round-1/after.png`

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablecell-ay/REPORT.md checks/gui-tablecell-ay/round-1/after.png checks/gui-tablecell-ay/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_table.rs checks/gui-tablecell-ay/GATES.md checks/gui-tablecell-ay/REPORT.md checks/gui-tablecell-ay/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed), sirf OWNS paths staged, working tree me meri files clean after commit, NO push
