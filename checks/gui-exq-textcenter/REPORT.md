# REPORT — exq-textcenter round-1 (gutter + viewport + typo hierarchy)

Wave: TEXTCENTER exquisite, headless geometry + TypoSpec (paint-neutral).
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## Kya kiya (geometry FIRST)

- Center gutter single-source: `theme::TEXT_CENTER_GUTTER_W` = 48
  (`CHROME_HEADER_BAR_H` 28 + `GUTTER_BAR_WIDE` 12 + 2 x `GEO_TIGHT_GAP` 4).
  Koi naya literal nahi; `views::TEXT_CENTER_GUTTER_W` paint baad me adopt karega.
- Center-gutter-aware budget: `textcenter_chrome_w()` = 88 (40 dual + 48 center),
  `textcenter_pane_widths()` (960 -> 436/436, 720 -> 316/316),
  `textcenter_x_positions()` (960 -> left 12, gutter 452, right 504).
- Viewport fill: `textcenter_fill_rows()` — `ceil(panes_h / row_h) - visible`
  (600/24 par 6 rows -> 19 ruled blanks, full/overflow -> 0, NaN/0 guard -> 0).
- Hunk heads: `textcenter_hunk_head()` — `section_bounds` reuse, sirf block
  start par copy-arrow pair, continuation blank click-to-select.
- Inline spans: `textcenter_accent_spans()` — Changed rows par hi char-hugging
  ranges (zero char pad), baaki kinds blank; emphasis block + 1px underline
  (`TEXT_INLINE_SPAN_UNDERLINE_W`, 1px visual precedent).

## Phir typography SECOND

- `TypoRole::content_textcenter()` = Mono (grid/gutter ke barabar).
- `typo_textcenter_steps_ok()`: Body 13 > Mono 12 > Small 11;
  line-height px body 18.2 > mono 15.6 > small 13.2;
  weight 400 body/mono, 600 small-counts, 500 headers;
  tracking trio 0.0 (sirf Heading 0.02em); tabular Mono + Small.
- TypoSpec delegation per role test-locked. Koi naya size, koi color move nahi.

## Edge fixtures (headless tests)

- empty-vs-nonempty: left khali vs 3-line right — sab Inserted, right-only
  numbers, gutter floor 4, fill positive, head sirf row 0.
- tail-insertion: Equal head + Insert tail — paired numbers, zero accents,
  head index 2, gutter x-order bana rehta hai.

## Harness

`tools/exquisite/run.sh checks/gui-exq-textcenter/round-1` -> RESULT: PASS
(PNG 10401 bytes, magic ok; colors touches 33, pehle jitna hi — frozen).
Naye tests: 10 (8 viewmodel + 2 theme). Suite: 816 passed, 0 failed.

## Gap note (doosri wave ka, hamare scope bahar)

`cargo clippy --all-targets -D warnings` HEAD par red hai —
`views_table.rs:3110` `useless_vec` (commit db4d8e9, hextable3 wave).
`views_table.rs` hamare liye FORBIDDEN hai, isliye chhua nahi.
Lib clippy hamari files par 0 warnings; GATES.md me darj hai.
