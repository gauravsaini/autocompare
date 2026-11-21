# exq-typo54 round-1 — real-egui render delta

Headless chains only (`crates/gui/src/theme.rs`): paint untouched, so
`after.png` is bit-identical to baseline (10946 bytes, distinct 256).

Kittest (`--test real_egui_shot`, 2026-11-21):

- dark: rows=3 bytes=52824 colors=503 base=10 ratio=50
- light: rows=3 bytes=53512 colors=549 base=10 ratio=54
- verdict: ok, 2 passed. Delta vs `after.png` is glyph rasterization only
  (real egui anti-aliased fonts vs harness 5x7 glyphs) — no layout delta.

Boundary (honest): this wave supplies `typo54_candidate_stack` /
`TypoRole::candidate_stack` chains only. Live install + startup log line
stay owned by `app.rs` (`typo48_live_font_plan`, staged stock families);
no display calls added in `theme.rs`.
