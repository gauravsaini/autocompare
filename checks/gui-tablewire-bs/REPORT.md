# Wave BS-TABLEWIRE REPORT (livecomp-BM G-B1/G-B2/G-B3 + table-BQ follow-ups)

Scope: `crates/gui/src/views_table.rs` + `crates/gui/src/app.rs` (Table branch toolbar hunk only) + `checks/gui-tablewire-bs/**`.
KEVL untouched: `views.rs` / `theme.rs` / `gutter.rs` / `home.rs` / `tree.rs` / `viewmodel.rs` / `screenshot.rs` / `scripts/` / `checks/opencode-waves/STATUS.md` + baaki `checks/` dirs. `views.rs` BR owns karta hai — is wave me sirf exact spec neeche hai.
Clean-room: bahar se observation only, koi string/asset copy nahi. NO push.

## Geometry table before/after

| # | area (G-Bx) | before | after |
|---|-------------|--------|-------|
| 1 | G-B2 single Find + Prev/Next | Do Find implementations: `paint_table_find_bar` me full Find row body, `paint_table_toolbar` me wahi body copy-paste (drift risk) | Single source: naya private `paint_table_find_controls` (Find label + needle + Prev/Next + N/M badge); `paint_table_find_bar` sirf horizontal wrapper hai (signature stable, `views.rs` caller untouched), toolbar wahi helper chalata hai + page pair + page label |
| 2 | G-B2 app.rs duplicate toolbar | Table branch me `paint_table_find_bar` + alag Prev/Next page row (do control rows), jabki neeche `paint_table_keyed` -> `paint_table` phir apna Find + page row chalata hai | Single toolbar call: `paint_table_toolbar(ui, &mut self.table_find_needle, &table_badge, &table_page_label)` — Find pair App cursor me (`table_find_next_row`/`prev_row`), page pair `table_page_next`/`prev_page` me, needle-change reset barkarar, page label `format_table_page_label(self.table_pager)` se |
| 3 | G-B1/G-B3 header/data Grid columns | Header Grid cols `header.len() + 1`, data Grid cols `table_cell_grid_columns(&widths)` — do ginne ke tareeke | Dono `table_cell_grid_columns` single source se (invariant comment ke saath: `widths` hamesha `table_column_widths(header, rows)` se banta hai, isliye barabar hai) |
| 4 | G-B4 groundwork (letters + row gutter) | Koi column-letter / row-number helper nahi tha (grep-proof: poore crate me koi letter helper nahi; `hex_gutter_row_label` hex-offset specific hai — table ke liye reuse galat hota, isliye naya) | Do pure helpers + tests: `table_column_letter` (0->A ... 25->Z, 26->AA spreadsheet order) aur `table_row_gutter_label` (1-based). Paint BR wave me karega (`views.rs` NO-TOUCH) — spec neeche |

## Exact lines (post-edit `views_table.rs`)

- `paint_table_find_controls` (private fn, Find single source) + `paint_table_find_bar` (wrapper, signature stable) — find-bar section.
- `paint_table_header_row`: `let cols = table_cell_grid_columns(widths)` (data Grid wala source; `&widths` nahi — clippy needless-borrow fix).
- `table_column_letter` + `table_row_gutter_label` (`#[allow(dead_code)]`, Test-only until paint wiring lands — BQ precedent).
- `paint_table_toolbar`: Find segment helper call hai (duplicate Find/badge block hata), page pair + `TypoRole::Small` page label barkarar.
- Tests: `table_column_letters_follow_spreadsheet_order`, `table_row_gutter_labels_are_one_based_and_grids_share_columns` (+ toolbar geometry test pehle se tha).

Exact lines (post-edit `app.rs`, Table branch): `self.sync_table_pager()` + `format_table_page_label` + single `paint_table_toolbar` call + 4 action arms (find_next/find_prev/page_next/page_prev); purana find_bar call + `horizontal_wrapped` page row hata.

