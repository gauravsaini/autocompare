# GATES — exq-views53 round-1 (leaf: make live paint move pixels, views.rs ONLY)

OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-views53/**` only.
FROZEN: `app.rs`, `home.rs`, `screenshot.rs`, harness, colors (zero new `Color32`).

## G1 — gates written before code

- CHECK: `test -f checks/gui-exq-views53/round-1/GATES.md && echo GATES-EXISTS`
- EXPECT: `GATES-EXISTS`

## G2 — baseline harness green before change

- CHECK: `tools/exquisite/run.sh checks/gui-exq-views53/round-1`
- EXPECT: `RESULT: PASS`

## G3 — live paint wiring audit honest (manual)

Document each site in `checks/gui-exq-views53/round-1/report.md`:
Toolbar buttons (Swap/Reload/Prev/Next) and tabs (session strip/mode tabs).
Adopt staged vector mark/tab-pill geometry in `views.rs` without interaction risk.
Record WIRED vs UNWIRED with reason + real-egui.png pixel proof.

## G4 — unit tests proving wired vector path green

- CHECK: `cargo test -p autocompare-gui exq_views53 2>&1 | tail -3`
- EXPECT: `ok`

## G5 — full suite green, fmt clean, zero new colors/banned calls

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -2 && cargo fmt -p autocompare-gui -- --check && echo FMT-CLEAN && git diff -- crates/gui/src/views.rs | grep -c 'Color32::from_rgb\|Visuals::\|unsafe\|unwrap()\|expect(\|panic!\|todo!'`
- EXPECT: `0`

## G6 — harness re-run PASS + kittest real-egui.png delta documented

- CHECK: `tools/exquisite/run.sh checks/gui-exq-views53/round-1 && test -f checks/gui-exq-views53/round-1/after.png`
- EXPECT: `RESULT: PASS`

## G7 — commit exists, conventional English, no push, strays reverted

- CHECK: `git log --oneline -1 && git status --short`
- EXPECT: `feat(gui): exq-views53`
