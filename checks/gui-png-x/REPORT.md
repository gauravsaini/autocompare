# Wave X-PNG — REPORT (Gap #10: 5x7 screenshot glyph readability)

OWNS: `crates/gui/src/screenshot.rs` + `checks/gui-png-x/**` ONLY.
KEVL (`views.rs`/`app.rs`/`tree.rs`/`viewmodel.rs`) untouched. Colors frozen. NO push.

## Kya badla (screenshot.rs only)

1. **Advance table** — `glyph_advance()`: default 7→6px, narrow/single-stem glyphs
   (`space ' , . : ; ! I i j l t | \``) 4px. Fixture rows pane me bina clip hue fit hote hain.
2. **Kerning** — `kern_adjust(prev, curr)`: tuck pairs par -1px (explicit
   `AV/AW/LT/LV/LY/PA/To/Te/Ta/VA/YV` + general rule `T/Y/V/r/f` × `a/c/e/o/punct`).
   Kabhi + nahi, max 1px tuck — overlap impossible.
3. **Bitmap tuning** — `. , : ;` single-stem (purane doubled blocks hataye;
   `,`/`;` me 1px descender tail), unknown-byte fallback solid-block → `?` outline.
4. **Layout proof** — `text_width()` + `draw_string()` ab kerning-aware pen return karta hai;
   fixture strings (path field 380px, toolbar/status 928px) ke fit-bounds unit tests me pin hain.
5. **5 naye unit tests**: narrow advance, kern pairs, single-stem dots,
   text-width fits, pen-return + edge-clip safety. Purane 3 PNG tests green.

## Gate evidence

- G1 baseline: `257 passed; 0 failed` (`round-1/baseline-tests.out`)
- G2 owns-only: `git diff --name-only` me sirf `crates/gui/src/screenshot.rs`
  (+ untracked `checks/gui-png-x/`); KEVL clean; color consts me koi `-` line nahi
- G3 tests: `271 passed; 0 failed` (257 me se 5 mere + 9 sibling TREE/tests, sab green)
- G4: `clippy --all-targets -- -D warnings` zero warnings (byte_char_slices fix:
  `*b"..."` iteration), `cargo fmt --check` OK, non-test me ZERO-BANNED
  (`unwrap(/expect(/panic!/todo!` nahi), `#![forbid(unsafe_code)]` kept
- G5 harness: `run-geo-typo.sh checks/gui-png-x/round-1` → exit 0, `RESULT: PASS`;
  `after.png` 9624 bytes (≥8192 ok), magic `89504e470d0a1a0a` ok, distinct 256 (>64 ok)
- G6: ye REPORT + `round-1/` artifacts (after.png, geo-typo-report.md,
  screenshot.out, tests.out, baseline-tests.out); conventional commit, NO push

## Artifacts

- `checks/gui-png-x/GATES.md` — gates (pehle likha)
- `checks/gui-png-x/REPORT.md` — ye file
- `checks/gui-png-x/round-1/` — harness outputs + baseline tail
