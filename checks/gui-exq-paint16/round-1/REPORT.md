# REPORT: exq-paint16 egui gutter mirror (round-1)

Scope: runtime egui path `paint_text_body` mirrors the shot15 fixture look —
20px center gap rect, yellow `=>` markers on changed-row hunk heads only,
floor fill below last row. Geometry FIRST (existing helpers only),
typography SECOND (none touched), colors FROZEN at 33 touches
(`CHANGED_FG`/`CHANGED_BG` reuse only).

## Paint call sites (all in `crates/gui/src/views.rs`, fn `paint_text_body` @ 3185)

ADDED this wave (new wiring):
- `text_center_marker_fg` @ 4877 — pure helper returning frozen `CHANGED_FG`;
  single ink source for both marker sites below (headless-locked).
- wrap-on hunk-head marker @ 3638 — `ui.small_button(RichText::new(label)
  .color(text_center_marker_fg()))` (was uncolored `small_button(label)`);
  wrap-on head row, gate `text_center_hunk_head` @ 3618.
- wrap-off hunk-head marker @ 3913 — same colored marker (was uncolored);
  wrap-off head row, gate `text_center_hunk_head` @ 3900.

PRE-EXISTING mirror structure (verified live this wave, untouched):
- 20px gap rects: fixed-width columns @ 3607/3612 (wrap-on head),
  3654/3673 (wrap-on head blank fallback), 3704/3706 (wrap-on continuation),
  3750/3752 (wrap-on trailing floor), 3810/3812 (wrap-off trailing floor),
  3892/3896 (wrap-off head band), 3927/3943 (wrap-off blank fallback) —
  all `allocate_exact_size(egui::vec2(text_center_gutter_width(), row_height))`.
- Selection band fill (`CHANGED_BG` reuse, no new hue): @ 3611, 3659, 3678,
  3709, 3895, 3929, 3944 — gate `text_center_gutter_band_fill(is_selected)`.
- Floor fill below last row: `text_trailing_blank_rows_balanced` @ 3730
  (wrap-on) and @ 3777 (wrap-off); trailing rows paint both panes hatched +
  blank 20px center gutter so the gap reads below the last row.

## Evidence

- Harness: `CARGO_TARGET_DIR=/tmp/target-exq-paint16 CARGO_BUILD_JOBS=2
  tools/exquisite/run.sh checks/gui-exq-paint16/round-1 crates/gui/src`
  -> `RESULT: PASS` (PNG 10449B >= 8192, magic ok, distinct 256;
  full suite 1037 passed / 0 failed).
- Edge fixture: `comparison/paint-egui-mirror-1200.json`
  (20px center, `=>` on hunk heads, 1200 = 582+582+20+2x8, floor 16).
- Headless test: `views::tests::paint16_egui_mirror_gap_plus_markers_plus_floor`
  @ 9204 — gap math (SPACE_LG+SPACE_XS, arrow box 16, inset 2, slop 2,
  column 590, cx 600, hit band 588-612), marker mapping per kind, hunk heads
  [2] on default 7-row fixture, marker ink == CHANGED_FG, floor 16 balanced.
- MS Rust: non-test delta has no unwrap/expect/panic/todo/unreachable/indexing;
  `rustfmt --edition 2024` clean on `crates/gui/src/views.rs`.
- Colors: exquisite-report `Color32/Visuals touches 33`, unchanged.
- No pxdiff-on-after gate this wave (fixture renderer is a separate path);
  orchestrator runtime-verifies with a live screenshot after landing.
