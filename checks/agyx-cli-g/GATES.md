# GATES — AGYX-G CLI Headless Parity

Wave: AGYX-G CLI rescue node.
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)
- `crates/cli/src/args.rs`
- `crates/cli/src/bcomp.rs`
- `crates/cli/src/outcome.rs`
- `crates/cli/src/report.rs`
- `crates/cli/src/runner.rs`
- `crates/cli/tests/**`
- `checks/agyx-cli-g/**`

## FORBIDDEN
`everything else`, `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`.

---

## Acceptance Gates Status

### G1 — Beyond Compare (bcomp) CLI Compatibility & VCS Switches (`bcomp.rs`, `args.rs`)
- Single-dash and slash switches normalizer (`-ro`, `/ro`, `-ro1`, `-ro2`, `-leftreadonly`, `-rightreadonly`, `-centerreadonly`, `-automerge`, `-reviewconflicts`, `-mergeoutput`, `-vcs1..4`, `-ancestor`, `-base`, `-center`, `-title1..4`, `-lefttitle`, `-righttitle`, `-centertitle`, `-ancestortitle`, `-outputtitle`, `-solo`, `-nobackups`, `-closescript`, `-report`, `-report-title`, `-session`, `-savesession`, `-qc`, `-quickcompare`, `-bcexitcode`, `@session`).
- Shorthand subcommand normalization (`-text`, `-folder`, `-hex`, `-merge`, `-table`, `-sync`, `-picture`, `-folder-merge`, `-audio`).
- Directional 2-way and 3-way positional argument resolution to subcommands based on path existence and directory detection.
- Status: PASS

### G2 — 4-Way Git Mergetool Mapping (`bcomp.rs`, `runner.rs`, `bcomp_test.rs`)
- Standard Git mergetool syntax `bcomp $LOCAL $REMOTE $BASE $MERGED` automatically mapped to 3-way merge with base/left/right and `--output=$MERGED`.
- Automatic detection of folder-merge when positional inputs are directories.
- Verified with integration tests in `crates/cli/tests/bcomp_test.rs` (`test_bcomp_four_way_git_mergetool`).
- Status: PASS

### G3 — Exit Code Modes: diff (0/1/2) and Beyond Compare (0/11/13/14) (`outcome.rs`, `args.rs`, `runner.rs`)
- `ExitCodeMode::Diff`: 0 = identical/clean merge, 1 = difference/conflicts, 2 = error / read-only / bad args.
- `ExitCodeMode::BeyondCompare` (`--bc-exit-code`, `-bcexitcode`, `-qc`, `-quickcompare`):
  - 0 = success / identical / clean merge
  - 11 = binary difference
  - 13 = rules-based / text difference
  - 14 = merge conflicts detected
- Verified across subcommands in integration tests (`test_bcomp_exit_codes_bc_mode`).
- Status: PASS

### G4 — Report Adapters & Session Command Integration (`report.rs`, `runner.rs`)
- Formats supported: HTML (`html`, `html-color`), Monochrome HTML (`html-mono`), Plain Text (`text`), XML (`xml`), Unified Patch (`patch`, `unified`), Context Patch (`context`).
- Custom report title override (`--report-title`).
- Session loading and execution via `session` subcommand, `--session`, or `@session.json` syntax.
- Status: PASS

### G5 — Parity Harness & ac-loop Check Verification
- `tools/parity-harness/run-round.sh checks/ac-loop/round-g1/cli`: RESULT: PASS (10/10 subcommands exit exactly as expected).
- `tools/ac-loop/run.sh checks/ac-loop/round-g1`: RESULT: PASS.
- Status: PASS

### G6 — Test Suite Green
- `cargo test -p autocompare-cli`: 23 passed, 0 failed, 0 ignored.
- All 5 test suites (`bcomp_test.rs`, `errors_test.rs`, `session_test.rs`, `subcommands_media_test.rs`, `subcommands_text_test.rs`) green.
- Status: PASS

### G7 — Clippy Zero Warnings & Formatting Clean
- `cargo clippy --package autocompare-cli --all-targets --no-deps -- -D warnings`: ZERO warnings.
- `rustfmt --edition 2024 --check crates/cli/src/args.rs crates/cli/src/bcomp.rs crates/cli/src/outcome.rs crates/cli/src/report.rs crates/cli/src/runner.rs crates/cli/tests/*.rs`: exit 0.
- Status: PASS

### G8 — MS Rust Banned Patterns Zero in Non-Test Code
- Non-test code contains zero `unwrap()`, zero `.expect(`, zero `panic!`, zero `todo!`, zero `unimplemented!`, zero `unreachable!`, zero unsafe blocks (`#![forbid(unsafe_code)]`), and zero indexing/slicing (`[...]`).
- `cargo clippy --package autocompare-cli --lib --no-deps -- -D clippy::indexing_slicing`: 0 warnings.
- `cargo clippy --package autocompare-cli --bin autocompare --no-deps -- -D clippy::indexing_slicing`: 0 warnings.
- Status: PASS

### G9 — OWNS Boundary Proof
- Only owned files modified and staged: `crates/cli/src/args.rs`, `crates/cli/src/bcomp.rs`, `crates/cli/src/outcome.rs`, `crates/cli/src/report.rs`, `crates/cli/src/runner.rs`, `crates/cli/tests/bcomp_test.rs`, `checks/agyx-cli-g/**`.
- Committed strictly using pathspec. NEVER git add -A. NEVER push.
- Status: PASS
