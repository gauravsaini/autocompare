# GATES — exq-take44 round-1 (live merge take resolution)

## CHECK: baseline harness PASS
EXPECT: `tools/exquisite/run.sh checks/gui-exq-take44/round-1` exits 0,
  `exquisite-report.md` contains `RESULT: PASS`, PNG >= 8192B, magic ok, non-blank.
  (Observed PASS before geometry edit.)

## CHECK: geometry — resolved row y delegates to marker row y
EXPECT: new `take_resolved_row_y(row) == take_marker_row_y(row)` for all rows
  (no new literals, no new arithmetic); OOB rows flow through marker None-band
  logic; headless test asserts y0=0, y10=156.0, y76=1185.6.

## CHECK: geometry — live output height delegates to conflict block height
EXPECT: new `take_live_output_height_px(n) == take_conflict_block_height_px(n)`
  (same Mono stack single source); 0 -> 0.0, 1 -> 15.6, 4 -> 62.4;
  headless test proves equality for 0/1/4/76.

## CHECK: geometry — live resolution fraction + bar fill width
EXPECT: new `take_resolution_fraction(resolved, total)` in [0,1]:
  total 0 -> 1.0 (nothing to resolve), resolved >= total -> 1.0,
  else resolved/total as f32; new `take_resolution_bar_width(avail_w, r, t)`
  = avail_w * fraction, non-finite/non-positive avail -> 0.0;
  headless test asserts 0/2=0.0, 1/2=0.5, 2/2=1.0, 0/0=1.0, 960@1/2=480.0.

## CHECK: geometry — 960px live fixture + narrow360 edge
EXPECT: fixture `comparison/take-live-960.json` (avail_w 960, pane thirds
  314.666656, bar total 287, row_height 15.6, live heights, fraction fills)
  + narrow edge `avail_w 360 -> panes 130 floor, hscroll true`;
  headless test `take_exq_take44_live_take_geometry` `include_str!` recompute match.

## CHECK: typography — TypoRole reuse only
EXPECT: new `take_resolution_text_role() == take_counter_text_role() ==
  TypoRole::Small` (status precedent, same single source), size 11.0 via
  `.size()` delegation; zero `.size(<num>)` literals and zero new
  weight/line-height/letter-spacing consts in take44 block.

## CHECK: harness re-run PASS + banned 0
EXPECT: harness re-run `RESULT: PASS`; `cargo fmt --check` clean;
  `rg -n "unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe"` on new block = 0
  (indexing-free: delegation + saturating/match only);
  `Color32::from_rgb|Visuals::` count unchanged (38);
  OWNS only `crates/gui/src/take.rs` + `checks/gui-exq-take44/**`.
