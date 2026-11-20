# GATES — SWARM-CLI-H2 CLI & VCS Compatibility Hardening

Wave: leaf-3.1 compat hardening node (PLAN C9 follow-up).
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (Strict Disjoint Boundary)
- `crates/cli/src/bcomp.rs`
- `crates/cli/src/report.rs`
- `crates/cli/src/report_adapter.rs`
- `crates/cli/src/report_adapter_media.rs`
- `checks/agyx-cli-h2/**`

## FORBIDDEN (Never Touch)
`crates/cli/src/runner.rs`, `crates/cli/src/args.rs`, `crates/cli/src/main.rs`, `crates/cli/src/lib.rs`, `crates/cli/tests/**`, `STATUS.md`, `PLAN.md`, `inbox/**`, `crates/gui/**`, `crates/diff/**`, `crates/compare/**`, `crates/sessions/**`, `crates/reports/**`, `crates/vfs/**`.

---

## Acceptance Gates Status

### Gate 1 (G1): Full autocompare-cli test suite green
All existing integration tests plus new unit and fixture tests pass cleanly.
- CHECK:
  `CARGO_TARGET_DIR=/tmp/target-agyx-cli CARGO_BUILD_JOBS=2 cargo test -p autocompare-cli`
- EXPECT:
  `test result: ok.`

### Gate 2 (G2): Clippy zero warnings on all targets
Zero clippy warnings across workspace CLI package with `-D warnings`.
- CHECK:
  `CARGO_TARGET_DIR=/tmp/target-agyx-cli cargo clippy -p autocompare-cli --all-targets -- -D warnings`
- EXPECT:
  `Finished`

### Gate 3 (G3): Rustfmt clean
All owned source files format cleanly according to Rust 2024 edition.
- CHECK:
  `cargo fmt -p autocompare-cli --check`
- EXPECT:
  `exit 0`

### Gate 4 (G4): Zero banned patterns in non-test owned code
Zero raw `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, or raw indexing `[...]` in non-test owned source.
- CHECK:
  `python3 -c 'import subprocess, sys; out = subprocess.run(["rg", "-n", "(unwrap\\(|\\.expect\\(|panic!|todo!|unimplemented!|unreachable!)", "crates/cli/src/bcomp.rs", "crates/cli/src/report.rs", "crates/cli/src/report_adapter.rs", "crates/cli/src/report_adapter_media.rs"], capture_output=True, text=True); lines = [l for l in out.stdout.splitlines() if "mod tests" not in l and "#[cfg(test)]" not in l]; non_test = [l for l in lines if not any(k in l for k in ["tests", "test_"])]; sys.exit(0 if len(non_test) == 0 else 1)'`
- EXPECT:
  `0`

### Gate 5 (G5): bcomp-compatible VCS switches (-ro, -automerge, -mergeoutput, -vcs1..4) tests green
Unit and fixture integration tests for bcomp flag normalization, space/equals syntax, slash switches, and direct invocation resolution pass.
- CHECK:
  `CARGO_TARGET_DIR=/tmp/target-agyx-cli CARGO_BUILD_JOBS=2 cargo test -p autocompare-cli --lib bcomp::tests`
- EXPECT:
  `test result: ok.`

### Gate 6 (G6): Report generation wiring (HTML, Text, XML, Patch) and adapters tests green
Unit and fixture integration tests for all 9 adapters and report formats (HTML color/mono, Plain Text, XML, Unified/Context patch) pass.
- CHECK:
  `CARGO_TARGET_DIR=/tmp/target-agyx-cli CARGO_BUILD_JOBS=2 cargo test -p autocompare-cli --lib report::tests report_adapter::tests report_adapter_media::tests`
- EXPECT:
  `test result: ok.`

### Gate 7 (G7): Edge fixtures in checks/agyx-cli-h2/round-1/comparison/ verified
Edge fixtures for bcomp VCS switches, report wiring, and exit code conventions present and verified.
- CHECK:
  `test -f checks/agyx-cli-h2/round-1/comparison/edge-bcomp-compat-vcs.txt && test -f checks/agyx-cli-h2/round-1/comparison/edge-report-wiring.txt && test -f checks/agyx-cli-h2/round-1/comparison/edge-exit-codes-bc-mode.txt && echo "EDGE_FIXTURES_OK"`
- EXPECT:
  `EDGE_FIXTURES_OK`

### Gate 8 (G8): Strict OWNS boundary proof
Only owned files touched, zero modifications outside OWNS paths.
- CHECK:
  `git status --porcelain crates/cli/src/bcomp.rs crates/cli/src/report.rs crates/cli/src/report_adapter.rs crates/cli/src/report_adapter_media.rs checks/agyx-cli-h2/`
- EXPECT:
  `clean for non-owned files`
