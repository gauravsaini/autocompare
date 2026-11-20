# TEXTVIEW9 gates

- `cargo test -p autocompare-gui`: PASS (864 passed, 0 failed; 1 naya textview9 test)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: PASS (0 warnings)
- `rustfmt --edition 2024 --check crates/gui/src/views.rs`: PASS (clean)
- banned (`unwrap()`/`expect(`/`panic!`/`todo!`/`unimplemented!` in non-test views.rs): PASS (0)
- colors frozen (naya `Color32`/`Visuals` in views.rs diff): PASS (0)
- `tools/exquisite/run.sh checks/gui-exq-textview9/round-1`: RESULT PASS
  (PNG 10401 bytes >= 8192, magic ok, distinct 256 > 64; max6 me se 1 round laga)
- OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-textview9/**` only
  (`app.rs`/`viewmodel.rs`/dusre checks dirs untouched — parallel waves ka kaam)
- commit: pathspec only, NO PUSH (orchestrator verify karke push karega)
