# AGYX-DIFF-C GATES — Grammar & Unimportant Rules Diffing (Beyond Compare Parity)

Track: AGYX-C DIFF swarm node
Scope: crates/diff/** + checks/agyx-diff-c/** ONLY
Goal: Beyond Compare 5 parity in Text Compare grammar & unimportant text handling (pure function, zero I/O)

## GATES STATUS

- [x] G1: tests green
  CHECK: `cargo test -p autocompare-diff`
  EXPECT: test result: ok. 59 passed in unit tests, 199 total across all test suites, 0 failed.
  RESULT: PASS (59 passed in unit tests, 38 api, 44 audio, 20 image, 38 table; total 199 passed, 0 failed).

- [x] G2: clippy zero warnings
  CHECK: `cargo clippy -p autocompare-diff --all-targets`
  EXPECT: Finished `dev` profile; exit 0 with zero warnings.
  RESULT: PASS (exit 0, 0 warnings).

- [x] G3: rustfmt check on touched files
  CHECK: `cargo fmt --check -- crates/diff/src/grammar.rs crates/diff/src/lib.rs`
  EXPECT: exit 0, formatting clean.
  RESULT: PASS (exit 0, formatting completely clean).

- [x] G4: banned grep in non-test code
  CHECK: `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/diff/src/grammar.rs | grep -v '^\s*//' | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!|unsafe" || echo ZERO-BANNED`
  EXPECT: ZERO-BANNED
  RESULT: PASS (ZERO-BANNED; `#![forbid(unsafe_code)]` intact in lib.rs, zero unwrap/expect/panic/indexing in non-test).

- [x] G5: OWNS proof
  CHECK: Staging pathspec `git add crates/diff checks/agyx-diff-c` ensuring only owned directories are committed.
  EXPECT: Only crates/diff/** and checks/agyx-diff-c/** in commit.
  RESULT: PASS.
