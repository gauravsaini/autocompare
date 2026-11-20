# GATES — exq-take33 round-1 (live merge take)

## CHECK: baseline harness PASS
EXPECT: `tools/exquisite/run.sh checks/gui-exq-take33/round-1` exits 0,
  `exquisite-report.md` contains `RESULT: PASS`, PNG >= 8192B, magic ok, non-blank.
  (Observed PASS before geometry edit.)

## CHECK: geometry — take buttons row reuses SPACE_* only
EXPECT: new `take_buttons_row_x(i)` = `i * (take_bar_button_width + take_bar_gap)`;
  `take_bar_gap == GEO_TIGHT_GAP == SPACE_XS`; no numeric gap literal in new code.
  Headless test asserts x0=0, x1=97, x2=194, OOB index -> None.

## CHECK: geometry — 3-way center pane alignment
EXPECT: new `merge_center_pane_x(avail_w)` = left_w + divider_gap from
  `merge_three_pane_widths`/`merge_divider_gap` (no new literals);
  `merge_center_pane_aligned(1200)` true (center mid == avail/2 ±0.01);
  `merge_three_panes_fill_exact(1200)` true.

## CHECK: geometry — conflict badge placement
EXPECT: new `take_conflict_badge_inset() == SPACE_XS` (4.0);
  `take_conflict_badge_pos(row, pane_x, pane_w)` = trailing-inset x + marker row y,
  `None` on OOB row / non-finite input; headless test proves badge inside pane.

## CHECK: geometry — 1200px no h-scroll + narrow700 edge
EXPECT: `!merge_three_pane_needs_hscroll(1200)` and `!take_bar_needs_hscroll(1200)`;
  `!merge_three_pane_needs_hscroll(700)`; fixture
  `comparison/take-narrow-700.json` (avail_w 700, pane widths, center x,
  bar total 287, both needs_hscroll false) + headless test `include_str!` recompute match.

## CHECK: typography — TypoRole reuse only
EXPECT: new `take_conflict_badge_text_role() == TypoRole::Small` (status precedent,
  same source as `take_counter_text_role`), size 11.0 via `.size()`; zero `.size(`
  literals and zero new weight/line-height/letter-spacing consts in take33 block.

## CHECK: harness re-run PASS + banned 0
EXPECT: harness re-run `RESULT: PASS`; `cargo fmt --check` clean;
  `rg -n "unwrap\(|expect\(|panic!|todo!|unimplemented!|\[.+\]"` on new block = 0
  (indexing-free: `.get()`/pattern only); `Color32::from_rgb|Visuals::` count unchanged (38);
  OWNS only `crates/gui/src/take.rs` + `checks/gui-exq-take33/**`.
