# GATES — wave hatch25 (round-1): calm trailing fill

Scope: `crates/gui/src/screenshot.rs` renderer only (`render_text_fixture_png`
trailing region) + `checks/gui-exq-hatch25/**`. NEVER `views.rs`/`theme.rs`/
`sessions/*`. Geometry FIRST, typo second (no typo change), colors FROZEN (33).

Reference behavior (own words): the reference tool leaves the area below the
last content row as clean empty space with no filler glyphs. Our live window
currently fills that area with a dense repeating vertical-bar texture over
roughly half the pane height — that texture is the gap. The fixture renderer
today leaves a darker pane-base band below the content rows, i.e. a visible
seam/edge partway down each pane (banding). Fix = paint the trailing slots
with the existing content-base tone so the whole pane reads as one calm
field; zero filler glyphs, zero new tones.

## Gate list (CHECK = command, EXPECT = required outcome)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-hatch25 CARGO_BUILD_JOBS=2
  tools/exquisite/run.sh checks/gui-exq-hatch25/round-1 crates/gui/src`
  EXPECT: exit 0, `RESULT: PASS`, PNG >= 8192 B, magic ok, distinct > 64.
- CHECK: `tools/exquisite/pxdiff.py before.png after.png --expect-diff`
  EXPECT: exit 0, `VERDICT: DIFFERENT`, diff count > 0, diff bbox y-range
  sits inside the trailing fill band (below last content row y=300, above
  per-pane strips y=542), x-range inside pane columns; chrome rows (y < 174),
  center markers, gutter column, strips/legend (y >= 542) identical.
- CHECK: `cargo test -p autocompare-gui` (via harness `tests.out`)
  EXPECT: all pass including new `hatch25_tall_no_banding` (1048 + 1 = 1049).
- CHECK: `cargo fmt -p autocompare-gui -- --check`
  EXPECT: clean (exit 0).
- CHECK: banned-token grep over non-test delta
  (`unwrap|expect(|panic!|todo!|unreachable!|[a-z_]+\[[0-9a-z_"']` on added
  non-test lines)
  EXPECT: empty (ZERO-BANNED).
- CHECK: `grep -c 'Color32::from_rgb\|Visuals::' crates/gui/src/*.rs`
  EXPECT: total exactly 33 (frozen; this wave adds zero color literals —
  trailing fill reuses `fixture_palette().bg`).
- CHECK: edge fixture `comparison/hatch-tall-1200.json` exists and headless
  test `hatch25_tall_no_banding` reads it
  EXPECT: tall viewport math proves filler stays calm (no banding: trailing
  region single tone, no alternating bands), content-end/strip padding exact,
  shot15/18/19 + status20 geometry intact.
- CHECK: `git status --short` touched paths
  EXPECT: only `crates/gui/src/screenshot.rs` + `checks/gui-exq-hatch25/**`;
  commit message `feat(gui): exq-hatch25 calm trailing fill`, no push.
