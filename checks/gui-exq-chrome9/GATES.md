# CHROME9 GATES (round-1)

- `cargo test -p autocompare-gui`: GREEN — 997 passed, 0 failed
  (`CARGO_TARGET_DIR=/tmp/target-exq-chrome9 CARGO_BUILD_JOBS=2`)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: CLEAN in OWNS (`crates/gui/src/app.rs`)
- `rustfmt --edition 2024 --check crates/gui/src/app.rs`: CLEAN — exit 0
- Banned (`unwrap()`/`expect()`/`panic!` / `todo!` / `unreachable!` / indexing in `crates/gui/src/app.rs` non-test): 0
- Colors: `Color32/Visuals` touches 33 — baseline frozen, growth 0
- Exquisite harness `checks/gui-exq-chrome9/round-1`: **RESULT: PASS**
  (PNG 10401 bytes ≥ 8192, magic ok, distinct 256)
- Edge fixture: `comparison/chrome-bar-960.json` + headless tests proving toolbar-size + filebar-height
- OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-chrome9/**` + `comparison/chrome-bar-960.json` only
