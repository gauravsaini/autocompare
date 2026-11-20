# Wave BZ-SELWIRE — GATES (selective filler-hatch live paint wiring)

OWNS: `crates/gui/src/views.rs` + `checks/gui-selwire-bz/**` ONLY.
KEVL: `app.rs` / `home.rs` / `viewmodel.rs` / `views_table.rs` / `theme.rs` / `gutter.rs` / `tree.rs` / `screenshot.rs` / `checks/opencode-waves/STATUS.md` mat chhoona (read bhi grep-proof only, edit MANA). Baaki `checks/**` dirs MANA.
Colors FROZEN (koi naya hue/token nahi — sirf existing `gutter_dim` reuse hatch stroke me; `Color32::from_rgb|from_rgba|Visuals::` add MANA).
Geometry first, typography second (hatch step `TIGHT_GAP` reuse; mono `FontId`/line-height untouched).
MS rust: `forbid(unsafe_code)` kept; non-test me `unwrap/expect/panic/todo` MANA.
NO push (orchestrator pushes). Dusre agent ki files kabhi touch/stage mat karo. `git stash` kabhi nahi. `git add` sirf OWNS (`git add crates/gui/src/views.rs checks/gui-selwire-bz`); KABHI `git add -A` / `git add .` / `git add checks/` mat chalana.

Scope: BU-LOCALRUN2 P0-2 + BX-SELFIX follow-up. Selective palette live paint me pehle se wired hai (`selective_side_palette` wrap-on/wrap-off dono paths me; `selective_span_format`/`selective_find_format` pane jobs me). Sirf filler-hatch helpers contract-only dead the (`filler_hatch_step`, `filler_hatch_count` par `#[allow(dead_code)]`). Is wave ka delta `views.rs` me hai: filler (khaali) side par step-spaced hatch ticks ka minimal paint call-site (default wrap-off path, left+right), geometry `filler_hatch_*` reuse, stroke me existing `gutter_dim` + `INTRA_SPAN_UNDERLINE_W` reuse, TypoRole/mono rhythm untouched. Fixture PNG bytes `screenshot.rs` se bante hain (KEVL, decoupled) isliye same-scene before/after identical rehta hai — visibility proof cross-scene (default vs accent scene) + pixel-scan se hai.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3` (pehle `git status --short` me sirf OWNS dirty ho)
- EXPECT: `0 failed` (passed count note karo, HEAD `git log --oneline -1` REPORT me)
- EVIDENCE: `round-1/baseline-tests.out`

## G2 — OWNS-only diff, KEVL + colors + STATUS untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-selwire-bz/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|home.rs|viewmodel.rs|views_table.rs|theme.rs|gutter.rs|tree.rs|screenshot.rs|STATUS.md" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
  + `git diff --name-only --cached | grep -vE "^(crates/gui/src/views.rs|checks/gui-selwire-bz/)" || echo STAGE-CLEAN`
- EXPECT: OWNS-CLEAN; KEVL-CLEAN; NO-COLOR-ADD; STAGE-CLEAN

## G3 — Hatch wiring live, dead_code gone, unit-tested

- CHECK: `grep -n "allow(dead_code)" crates/gui/src/views.rs | grep -i hatch || echo HATCH-LIVE`
  + `grep -n "paint_filler_hatch" crates/gui/src/views.rs`
  + `cargo test -p autocompare-gui filler`
- EXPECT: HATCH-LIVE (dono hatch `allow(dead_code)` hate); `paint_filler_hatch` def + 2 call-sites (wrap-off left/right); filler tests green
- WHAT: `views.rs` me private `paint_filler_hatch(painter, rect, color)` (loop me `filler_hatch_step`/`filler_hatch_count` reuse, `INTRA_SPAN_UNDERLINE_W` stroke, early-return guards, koi unwrap/panic nahi) + wrap-off row paint me `text_side_is_filler` gate par 2 call-sites (label `Response.rect` reuse = highlight rect alignment) + 1 wiring-contract test

## G4 — Lints clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  + `rustfmt --check crates/gui/src/views.rs`
  + `grep -nE "unwrap\(|expect\(|panic!|todo!" crates/gui/src/views.rs | grep -v "_test\|#[test]\|fn .*test" | grep -v "^\s*//" || echo ZERO-BANNED` (non-test only)
- EXPECT: clippy 0 warnings; fmt 0 diff; ZERO-BANNED

## G5 — Harness PASS + visibility proof (max 6 rounds, round-1 target)

- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-selwire-bz/round-1/`
  + `before.png` (pre-edit binary se) vs `after.png` (wired binary se): `cmp` + python pixel-scan
  + extra `--screenshot after-accent.png` (accent-scene route) pixel-scan
- EXPECT: `RESULT: PASS` (tests green + after.png magic ok + >=8192B + distinct>64);
  same-scene before-vs-after EXPECT identical (decouple invariant: fixture bytes `screenshot.rs`+`viewmodel` se, `views::` paint call nahi hota — BX precedent);
  visibility EXPECT: `after.png` me Changed-row accent block visible (y 176..192, x~597..641, right pane `std::fs::` span) + `after-accent.png` me dono panes me blocks + cross-scene pixel-diff highlight region me nonzero
- EVIDENCE: `round-1/after.png`, `round-1/after-accent.png`, `round-1/before.png`, `round-1/pixel-scan.out`, `round-1/tests.out`, `round-1/geo-typo-report.md`

## G6 — REPORT + commit, no push

- CHECK: `checks/gui-selwire-bz/REPORT.md` (Hinglish-Roman ONLY, Devanagari MANA): wiring table + before/after geometry + visibility proof + follow-up; `git status --short` me sibling files dirty nahi; commit sirf OWNS stage karke
- EXPECT: REPORT me 4 sections; `git log --oneline -1` me `feat(gui): ...` conventional commit; PUSH nahi hua; `checks/opencode-waves/STATUS.md` untouched
