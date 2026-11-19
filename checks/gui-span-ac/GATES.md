# Wave AC-SPAN — GATES (Gap #11 paint half: intra-line spans)

OWNS: `crates/gui/src/views.rs` + `checks/gui-span-ac/**` ONLY.
KEVL `app.rs` / `tree.rs` / `viewmodel.rs` / `views_table.rs` / `views_hex.rs` /
`gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona
(read-only, grep-proof only — model `accent_a/accent_b` pehle se landed hai).
Colors FROZEN (koi hue/token change nahi — sirf row ke apne frozen pair ka reuse).
Geometry first, typography second (koi naya token/size nahi — same mono FontId,
same row heights, same `expand_bg`).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
Clean-room: Beyond Compare bahar se observe only, apne shabdon me.

Scope: Gap #11 ka paint half. Model half pehle se landed hai (`viewmodel.rs`
`accent_a/accent_b` + `pane_segments` accents — KEVL, verify-only). Is wave ka
delta `views.rs` me hai: `side_job_with_prefs` accented spans ko poori-row strip
ke upar alag block me paint karega — per-span background (row pair ka inversion)
+ weight (inverted block + underline), taaki changed words/chars line ke andar
alag dikhen.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `ok. 296+ passed; 0 failed` (recorded 2026-11-20, `main`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-span-ac/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|tree.rs|viewmodel.rs|views_table.rs|views_hex.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+" | grep -E "Color32::from_rgb|from_rgba|Visuals::" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `checks/gui-span-ac/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (span bg sirf row ke apne fg/bg ka
  inversion hai — koi naya hue/token nahi)

## G3 — Per-span background/weight unit-tested

- CHECK: `cargo test -p autocompare-gui span`
- EXPECT: green, kam se kam ye tests:
  + `span_colors_inverts_accent_over_row_strip` (plain par passthrough `(fg, bg)`;
    accented par inversion `(bg, fg)` — strip ke upar alag block)
  + `side_job_accents_paint_inverted_block_over_strip` (accented section ka
    background row bg se alag hai, color plain se alag hai, underline barkarar)
  + purane span tests same values par pass (`accent_span_splits_the_pane_into_three_segments`,
    `accents_at_line_start_and_truncation_are_handled`,
    `prefs_syntax_off_clears_accents_but_keeps_text`)
- WHAT: `views.rs` me pure `span_colors(fg, bg, accented)` helper +
  `side_job_with_prefs` loop usko use karta hai; `pane_segments` model untouched

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views.rs && git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -12 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naya helper pure fn (koi indexing nahi;
  `format!` allocation nahi — sirf tuple swap)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-span-ac/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-span-ac/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64 (paint wave — span blocks PNG me dikhte hain)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-span-ac/REPORT.md checks/gui-span-ac/round-1/after.png checks/gui-span-ac/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-span-ac/GATES.md checks/gui-span-ac/REPORT.md checks/gui-span-ac/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
