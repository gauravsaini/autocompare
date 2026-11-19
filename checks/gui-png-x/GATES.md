# Wave X-PNG — GATES (Gap #10: 5x7 screenshot glyph readability)

OWNS: `crates/gui/src/screenshot.rs` + `checks/gui-png-x/**` ONLY.
KEVL `views.rs` / `app.rs` / `tree.rs` / `viewmodel.rs` mat chhoona (read-only, grep-proof only).
Colors FROZEN (koi hue change nahi — BG/FG/DELETED/INSERTED/CHANGED/ACCENT consts untouched).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: `screenshot.rs` bitmap 5x7 glyphs me kerning/proof sudharo taaki headless PNG
text-fixture aur readable ho — glyph advance/kerning pairs, 5x7 bitmap tuning, layout proof.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `257 passed; 0 failed` (recorded 2026-11-19, `main@02adb7c`)
- EVIDENCE: round-1/baseline-tests.out (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/screenshot.rs|checks/gui-png-x/)" || echo OWNS-CLEAN`
  + `git diff -- crates/gui/src/screenshot.rs | grep -E "^-.*(BG_DARK|BG_PANEL|FG_TEXT|FG_DIM|DELETED_|INSERTED_|CHANGED_|ACCENT_TAB|fill_rect.*255\])" || echo NO-COLOR-REMOVAL`
  + `git diff --name-only | grep -E "views.rs|app.rs|tree.rs|viewmodel.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
- EXPECT: only `crates/gui/src/screenshot.rs` + `checks/gui-png-x/**`;
  NO `-` lines on color consts; KEVL-CLEAN; OWNS-CLEAN

## G3 — Glyph advance + kerning + bitmap tuning land, tests green

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `>= 262 passed; 0 failed` (+5 naye glyph-kerning unit tests,
  existing 3 screenshot PNG tests green):
  + `narrow_glyphs_use_tighter_advance`
  + `kern_tight_pairs_tuck_one_pixel`
  + `punctuation_dots_are_single_stem`
  + `text_width_respects_kerning_and_fits_panes`
  + `draw_string_returns_pen_and_clips_safely`
- WHAT: default advance 7→6, narrow glyphs (`i l ! | : ; , . ' I t` + space) advance 4,
  `kern_adjust` -1 on tuck pairs (explicit `AV/LT/To` table + `T/Y/V/r/f` × `a/c/e/o/punct` rule),
  `. , : ;` single-stem bitmaps, unknown-glyph fallback solid-block → `?` outline,
  `draw_string` kerning-aware + pen return, `text_width` layout-proof helper

## G4 — MS Rust guidelines + fmt clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- --check; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/screenshot.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -8 crates/gui/src/screenshot.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or_default` sirf `#[cfg(test)]` me), `#![forbid(unsafe_code)]` kept,
  koi indexing/slicing `[]` nahi (`.get()`/iter only), naye helpers `pub(crate)`-free pure fns

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-png-x/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-png-x/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-png-x/REPORT.md checks/gui-png-x/round-1/after.png checks/gui-png-x/round-1/geo-typo-report.md`
  + `git add crates/gui/src/screenshot.rs checks/gui-png-x/GATES.md checks/gui-png-x/REPORT.md checks/gui-png-x/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
