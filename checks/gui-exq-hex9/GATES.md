# HEX9 Gates (round-1)

- `cargo test -p autocompare-gui --lib hex_exq_hex9`: PASS — GREEN
- `cargo clippy -p autocompare-gui`: 0 warnings in `crates/gui/src/views_hex.rs` — GREEN
- `rustfmt --edition 2024 --check crates/gui/src/views_hex.rs`: clean — GREEN
- Banned tokens non-test code me (`unsafe`, `unwrap()`, `expect(`, `panic!`, `todo!`, `unreachable!`): 0 — GREEN
- Harness `tools/exquisite/run.sh checks/gui-exq-hex9/round-1 crates/gui/src`: RESULT: PASS — GREEN
  - PNG >= 8192 bytes, magic ok, distinct > 64 (non-blank ok)
  - Colors frozen: Color32/Visuals touches 33 (unchanged, info only)
- Evidence: `round-1/after.png`, `round-1/exquisite-report.md`, `round-1/tests.out`, `round-1/comparison/hex-narrow-720.json`, `comparison/hex-narrow-720.json`
- OWNS: `crates/gui/src/views_hex.rs`, `checks/gui-exq-hex9/**` — bahar koi edit nahi
