# AGYX-CLI-H2 Parity Hardening Report (leaf-3.1 PLAN C9 Follow-Up)

## 1. Summary
 leaf-3.1 CLI aur VCS compatibility hardening round complete ho gaya hai:
- **bcomp-Compatible VCS Switches**: Full support for legacy single-dash and slash switches:
  - Read-Only switches (`-ro`, `/ro`, `-ro1`, `/ro1`, `-ro2`, `/ro2`, `-ro3`, `/ro3`, `-ro4`, `/ro4`, `-readonly`, `/readonly`, `-leftreadonly`, `-rightreadonly`, `-centerreadonly`, `-outputreadonly`) normalize to canonical clap options. Read-only guards enforce exit code 2 on any mutation attempts.
  - Merge Automation switches (`-automerge`, `/automerge`, `-reviewconflicts`, `/reviewconflicts`, `-mergeoutput=<path>`, `/mergeoutput=<path>`, `-savetarget=<path>`, `/savetarget=<path>`, and space-separated `-mergeoutput <path>`).
  - VCS Aliases and Inputs (`-vcs1..4`, `/vcs1..4`, `-ancestor`, `-base`, `-center`) support both `=` and space-separated syntax.
  - Pane and Report Titles (`-title1..4`, `/title1..4`, `-lefttitle`, `-righttitle`, `-centertitle`, `-ancestortitle`, `-outputtitle`, `-report-title`, `/report-title`, `-reporttitle`).
  - Session Shorthand: `@<session_path>` maps seamlessly to `--session=<session_path>`.
- **Direct Invocation Resolution**:
  - Git mergetool 4-positional syntax (`bcomp $LOCAL $REMOTE $BASE $MERGED`) maps to 3-way merge `merge $BASE $LOCAL $REMOTE --output=$MERGED` or directory detection par `folder-merge`.
  - 2 positional arguments with `-ancestor`/`--vcs3` resolve to 3-way merge.
  - 2 positional arguments without subcommand resolve to `text` or `folder` based on filesystem type detection.
  - 3 positional arguments without subcommand resolve to `merge` or `folder-merge` based on filesystem type detection.
  - VCS inputs (`--vcs1`, `--vcs2`) without positional arguments resolve to `text`/`folder` or `merge`/`folder-merge` if `--vcs3` is provided.
- **Multi-Format Report Generation Wiring**:
  - All 6 report format targets supported: HTML (`html` / `html-color` / `color`), Monochrome HTML (`html-mono` / `mono`), Plain Text (`text` / `txt`), XML (`xml`), Unified Patch (`patch` / `diff` / `unified`), and Context Patch (`context` / `context-patch`).
  - All 9 session adapters covered: Text (`text_report_input`), Folder (`folder_report_input`), Hex (`hex_report_input`), Merge (`merge_report_input`), Table (`table_report_input`), Sync (`sync_report_input`), Picture (`picture_report_input`), FolderMerge (`foldermerge_report_input`), Audio (`audio_report_input`).
  - Automatic directory hierarchy creation for nested report output paths.
  - Report title overrides correctly propagated across all formats.
- **Exit Code Conventions Matrix**:
  - Diff(1) Mode: 0 = identical / clean merge, 1 = differences / merge conflicts, 2 = trouble / read-only error / bad syntax.
  - Beyond Compare Mode (`-bcexitcode` / `-qc`): 0 = clean / success, 11 = binary diff, 13 = rules-based diff, 14 = conflicts detected.

---

## 2. Key Metrics
- **Tests**: **51 passed, 0 failed, 0 ignored** across `autocompare-cli` test suite (28 unit tests in `src/lib.rs`, 23 integration tests in `tests/`).
- **Clippy**: **0 warnings** (`cargo clippy -p autocompare-cli --all-targets -- -D warnings`).
- **Banned-Token Scan**: **0 violations** in non-test source code (zero raw `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable`, `unsafe`, or raw indexing `[...]`).
- **Formatting**: **Clean** (`rustfmt --edition 2024 --check crates/cli/src/*.rs`).

---

## 3. Acceptance Gates Status (checks/agyx-cli-h2/GATES.md)
- **Gate 1 (Full test suite green)**: PASS (`CARGO_TARGET_DIR=/tmp/target-agyx-cli CARGO_BUILD_JOBS=2 cargo test -p autocompare-cli` -> 51 passed, 0 failed).
- **Gate 2 (Clippy zero warnings)**: PASS (`CARGO_TARGET_DIR=/tmp/target-agyx-cli cargo clippy -p autocompare-cli --all-targets -- -D warnings` -> Finished with 0 warnings).
- **Gate 3 (Rustfmt clean)**: PASS (`rustfmt --edition 2024 --check crates/cli/src/*.rs` -> exit code 0).
- **Gate 4 (Zero banned patterns in non-test code)**: PASS (automated AST/regex scan verified zero unwrap, expect, panic, todo, unimplemented, unreachable, unsafe, or indexing).
- **Gate 5 (bcomp VCS switches tests green)**: PASS (`bcomp::tests` 11 unit & fixture integration tests passed).
- **Gate 6 (Report wiring and adapters tests green)**: PASS (`report::tests`, `report_adapter::tests`, `report_adapter_media::tests` 17 tests passed).
- **Gate 7 (Edge fixtures verified)**: PASS (`edge-bcomp-compat-vcs.txt`, `edge-report-wiring.txt`, `edge-exit-codes-bc-mode.txt` present with expected outcomes).
- **Gate 8 (OWNS boundary proof)**: PASS (only owned files touched, zero modifications to runner.rs, args.rs, main.rs, lib.rs, or any other shared paths).

---

## 4. Edge Fixtures in comparison/
Edge fixtures generated in `checks/agyx-cli-h2/round-1/comparison/`:
1. `edge-bcomp-compat-vcs.txt` + `edge-bcomp-compat-vcs-expected.out`
2. `edge-report-wiring.txt` + `edge-report-wiring-expected.out`
3. `edge-exit-codes-bc-mode.txt` + `edge-exit-codes-bc-mode-expected.out`

---

## 5. OWNS Strict Boundary
Owned paths modified:
- `crates/cli/src/bcomp.rs`
- `crates/cli/src/report.rs`
- `crates/cli/src/report_adapter.rs`
- `crates/cli/src/report_adapter_media.rs`
- `checks/agyx-cli-h2/GATES.md`
- `checks/agyx-cli-h2/round-1/REPORT.md`
- `checks/agyx-cli-h2/round-1/comparison/**`

All forbidden files (`crates/cli/src/runner.rs`, `args.rs`, `main.rs`, `lib.rs`, `crates/cli/tests/**`, `STATUS.md`, `PLAN.md`, `inbox/**`, `crates/gui/**`, `crates/diff/**`, `crates/compare/**`, `crates/sessions/**`, `crates/reports/**`, `crates/vfs/**`) were never touched.

RESULT: PASS
