# BW-SES-APP Gates (CHECK / EXPECT)

Source: `checks/gui-sessionplan-bu/PLAN.md` section 4 + section 6 Wave BU-SES-APP
(line 165+), dir `bu` se `bw` me map (sibling `bu` run se takrao nahi).
DEPENDENCY SATISFIED: MODEL API landed (commit 334dee5, `home.rs`
Session/SessionList+T1-T7). `home.rs` READ-ONLY use, usme change MANA.

## G1 — bridge tests green

- CHECK `cargo test -p autocompare-gui bridge -- --nocapture`
- EXPECT ok, naye tests: derived tab khaali=Home, set_tab switch-vs-open,
  close neighbour rule (left, na ho to right; aakhri band par khaali=Home),
  type-ke-baad-save (T5) + cursor-save (T6), open_home_session live session
  banata hai, cap/drop (9th open par sabse purani INACTIVE gire, active rahe).

## G2 — full suite green (legacy compat)

- CHECK `cargo test -p autocompare-gui`
- EXPECT 0 failed; purane tests (`switching_tabs_preserves_paths`,
  `home_session_open_records_recent`, restore round-trip) bina badle PASS.

## G3 — clippy zero

- CHECK `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
- EXPECT zero warnings.

## G4 — fmt clean

- CHECK `cargo fmt --check -p autocompare-gui`
- EXPECT clean.

## G5 — banned pattern non-test zero

- CHECK `rg 'unwrap|expect\(|panic!|todo!|unsafe' crates/gui/src/app.rs`
- EXPECT sirf `#[cfg(test)]` block + pehle se maujood
  `#![forbid(unsafe_code)]` line; non-test code me zero.

## G6 — fixtures byte-stable (paint untouched)

- CHECK harness + `git status --short`
- EXPECT `tools/gui-polish/run-geo-typo.sh checks/gui-sessionapp-bw/round-1/`
  `RESULT: PASS`; Default screenshot bytes byte-same (is wave me paint
  change nahi, sirf App bridge); farq ho to REPORT me justify.

## G7 — OWNS-only dirt

- CHECK `git status --short`
- EXPECT sirf `M crates/gui/src/app.rs` + untracked
  `checks/gui-sessionapp-bw/`. KEVL NO-TOUCH: `home.rs`, `views.rs`,
  `viewmodel.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `tree.rs`,
  `screenshot.rs`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
