# Wave BQ-TABLE — GATES (livecomp-BM TABLE G-B1 + G-B2 + G-B3: header row + compact toolbar + Grid)

OWNS: `crates/gui/src/views_table.rs` + `checks/gui-table-bq/**` ONLY.
KEVL read-only (grep-proof only, kabhi edit nahi): `views.rs`, `theme.rs`, `gutter.rs`,
`home.rs`, `tree.rs`, `viewmodel.rs`, `app.rs`, `screenshot.rs`,
`checks/opencode-waves/STATUS.md` (MANA), `checks/gui-gutter-bq/`.
Colors FROZEN (koi hue change nahi — koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi,
koi paint `.size()` override nahi; typo sirf TypoRole alias reuse).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: `views_table.rs` me header-row + compact-toolbar paint pieces
(`paint_table_header_row`, `paint_table_toolbar`, `format_table_page_label`,
`table_header_text_role`) taaki header data-row na rahe (`=` marker / dashed rule nahi),
Find + nav duplicate rows ek compact toolbar me aaye, cells Grid columns me align rahen
(`TABLE_CELL_GRID_ID` reuse, header ka apna `TABLE_CELL_HEADER_GRID_ID`).
Caller wiring orchestrator ke paas rehti hai (signatures stable, `views.rs` untouched);
naye paint fns test-only until wiring lands (file convention), pure helpers unit-tested.
GEO tokens reuse, koi naya token/color nahi.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `607 passed; 0 failed` (recorded 2026-11-20, `main@3007170`)
- EVIDENCE: pre-edit tail (koi round dir nahi — baseline shell output)

## G2 — OWNS-only diff, KEVL + colors + typo-adds untouched

- CHECK: `git status --short && git diff HEAD --stat`
  + `git diff HEAD --name-only -- crates/gui/src/views.rs crates/gui/src/theme.rs crates/gui/src/gutter.rs crates/gui/src/home.rs crates/gui/src/tree.rs crates/gui/src/viewmodel.rs crates/gui/src/app.rs crates/gui/src/screenshot.rs | grep . && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff HEAD -- checks/opencode-waves/STATUS.md checks/gui-gutter-bq/ | grep . && echo MANA-TOUCHED || echo MANA-CLEAN`
  + `git diff HEAD -- crates/gui/src/views_table.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: sirf `views_table.rs` + `checks/gui-table-bq/**` change hote hain
  (sibling `views.rs`/`checks/gui-gutter-bq/` workdir me dirty reh sakta hai — parallel wave;
  commit pathspec sirf OWNS paths par taaki sibling ka kaam usi ka rahe);
  KEVL-CLEAN (mere diff me); MANA-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Header row + compact toolbar + Grid, pure cells + Grid paint, unit-tested

- CHECK: `cargo test -p autocompare-gui table_`
- EXPECT: green, kam se kam ye tests:
  + `table_header_role_reuses_mono_grid_content` (naya: role == Mono == content_grid, is_mono, zoomed size == TYPO_MONO_SIZE, grid ids distinct)
  + `table_page_label_mirrors_pager_state` (naya: 25 rows/10pp → 1/3, 2/3, 3/3 showing 5; empty → 1/1 showing 0)
  + `table_toolbar_defaults_quiet_and_reuses_find_geometry` (naya: default action sab false; BAR_H == KEY_BAR_H, NEEDLE_W 120, BADGE_PAD reuse)
  + purane `tablecell_grid_columns_counts_glyph_plus_data`, `pad_table_row_pads_and_marks_missing` same pass
- WHAT: `table_header_text_role() -> TypoRole` (Mono const fn, gutter/hex precedent);
  `paint_table_header_row(ui, header, widths)` (header Grid, glyph pad + strong cells,
  koi `=` marker / dashed rule nahi; khali header par no-op);
  `paint_table_cell_grid` ka header block isi ko call karta hai (single source);
  `format_table_page_label(pager)` (`views.rs` wording reuse, pager methods reuse);
  `TableToolbarAction` + `paint_table_toolbar(ui, needle, badge, page_label)`
  (ek `TABLE_FIND_BAR_H` row: Find + Prev/Next + page Prev/Next + Small page label + right badge;
  labels/geometries find-bar precedent reuse, koi naya wording/token nahi)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views_table.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_table.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views_table.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye fns `.get()`/saturating only, koi indexing/slicing nahi

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-table-bq/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-table-bq/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-table-bq/REPORT.md checks/gui-table-bq/round-1/after.png checks/gui-table-bq/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_table.rs checks/gui-table-bq/GATES.md checks/gui-table-bq/REPORT.md checks/gui-table-bq/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged (pathspec commit — sibling `views.rs`/`checks/gui-gutter-bq/` untouched),
  NO push
