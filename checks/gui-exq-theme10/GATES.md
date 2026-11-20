# THEME10 GATES (round-1)

- `cargo test -p autocompare-gui`: GREEN — 868 passed, 0 failed
  (`CARGO_TARGET_DIR=/tmp/target-op-theme10 CARGO_BUILD_JOBS=2`)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: owned file
  `theme.rs` CLEAN (0 hits); package red sirf sibling dirty `gutter.rs`
  (question-mark x3 + excessive-precision x1) — OWNS bahar, touch nahi kiya
- `rustfmt --edition 2024 --check crates/gui/src/theme.rs`: CLEAN — exit 0
- Banned (`unwrap()`/`expect(`/`panic!`/`todo!` in `crates/gui/src/theme.rs`
  non-test): 0
- Colors: `Color32/Visuals` touches 33 — baseline jaisa, growth 0 (frozen)
- Exquisite harness `checks/gui-exq-theme10/round-1`: **RESULT: PASS**
  (PNG 10401 bytes ≥ 8192, magic ok, distinct 256)
- Edge fixture: `round-1/comparison/pane-hierarchy-720.json` + headless test
  (`theme10_pane_hierarchy_sharp_without_new_sizes`, narrow assertions inside)
- OWNS: `crates/gui/src/theme.rs` + `checks/gui-exq-theme10/**` only
