# CD-STATUS GATES - round-1 (paint wiring; model BT already landed)

## Scope

- OWNS: sirf `crates/gui/src/views.rs` + `checks/gui-status-cd/`. NO-TOUCH: `app.rs`, `home.rs`, `main.rs`, `viewmodel.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `tree.rs`, `screenshot.rs`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
- Focus: BT `checks/gui-status-bt/REPORT.md` section 5 ka paint spec adopt karna - `paint_text_cursor_strip` + `paint_text_status_bar` + call-site wiring (`paint_text_body` me single counts line ki jagah). Model reuse (`status_zones_for`, `focused_kind_label`, `cursor_label`/`cursor_pos_for`, `focused_line_preview_for`, `diff_nav_count_label` reuse, nav math duplicate nahi). Geometry first (zone alignment/spacing: `HEADER_H` row, `TIGHT_GAP` spacing, `BADGE_PAD` right dock), typo second (`chrome_status_role` labels, `TypoRole::Mono` preview lines), colors FROZEN.
- Harness `tools/gui-polish/run-geo-typo.sh` (`skills/gui-polish/SKILL.md` loop, max 6 rounds).

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | OWNS ghinlo: `git status --short` | sirf `views.rs` + `checks/gui-status-cd/` badle; NO-TOUCH files me zero diff; STATUS.md untouched |
| C2 | `cargo test -p autocompare-gui` green | sab pass, 0 failed (naye wiring-contract tests samet) |
| C3 | `cargo clippy -p autocompare-gui --all-targets` | mere diff se zero warnings |
| C4 | rustfmt OWNS only | `rustfmt --check` clean on `views.rs`; baaki files me zero diff meri taraf se |
| C5 | banned tokens (non-test) | `unwrap(` / `expect(` / `panic!` / `todo!` zero in non-test code |
| C6 | colors frozen | `git diff` me `Color32` / `Visuals` zero new |
| C7 | harness `run-geo-typo.sh checks/gui-status-cd/round-1/` | `geo-typo-report.md` me `RESULT: PASS` (max 6 rounds) |
| C8 | PNG gates | `after.png` magic-ok + `>= 8192` bytes + distinct `> 64` |
| C9 | REPORT | Hinglish-Roman, Devanagari MANA (non-ASCII zero); zone table + geometry + proof + follow-up |
| C10 | commit | `feat(gui): ...` local commit, PUSH MANA |

## Result (implement ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| C1 OWNS | `git status --short`: sirf `M crates/gui/src/views.rs` + `?? checks/gui-status-cd/`; NO-TOUCH zero diff; STATUS.md untouched | PASS |
| C2 tests | `cargo test -p autocompare-gui`: 661 passed, 0 failed (2 naye wiring tests samet) | PASS |
| C3 clippy | `cargo clippy -p autocompare-gui --all-targets`: 0 warnings | PASS |
| C4 rustfmt | `rustfmt --edition 2024 --check crates/gui/src/views.rs`: clean | PASS |
| C5 banned | mere diff me non-test `unwrap(`/`expect(`/`panic!`/`todo!`/`unsafe` zero | PASS |
| C6 colors | `git diff` me `Color32`/`Visuals` zero new | PASS |
| C7 harness | `run-geo-typo.sh checks/gui-status-cd/round-1/`: exit 0, `RESULT: PASS` (round 1/6) | PASS |
| C8 PNG | `after.png` 9765 bytes, magic ok, distinct 256 | PASS |
| C9 REPORT | Hinglish-Roman, non-ASCII zero; zone table + geometry + proof + follow-up | PASS |
| C10 commit | `feat(gui): ...` local commit, PUSH MANA | commit par PASS |
