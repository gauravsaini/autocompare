# GATES — exq-views57 round-1 (leaf: live tabs/session-strip polish, views.rs ONLY)

OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-views57/**` only.
FROZEN: `app.rs`, `home.rs`, `screenshot.rs`, `theme.rs`, harness, colors (zero new `Color32`).

## G1 — gates written before code

- CHECK: `test -f checks/gui-exq-views57/round-1/GATES.md && echo GATES-EXISTS`
- EXPECT: `GATES-EXISTS`

## G2 — baseline harness green before change

- CHECK: `tools/exquisite/run.sh checks/gui-exq-views57/round-1`
- EXPECT: `RESULT: PASS`

## G3 — live tabs wiring audit honest (manual)

Document each site in `checks/gui-exq-views57/round-1/report.md`:
Mode tabs (`paint_tabs`) and session tab strip (`paint_session_tab_strip` / `paint_session_tab`).
Adopt staged vector tab pill / underline geometry in `views.rs` without text occlusion risk.
Keep tab hit-rects + interactions byte-identical (painting only).
Record WIRED vs UNWIRED with reason + real-egui.png pixel proof.

## G4 — unit tests proving wired vector path green

- CHECK: `cargo test -p autocompare-gui --lib exq_views57 2>&1 | tail -3`
- EXPECT: `ok`

## G5 — full suite green, fmt clean, zero new colors/banned calls

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -2 && cargo fmt -p autocompare-gui -- --check && echo FMT-CLEAN && git diff -- crates/gui/src/views.rs | grep -c 'Color32::from_rgb\|Visuals::\|unsafe\|unwrap()\|expect(\|panic!\|todo!'`
- EXPECT: `0`

## G6 — harness re-run PASS + kittest real-egui.png delta documented

- CHECK: `tools/exquisite/run.sh checks/gui-exq-views57/round-1 && test -f checks/gui-exq-views57/round-1/after.png`
- EXPECT: `RESULT: PASS`

## G7 — commit exists, conventional English, no push, strays reverted

- CHECK: `git log --oneline -1 && git status --short`
- EXPECT: `feat(gui): exq-views57`
