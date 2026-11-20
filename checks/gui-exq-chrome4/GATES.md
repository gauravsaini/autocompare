# CHROME4 GATES (round-1)

- `cargo test -p autocompare-gui`: GREEN
  (`CARGO_TARGET_DIR=/tmp/target-exq-chrome4 CARGO_BUILD_JOBS=2`)
- `cargo check -p autocompare-gui --lib`: GREEN — 0 warnings
- `rustfmt --edition 2024 --check crates/gui/src/app.rs crates/gui/src/home.rs`: CLEAN — exit 0
- Banned (`unwrap()`/`expect(`/`panic!`/`todo!` in non-test code): 0
- Colors: `Color32/Visuals` touches 33 — baseline jaisa, growth 0 (frozen)
- Exquisite harness `checks/gui-exq-chrome4/round-1`: **RESULT: PASS**
  (PNG bytes ≥ 8192, magic ok, distinct 256)
- Edge fixture: `comparison/chrome-wide-960.json` + headless tests
- OWNS: `crates/gui/src/app.rs`, `crates/gui/src/home.rs`, `checks/gui-exq-chrome4/**` only
