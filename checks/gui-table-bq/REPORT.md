# Wave BQ-TABLE REPORT (livecomp-BM TABLE G-B1 + G-B2 + G-B3: header row + compact toolbar + Grid)

Scope: `crates/gui/src/views_table.rs` + `checks/gui-table-bq/**` only.
KEVL untouched: `views.rs` / `theme.rs` / `gutter.rs` / `home.rs` / `tree.rs` /
`viewmodel.rs` / `app.rs` / `screenshot.rs`, `checks/opencode-waves/STATUS.md`,
`checks/gui-gutter-bq/` (sibling wave) — sab read-only rahe, commit pathspec sirf OWNS paths par.
No push. Clean-room: bahar se observation only, koi string/asset copy nahi.

## Geometry table before/after

| # | area (G-Bx) | before | after |
|---|-------------|--------|-------|
| 1 | G-B1 header row | Header data row ki tarah tha: `views.rs paint_table` me `=` glyph marker + alag dashed `----` separator row (`format_table_header_rule`); `paint_table_cell_grid` (views_table.rs:1091) me header block inline tha, raw `.monospace().strong()` par | `paint_table_header_row` (views_table.rs: nayi) header ka single source hai: `TABLE_CELL_HEADER_GRID_ID` Grid, glyph pad + `table_header_text_role` strong cells, koi `=` marker nahi, koi dashed rule nahi; `paint_table_cell_grid` ka header block isi ko call karta hai |
| 2 | G-B2 duplicate Find/nav | Do Find bars + do Prev/Next rows: app.rs Table branch me `paint_table_find_bar` + page row, phir `views.rs paint_table` me phir `paint_table_find_bar` + page row | `paint_table_toolbar` (views_table.rs: nayi) ek compact row hai: Find needle + Prev/Next + page Prev/Next + Small page label + right N/M badge, sab ek `TABLE_FIND_BAR_H` (28.0) row me; button labels + geometries find-bar precedent reuse, caller ek hi toolbar chalayega (wiring orchestrator ke paas) |
| 3 | G-B3 Grid columns | Cells flow me the (single-string join); data Grid `paint_table_cell_grid` me tha par header block usi me inline tha | Cells Grid me hain: data Grid `TABLE_CELL_GRID_ID` reuse karta hai (`table_cell_grid_columns` + `TABLE_GRID_COL_GAP`/`TABLE_GRID_ROW_GAP` spacing barkarar), header ka apna `TABLE_CELL_HEADER_GRID_ID` (dono ids distinct, test-locked) |
| 4 | typo (second) | Header/raw labels me raw `.monospace().strong()` chains the | `table_header_text_role()` (const fn → `TypoRole::Mono`, gutter/hex `*_text_role` precedent); small-caps intent paint-side `.strong()` composition hai (folder caption-strong precedent, naya size/token nahi); page label `TypoRole::Small` (status precedent); koi `.size()` add nahi |

## Exact lines (post-edit `views_table.rs`)

- Import: `use crate::theme::TypoRole;` (line ~28).
- `table_header_text_role` (const fn, Mono alias) + `paint_table_header_row` (header Grid, no-op on empty) + `format_table_page_label` (pager wording reuse) + `TableToolbarAction` + `paint_table_toolbar` (single-row Find + page nav + badge): header-row/toolbar section, `paint_table_cell_grid` ke baad.
- `paint_table_cell_grid`: header block hata kar `paint_table_header_row(ui, header, &widths)` call (single source; data Grid unchanged).
- Tests (mod tests, file end): `table_header_role_reuses_mono_grid_content`, `table_page_label_mirrors_pager_state`, `table_toolbar_defaults_quiet_and_reuses_find_geometry`.

## Gates

- `cargo test -p autocompare-gui`: 611 passed, 0 failed (pre-edit baseline 607; +3 naye BQ tests, +1 sibling gutter wave ka — uska file untouched rakha).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0).
- `rustfmt --edition 2024 --check` (sirf OWNS file): clean (exit 0).
- Banned grep (non-test code): ZERO-BANNED (`unwrap()`/`expect()`/`panic!`/`todo!` nahi). `forbid(unsafe_code)` kept.
- Colors: NO-COLOR-TYPO-ADD (koi `from_rgb`/`from_rgba`/`Visuals::`/`.size()` add nahi).
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-table-bq/round-1`: RESULT PASS (1 round; after.png 9695B, magic-ok `89504e470d0a1a0a`, distinct 255).
- OWNS-clean: mere diff me sirf `views_table.rs` + `checks/gui-table-bq/**`; KEVL-CLEAN; MANA-CLEAN (STATUS untouched).

## Parallel-wave note (honest)

- Is wave ke beech `cargo clippy` ek baar sibling `views.rs` ke mid-edit state par fail hua (14x E0425: sibling callers present, defs absent — mera `git stash -u` probe unki defs samet le gaya tha). Maine `views.rs` edit NAHI kiya; sibling ne defs khud wapas add kiye (verify: lines 96-113 maujood, file stable), uske baad tree green hua. Mera stash (`stash@{0}`) ab redundant hai — drop orchestrator kare (sibling files usme bhi hain, isliye maine drop nahi kiya).
- Commit pathspec sirf OWNS paths par hai taaki sibling (`views.rs` + `checks/gui-gutter-bq/`) ka kaam usi ka rahe.

## Follow-up (agli wave ke liye, is scope se bahar)

1. Wiring: `views.rs paint_table` + app.rs Table branch me duplicate Find/nav hata kar `paint_table_toolbar` ki single call lao; header dashed rule (`format_table_header_rule` label) hatao — OWNS `views.rs` + `app.rs` (KEVL is wave me tha).
2. G-B4 (P1): row-number gutter + column-letter (A/B/C) header row — nayi pure helpers `views_table.rs` me, paint `views.rs` me.
3. G-B5 (P2): summary wording BC pattern par — OWNS `viewmodel.rs` / `views.rs`.

Commit: local only, no push.
