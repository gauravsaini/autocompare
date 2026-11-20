# EXQ-TYPO10 GATES (round-1)

- `cargo test -p autocompare-gui`: GREEN — 886 passed, 0 failed
  (`CARGO_TARGET_DIR=/tmp/target-exq-theme CARGO_BUILD_JOBS=2`)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: owned files
  `theme.rs`, `viewmodel.rs`, `tree.rs` CLEAN (0 hits)
- `rustfmt --edition 2024 --check crates/gui/src/theme.rs crates/gui/src/viewmodel.rs crates/gui/src/tree.rs`: CLEAN — exit 0
- Banned (`unwrap()`/`expect(`/`panic!`/`todo!` in non-test owned files): 0
- Colors: `Color32/Visuals` touches 33 — baseline jaisa, growth 0 (frozen)
- Exquisite harness `checks/gui-exq-theme10/round-1`: **RESULT: PASS**
  (PNG 10401 bytes ≥ 8192, magic ok, distinct 256)
- Edge fixtures:
  * `round-1/comparison/empty-vs-nonempty.json` + headless test `exq_typo10_empty_vs_nonempty_rhythm_and_density_edge`
  * `round-1/comparison/pane-hierarchy-720.json` + headless test `theme10_pane_hierarchy_sharp_without_new_sizes`
- OWNS: `crates/gui/src/theme.rs`, `crates/gui/src/viewmodel.rs`, `crates/gui/src/tree.rs`, `checks/gui-exq-theme10/**` only
