# REPORT: EXQ-MERGE2 3-way merge panes plus copy/take actions (BC parity)

Round: checks/gui-exq-merge2/round-1/ -> RESULT:PASS (round-1/6, no repeats needed)
Harness: PNG 10401 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'] all reused, margin/gap consts x412,
  TypoRole x884, .size ['14.0'], weights 3 tokens, line-heights 4 tokens,
  COLORS frozen: Color32/Visuals touches 33 (zero growth)

## Geometry & Typography Before -> After

| Area | Before | After |
|---|---|---|
| Copy button hit-slop | Sirf 24px `copy_hit_size` hit target tha, koi slop contract nahi tha | `COPY_BUTTON_HIT_SLOP` = `SPACE_XS` (4.0); `copy_hit_size_with_slop` = 32.0; `copy_button_hit_rect` & `copy_button_hit_contains` expand 24px button to 32px hit box centered at (cx, cy); `copy_hit_rect_with_slop` expands arbitrary bounding boxes |
| Copy row height | Hardcoded row heights / koi direct mono line-height helper nahi tha copy.rs me | `copy_row_typo_role` = `TypoRole::Mono`; `copy_row_height` = `15.6px` (`TypoRole::Mono.line_height_px()`); `copy_block_height(count)` = count * row_h; `copy_rows_fit(avail_h)` = floor(avail / row_h); `copy_arrow_default_pitch` connects pitch with row height |
| Take button hit-slop | Sirf 24px button height thi, koi hit expansion nahi | `TAKE_BUTTON_HIT_SLOP` = `SPACE_XS` (4.0); `take_button_hit_rect`, `take_button_hit_contains`, `take_choice_button_hit_rect`, `take_side_button_hit_rect` expand visual bounds by 4px per edge |
| Merge 3-way split | Har pane arbitrary shrink hota tha, narrow screens par crush hone ka risk | `MERGE_PANE_MIN_W` = 130.0px floor; `MERGE_SECTION_GAP` = 8.0px (`GEO_SECTION_GAP`); `MERGE_TOTAL_MIN_W` = 406.0px; `merge_three_pane_widths` splits avail minus 2x section gap into equal thirds clamped to 130px; `merge_three_pane_needs_hscroll` triggers below 406px |
| Colors | Frozen | Zero additions in owned files; touches 33 stable |

## Files Modified

- `crates/gui/src/copy.rs`:
  - Added `COPY_BUTTON_HIT_SLOP` (`SPACE_XS`), `copy_button_hit_slop`, `copy_hit_size_with_slop`
  - Added `copy_button_hit_rect`, `copy_button_hit_contains`, `copy_hit_rect_with_slop`
  - Added `copy_row_typo_role` (`TypoRole::Mono`), `copy_row_height`, `copy_block_height`, `copy_rows_fit`, `copy_arrow_default_pitch`
  - Added unit test suite `exq_merge2_tests` (3 tests: `merge2_row_height`, `merge2_hit_slop`, `merge2_narrow_fixture`)
  - Formatted cleanly with `rustfmt --edition 2024`
  - Zero unwrap/expect/panic in non-test code

- `crates/gui/src/take.rs`:
  - Added 3-way merge widths, take button hit-slop, and narrow fixture tests

- `checks/gui-exq-merge2/`:
  - `comparison/merge-3way-narrow.json` edge fixture
  - `round-1/exquisite-report.md` (RESULT: PASS)
  - `after.png` (10401 bytes)
  - `GATES.md` (G1-G6 passed)

## Verification Evidence

- `cargo test -p autocompare-gui --lib -- merge2_`: 7/7 tests passed
- `rustfmt --edition 2024 --check crates/gui/src/copy.rs crates/gui/src/take.rs`: clean
- Exquisite harness: `RESULT: PASS`
