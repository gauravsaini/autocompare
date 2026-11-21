# hygiene52 round-1 — byte behavior

- Default `cargo test -p autocompare-gui` no longer touches
  `checks/gui-exq-kittest49/round-1/real-egui.png` (tracked copy frozen).
  Shot goes to `$CARGO_TARGET_TMPDIR/real-egui.png`, else `$TMPDIR/real-egui.png`.
- `EXQ_KEEP_PNG=1` refreshes the tracked copy explicitly (verified 53024 B).
- Alpha: wgpu readback frame carries alpha 0 on a fully-covered opaque
  window. `encode_png` is pass-through (no fabrication); capture site now
  applies `RgbaImage::opaque()` (RGB untouched). Machine proof on tmp PNG
  (960x600 RGBA): alpha-0 pixels 0/576000 (localrun7 forensics had
  568630/576000). RGB means unchanged (dark theme ~32).
- Out of scope (other owners): `cargo test` also rewrites
  `checks/gui-sessionpaint-by/round-1/session-{2,8}tabs.png` via views.rs
  tests; reverted, not committed.
