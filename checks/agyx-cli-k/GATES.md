# GATES — SWARM-CLI-K CLI Parity Hardening

Wave: SWARM-CLI-K leaf-3.1 hardening node.
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)
- `crates/cli/**`
- `checks/agyx-cli-k/**`

## FORBIDDEN
`everything else`, especially `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`, `crates/gui/**`, `crates/diff/**`, `crates/compare/**`, `crates/sessions/**`, `crates/reports/**`, `crates/vfs/**`.

---

## Acceptance Gates Status

### G1 — 10 Subcommands CLI Parity & Execution (`crates/cli/src/args.rs`, `runner.rs`)
Ten subcommands fully supported: `text`, `folder`, `hex`, `merge`, `table`, `sync`, `picture`, `folder-merge`, `audio`, `session`.
- CHECK:
  `cargo test -p autocompare-cli --test subcommands_text_test --test subcommands_media_test --test session_test`
- EXPECT:
  `test result: ok.`

### G2 — Multi-Format Reporting Pipeline (`crates/cli/src/report.rs`, `runner.rs`)
HTML (color & mono), Plain Text, XML, Unified Patch, and Context Patch generation across comparison types with `--report <fmt:path>` and `--report-title`.
- CHECK:
  `cargo test -p autocompare-cli --test subcommands_text_test test_text_subcommand_exit_codes_and_reports`
- EXPECT:
  `test result: ok.`

### G3 — Beyond Compare (bcomp) CLI Flags Normalization & Execution (`crates/cli/src/bcomp.rs`, `args.rs`)
Legacy switches (`-ro`, `/ro`, `-ro1`, `-ro2`, `-automerge`, `/automerge`, `-mergeoutput`, `/mergeoutput`, `-vcs1..4`, `-title1..4`, `-bcexitcode`, `-qc`) correctly normalized and executed.
- CHECK:
  `cargo test -p autocompare-cli --test bcomp_test`
- EXPECT:
  `test result: ok.`

### G4 — 4-Way Git Mergetool Mapping & Automated Resolution (`crates/cli/src/bcomp.rs`, `runner.rs`)
Standard git mergetool invocation `bcomp $LOCAL $REMOTE $BASE $MERGED` mapped to 3-way merge with base/local/remote order and output destination.
- CHECK:
  `cargo test -p autocompare-cli --test bcomp_test test_bcomp_four_way_git_mergetool`
- EXPECT:
  `test result: ok.`

### G5 — Exit Code Conventions Matrix (0/1/2 and BC modes 0/11/13/14) (`crates/cli/src/outcome.rs`)
Diff mode (0=same/clean, 1=diff/conflict, 2=error/ro) and Beyond Compare mode (0=clean, 11=binary diff, 13=rules diff, 14=conflicts) properly surfaced.
- CHECK:
  `cargo test -p autocompare-cli --test errors_test`
- EXPECT:
  `test result: ok.`

### G6 — Unified ac-loop Verification (`tools/ac-loop/run.sh`)
Unified craft-anything harness completes with clean CLI parity and GUI geo-typo checks.
- CHECK:
  `tools/ac-loop/run.sh checks/agyx-cli-k/round-1`
- EXPECT:
  `RESULT: PASS`

### G7 — Full autocompare-cli Test Suite
All unit, integration, error, and hardening tests pass.
- CHECK:
  `cargo test -p autocompare-cli`
- EXPECT:
  `test result: ok.`

### G8 — Clippy Zero Warnings & Formatting Clean
Zero clippy warnings with all targets and deny warnings; formatting clean across all source files.
- CHECK:
  `cargo clippy --package autocompare-cli --all-targets --no-deps -- -D warnings`
- EXPECT:
  `Finished`

### G9 — MS Rust Banned Patterns Zero in Non-Test Code
Zero unwrap, expect, panic, todo, unimplemented, unreachable, or indexing/slicing in non-test code; forbid unsafe enforced.
- CHECK:
  `cargo clippy --package autocompare-cli --lib --no-deps -- -D clippy::indexing_slicing`
- EXPECT:
  `Finished`

### G10 — OWNS Boundary Proof
Only `crates/cli/**` and `checks/agyx-cli-k/**` modified. Pathspec-only commits, no push.
- CHECK:
  `git status -s`
- EXPECT:
  `clean for non-owned files`
