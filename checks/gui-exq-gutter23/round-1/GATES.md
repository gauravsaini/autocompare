# GATES gutter23 (written BEFORE implementing per $unlazy)

## CHECK: baseline harness runs on fresh round-1 dir
- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-gutter23 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-gutter23/round-1 crates/gui/src` exits 0 and writes `after.png` + `exquisite-report.md` with `RESULT: PASS`; then `cp round-1/after.png round-1/before.png`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`; `before.png` byte-identical to `after.png`.

## CHECK: HALF A — track contrast (48,48,48), no hue change
- CHECK: `DARK_TRACK_RGB` / `DARK_TRACK` in `crates/gui/src/theme.rs` change `[32,32,32]` -> `[48,48,48]` only (value edit, no new const, no new hue); locking tests updated honestly (`theme.rs` `DARK_TRACK_RGB` assert, `views.rs` `track_for` assert stays via `DARK_TRACK` reuse); shared helper also fixes hex-minimap + merge-overview tracks (same `track_for(dark)` path) — full suite must stay green.
- EXPECT: track (48,48,48) vs pane bg (27,27,27) delta 21/255 visible; `cargo test -p autocompare-gui` green; banned-token grep on non-test delta empty; colors exactly 33.

## CHECK: HALF B — marker ink via painter.text in CHANGED_FG (no Button override)
- CHECK: the two live hunk-head sites (wrap-on + wrap-off in `paint_text_body`, `Button::small().frame(false)` + `RichText::color(text_center_marker_fg())`) replaced with `allocate_exact_size(vec2(text_center_marker_content_w(), row_height), Sense::click())` + `ui.painter().text(center, CENTER_CENTER, label, font_id.clone(), text_center_marker_fg())` + `resp.on_hover_text("Copy changed hunk")` + `resp.clicked()` -> same `copy_action = Some((orig, dir))`; same leading `text_center_marker_inset()` space; same hover string.
- EXPECT: `=>` ink is `CHANGED_FG` (255,235,160) not Button gray; zero behavior change except ink; headless gates (`text_center_marker_fg() == CHANGED_FG`, label gate, hunk-head gate) green; `cargo fmt --check` clean.

## CHECK: exquisite re-run PASS + fixture pxdiff ABANDONED with reason
- CHECK: re-run same harness command into same round dir; paste full `exquisite-report.md` output into REPORT.md.
- EXPECT: `RESULT: PASS`; fixture pxdiff IDENTICAL (egui-only change, no fixture repaint) — mark ABANDONED with reason "egui-only paint change; no PNG delta expected".

## CHECK: live-pixel gate (first of its kind, MANDATORY)
- CHECK: build, launch `./target/debug/autocompare-gui --open tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --tab text`, sleep 6, `screencapture -x -o round-1/live.png`, kill GUI; then `uv run --with pillow` crop center-column band over text rows and ASSERT (a) median brightness of unselected-row track cells >= 40 (visible vs 27 bg), (b) changed-row marker cells contain pixels within 30 of (255,235,160).
- EXPECT: both asserts hold; script output pasted into GATES.md + REPORT.md; zoomed crop saved as `round-1/live-crop.png`; gate passes only if both hold.

## CHECK: edge fixture + headless test
- CHECK: add `round-1/comparison/gutter-live-1200.json` (tall: 1200 rows, heads [2,500,1000], track math stable, marker frameless yellow) + one headless test named `gutter23_live_track_contrast` proving it.
- EXPECT: test green; fixture `include_str!` path correct; tall heads/window math mirrors mirror22 precedent.

## CHECK: MS Rust + touch/scope + colors
- CHECK: `#![forbid(unsafe_code)]` holds; no `unwrap/expect/panic/todo/unreachable/indexing` in non-test delta; GUI stays thin client; `cargo fmt` run on touched files; `git status` shows ONLY `crates/gui/src/views.rs` + `crates/gui/src/theme.rs` + `checks/gui-exq-gutter23/**`; NEVER touch `crates/sessions/*`, never `git add -A`; colors `Color32::from_rgb|Visuals::` == 33.
- EXPECT: full `autocompare-gui` suite green; `cargo fmt --check` clean; banned-token grep empty; colors 33.

## LIVE-PIXEL GATE OUTPUT (round-1, pasted verbatim)

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

Gate passes: (a) 48.0 >= 40, (b) 100 > 0. Script:
`round-1/live-pixel-check.py` (run via `uv run --with pillow`).
Zoomed crop: `round-1/live-crop.png` (560x492).
