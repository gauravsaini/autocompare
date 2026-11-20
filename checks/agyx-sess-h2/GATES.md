# AGYX-SESS-H2 SESSIONS HARDEN GATES — Beyond Compare Parity in crates/sessions

Scope: `crates/sessions/src/**`, `crates/sessions/tests/**`, and `checks/agyx-sess-h2/**` ONLY.
FORBIDDEN: `crates/cli/**`, `crates/gui/**`, `crates/compare/**`, `crates/diff/**`, `crates/vfs/**`, `crates/reports/**`, `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`.

## Acceptance Gates

- [x] **G1: Unit & integration test suite green**
  - CHECK: `CARGO_TARGET_DIR=/tmp/target-agyx-s1 CARGO_BUILD_JOBS=2 cargo test -p autocompare-sessions`
  - EXPECT: exit 0, all tests passed, 0 failed.
  - RESULT: PASS.

- [x] **G2: Clippy zero warnings**
  - CHECK: `CARGO_TARGET_DIR=/tmp/target-agyx-s1 CARGO_BUILD_JOBS=2 cargo clippy -p autocompare-sessions --all-targets -- -D warnings`
  - EXPECT: exit 0, 0 warnings.
  - RESULT: PASS.

- [x] **G3: Rustfmt check clean**
  - CHECK: `cargo fmt --check -- crates/sessions/src/session.rs crates/sessions/src/settings.rs crates/sessions/src/workspace.rs crates/sessions/src/options.rs crates/sessions/src/lib.rs crates/sessions/tests/round_trip.rs crates/sessions/tests/migration.rs crates/sessions/tests/corrupt_input.rs`
  - EXPECT: exit 0, no formatting diffs.
  - RESULT: PASS.

- [x] **G4: Banned pattern scan zero in non-test code**
  - CHECK: `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/sessions/src/*.rs | grep -v '^\s*//' | grep -v 'forbid(unsafe_code)' | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!|unsafe\b" || echo ZERO-BANNED`
  - EXPECT: ZERO-BANNED, exit 0, zero violations in non-test code.
  - RESULT: PASS.

- [x] **G5: Edge fixture and schema migration verification**
  - CHECK: Verify `checks/agyx-sess-h2/round-1/comparison/edge-merge-3way-session.json` round-trip test passed in test suite.
  - EXPECT: PASS.
  - RESULT: PASS.

- [x] **G6: Disjoint OWNS pathspec verification**
  - CHECK: Staging pathspec check (`git add crates/sessions checks/agyx-sess-h2`)
  - EXPECT: Only files in OWNS touched and committed locally; no forbidden files touched.
  - RESULT: PASS.
