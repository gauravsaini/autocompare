# AGYX-G CLI Headless Parity Report

## 1. Summary
Humne CLI headless parity complete kar diya hai according to Beyond Compare (bcomp) specifications:
- `crates/cli/src/args.rs`: Clap options expand kiye gaye hain for bcomp compatibility (`--ro1`, `--ro2`, `--title1..4`, `--vcs1..4`, `--automerge`, `--mergeoutput`, `--solo`, `--nobackups`, `--reviewconflicts`, `--report-title`, `--exit-code-mode`, `--bc-exit-code`). Effective exit code calculation (`effective_exit_code_mode`) aur read-only check (`is_read_only`) methods provide kiye gaye. Subcommands me short flags (`-i`, `-b`, `-u`, `-w`, `-d`, `-r`) add kiye gaye.
- `crates/cli/src/bcomp.rs`: Normalization engine implement kiya gaya jo legacy single-dash (`-ro`, `-automerge`, `-mergeoutput`, `-vcs1..4`, `-title1..4`, `-qc`, `-bcexitcode`) aur slash (`/ro`, `/automerge`, etc.) arguments ko canonical CLI flags me translate karta hai. Direct positional argument resolver 2-way text/folder, 3-way merge/folder-merge, session shorthand (`@path`), aur 4-way git mergetool mapping (`bcomp local remote base merged` -> `merge base local remote --output=merged`) ko accurately resolve karta hai.
- `crates/cli/src/outcome.rs`: Exit code conventions ko standard diff (0=same, 1=diff, 2=error) aur Beyond Compare modes (0=success, 11=binary diff, 13=rules diff, 14=conflicts) me cleanly map kiya gaya hai.
- `crates/cli/src/report.rs`: Report adapters HTML (color & mono), Plain Text, XML, Unified Patch, aur Context Patch formats support karte hain with automatic extension inference aur format parsing (`format:path`). Parent directory creation aur safe I/O handling added.
- `crates/cli/src/runner.rs`: Subcommand execution pipeline update kiya gaya hai to respect read-only constraints (`-ro`, `-ro1`, `-ro2`), report title overrides (`--report-title`), output redirection, session loading (`session_cmd::load_and_run`), aur mode-dependent exit code translation (`to_mode(mode)`).
- `crates/cli/tests/bcomp_test.rs`: Exhaustive integration tests add kiye gaye hain covering 4-way git mergetool invocation, BC mode exit codes (0/11/13/14), title flags, aur `@session` syntax.

## 2. Test Verification
Command: `cargo test -p autocompare-cli`
Result: 23 passed, 0 failed, 0 ignored across 5 test suites:
- `bcomp_test.rs`: 8 passed, 0 failed
- `errors_test.rs`: 5 passed, 0 failed
- `session_test.rs`: 1 passed, 0 failed
- `subcommands_media_test.rs`: 5 passed, 0 failed
- `subcommands_text_test.rs`: 4 passed, 0 failed

## 3. Parity Harness & ac-loop Verification
- `tools/parity-harness/run-round.sh checks/ac-loop/round-g1/cli`: RESULT: PASS (10/10 subcommands match exact exit code expectation).
- `tools/ac-loop/run.sh checks/ac-loop/round-g1`: RESULT: PASS (CLI parity PASS + GUI geo-typo PASS).

## 4. Clippy & Formatting
- `cargo clippy --package autocompare-cli --all-targets --no-deps -- -D warnings`: ZERO warnings.
- `cargo clippy --package autocompare-cli --lib --no-deps -- -D clippy::indexing_slicing`: ZERO warnings.
- `cargo clippy --package autocompare-cli --bin autocompare --no-deps -- -D clippy::indexing_slicing`: ZERO warnings.
- `rustfmt --edition 2024 --check crates/cli/src/args.rs crates/cli/src/bcomp.rs crates/cli/src/outcome.rs crates/cli/src/report.rs crates/cli/src/runner.rs crates/cli/tests/*.rs`: Clean (exit code 0).

## 5. MS Rust Guidelines & Banned Token Verification
Non-test code me strict safety rules follow kiye gaye hain:
- `#![forbid(unsafe_code)]` enforced in `crates/cli/src/lib.rs`.
- Zero raw `unwrap()`, zero `expect()`, zero `panic!`, zero `todo!`, zero `unimplemented!`, zero `unreachable!`.
- Zero indexing/slicing (`[...]`) in library code and binary code.

## 6. OWNS Boundary Proof
Staging and commit will strictly touch only owned paths:
- `crates/cli/src/args.rs`
- `crates/cli/src/bcomp.rs`
- `crates/cli/src/outcome.rs`
- `crates/cli/src/report.rs`
- `crates/cli/src/runner.rs`
- `crates/cli/tests/bcomp_test.rs`
- `checks/agyx-cli-g/GATES.md`
- `checks/agyx-cli-g/REPORT.md`

All forbidden files (`STATUS.md`, `PLAN.md`, `inbox/build-progress.json`, other crates) remain completely untouched.
Local commit will use pathspec-only syntax and NO push will be performed.
