# BT-STATUS GATES - round-1 (model-only wave; paint BT-DIFF owns)

## Scope

- OWNS: sirf `crates/gui/src/viewmodel.rs` + `checks/gui-status-bt/`. NO-TOUCH: `views.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `home.rs`, `tree.rs`, `app.rs`, `screenshot.rs`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
- Focus: P1-2 (3-zone status bar model) + P1-3 (cursor `1:1` + focused-line preview model), `checks/gui-localrun-bs/REPORT.md` section 5 se. Geometry + typography only, colors frozen.
- Harness `tools/gui-polish/run-geo-typo.sh` (`skills/gui-polish/SKILL.md` loop, max 6 rounds).

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | OWNS ghinlo: `git status --short` | sirf `viewmodel.rs` + `checks/gui-status-bt/` badle; NO-TOUCH files me zero diff; STATUS.md untouched |
| C2 | `cargo test -p autocompare-gui` green | sab pass, 0 failed (naye status/cursor/preview tests samet) |
| C3 | `cargo clippy -p autocompare-gui --all-targets` | zero warnings |
| C4 | rustfmt OWNS only | `rustfmt --check` clean on `viewmodel.rs`; baaki files me zero diff |
| C5 | banned tokens (non-test) | `unwrap(` / `expect(` / `panic!` / `todo!` zero in non-test code |
| C6 | colors frozen | `Color32` / `Visuals` count `git diff` me zero new |
| C7 | harness `run-geo-typo.sh checks/gui-status-bt/round-1/` | `geo-typo-report.md` me `RESULT: PASS` (max 6 rounds) |
| C8 | PNG gates | `after.png` magic-ok + `>= 8192` bytes + distinct `> 64` |
| C9 | REPORT | Hinglish-Roman, Devanagari MANA; model spec + `views.rs` paint spec + follow-up |
| C10 | commit | `feat(gui): ...` local commit, PUSH MANA |

## Result (implement ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| C1 OWNS | mera diff: `viewmodel.rs` +298/-0, `checks/gui-status-bt/` (GATES+REPORT+round-1/); tree me sibling waves ke uncommitted edits (`home.rs`, `screenshot.rs`, `views.rs`) hain, unko touch nahi kiya | PASS |
| C2 tests | `cargo test -p autocompare-gui`: 629 passed, 0 failed (mere 7 naye tests samet) | PASS |
| C3 clippy | meri file se 0 warnings; kul 5 warnings sab `views.rs` me sibling ke uncommitted filler helpers ke (mera scope nahi) | BLOCKED (sibling) |
| C4 rustfmt | `rustfmt --edition 2024 --check` meri file par clean; baaki files me meri taraf se zero diff | PASS |
| C5 banned | non-test me `unwrap(`/`expect(`/`panic!`/`todo!` zero | PASS |
| C6 colors | `git diff` me `Color32`/`Visuals` zero new | PASS |
| C7 harness | `run-geo-typo.sh checks/gui-status-bt/round-1/`: exit 0, `RESULT: PASS` (round 1/6) | PASS |
| C8 PNG | `after.png` 960x600 RGBA, 9765 bytes, magic ok, distinct 256 | PASS |
| C9 REPORT | Hinglish-Roman, non-ASCII zero (Devanagari nahi); model spec + `views.rs` paint spec + follow-up | PASS |
| C10 commit | `feat(gui): ...` local commit, PUSH MANA | commit par PASS |
