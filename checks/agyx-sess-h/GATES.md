# AGYX-SESS-H SESSIONS HARDEN GATES — Beyond Compare Parity in crates/sessions

Scope: `crates/sessions/src/session.rs`, `crates/sessions/src/settings.rs`, `crates/sessions/src/workspace.rs`, `crates/sessions/tests/**`, and `checks/agyx-sess-h/**` ONLY.
FORBIDDEN: `crates/cli/**`, `crates/gui/**`, `crates/compare/**`, `crates/diff/**`, `crates/vfs/**`, `crates/reports/**`, `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`.

## Acceptance Gates

- [x] **G1: Unit & integration test suite green**
  - CHECK: `cargo test -p autocompare-sessions`
  - EXPECT: exit 0, all tests passed, 0 failed.
  - RESULT: PASS. 47 passed; 0 failed (18 lib unit, 9 corrupt_input, 12 migration, 8 round_trip).

- [x] **G2: Clippy zero warnings**
  - CHECK: `cargo clippy -p autocompare-sessions --all-targets -- -D warnings`
  - EXPECT: exit 0, 0 warnings.
  - RESULT: PASS. exit 0, zero warnings.

- [x] **G3: Rustfmt check clean**
  - CHECK: `cargo fmt --check -- crates/sessions/src/session.rs crates/sessions/src/settings.rs crates/sessions/src/workspace.rs crates/sessions/tests/round_trip.rs crates/sessions/tests/migration.rs crates/sessions/tests/corrupt_input.rs`
  - EXPECT: exit 0, no formatting diffs.
  - RESULT: PASS. exit 0, clean.

- [x] **G4: Banned pattern scan zero in non-test code**
  - CHECK: python regex scan for unwrap/expect/panic/todo/unimplemented/unreachable/unsafe in non-test src/
  - EXPECT: exit 0, zero violations in non-test code.
  - RESULT: PASS. 0 violations found.

- [x] **G5: ac-loop round-1 verification PASS**
  - CHECK: `tools/ac-loop/run.sh checks/agyx-sess-h/round-1`
  - EXPECT: exit 0, RESULT: PASS.
  - RESULT: PASS. CLI parity PASS, GUI geo-typo PASS, comparison fixtures verified.

- [x] **G6: Disjoint OWNS pathspec verification**
  - CHECK: Staging pathspec check (`git add crates/sessions/src/session.rs crates/sessions/src/settings.rs crates/sessions/src/workspace.rs crates/sessions/tests checks/agyx-sess-h`)
  - EXPECT: Only files in OWNS touched and committed locally; no forbidden files touched.
  - RESULT: PASS. Disjoint paths strictly enforced.
