# Wave O PAINT REPORT — pagers consumed in paint

## Scope
- OWNS: `crates/gui/src/views.rs`, `crates/gui/src/app.rs`, `checks/gui-paint-o/**`
- No theme/gutter/home/hex/table/tree/viewmodel/main/screenshot edits, no STATUS.md, no push.

## Before (HEAD baseline)
- `app.rs:93-95` `hex_pager/table_pager/folder_pager` fields exist, `app.rs:137-139` constructed `new(0,16/20/20)`, never read in paint.
- `views.rs:365` (old) comment only: "`App::hex_pager` (`next_page`/`prev_page`/`bring_into_view`) chalaye" — no call.
- `paint_hex(ui, view, lines)`, `paint_table(ui, model)`, `paint_table_keyed(ui, view, model)` — full-list `show_rows`, no window.
- Folder tab inline in `app.rs` looped full `nodes`, no paging.
- `hex_gutter_jump_to_row` selected hunk but never `bring_into_view`; `table_find_next_cell` static, no UI.

## After (this wave)
### 1. Hex — pager drives visible hunks + gutter jump in handlers
- `views.rs:375` `paint_hex(ui, view, pager, lines)`: `pager.set_total(hunks.len)`, PageUp/Down → `prev/next_page`, Prev/Next page buttons, window `(wstart,wend)=visible_range()`, `show_rows` only window, selection → `bring_into_view`, click → `bring_into_view`, page label via `small().weak()` only.
- `views.rs:344` `parse_gutter_row_text` (pure, tested).
- `app.rs:480` `hex_gutter_jump_to_row` now `set_total` + `bring_into_view`; `app.rs:494,499,510,521` `sync_hex_pager/hex_page_next/hex_page_prev/hex_select_next_paged/prev_paged`.
- `app.rs:2503-2508` Hex tab: gutter-row TextEdit (60px, existing width) + Jump/Prev-change/Next-change → paged helpers; `paint_hex` call passes `&mut hex_pager` (clone lines to satisfy borrow).

### 2. Table — pager drives visible rows + find-box in handlers
- `views.rs:485` `paint_table_keyed(..., pager)`, `views.rs:559` `paint_table(..., pager)`: same paging pattern, page label, window slice.
- `views.rs:353` `table_model_text_rows` adapts painted `label|text` to `table_find_next` API.
- `app.rs:602,609,615,623` `sync_table_pager/table_page_next/table_page_prev/table_find_next_paged` (find over painted texts + `bring_into_view`, cursor advance).
- `app.rs:2639` Table tab find row: needle TextEdit (120px) + Next-match/Prev-page/Next-page; passes `&mut table_pager`.

### 3. Folder — pager drives visible tree + gutter/find in handlers
- `views.rs:1082` `paint_folder(..., pager)` (future rewiring): same paging pattern.
- `app.rs:738,745,751,757,793` `sync_folder_pager/folder_page_next/prev/folder_next_changed_paged/prev_paged/folder_find_next_paged` (visible-index map + `bring_into_view`).
- `app.rs:2336` Folder inline: Prev/Next page + Prev/Next change + Find TextEdit, `set_total(nodes.len)`, window `nodes[wstart..wend]`, page label. Toggle/double-click preserved.

## Tokens (no new geometry/color/typo)
- Only `SECTION_GAP/TIGHT_GAP/TAB_BOTTOM_GAP`, `text_row_height`, `tree_indent`, `small().weak().monospace()` reused. No `Color32`, no `.size()`, no new consts.

## Checks
- `cargo test -p autocompare-gui`: 211 passed, 0 failed (baseline 204 + 7 new: 4 views + 3 app).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: green.
- `cargo fmt --all -- --check`: green (only owned files formatted).
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-paint-o/round-O-1`: RESULT PASS — PNG 9914B, magic ok, distinct 256, Grid 1, row_height x3, monospace x33, small x37, .size ['14.0'].
- Rounds used: 2 (clippy+fmt fix, harness).

## Files
- `crates/gui/src/views.rs`, `crates/gui/src/app.rs`
- `checks/gui-paint-o/round-O-1/{after.png,geo-typo-report.md,screenshot.out,tests.out}`
