# RENDER_DELTA — exq-home65 round-1

Change is headless-only: `crates/gui/src/home.rs` pure geometry consts adopt
existing SPACE tokens at 7 value-identical call sites
(`HOME_CARD_MIN_W` 240 -> `SPACE_XL * 10.0`,
`HOME_TOOLBAR_BUTTON_H` 24 -> `SPACE_XL`,
`HOME_CARD_CTA_FRAME_PAD` 24 -> `SPACE_XL`,
`RECENT_COL_KIND_W` 96 -> `SPACE_XL * 4.0`,
`RECENT_COL_AGE_W` 72 -> `SPACE_XL * 3.0`,
`SESSION_TREE_EXPANDER_W`/`SESSION_TREE_ICON_W` 16 -> `SPACE_LG`).
`views.rs`/`views_hex.rs` paint untouched (hex64 owns views_hex.rs);
all derived consts (`HOME_CARD_CTA_W`, `HOME_CARD_BUTTON_MIN_W`,
`SESSION_TREE_WIDTH`, row heights) evaluate bit-identical, so raster
output cannot move. Zero new tokens/sizes/colors.

- `after.png` (harness fixture): 10946 bytes baseline -> 10946 bytes
  post-change, md5 `3ea634d160afca6fc02567487e4e8466` post-change,
  magic ok, distinct 256, colors 35 (frozen, unchanged from baseline 35).
- Live proof: `cargo test -p autocompare-gui --test real_egui_shot` ->
  `test result: ok`, 2 passed, 0 failed (real App render path).
- Unit proof: `exq_home65_geometry_tokens_value_identical` asserts each
  const equals both its token expression and its legacy literal;
  7.0/75.0/150.0/21.2 literals documented as token-less (repo-wide grep).

Verdict: render delta = zero by construction (value-identical token swap) and
confirmed by byte-size-identical after.png + green live kittest render.
