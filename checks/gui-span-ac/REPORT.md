# Wave AC-SPAN — REPORT (Gap #11 paint half: intra-line spans)

Scope: Gap #11 ka paint half. Model half pehle se landed tha
(`viewmodel.rs` `accent_a`/`accent_b` + `pane_segments` accents —
KEVL, sirf verify kiya, chhooa nahi). Is wave ka delta sirf
`crates/gui/src/views.rs` me hai: `side_job_with_prefs` ab accented
spans ko poori-row strip ke upar alag block me paint karta hai —
per-span background inversion + underline, same mono FontId, same
row heights, koi naya hue/token nahi.

## Delta (views.rs only)

1. **Pure helper `span_colors(fg, bg, accented)`**: plain par
   passthrough `(fg, bg)`; accented par inversion `(bg, fg)`. Sirf
   tuple swap — koi indexing nahi, koi `format!` allocation nahi.
2. **`side_job_with_prefs` loop** usko use karta hai; accented
   section par `Stroke::new(1.0, color)` underline barkarar.
   Purana `ACCENT_FG` token path hata — span bg ab row ke apne
   frozen pair ka reuse hai (colors frozen).
3. **`#![forbid(unsafe_code)]`** file top par (baki gui modules
   jaisa; MS rust guideline).
4. **2 naye tests**: `span_colors_inverts_accent_over_row_strip`,
   `side_job_accents_paint_inverted_block_over_strip` (iterator +
   `assert_eq!` style — workspace lints `unwrap_used`/`expect_used`/
   `panic`/`indexing_slicing` deny ko satisfy karta hai).

## Gates evidence

- G1 baseline: `round-1/baseline-tests.out` — 299 passed, 0 failed.
- G2 OWNS-only: `git status --short` me sirf `crates/gui/src/views.rs`
  (M) + `checks/gui-span-ac/` (untracked artifacts); KEVL files
  (`app.rs`/`tree.rs`/`viewmodel.rs`/`views_table.rs`/`views_hex.rs`/
  `gutter.rs`/`theme.rs`/`screenshot.rs`) untouched — KEVL-CLEAN;
  `^+` lines me `Color32::from_rgb`/`from_rgba`/`Visuals::` nahi —
  NO-COLOR-ADD.
- G3: `cargo test -p autocompare-gui` → 301 passed (299 + 2 naye),
  0 failed. `span` filter me `span_colors_...` ok; `side_job` filter
  me `side_job_accents_...` ok; purane span tests same values par
  pass (`accent_span_splits_the_pane_into_three_segments`,
  `accents_at_line_start_and_truncation_are_handled`,
  `prefs_syntax_off_clears_accents_but_keeps_text`).
- G4: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  → clippy:0 zero warnings; `rustfmt --check` → fmt:0; non-test
  code me `unwrap(`/`expect(` /`panic!`/`todo!` nahi — ZERO-BANNED;
  `UNSAFE-FORBID-OK`.
- G5: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-span-ac/round-1`
  → `RESULT: PASS`; `after.png` magic `89504e470d0a1a0a`, 9624
  bytes (>= 8192), distinct 256 (> 64).
- G6: artifacts `REPORT.md` + `round-1/` (after.png,
  geo-typo-report.md, tests.out, baseline-tests.out); local commit,
  NO push.

## Artifacts

- `checks/gui-span-ac/GATES.md` (plan)
- `checks/gui-span-ac/REPORT.md` (ye file)
- `checks/gui-span-ac/round-1/after.png` (headless proof)
- `checks/gui-span-ac/round-1/geo-typo-report.md` (harness)
- `checks/gui-span-ac/round-1/tests.out` (301 passed tail)
- `checks/gui-span-ac/round-1/baseline-tests.out` (pre-edit 299)

## Notes

- Purane session ka 1-line diff (`ACCENT_FG` import cleanup) rakha;
  usne body me 2 dangling `ACCENT_FG` use chhode the (tree red tha)
  — is wave ne use hata ke inversion lagaya, tree green.
- Equal rows (`TRANSPARENT` bg) par inversion ka edge model me
  matter nahi karta: accents sirf diff rows par bante hain; helper
  phir bhi pure swap hai, koi special-case nahi.
