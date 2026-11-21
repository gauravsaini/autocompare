# RENDER_DELTA — exq-hex64 round-1

Change is headless-only: `crates/gui/src/views_hex.rs` adopts existing
tokens at 14 value-identical const/fn definitions (gutter59/tree62
precedent). Paint resolves through the same `views_hex` accessors, so
raster output cannot move. Zero new tokens/sizes/colors, hit behavior
unchanged.

Adoptions (all value-identical, repo-wide grep per site):

- `HEX_FIND_BAR_H` 28.0 → `theme::CHROME_HEADER_BAR_H`
  (`theme.rs:179`; `views::HEADER_H` itself rides 28.0 at `views.rs:107`).
- `HEX_FIND_NEEDLE_W` 120.0 → `views::FIND_NEEDLE_W`
  (`views.rs:113`; tree62 precedent: `FOLDER_FILTER_NAME_W` locks
  `== FIND_NEEDLE_W` at `tree.rs:3249`).
- `HEX_FIND_BADGE_PAD` 4.0 → `theme::GEO_TIGHT_GAP`
  (`theme.rs:288 == SPACE_XS`; `views::BADGE_PAD` rides `TIGHT_GAP`
  4.0 at `views.rs:112`; `hex_offset_gutter_pad` precedent).
- `hex_minimap_alloc_height()` 220.0 → `views::HEX_MINIMAP_H`
  (`views.rs:114`; paint already `debug_assert_eq!`s the equality at
  `views.rs:2396`, test locks at `views_hex.rs:4513` pre-change).
- `HEX_CANVAS_W` 960.0 → `theme::VIEWPORT_W as f32`
  (`theme.rs:313`; `TABLE_CANVAS_W` precedent at `views_table.rs:2193`).
- `HEX_CANVAS_H` 600.0 → `theme::VIEWPORT_H as f32` (`theme.rs:315`).
- `HEX_STACK_BREAKPOINT_W` 720.0 → `theme::VIEWPORT_MIN_W as f32`
  (`theme.rs:318`; gutter59 precedent for 720 call-sites,
  `TABLE_NARROW_CANVAS_W` at `views_table.rs:2278`).
- `HEX_TALL_CANVAS_H` 1200.0 → `gutter::GUTTER_CANVAS_WIDE_1200`
  (`gutter.rs:4086`; gutter59 precedent for 1200 call-sites).
- `HEX_NARROW_CANVAS_W/H` 720.0 → `theme::VIEWPORT_MIN_W as f32`.
- `HEX_WIDE1200_CANVAS_W` 1200.0 → `gutter::GUTTER_CANVAS_WIDE_1200`.
- `HEX_WIDE1200_CANVAS_H` 600.0 → `theme::VIEWPORT_H as f32`.
- `HEX_NARROW700_CANVAS_W/H` 700.0 → `gutter::GUTTER_CANVAS_NARROW_700`
  (`gutter.rs:4080`).
- `HEX_GOTO_*` transitively tokenized (delegate to `HEX_FIND_*`, unchanged).
- Already-tokenized pins (no edit, locked by new test): `HEX_PANES_MIN_H`
  `== viewmodel::TEXT_PANES_MIN_H`, `HEX_CELL_PAD_X == theme::SPACE_SM`,
  `hex_offset_gutter_pad() == GEO_TIGHT_GAP`,
  `hex_offset_gutter_gap() == GEO_SECTION_GAP`.

Deliberately untouched (no value-identical token exists): `HEX_TALL_PANES_H`
1070.0, `HEX_NARROW_PANES_H` 590.0, `HEX_NARROW700_PANES_H` 570.0 (canvas
minus 130.0 chrome reserve — no 130 token); `HEX42_REF_CHAR_W` 7.0 (char
metric, no SPACE/GEO/GUTTER/TYPO member at 7.0); `2.0/3.0` char multipliers.

- `after.png` (harness software rasterizer, bypasses App paint — NOT live proof):
  10946 bytes baseline → 10946 bytes post-change, md5
  `3ea634d160afca6fc02567487e4e8466` both runs, magic ok, distinct 256,
  colors 35 (frozen, unchanged from baseline 35).
- Live proof: `cargo test -p autocompare-gui --test real_egui_shot` →
  `test result: ok`, 2 passed (dark + light text-compare shots through `App`).
- Unit proof: `exq_hex64_geometry_tokens_value_identical` (1 test) locks every
  const to its token AND its legacy literal, plus GOTO transitivity and the
  already-tokenized pins.
- Suite: 1128 gui tests green (baseline 1127 + 1 new), fmt clean,
  banned-scan 0 on added lines.

Verdict: render delta = zero by construction (value-identical token swap) and
confirmed by byte-identical after.png + green live kittest render.
