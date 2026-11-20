# AGYX-CLI-K Parity Hardening Report

## 1. Summary (Hinglish-Roman)
Humne leaf-3.1 CLI parity ko Beyond Compare (bcomp) aur diff(1) conventions ke saath fully harden kar diya hai:
- **10 Subcommands Supported**: `text`, `folder`, `hex`, `merge`, `table`, `sync`, `picture`, `folder-merge`, `audio`, aur `session` sabhi commands headless CLI me direct execute hote hain with full argument parsing and flags.
- **Reporting Pipeline**: Sabhi 10 subcommands `--report <fmt:path>` parameter support karte hain with automatic format inference aur multi-format dispatch (HTML color, HTML mono, Plain Text, XML, Unified Patch, Context Patch) plus title overrides (`--report-title`).
- **bcomp VCS Switches**: Legacy single-dash (`-ro`, `-ro1`, `-ro2`, `-automerge`, `-mergeoutput`, `-vcs1..4`, `-title1..4`, `-bcexitcode`, `-qc`) aur slash (`/ro`, `/automerge`, `/mergeoutput`) normalization seamlessly translate hoti hai into canonical options. Read-only safety guard mutations ko block karta hai with exit code 2.
- **4-Way Git Mergetool Mapping**: Git mergetool syntax `bcomp $LOCAL $REMOTE $BASE $MERGED` automatically resolve hoti hai into 3-way merge (`merge $BASE $LOCAL $REMOTE --output=$MERGED`) ya directory detection par `folder-merge`.
- **Exit Code Conventions Matrix**:
  - Diff mode (default): 0 = identical/clean merge, 1 = diff/conflicts, 2 = error / read-only violation / bad args.
  - Beyond Compare mode (`-bcexitcode` / `-qc`): 0 = success, 11 = binary diff, 13 = rules-based diff, 14 = conflicts detected.
- **Session Dispatch**: `crates/cli/src/session_cmd.rs` me `audio` aur `sync` tab types add karke session reload coverage complete ki gayi.
- **Folder Merge Automation**: `crates/cli/src/runner.rs` me `-automerge` with output path folder tree merge ko directly apply karta hai.

---

## 2. Test Verification
Command: `CARGO_BUILD_JOBS=2 cargo test -p autocompare-cli`
Result: **27 passed, 0 failed, 0 ignored** across 6 test suites:
- `bcomp_test.rs`: 8 passed, 0 failed
- `errors_test.rs`: 5 passed, 0 failed
- `session_test.rs`: 1 passed, 0 failed
- `subcommands_media_test.rs`: 5 passed, 0 failed
- `subcommands_text_test.rs`: 4 passed, 0 failed
- `parity_hardening_test.rs`: 4 passed, 0 failed

---

## 3. Parity Harness & ac-loop Verification
Command: `tools/ac-loop/run.sh checks/agyx-cli-k/round-1`
Result: **RESULT: PASS**
- CLI parity: PASS (10/10 subcommands match exact expected exit codes in `cli/parity-report.md`)
- GUI geo-typo: PASS (screenshot size, magic bytes, distinct bytes validated in `gui/geo-typo-report.md`)
- Comparison edge fixtures: 5 sets of edge fixtures and expected outcomes generated in `checks/agyx-cli-k/round-1/comparison/`:
  - `edge-mergetool-4way.txt` + `edge-mergetool-4way-expected.out`
  - `edge-bcomp-vcs-flags.txt` + `edge-bcomp-vcs-flags-expected.out`
  - `edge-report-all-formats.txt` + `edge-report-all-formats-expected.out`
  - `edge-exit-codes-matrix.txt` + `edge-exit-codes-matrix-expected.out`
  - `edge-10-subcommands.txt` + `edge-10-subcommands-expected.out`

---

## 4. Clippy & Formatting
- `cargo clippy --package autocompare-cli --all-targets --no-deps -- -D warnings`: **ZERO warnings**
- `cargo clippy --package autocompare-cli --lib --no-deps -- -D clippy::indexing_slicing`: **ZERO warnings**
- `cargo clippy --package autocompare-cli --bin autocompare --no-deps -- -D clippy::indexing_slicing`: **ZERO warnings**
- `rustfmt --edition 2024 --check crates/cli/src/*.rs crates/cli/tests/*.rs`: **Clean (exit 0)**

---

## 5. MS Rust Guidelines & Banned Tokens Check
Non-test code (`crates/cli/src/**`) strictly enforces:
- `#![forbid(unsafe_code)]` in both `lib.rs` and `main.rs`.
- Zero raw `unwrap()`, zero `expect()`, zero `panic!`, zero `todo!`, zero `unimplemented!`, zero `unreachable!`.
- Zero indexing/slicing (`[...]`) in library and binary code.
- Verification command output: `ZERO banned tokens`.

---

## 6. OWNS Boundary Proof
Only owned files touched:
- `crates/cli/src/runner.rs`
- `crates/cli/src/session_cmd.rs`
- `crates/cli/tests/parity_hardening_test.rs`
- `checks/agyx-cli-k/GATES.md`
- `checks/agyx-cli-k/REPORT.md`
- `checks/agyx-cli-k/round-1/**`

All forbidden files (`STATUS.md`, `PLAN.md`, `inbox/build-progress.json`, `crates/gui/**`, `crates/diff/**`, `crates/compare/**`, `crates/sessions/**`, `crates/reports/**`, `crates/vfs/**`) are completely untouched.
Local commit executed via strict pathspec ONLY. NO git push.
