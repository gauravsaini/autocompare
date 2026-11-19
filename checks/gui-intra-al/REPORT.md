# Wave AL-INTRA — REPORT (Gap #11 refinement: intra-line spans)

Scope: Gap #11 ka refinement, AC-SPAN paint half ke upar. AC-SPAN ne
`span_colors` inversion + underline lagaya tha. Is wave ka delta sirf
`crates/gui/src/views.rs` me hai: span rects geometry se chipkaye
(normalize: sort + clamp + merge), typography TypoRole precedent se lock
ki (same mono FontId/size — zor sirf inversion + 1.0 underline se,
line-height untouched). Colors frozen — koi naya `Color32` nahi.

## Delta (views.rs only)

1. **Pure `normalize_intra_spans(spans, line_len, pane)`**: limit
   `min(line_len, pane)`; zero-length/bahar ranges drop, sort, phir
   overlapping/aas-paas (`next.start <= cur.end`) merge — ek hi paint
   block. Koi indexing nahi (iterators + `last_mut` + `sort`), koi
   `format!` allocation nahi.
2. **`pane_segments` normalize use karta hai**: unsorted/overlapping
   model spans bhi deterministic paint dete hain; signature same.
3. **Pure `span_text_format(font_id, fg, bg, accented)` + token
   `INTRA_SPAN_UNDERLINE_W = 1.0`**: same mono size (line-height
   untouched), background `span_colors` inversion se, accented par 1.0
   underline. TypoRole precedent: chrome `strong` bina size badle zor
   deta hai; mono rows me zor inversion + underline se hai.
   `side_job_with_prefs` loop isi helper se paint karta hai.
4. **6 naye tests**: `normalize_intra_spans_sorts_and_merges`,
   `normalize_intra_spans_clamps_and_drops_empty`,
   `pane_segments_refinement_stays_deterministic`,
   `span_text_format_keeps_mono_size_with_accent_underline`,
   `side_job_intra_spans_keep_mono_size_and_underline`,
   `intra_spans_visible_on_real_diff_words` (`hello world` me `world`
   accented + underline — real diffs par dikhta hai).

## Gates evidence

- G1 baseline: `round-1/baseline-tests.out` — 378 passed, 0 failed (pre-edit).
- G2 OWNS-only: staged diff me sirf `crates/gui/src/views.rs` +
  `checks/gui-intra-al/**` (staged check: `git diff --cached --name-only`).
  Working tree me `crates/gui/src/app.rs` bhi Modified dikhta hai — wo
  sibling wave AL-MERGEAPP ka in-progress kaam hai (same checkout, parallel),
  usko chhooa nahi, stage nahi kiya. `views.rs` `+` lines me
  `Color32::from_rgb`/`from_rgba`/`Visuals::`/`ACCENT_` nahi — NO-COLOR-ADD.
- G3: `cargo test -p autocompare-gui` → 390 passed (378 + 6 is wave +
  6 sibling wave ke), 0 failed. `intra_span` filter 4/4, `span` filter
  10/10, `refinement` 1/1; purane AC-SPAN tests same values par pass.
- G4: `rustfmt --check` → fmt:0 (OWNS file par `cargo fmt -- <file>`);
  non-test code me `unwrap(`/`expect(`/`panic!`/`todo!` nahi — ZERO-BANNED;
  `forbid(unsafe_code)` kept. Crate-level
  `cargo clippy --all-targets -- -D warnings` abhi sibling ki `app.rs`
  test code par 2 `field_reassign_with_default` errors deta hai (KEVL,
  in-progress, untouched) — `views.rs` ka koi clippy error/warning nahi
  (output me `views.rs` 0 baar).
- G5: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-intra-al/round-1`
  → `RESULT: PASS`; `after.png` magic `89504e470d0a1a0a`, 9624 bytes
  (>= 8192), distinct 256 (> 64).
- G6: artifacts `REPORT.md` + `round-1/` (after.png,
  geo-typo-report.md, tests.out, baseline-tests.out, screenshot.out);
  local commit, NO push.

## Artifacts

- `checks/gui-intra-al/GATES.md` (plan)
- `checks/gui-intra-al/REPORT.md` (ye file)
- `checks/gui-intra-al/round-1/after.png` (headless proof)
- `checks/gui-intra-al/round-1/geo-typo-report.md` (harness)
- `checks/gui-intra-al/round-1/tests.out` (390 passed tail)
- `checks/gui-intra-al/round-1/baseline-tests.out` (pre-edit 378)
- `checks/gui-intra-al/round-1/screenshot.out` (harness)

## Notes

- Equal rows (`TRANSPARENT` bg) par inversion ka edge matter nahi karta:
  accents sirf diff rows par bante hain; helper phir bhi pure swap hai.
- Underline 1px same line-height ke andar rehta hai — row rhythm
  (`row_height_for_line`) untouched, isliye geometry-first DoD poora.
- Sibling wave ke commit ke baad crate-level clippy dobara green hona
  chahiye; us waqt `views.rs` me koi badlav ki zaroorat nahi hai.
