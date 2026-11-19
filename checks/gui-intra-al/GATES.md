# Wave AL-INTRA — GATES (Gap #11 refinement: intra-line spans)

OWNS: `crates/gui/src/views.rs` + `checks/gui-intra-al/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` /
`home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona
(read bhi avoid karo — model `accent_a/accent_b` AC-SPAN se landed hai, verify-only).
Colors FROZEN: existing palette reuse (row ke apne fg/bg + `span_colors` inversion),
koi naya `Color32` nahi. Geometry first, typography second.
NO `git stash` / `git push` (orchestrator pushes). Dusre agent ki files kabhi mat chhoona.
Clean-room: Beyond Compare bahar se observe only, apne shabdon me.

Scope: Gap #11 ka refinement (AC-SPAN paint half ke upar). AC-SPAN ne
`span_colors` inversion + 1.0 underline lagaya tha. Is wave ka delta `views.rs`
me hai: span rects ko geometry se chipkaya (normalize: sort + clamp + merge),
typography ko TypoRole precedent se lock kiya (same mono FontId/size — emphasis
sirf inversion + 1.0 underline se, line-height untouched).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `ok. 300+ passed; 0 failed` (record karo `round-1/baseline-tests.out` me)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail copy)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-intra-al/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex.rs|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+" | grep -E "Color32::from_rgb|from_rgba_premultiplied|Visuals::|ACCENT_" || echo NO-COLOR-ADD`
- EXPECT: sirf `views.rs` + `checks/gui-intra-al/**`; KEVL-CLEAN; OWNS-CLEAN;
  NO-COLOR-ADD (span bg sirf `span_colors` inversion ka reuse hai)

## G3 — Intra-span refinement unit-tested (geometry + typo)

- CHECK: `cargo test -p autocompare-gui intra_span` + `cargo test -p autocompare-gui span`
- EXPECT: green, kam se kam ye tests:
  + `normalize_intra_spans_sorts_and_merges` (unsorted + overlapping merge hokar ek range)
  + `normalize_intra_spans_clamps_and_drops_empty` (out-of-range clamp, zero-length drop)
  + `pane_segments_refinement_stays_deterministic` (unsorted/overlapping input par bhi wahi 3 segments)
  + `side_job_intra_spans_keep_mono_size_and_underline` (accented + plain ka `FontId` size same,
    accented par underline width `INTRA_SPAN_UNDERLINE_W == 1.0`, plain par none)
  + `intra_spans_visible_on_real_diff_words` (e.g. `hello world` vs `hello earth` jaisi line
    par beech ka badla hissa accented dikhta hai)
  + purane AC-SPAN tests same values par pass (`span_colors_inverts_accent_over_row_strip`,
    `side_job_accents_paint_inverted_block_over_strip`,
    `accent_span_splits_the_pane_into_three_segments`,
    `prefs_syntax_off_clears_accents_but_keeps_text`)
- WHAT: `views.rs` me pure `normalize_intra_spans` + `span_text_format` /
  `INTRA_SPAN_UNDERLINE_W`; `pane_segments` normalize use karta hai;
  `side_job_with_prefs` format helper use karta hai (same row_height, same mono size)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views.rs; git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -15 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi — sirf iterators + `get`;
  koi `format!` allocation naye path me nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-intra-al/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-intra-al/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64 (paint wave — span blocks PNG me dikhte hain)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-intra-al/REPORT.md checks/gui-intra-al/round-1/after.png checks/gui-intra-al/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-intra-al/GATES.md checks/gui-intra-al/REPORT.md checks/gui-intra-al/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
