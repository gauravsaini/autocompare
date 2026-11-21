# GATES — exq-icons90 round-1 (toolbar vector-icon geometry + active tab pill, views.rs ONLY)

OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-icons90/**` ONLY.
FORBIDDEN: `crates/gui/src/theme.rs` `crates/gui/src/app.rs` `crates/gui/src/main.rs` `crates/gui/src/screenshot.rs`, `checks/gui-sessionpaint-by/**`, `crates/sessions/**`, `unsafe`.
FROZEN: colors (zero new `Color32::from_rgb`/`Visuals::`), typography (zero new sizes/weights).
Base: 71bd65d. Worktree: /tmp/wt-icons90.

## G1 — gates written before code

- CHECK: `test -f checks/gui-exq-icons90/round-1/GATES.md && echo GATES-EXISTS`
- EXPECT: `GATES-EXISTS`

## G2 — baseline views tests green before change

- CHECK: `cargo test -p autocompare-gui --lib views 2>&1 | tail -2`
- EXPECT: `395 passed`

## G3 — toolbar glyph paint uses vector path geometry, zero fill_rect in paint path

- CHECK: `cargo test -p autocompare-gui --lib icons90_toolbar_vector 2>&1 | tail -3`
- EXPECT: `ok`

## G4 — active tab pill geometry via existing SPACE_* tokens, dim/track reuse

- CHECK: `cargo test -p autocompare-gui --lib icons90_tab_pill 2>&1 | tail -3`
- EXPECT: `ok`

## G5 — full gui suite green, fmt clean, banned 0 non-test

- CHECK: `cargo test -p autocompare-gui --lib views 2>&1 | tail -2 && cargo fmt -p autocompare-gui -- --check && echo FMT-CLEAN && git diff -U0 -- crates/gui/src/views.rs | grep '^+' | grep -v '^+++' | grep -cE 'unsafe|unwrap\(|expect\(|panic!|todo!|unreachable!'`
- EXPECT: `0`

## G6 — harness PASS + PNG valid

- CHECK: `tools/exquisite/run.sh checks/gui-exq-icons90/round-1 2>&1 | tail -5`
- EXPECT: `RESULT: PASS`

## G7 — commit exists, English conventional, no push, strays reverted

- CHECK: `git log --oneline -1 && git status --short | head -5 && git push --dry-run 2>&1 | head -2; echo NO-PUSH-DONE`
- EXPECT: `feat(gui): exq-icons90`
