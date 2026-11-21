# GATES — wave toolbar37 (toolbar geometry)

OWNS: `crates/gui/src/views.rs` (toolbar fns ONLY) + `checks/gui-exq-toolbar37/**`
ONLY. NEVER `crates/gui/src/app.rs`, `copy.rs`, `take.rs`, `tree.rs`,
`home.rs`, `screenshot.rs`, `gutter.rs`, `crates/sessions/**`,
`crates/compare/**`, `crates/diff/**`, `crates/vfs/**`, other gui files.
Colors FROZEN (no `Color32`/`Visuals` delta).

Clean-room: own words only. MS Rust: forbid unsafe, no unwrap/expect/panic/todo
in non-test code, thin client, fmt.

## G1 — Baseline green (pre-edit)

- CHECK: `tools/exquisite/run.sh checks/gui-exq-toolbar37/round-1`
- EXPECT: harness `RESULT: PASS`; PNG magic ok, >= 8192 B, distinct > 64;
  full suite green.
- EVIDENCE: `round-1/exquisite-report.md`, `round-1/tests.out`.

## G2 — OWNS-only diff

- CHECK: `git status --short` + `git diff --name-only`
- EXPECT: only `crates/gui/src/views.rs` + `checks/gui-exq-toolbar37/**`;
  none of the NEVER list touched.

## G3 — Geometry FIRST (toolbar region, SPACE_* reuse only)

- CHECK: `git diff -- crates/gui/src/views.rs`
- EXPECT: minimal delta, all inside toolbar fns:
  + `TOOLBAR_H` = 40.0 = `SPACE_XL`(24) + `SPACE_LG`(16), no new literal scale;
  + button spacing reuse `SPACE_*`: `toolbar_button_spacing()` -> `SPACE_SM`(8),
    icon gap stays `SPACE_XS`(4) via existing `toolbar_icon_gap`;
  + icon+label alignment: glyph 16px slot vertically centered in 24px button
    (`(24-16)/2 = 4` lead parity), locked by `toolbar_icon_label_aligned()`;
  + overflow `>>`: `TOOLBAR_OVERFLOW_GLYPH` + `toolbar_overflow_label()` +
    `toolbar_needs_overflow(avail, needed)` pure gate;
  + 1200 no h-scroll: `toolbar37_fits_1200_gate()` (pair 200 fits 1200-inner,
    `action_bar_needs_hscroll(1200)` false);
  + narrow700 edge: `toolbar37_narrow700_overflow_gate()` (700-inner < pair
    or badge row forces `>>` overflow, never clip);
  + NO new deps, NO colors, NO other-file touches.

## G4 — Typography SECOND, minimal

- CHECK: `git diff -- crates/gui/src/views.rs | grep -E "^\+" | grep -E "\.size\(|RichText|font"`
- EXPECT: no new sizes/weights/families; toolbar keeps `toolbar_button_role`
  (Body 13 chrome) + `status_side_role` (Small 11) ladder.

## G5 — Edge fixture + headless test (narrow700 + 1200)

- CHECK: `ls checks/gui-exq-toolbar37/round-1/comparison/toolbar-narrow-700.json`
  + `ls checks/gui-exq-toolbar37/round-1/comparison/toolbar-1200.json`
  + `cargo test -p autocompare-gui toolbar37`
- EXPECT: fixtures exist (700: overflow true / scroll-or->>; 1200: no h-scroll);
  tests green (`toolbar37_fits_1200_gate`, `toolbar37_narrow700_overflow_gate`,
  spacing/alignment/overflow unit tests).

## G6 — Harness PASS + PNG valid

- CHECK: re-run harness (same command as G1)
  + `python3 -c "d=open('checks/gui-exq-toolbar37/round-1/after.png','rb').read();
    print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness `RESULT: PASS`; PNG magic `89504e470d0a1a0a`, >= 8192 B,
  distinct > 64.

## G7 — Suite green + fmt + banned-token + colors

- CHECK: `cargo test -p autocompare-gui 2>&1 | grep "test result"`
  + `cargo fmt -p autocompare-gui -- --check; echo "fmt:$?"`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest'
    crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|
    unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `grep -rn "Color32::from_rgb\|Visuals::" crates/gui/src/ | wc -l`
- EXPECT: full suite green; `fmt:0`; ZERO-BANNED in non-test code;
  colors unchanged; `#![forbid(unsafe_code)]` intact.
