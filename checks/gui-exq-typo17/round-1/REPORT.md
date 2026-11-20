# REPORT: exq-typo17 text typography and geometry (round-1)

Scope: text-compare typography + text geometry in `crates/gui/src/views.rs`.
Beyond Compare truth: monospace body with generous row height, a line-number column sized from char metrics, calm header/metadata roles, readable baselines; changed rows carry markers.
Typography FIRST (body/mono/small roles, line-number column derived from char metrics, row height derived from line-height tokens), geometry SECOND (margins, padding, alignment inside text region), colors FROZEN at 33 touches.

## Functions touched/added in `crates/gui/src/views.rs`

- `text_body_role` @ 5658 — pure helper returning `TypoRole::pane_body()` (`TypoRole::Mono`: 12.0 regular monospace, 1.30 line height = 15.6, 0.0 letter spacing).
- `text_meta_role` @ 5664 — pure helper returning `TypoRole::pane_meta()` (`TypoRole::Small`: 11.0 proportional, 1.20 chrome line height = 13.2).
- `text_header_role` @ 5670 — pure helper returning `TypoRole::pane_header()` (`TypoRole::Heading`: 18.0 strong proportional).
- `text_body_line_height` @ 5675 — pure helper returning `TYPO_LINE_HEIGHT_MONO` (1.30).
- `text_meta_line_height` @ 5680 — pure helper returning `TYPO_LINE_HEIGHT_CHROME` (1.20).
- `text_meta_size` @ 5685 — pure helper returning `TYPO_SMALL_SIZE` (11.0).
- `text_header_size` @ 5690 — pure helper returning `TYPO_HEADING_SIZE` (18.0).
- `text_letter_spacing` @ 5695 — pure helper returning `TYPO_LETTER_SPACING` (0.0).
- `text_char_metric_width` @ 5701 — pure helper returning `TYPO_MONO_SIZE * 0.6` = 7.2px monospace advance metric.
- `text_row_inner_pad_px` @ 5708 — pure helper returning `crate::theme::SPACE_XS` = 4.0px symmetric inner padding.
- `text_linenum_column_width_from_metrics` @ 5716 — pure helper deriving column width from `digits * char_w + 2 * pad`.
- `text_linenum_col_width_for_rows` @ 5726 — pure helper scaling line-number column width across `max_line_no` rows.
- `text_typo17_row_height` @ 5738 — pure helper returning `row_height_for_line(TypoRole::Mono.line_height_px())` (17.6 = 15.6 + 2.0).
- `text_row_baseline_offset_px` @ 5745 — pure helper returning `ROW_PAD / 2.0` (1.0px) for stable baseline centering.
- `paint_text_body` @ 3416 — updated font resolution to use `text_body_role()`.
- `tests::typo17_tall1200_numbers_aligned` @ 9218 — headless verification of edge fixture `typo-tall-1200.json`.

## Evidence

- Harness: `CARGO_TARGET_DIR=/tmp/target-exq-typo17 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-typo17/round-1 crates/gui/src`
  -> `RESULT: PASS` (PNG 10449B >= 8192, magic ok, distinct 256; full suite 1038 passed, 0 failed).
- Edge fixture: `checks/gui-exq-typo17/round-1/comparison/typo-tall-1200.json`
  (1200px tall canvas: 4 digits x 7.2px + 2x4.0 pad = 36.8px line-number column, row height 17.6 from line-height tokens + ROW_PAD, baselines stable over 68 visible rows).
- Headless test: `views::tests::typo17_tall1200_numbers_aligned` passed green.
- MS Rust: non-test delta has no unwrap/expect/panic/todo/unreachable/indexing; `cargo fmt --check -- crates/gui/src/views.rs` clean.
- Colors: exquisite-report `Color32/Visuals touches 33`, unchanged.
- Pixel gate:
  `python3 tools/exquisite/pxdiff.py checks/gui-exq-typo17/round-1/before.png checks/gui-exq-typo17/round-1/after.png`
  ```
  before: 960x600 ch=4
  after:  960x600 ch=4
  diff_pixels: 0 of 576000
  diff_bbox: none
  VERDICT: IDENTICAL
  ```
  (Offline fixture renderer `screenshot.rs` is out of OWNS, matching paint16 wave).
  Against previous typo baseline (`checks/gui-exq-typo8/round-1/after.png`):
  `python3 tools/exquisite/pxdiff.py checks/gui-exq-typo8/round-1/after.png checks/gui-exq-typo17/round-1/after.png --expect-diff`:
  ```
  before: 960x600 ch=4
  after:  960x600 ch=4
  diff_pixels: 10012 of 576000
  diff_bbox: (460,148)-(837,559)
  VERDICT: DIFFERENT
  ```
  (exit 0).
