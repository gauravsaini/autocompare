# GATES — wave chrome17 (app-chrome geometry)

OWNS: `crates/gui/src/views.rs` (chrome fns ONLY: header/toolbar/tabstrip/file-bar)
+ `checks/gui-exq-chrome17/**` ONLY. NEVER touch `paint_text_body`,
gutter/marker code, `app.rs`, `theme.rs`, `screenshot.rs`, or any other path.
Colors FROZEN (no `Color32`/`Visuals` delta; repo count must stay 33).

Clean-room: BC behavior described in own words only. No copied strings/icons/assets.

## G1 — Baseline green (pre-edit, recorded 2026-11-21)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-chrome17 CARGO_BUILD_JOBS=2
  tools/exquisite/run.sh checks/gui-exq-chrome17/round-1 crates/gui/src`
  + `cp round-1/after.png round-1/before.png`
- EXPECT: harness `RESULT: PASS`; `after.png` 10449 B, magic ok, distinct 256;
  full suite 1037 passed / 0 failed.
- EVIDENCE: `round-1/exquisite-report.md`, `round-1/tests.out` (this state committed as baseline before edits).

## G2 — OWNS-only diff

- CHECK: `git status --short`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-exq-chrome17/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "paint_text_body" || echo NO-BODY-TOUCH`
  (plus REPORT lists every touched fn by name).
- EXPECT: only `checks/gui-exq-chrome17/**` + `crates/gui/src/views.rs`;
  no `paint_text_body`/gutter-marker/app/theme/screenshot paths.

## G3 — Geometry FIRST (chrome region, token reuse only)

- CHECK: `git diff -- crates/gui/src/views.rs`
- EXPECT: minimal delta, all inside header/toolbar/tabstrip/file-bar fns:
  + tabstrip: `paint_tabs` item spacing `TIGHT_GAP`(4) -> `SECTION_GAP`(8),
    trailing `TIGHT_GAP` -> `TAB_BOTTOM_GAP`(6) (strip rhythm parity);
  + toolbar band: `action_bar_row_gap` `TIGHT_GAP`(4) -> `SECTION_GAP`(8),
    so two-row band = 24+8+24 = 56 (+ surrounding gaps ≈ roomy BC band);
  + header row: `paint_header` item spacing `TIGHT_GAP` -> `SECTION_GAP`;
  + file-bar: NO paint change (row already `HEADER_H` 28, Browse already
    120x28 = row height, i.e. Browse vertically aligned by construction) —
    locked by headless asserts instead (ponytail: don't touch what's at parity);
  + new narrow-720 pure helpers + fixture + headless test (G5);
  + NO new constants (reuse `SECTION_GAP`/`TAB_BOTTOM_GAP`/`HEADER_H` family),
    NO new dependencies.

## G4 — Typography SECOND, minimal

- CHECK: `git diff -- crates/gui/src/views.rs | grep -E "^\+" | grep -E "\.size\(|RichText|font"`
- EXPECT: no new sizes/weights/families; chrome keeps existing
  `chrome_tab_role` (Body 13) / `chrome_section_role` (Heading 18) /
  `chrome_status_role` (Small 11) ladder — already BC-scale from prior wave,
  so typography delta is intentionally empty (parity documented in REPORT).

## G5 — Edge fixture + headless test (narrow 720)

- CHECK: `ls checks/gui-exq-chrome17/round-1/comparison/chrome-narrow-720.json`
  + `cargo test -p autocompare-gui chrome17_narrow720_no_overlap`
- EXPECT: fixture exists (avail 720: toolbar side-air >= 0, strip scroll gates,
  file-bar floors, layout wrap boundary); test green, asserting toolbar fits
  without overlap at 720, tabstrip scrolls instead of squishing, file-bar
  floors hold, 720/719 layout boundary is SideBySide/Stacked.

## G6 — Harness PASS + PNG valid + pixel gate

- CHECK: re-run harness (same command as G1)
  + `python3 -c "d=open('checks/gui-exq-chrome17/round-1/after.png','rb').read();
    print(len(d), d[:8].hex(), len(set(d)))"`
  + `python3 tools/exquisite/pxdiff.py
    checks/gui-exq-chrome17/round-1/before.png
    checks/gui-exq-chrome17/round-1/after.png --expect-diff; echo "pxdiff exit: $?"`
- EXPECT: harness `RESULT: PASS`; PNG magic `89504e470d0a1a0a`, >= 8192 B,
  distinct > 64; pxdiff exit 0 (`DIFFERENT`) with full output pasted below
  and into REPORT.md.
- EVIDENCE (post-run, pasted verbatim):
```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL (expected diff)
pxdiff exit: 2
```
- VERDICT on G6-pixel: FAIL-BLOCKED (structural, not a quality regression).
  Root cause, verified: the harness fixture PNG is painted manually in
  `crates/gui/src/screenshot.rs` (hardcoded tab-bar/pathmeta/toolbar rects);
  no chrome-region helper touched by this wave feeds those pixels. The only
  `views::` geometry input to the fixture is `text_center_gutter_width`
  (20.0), whose value is locked by tests in `screenshot.rs` and whose paint
  sites live inside forbidden `paint_text_body`/gutter code — changing it
  would break the green suite and OWNS. Precedent: exq-paint16 REPORT records
  "No pxdiff-on-after gate this wave (fixture renderer is a separate path)".
  Harness `RESULT: PASS` still holds (tests green + PNG valid); the egui
  chrome delta is real but only visible in the live app, not the fixture.

## G7 — Suite green + fmt + banned-token + colors

- CHECK: `cargo test -p autocompare-gui 2>&1 | grep "test result"`
  + `cargo fmt -p autocompare-gui -- --check; echo "fmt:$?"`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest'
    crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|
    unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `grep -rn "Color32::from_rgb\|Visuals::" crates/gui/src/ | wc -l`
- EXPECT: full suite green (1038 passed / 0 failed: 1037 + 1 new);
  `fmt:0`; ZERO-BANNED in non-test delta; colors exactly 33;
  `#![forbid(unsafe_code)]` intact; no indexing in new non-test code.
