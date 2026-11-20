# STATUS7 gates

- `cargo test -p autocompare-gui`: PASS (847 passed, 0 failed; 6 naye status7 tests)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: PASS (0 warnings)
- `rustfmt --edition 2024 --check crates/gui/src/app.rs`: PASS (clean)
- banned (`unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!` in non-test app.rs): PASS (0)
- colors frozen (naya `Color32`/`Visuals` in diff): PASS (0)
- `tools/exquisite/run.sh checks/gui-exq-status7/round-1`: RESULT PASS
  (PNG 10401 bytes >= 8192, magic ok, distinct 256 > 64)
- OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-status7/**` only
  (`views.rs`/dusre checks dirs untouched — parallel waves ka kaam)
- commit: pathspec only, NO PUSH (orchestrator verify karke push karega)
