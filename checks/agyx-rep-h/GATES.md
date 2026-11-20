# SWARM-REP-H Acceptance Gates (leaf-2.2 Reports Parity Harden)

Scope:
- OWNS ONLY:
  - crates/reports/src/html.rs
  - crates/reports/src/text.rs
  - crates/reports/src/xml.rs
  - crates/reports/src/patch.rs
  - crates/reports/tests/**
  - checks/agyx-rep-h/**
- FORBIDDEN:
  - crates/cli/**, crates/gui/**, crates/sessions/**, crates/vfs/**, crates/diff/**, crates/compare/**, STATUS.md, PLAN.md, inbox/build-progress.json.

## Gates List

### Gate 1 (G1): Cargo test suite green
- CHECK: CARGO_TARGET_DIR=target/agent-rep-h cargo test -p autocompare-reports
- EXPECT: test result: ok. 22 passed; 0 failed; 0 ignored

### Gate 2 (G2): Clippy zero warnings
- CHECK: CARGO_TARGET_DIR=target/agent-rep-h cargo clippy -p autocompare-reports --all-targets -- -D warnings
- EXPECT: Finished `dev` profile

### Gate 3 (G3): Rustfmt clean
- CHECK: cargo fmt --check -- crates/reports/src/html.rs crates/reports/src/text.rs crates/reports/src/xml.rs crates/reports/src/patch.rs crates/reports/tests/snapshot_tests.rs
- EXPECT: exit 0

### Gate 4 (G4): Banned patterns zero in non-test source
- CHECK: rg -e 'unwrap\(' -e '\.expect\(' -e 'panic!' -e 'todo!' -e 'unimplemented!' -e 'unreachable!' crates/reports/src/
- EXPECT: exit 1 (zero matches in non-test source)

### Gate 5 (G5): Beyond Compare parity harden edge cases verified
- CHECK: 6 rounds of edge fixtures and snapshots verified across HTML (Color/Mono, Unified/Side-by-side), Plain Text, XML, and Patch (Unified/Context with timestamps and bundles).
- EXPECT: PASS

### Gate 6 (G6): ac-loop harness verification
- CHECK: tools/ac-loop/run.sh checks/agyx-rep-h/round-1
- EXPECT: RESULT: PASS

### Gate 7 (G7): Strict OWNS pathspec verification
- CHECK: git status --porcelain
- EXPECT: zero modifications outside OWNS paths
