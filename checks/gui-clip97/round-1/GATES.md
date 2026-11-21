# gui97 round-1 — clippy clean (behavior-preserving)

OWNS: crates/gui/** + checks/gui-clip97/** ONLY

- [x] G1: workspace clippy clean with -D warnings
  CHECK: CARGO_TARGET_DIR=/tmp/target-gui97 cargo clippy --workspace --all-targets -- -D warnings
  EXPECT: CLIPPY_OK
- [x] G2: gui tests pass 1163 lib + 2 real_egui_shot
  CHECK: CARGO_TARGET_DIR=/tmp/target-gui97 cargo test -p autocompare-gui
  EXPECT: TESTS_OK_1163_2
- [x] G3: fmt clean
  CHECK: cargo fmt --all --check
  EXPECT: FMT_OK
- [x] G4: owns + banned scan clean (prod diff has 0 unwrap/expect/panic/todo)
  CHECK: git status --porcelain + banned rg on crates/gui non-test
  EXPECT: OWNS_BANNED_OK
