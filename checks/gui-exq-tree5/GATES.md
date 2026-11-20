# TREE5 GATES

- `cargo test -p autocompare-gui`: 822 passed, 0 failed (CARGO_TARGET_DIR=/tmp/target-op-tree5, CARGO_BUILD_JOBS=2)
- `cargo clippy -p autocompare-gui --all-targets`: tree.rs 0 warnings (1 warning views_table.rs me hai — FORBIDDEN file, dusri wave ka, untouched)
- `rustfmt --edition 2024 --check crates/gui/src/tree.rs`: clean
- Banned patterns non-test code (`unwrap()`/`expect(`/`panic!`/`todo!`/`unsafe`): 0
- Colors: `Color32/Visuals` touches 33, TREE4 se unchanged (diff me zero color lines)
- Harness: `tools/exquisite/run.sh checks/gui-exq-tree5/round-1` → `RESULT: PASS` (PNG 10401 bytes, magic ok, distinct 256)
- Typo: `.size` sirf `14.0`, koi naya size nahi; SPACE/MD/LG/SM tokens reuse
- RESULT: PASS
