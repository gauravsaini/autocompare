# REPORT gutter23 — live center gutter ACTUALLY visible

RESULT: PASS (harness) + LIVE-PIXEL GATE: PASS. Suite: 1046 passed, 0 failed.

## What changed (two half-fixes, both required)

HALF A — track contrast: `DARK_TRACK_RGB` / `DARK_TRACK` in
`crates/gui/src/theme.rs` 32,32,32 -> 48,48,48. Delta vs pane bg
(27,27,27) goes 5/255 -> 21/255: visible, still dark, no new hue.
Same shared `track_for(dark)` helper also lifts the hex-minimap and
merge-overview tracks — intended shared fix, full suite stays green.

HALF B — marker ink: the two live hunk-head sites in `paint_text_body`
(wrap-on + wrap-off branches) no longer use
`Button::small().frame(false)` (egui Button paints its own ink over the
`RichText` color). Each site now does `allocate_exact_size(vec2(
text_center_marker_content_w(), row_height), Sense::click())` +
`ui.painter().text(center, CENTER_CENTER, label, font_id.clone(),
text_center_marker_fg())` + the same `on_hover_text("Copy changed
hunk")`, with `resp.clicked()` -> the same
`copy_action = Some((orig, dir))`. Zero behavior change except correct
yellow ink.

Touched functions: `paint_text_body` (both hunk-head marker sites),
new headless test `gutter23_live_track_contrast`; theme consts
`DARK_TRACK_RGB`, `DARK_TRACK` (+ honest assert update in
`tokens_keep_frozen_hues`; `chrome_selectors_follow_dark_mode` in
views.rs stays valid via const reuse).

## Harness re-run output (pasted verbatim)

```
# Exquisite report (round-1)
- PNG: 11254 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x327, width calls x85, gutter x1753
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
```

Fixture pxdiff: ABANDONED — `before.png` and `after.png` are
byte-identical (`cmp` clean, both 11254 bytes). Reason: the headless
`--screenshot` fixture does not exercise the live Text Compare center
gutter, and the change is egui-paint-only with no fixture repaint; no
PNG delta is expected. Live visibility is proven by the pixel gate
below instead.

## Live-pixel gate output (pasted verbatim)

```
live.png: 2880x1800
changed-row band: y in [1046, 1072] (1 yellow bands)
gutter columns: x in [696, 736) width=40px
continuity alpha(equal,above) y=1006: 18/20 track-ish
continuity gamma(equal,below) y=1112: 18/20 track-ish
gate (a): unselected track cells=684 median brightness=48.0
gate (b): changed-row marker pixels within 30 of (255, 235, 160): 100 (min dist 7)
crop saved: checks/gui-exq-gutter23/round-1/live-crop.png (560, 492)
LIVE-PIXEL GATE: PASS
```

(a) median 48.0 >= 40: track visible vs 27 bg. (b) 100 marker pixels
within 30 of CHANGED_FG (min dist 7): yellow ink present, centered
glyph ~x713-729 inside the 40px (= 20pt @2x) gutter.

Old-vs-new honesty check on `/tmp/live-gui23.png` (pre-fix build, same
window position): same gutter columns sample median 32.0 -> gate (a)
would FAIL (< 40); new build samples 684 cells at median 48.0. The
track-brightness half is what the gate discriminates.

## Edge fixture + gates

- `comparison/gutter-live-1200.json` (tall: 1200 rows, heads
  [2,500,1000], track math stable) + headless test
  `gutter23_live_track_contrast` proving it (track 48 vs bg 27 channel
  math, band/track complement, marker ink/label gates, tall-window
  stability, balanced trailing floor).
- Full `autocompare-gui` suite green (1046 passed); `cargo fmt --check`
  clean; banned-token grep on non-test delta empty
  (no unwrap/expect/panic/todo/unreachable/indexing); colors exactly 33.
- Scope: only `crates/gui/src/views.rs` + `crates/gui/src/theme.rs` +
  `checks/gui-exq-gutter23/**`. `crates/sessions/*` untouched.
  Pre-existing worktree dirt (sessions import-order fmt + two
  sessionpaint PNGs) left uncommitted.
