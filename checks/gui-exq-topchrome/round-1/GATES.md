# GATES — gui-exq-topchrome round-1

| Gate | CHECK | EXPECT | RESULT |
|---|---|---|---|
| tests | `CARGO_TARGET_DIR=/tmp/target-op-topchrome CARGO_BUILD_JOBS=2 cargo test -p autocompare-gui --lib` | ok, 0 failed | PASS (804 passed, 0 failed) |
| clippy | `cargo clippy -p autocompare-gui --all-targets -- -D warnings` | 0 warnings | PASS |
| fmt | `rustfmt --edition 2024 --check crates/gui/src/home.rs crates/gui/src/screenshot.rs` | clean | PASS |
| banned | grep non-test `unwrap()/expect(/panic!/todo!/unimplemented!/unsafe` + `indexing_slicing` deny via clippy | 0 | PASS |
| harness | `bash tools/exquisite/run.sh checks/gui-exq-topchrome/round-1` | RESULT PASS | PASS (PNG 10401B, magic ok, distinct 256) |
| edge fixture | `comparison/topchrome-narrow-empty.json` + `topchrome_toolbar_path_status_narrow_empty_edge` test | narrow overflow + empty hint proved | PASS |
| colors frozen | harness Color32/Visuals touches | 33 (no growth) | PASS |
| scope | OWNS home.rs + screenshot.rs + checks/gui-exq-topchrome only | no forbidden edits | PASS |
