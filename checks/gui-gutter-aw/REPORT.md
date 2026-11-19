# REPORT — gui-gutter-aw (overview ruler parity)

## Feedback addressed (checks/localrun-au/REPORT.md §3, text.png)

1. Gutter 14px bahut patla, BC overview ruler se door → ruler track
   12/6 se **16/8** (naya `gutter_ruler_width`, `SPACE_LG`/`SPACE_SM`).
2. Segments 2px min-height par dot lagte, click-target nahi →
   min-height **2→4px** (`GUTTER_SEGMENT_MIN_H = SPACE_XS`), slop
   hit-test **±2px** (`GUTTER_HIT_SLOP`), har band ~8px target.
3. Click-to-jump dono direction → `gutter_row_at_y` (offset→row) +
   `gutter_y_for_row` band-top aur naya `gutter_row_center_y`
   band-center (row→offset); `gutter_jump_target` nearest-diff snap
   unchanged; `gutter_segment_at_y_with_slop` segment hit-test.

## Geometry (first)

- `GUTTER_RULER_WIDE 16.0` / `GUTTER_RULER_NARROW 8.0` + font-scaled
  variants calibrated at `TYPO_MONO_SIZE` (12 → 16/8, 13 → ~17.33/~8.67).
- Layout budget (960px canvas): 12 margin + 460 pane + 4 gap + 460 pane
  + 16 ruler + 8 margin = 960. Ruler purana 4px inter-chrome gap + 2px
  right margin absorb karta hai; panes 460px untouched.
- Legacy `gutter_bar_width` (12/6) **unchanged** taaki `views.rs` /
  `views_hex.rs` chrome aur unke tests stable rahen — ruler adopt karna
  paint wiring ka kaam hai (neeche).
- `gutter_geometry` ab `GUTTER_SEGMENT_MIN_H` use karta hai; merge gap
  `GUTTER_MERGE_GAP` (2.0) same — paas-paas ke hunks auto-fuse hokar
  solid clickable bands bante hain (tall-file tests).
- `gutter_segment_at_y` ab zero-slop delegate hai (behavior identical);
  slop version low-side 0.0 par clamp, aakhri segment inclusive end.

## Typography (second)

- Koi naya size nahi: `gutter_text_role` = Mono, `gutter_text_size` =
  12.0 shared token, line-height source measured mono row height —
  existing `gutter_text_typo_follows_mono_tokens` test unchanged green.

## Colors

- FROZEN: `gutter.rs` me koi `Color32`/`Visuals`/palette touch nahi;
  kind→color mapping paint (`views.rs`) me hi hai.

## Tests (7 naye, gutter:: 11→18)

`ruler_tokens_match_spacing_scale`, `ruler_width_widens_track…`,
`ruler_width_for_font_scales…`, `min_segment_height_keeps_tall_file…`,
`adjacent_diffs_fuse_into_one_clickable_band`,
`slop_hit_test_expands_bands…`, `row_center_y_round_trips_both_directions`.
MS rust: `forbid(unsafe_code)`, non-test me zero unwrap/expect/panic/todo.

## Paint wiring note (agent A / orchestrator, yahan edit nahi)

`views.rs` me `text_minimap_ruler_width` (== 12/6 paint width) ko
`gutter_ruler_width` (16/8) par switch karna + allocate rect hi click
target hai (`text_gutter_click_select` already wired); selection marker
ke liye `gutter_row_center_y` band-center deta hai. `views_hex.rs`
`hex_gutter_bar_width*` same pattern par `gutter_ruler_width*` reuse
kar sakta hai. Yeh sab doosri waves ki files hain — is wave me chhua nahi.

## Harness

`tools/gui-polish/run-geo-typo.sh checks/gui-gutter-aw` round-1:
`RESULT: PASS` (PNG 9624 B, magic ok, non-blank; tests green).