## Dashed `----` remainder + single note (honest findings)

- `views_table.rs` code me koi dashed separator nahi (grep `---` sirf doc-comment me G-B1 zikr hai + test me `"x".repeat` hai). Asli remainder `views.rs paint_table` me hai — NO-TOUCH, BR spec neeche.
- `views_table.rs` me note single source hai (sirf key bar ka `positional_note()`). Live double-note `paint_table_keyed` me hai (bar + outer block) — NO-TOUCH, BR spec neeche.

## Gates

- `cargo test -p autocompare-gui`: 616 passed, 0 failed. Hisaab: baseline 611 (@7086d4f, pre-BS) + 3 sibling BR commit `6aa3362` + 2 mere (letter/gutter tests). Mera delta +2, koi fail nahi.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0; raaste me 3 mere errors aaye the — 2 dead-code + 1 needless-borrow — sab fix, BQ `allow(dead_code)` precedent).
- `rustfmt --edition 2024 --check` sirf OWNS files: dono clean (1 fmt collapse fix kiya).
- Banned grep (added lines): ZERO-BANNED-ADDED (`unwrap()`/`expect()`/`panic!`/`todo!` nahi). `forbid(unsafe_code)` kept.
- Colors: NO-COLOR-TYPO-ADD (koi `from_rgb`/`from_rgba`/`Visuals`/`SetVisuals`/`.size()` add nahi).
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-tablewire-bs/round-1`: RESULT PASS (1 round; after.png 9695B, magic `89504e470d0a1a0a`, distinct 255).
- OWNS: commit pathspec sirf `views_table.rs` + `app.rs` + `checks/gui-tablewire-bs/` hai. `gutter.rs`/`home.rs` me jo uncommitted diff hai wo sibling waves ka hai (maine chhua nahi — `git diff` per-file verify). KEVL-CLEAN (mere paths se bahar kuch stage nahi), STATUS-UNTOUCHED.
- Commit local only, NO push.

## Follow-up — BR ke liye exact `views.rs` spec (is wave me NO-TOUCH)

1. `paint_table` (~2002): `paint_table_find_bar` call (~2061) + page-nav horizontal row (~2094-2111) hata kar single `views_table::paint_table_toolbar(ui, &mut needle, &badge, &page_label)` lao, jahan `page_label = views_table::format_table_page_label(*pager)` (`TablePager` Copy hai). Needle/cursor `ui.data` temp me hi rahen (caller-local, App untouched — toolbar signature `&mut String` compatible hai). `find_cursor_step` + `bring_into_view` block (~2067-2075) `TableToolbarAction.find_next/find_prev` par chalao; `pager.next_page/prev_page` action ke page arms par (PageUp/PageDown keys barkarar).
2. Dashed rule: `format_table_header_rule(line.len())` label block (~2133-2137) hatao; `format_table_header_rule` fn (~1464) tab hatao jab koi caller na bache (grep-proof karna). `format_table_bordered_row` retention BR ka call hai.
3. Single note: `paint_table_keyed` (~1505-1519) me outer `positional_note` block (~1515-1517) hatao — key bar (~1512-1514) wahi note `.small().weak()` me pehle se dikhata hai. `status_line()` ka note text (status bar surface) rakho — alag surface hai.
4. Letters + row gutter paint: helpers taiyar hain (`table_column_letter`, `table_row_gutter_label`, tested). Header Grid ke upar ek letter row (A/B/C..., `table_header_text_role` strong) + data Grid me pehli gutter column me 1-based numbers (Mono; `gutter_text_role` precedent reuse, copy nahi). Dhyan: glyph column (`=`, `!`, `L`, `R` status markers, ~2201-2208) live feature hai — number column ALAG column hai, glyph replace nahi hoga; col count `table_cell_grid_columns(&widths) + 1` hoga. Colors frozen — sirf geometry/typo.

Commit: local only, no push.
