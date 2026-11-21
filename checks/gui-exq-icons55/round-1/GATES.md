# GATES — exq-icons55 round-1 (leaf: wire staged vector layer live, app.rs ONLY)

OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-icons55/**` only.
FROZEN: `views.rs`, `home.rs`, `screenshot.rs`, harness, colors (zero new `Color32`).

## G1 — gates written before code

- CHECK: `test -f checks/gui-exq-icons55/round-1/GATES.md && echo GATES-EXISTS`
- EXPECT: `GATES-EXISTS`

## G2 — baseline harness green before change

- CHECK: `tools/exquisite/run.sh checks/gui-exq-icons55/round-1`
- EXPECT: `RESULT: PASS`

## G3 — live call-site audit honest (manual)

Document each site in `checks/gui-exq-icons55/round-1/report.md`:
- Toolbar row 1 (`views::paint_session_action_bar` -> `views::paint_toolbar_action_buttons` Swap/Reload): WIRED to staged `crate::app::paint_exq_icons50_mark` (variants 0 & 1 -> `Ring` & `Bars`) through views53 live delegation.
- Toolbar row 2 (Prev/Next): UNWIRED (chevrons `<` / `>` preserved honestly; vector mark kinds lack directional chevrons).
- Mode tabs (`views::paint_tabs`): UNWIRED (text occlusion risk with `selectable_value`).
- Session strip (`views::paint_session_tab_strip`): UNWIRED (text occlusion risk + empty in default compare).
- 10-action TOPCHROME toolbar: N/A (fixture-only, no live egui call site).
No fake wiring in `app.rs`.

## G4 — unit tests proving wired call sites execute vector path green

- CHECK: `cargo test -p autocompare-gui --lib exq_icons55 2>&1 | tail -3`
- EXPECT: `ok`

## G5 — full suite green, fmt clean, zero new colors/banned calls

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -2 && cargo fmt -p autocompare-gui -- --check && echo FMT-CLEAN && git diff -- crates/gui/src/app.rs | grep -c 'Color32::from_rgb\|Visuals::\|unsafe\|unwrap()\|expect(\|panic!\|todo!'`
- EXPECT: `0`

## G6 — harness re-run PASS + kittest real-egui.png delta documented

- CHECK: `tools/exquisite/run.sh checks/gui-exq-icons55/round-1 && test -f checks/gui-exq-icons55/round-1/after.png`
- EXPECT: `RESULT: PASS`

## G7 — commit exists, conventional English, no push, strays reverted

- CHECK: `git log --oneline -1 && git status --short`
- EXPECT: `feat(gui): exq-icons55`
