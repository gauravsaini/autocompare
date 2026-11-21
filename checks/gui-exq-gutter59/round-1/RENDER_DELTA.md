# RENDER_DELTA — exq-gutter59 round-1

Change is headless-only: `crates/gui/src/gutter.rs` pure geometry fns adopt
canvas tokens (`GUTTER_CANVAS_NORMAL_960`, `GUTTER_CANVAS_WIDE_1200`,
`theme::VIEWPORT_MIN_W`) at 7 call sites with identical values.
`views.rs`/paint calls none of these fns (repo-wide grep: only `gutter.rs`
references), so raster output cannot move.

- `after.png` (harness software rasterizer, bypasses App paint — NOT live proof):
  10946 bytes baseline → 10946 bytes post-change, md5
  `3ea634d160afca6fc02567487e4e8466` post-change, magic ok, distinct 256,
  colors 35 (frozen, unchanged from baseline 35).
- Live proof: `cargo test -p autocompare-gui --test real_egui_shot` →
  `test result: ok`, `real-egui: rows=3 bytes=53178 colors=503 base=10 ratio=50`
  (AA ratio 50 ≥ floor 10, real App + wgpu render through `App` text-compare).
- Geometry signals: gutter refs 1942 → 1979 (+37, new unit tests only),
  TypoRole 1145 → 1146 (+1 test ref), SPACE tokens unchanged, zero new sizes.

Verdict: render delta = zero by construction (value-identical token swap) and
confirmed by byte-size-identical after.png + green live kittest render.
