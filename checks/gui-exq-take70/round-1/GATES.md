# GATES — exq-take70 round-1 (take geometry token adoption)

OWNS: `crates/gui/src/take.rs` + `checks/gui-exq-take70/**` only.
Colors FROZEN: zero new `Color32`, zero palette/Visuals touches in take.rs.

## CHECK: baseline harness PASS (pre-edit)
EXPECT: `tools/exquisite/run.sh checks/gui-exq-take70/round-1` exits 0,
  `exquisite-report.md` contains `RESULT: PASS`, PNG >= 8192B, magic ok, non-blank.
  (Observed PASS before any take.rs edit.)

## CHECK: geometry — tall canvas reuses viewport token, value-identical
EXPECT: `TAKE_TALL_CANVAS_H == crate::theme::VIEWPORT_H as f32 * 2.0`
  (600*2 = 1200.0, exact); no `1200.0` literal left in non-test code;
  headless test asserts `take_tall_canvas_h() == 1200.0` via token math.

## CHECK: geometry — buttons/panes/gaps already token-single-sourced, locked by test
EXPECT: new `exq_take70_tests::take70_geometry_tokens_are_value_identical`
  proves `TAKE_BUTTON_H == SPACE_XL` (24), `TAKE_BUTTON_PAD_X == SPACE_SM` (8),
  `MERGE_SECTION_GAP == GEO_SECTION_GAP` (8), `TAKE_BUTTON_HIT_SLOP == SPACE_XS` (4),
  `TAKE_MINIMAP_MARK_MIN_H == SPACE_XS` (4), `take_bar_gap == GEO_TIGHT_GAP`,
  `take_stepper_gap == GEO_TIGHT_GAP`, `merge_divider_gap == GEO_SECTION_GAP`,
  `take_conflict_badge_inset == SPACE_XS`, `merge_row_height ==
  TypoRole::Mono.line_height_px()` (15.6), `take_button_row_pitch ==
  TAKE_BUTTON_H + GEO_TIGHT_GAP` (28), `take_button_hit_height_with_slop == 32.0`
  via tokens; hit behavior unchanged (hit-rect/guard math untouched).

## CHECK: geometry — remaining literals documented, zero new tokens/sizes
EXPECT: `MERGE_PANE_MIN_W` stays `130.0` literal (no 130-valued SPACE/GEO/CHROME
  token exists; `rg` proves absence) and `TAKE_LABEL_ADVANCE_PX` stays `7.0`
  literal (no 7-valued typo token exists); headless test asserts both values
  plus `MERGE_TOTAL_MIN_W == 3*130 + 2*8 == 406.0`; zero new `pub const`
  tokens, zero `.size(<num>)` literals, zero new weight/line-height consts.

## CHECK: full test suite green + fmt clean + banned 0 + colors frozen
EXPECT: `cargo test -p autocompare-gui` all green; `cargo fmt --check` clean;
  `rg -n "unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe" crates/gui/src/take.rs`
  (non-test) = 0 besides the `forbid(unsafe_code)` header;
  `Color32::from_rgb|Visuals::` count across `crates/gui/src` unchanged vs baseline (35).

## CHECK: harness re-run PASS + kittest DOCUMENTED
EXPECT: harness re-run `RESULT: PASS`; PNG valid; kittest note recorded
  (headless-first per MS guidelines; headed kittest only if a paint path changed —
  none did, geometry values bit-identical).

## CHECK: commit OWNS only, conventional English, NO push + done file
EXPECT: `git status --porcelain` shows only `crates/gui/src/take.rs` +
  `checks/gui-exq-take70/**`; commit `feat(gui): exq-take70 take geometry`
  (English, conventional); no push; `/tmp/wave-take70.done` holds short hash + verdict.

## EVIDENCE (post-run, 2026-11-21)
- Baseline harness pre-edit: RESULT PASS (PNG 10946B, colors 35).
- Post-edit harness re-run: RESULT PASS (PNG 10946B byte-identical size,
  colors 35 unchanged, `.size` literals unchanged — take.rs has zero).
- `cargo test -p autocompare-gui`: 1133 passed, 0 failed (incl. 2 new take70 tests).
- `cargo fmt --check`: clean. Banned scan on take.rs non-test code: only the
  `forbid(unsafe_code)` header line; no unwrap/expect/panic/todo/unreachable/unsafe.
- Token-absence `rg` on theme.rs: no `= 130` token; sole `= 7.0` is
  `WCAG_AAA_NORMAL_TEXT_RATIO: f64` (contrast-ratio domain, not a px token);
  no `1200` literal in theme (VIEWPORT_H is `600u32`, x2 = tall canvas).
- KITTEST: headless-first. No headed/paint kittest run — no paint path changed:
  every geometry value is bit-identical (`600f32*2.0 == 1200.0` exact) and the
  harness PNG is byte-size identical pre/post. Existing edge fixtures
  (merge-3way-narrow, merge-wide-960, take-tall-1200, take-narrow-700,
  take-live-960) keep passing via `include_str!` recompute tests; no new
  fixture needed for a value-identical refactor.
