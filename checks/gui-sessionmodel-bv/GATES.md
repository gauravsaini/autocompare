# BV-SES-MODEL Gates (CHECK / EXPECT)

Source: `checks/gui-sessionplan-bu/PLAN.md` section 6 (Wave BU-SES-MODEL),
dir `bu` se `bv` me map (sibling `bu` run se takrao nahi).

## G1 — session tests green

- CHECK `cargo test -p autocompare-gui session`
- EXPECT ok, naye tests: open / close / switch / neighbour (left, na ho to
  right) / cap drop-oldest (active kabhi nahi) / unknown-id no-op / blank
  rename no-op / Home never-session / title fallback / id reuse nahi.

## G2 — clippy zero

- CHECK `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  (gate se sakht: tests samet)
- EXPECT zero warnings.

## G3 — fmt clean

- CHECK `cargo fmt --check -p autocompare-gui`
- EXPECT clean.

## G4 — banned pattern non-test zero

- CHECK `rg 'unwrap|expect\(|panic!|todo!|unsafe' crates/gui/src/home.rs`
- EXPECT sirf test-mod hits + pehle se maujood `#![forbid(unsafe_code)]`
  (line 5); non-test code me zero.

## G5 — OWNS-only dirt

- CHECK `git status --short`
- EXPECT sirf OWNS files: `M crates/gui/src/home.rs` + untracked
  `checks/gui-sessionmodel-bv/`. KEVL NO-TOUCH: `app.rs`, `views.rs`,
  `viewmodel.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `tree.rs`,
  `screenshot.rs`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
  NOTE: sibling waves ki dirt (`app.rs`, `views.rs`, `checks/gui-pathmeta-bu/`)
  pehle se tree me thi — stage/commit me haath nahi lagana.

## G6 — harness PASS

- CHECK `tools/gui-polish/run-geo-typo.sh checks/gui-sessionmodel-bv/round-1/`
- EXPECT `RESULT: PASS` (max 6 rounds; headless wave hai, paint nahi badla).
