# SWARM-REP-H2 Acceptance Gates (leaf-2.2 Reports Parity Harden)

Scope:
- OWNS ONLY:
  - `crates/reports/**`
  - `checks/agyx-rep-h2/**`
- FORBIDDEN:
  - `checks/agyx-rep-h/**`, `crates/cli/**`, `crates/gui/**`, `crates/sessions/**`, `crates/vfs/**`, `crates/diff/**`, `crates/compare/**`, `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`.

## Gates List

### Gate 1 (G1): Cargo test suite green
- CHECK: CARGO_TARGET_DIR=/tmp/target-agyx-s2 CARGO_BUILD_JOBS=2 cargo test -p autocompare-reports
- EXPECT: test result: ok. (31 passed)

### Gate 2 (G2): Clippy zero warnings
- CHECK: CARGO_TARGET_DIR=/tmp/target-agyx-s2 cargo clippy -p autocompare-reports --all-targets -- -D warnings
- EXPECT: Finished `dev` profile

### Gate 3 (G3): Rustfmt clean
- CHECK: cargo fmt -p autocompare-reports --check
- EXPECT: exit 0

### Gate 4 (G4): Banned patterns zero in non-test source
- CHECK: rg -e 'unwrap\(' -e '\.expect\(' -e 'panic!' -e 'todo!' -e 'unimplemented!' -e 'unreachable!' crates/reports/src/
- EXPECT: exit 1 (zero matches in non-test source)

### Gate 5 (G5): Builder snapshot coverage and edge fixture added
- CHECK: CARGO_TARGET_DIR=/tmp/target-agyx-s2 CARGO_BUILD_JOBS=2 cargo test -p autocompare-reports -- test_builder_snapshots test_edge_hunk_boundaries_and_special_paths test_edge_fixture_snapshot_match
- EXPECT: test result: ok. (3 passed)

### Gate 6 (G6): Pathspec clean verification
- CHECK: git status --porcelain (staged files strictly inside OWNS paths)
- EXPECT: zero modifications outside OWNS paths
