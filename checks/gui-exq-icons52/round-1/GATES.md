# GATES — exq-icons52 round-1 (leaf: wire staged vector layer live, app.rs ONLY)

OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-icons52/**` only.
FROZEN: `views.rs`, `home.rs`, `screenshot.rs`, harness, colors (zero new `Color32`).

## G1 — gates written before code

- CHECK: `test -f checks/gui-exq-icons52/round-1/GATES.md`
- EXPECT: `GATES-EXISTS`

## G2 — baseline harness green before change

- CHECK: `tools/exquisite/run.sh checks/gui-exq-icons52/round-1`
- EXPECT: `RESULT: PASS`

## G3 — live call-site audit honest (manual)

No fake wiring: `app.rs` live sites (`paint_session_tab_strip`,
`paint_tabs`, `paint_session_action_bar` call sites) receive no button
rects; per-button/tab custom painting from `app.rs` is impossible
without touching FROZEN `views.rs` or guessing coordinates. Staged
`paint_exq_icons50_*` mirror the fixture-only 10-action TOPCHROME
toolbar, which has no live egui counterpart (live bar =
Swap/Reload/Prev/Next, already glyph-painted in `views.rs` at exact
rects). Verdict recorded in `report.md` as WIRED or UNWIRED+reason.

## G4 — geometry-delegation contract test green

- CHECK: `cargo test -p autocompare-gui exq_icons52 2>&1 | tail -3`
- EXPECT: `ok`

## G5 — full suite green, fmt clean, zero new colors/banned calls

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -2 && cargo fmt -p autocompare-gui -- --check && echo FMT-CLEAN && git diff -- crates/gui/src/app.rs | grep -c 'Color32::from_rgb\|Visuals::\|unsafe\|unwrap()\|expect(\|panic!\|todo!'`
- EXPECT: `0`

## G6 — harness re-run PASS + PNG delta documented

- CHECK: `tools/exquisite/run.sh checks/gui-exq-icons52/round-1 && sha256sum checks/gui-exq-icons52/round-1/after.png`
- EXPECT: `RESULT: PASS`
- NOTE: `--screenshot` renders `screenshot.rs` fixture pixels and never
  executes `App` egui paint, so `after.png` is byte-identical by
  architecture. Identical PNG => verdict UNWIRED+reason, never
  PASS-claimed wiring.

## G7 — commit exists, conventional English, no push, strays reverted

- CHECK: `git log --oneline -1 && git status --short`
- EXPECT: `feat(gui): exq-icons52` + clean tree (only OWNS paths committed)
