# GATES — exq-geo55 round-1 (leaf: gutter geometry parity, crates/gui/src/gutter.rs ONLY)

OWNS: `crates/gui/src/gutter.rs` + `checks/gui-exq-geo55/**` only.
FROZEN: `views.rs`, `app.rs`, `home.rs`, `screenshot.rs`, harness, colors (zero new `Color32`).

Scope: Clean-room Beyond Compare 5 gutter geometry parity in `crates/gui/src/gutter.rs`.
Tighten gutter width, alignment, and padding across narrow (<720px, e.g. 700px),
normal (960px), and wide (1200px) canvases so text panes and gutters never clip
or double-pad. Single-source width helpers delegate to existing `SPACE_*` and
`GEO_OUTER_MARGIN` constants. Alignment enforces right-aligned tabular numbers
and centered markers/chevrons. Includes edge fixture `gutter-narrow-700.json`
with headless verification. Zero new colors/tokens/sizes, zero typography edits.

## G1 — gates written before code

- CHECK: `test -f checks/gui-exq-geo55/round-1/GATES.md && echo GATES-EXISTS`
- EXPECT: `GATES-EXISTS`

## G2 — baseline harness green before change

- CHECK: `tools/exquisite/run.sh checks/gui-exq-geo55/round-1`
- EXPECT: `RESULT: PASS`

## G3 — pure gutter geometry helpers (budget, alignment, padding)

- CHECK: `cargo test -p autocompare-gui exq_geo55 2>&1 | tail -3`
- EXPECT: `ok`

## G4 — narrow-700 edge fixture with headless test

- CHECK: `test -f checks/gui-exq-geo55/round-1/comparison/gutter-narrow-700.json && cargo test -p autocompare-gui geo55_narrow_700_edge_fixture 2>&1 | tail -3`
- EXPECT: `ok`

## G5 — full suite green, fmt clean, zero new colors/banned calls

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -2 && cargo fmt -p autocompare-gui -- --check && echo FMT-CLEAN && (git diff -- crates/gui/src/gutter.rs | grep -c 'Color32::from_rgb\|Visuals::\|unsafe\|unwrap()\|expect(\|panic!\|todo!' || true)`
- EXPECT: `0`

## G6 — harness re-run PASS + exquisite-report.md signal moves documented

- CHECK: `tools/exquisite/run.sh checks/gui-exq-geo55/round-1 && test -f checks/gui-exq-geo55/round-1/after.png`
- EXPECT: `RESULT: PASS`

## G7 — commit exists, conventional English, no push, strays reverted

- CHECK: `git log --oneline -1 && git status --short`
- EXPECT: `feat(gui): exq-geo55`
