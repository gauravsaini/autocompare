# sec99 round-1 GATES — xtask secrets false-positive fix

Scope OWNS: xtask/** + checks/gui-sec99/** ONLY.
Base: be05b07. Worktree: /tmp/wt-sec99. Env: CARGO_TARGET_DIR=/tmp/target-sec99.

- [x] G1: docs par secrets scanner clean exit 0 deta hai (REPORT.md:7 backticked pattern names FP nahi).
  CHECK: CARGO_TARGET_DIR=/tmp/target-sec99 cargo run -p xtask -- secrets
  EXPECT: secrets: ok
- [x] G2: xtask unit tests pass (shape-based detection + FP regression cover).
  CHECK: CARGO_TARGET_DIR=/tmp/target-sec99 cargo test -p xtask
  EXPECT: test result: ok
- [x] G3: xtask clippy strict clean hai.
  CHECK: CARGO_TARGET_DIR=/tmp/target-sec99 cargo clippy -p xtask --all-targets -- -D warnings
  EXPECT: Finished
- [x] G4: fmt clean hai.
  CHECK: cargo fmt --check -p xtask && echo FMT_OK
  EXPECT: FMT_OK
- [x] G5: asli secrets ab bhi pakde jaate hain (positive control: lamba ghp_/github_pat_/AKIA shape + private-key banner).
  CHECK: CARGO_TARGET_DIR=/tmp/target-sec99 cargo test -p xtask secrets_shape
  EXPECT: test result: ok
- [x] G6: prod diff me banned/asli secret nahi, diff sirf owns paths tak limited hai.
  CHECK: sh checks/gui-sec99/round-1/banned-scan.sh
  EXPECT: BANNED_SCAN_OK
