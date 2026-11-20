# checks/cli-vcs2 round-1 — CLI & VCS remainder (leaf-3.1 retry)

Date: 2026-11-20. Clean-room only: outside observation, no decompilation, no asset copying.
OWNS: `crates/cli/src/args.rs`, `crates/cli/src/runner.rs`, `crates/cli/src/session_cmd.rs`, `checks/cli-vcs2/**`.
Unowned paths with pre-existing WIP (gui, sessions, other checks) were neither read for edits nor staged.

## WIP resumed

Inherited uncommitted WIP in the three owned files only:

- `args.rs`: `--ro3`/`--ro4` flags + `is_left/right/base/output_read_only()` helpers (VCS pane granularity).
- `runner.rs`: `resolve_report_title()` (explicit `--report-title` > `--title1..3` > default),
  hex outcome via `render_hex::outcome().is_diff()`, merge/folder-merge session save with base+output paths,
  `ro4` read-only guards on merge/folder-merge apply paths.
- `session_cmd.rs`: session tab title propagation into all `--report` inputs; `merge` and
  `folder-merge`/`folder_merge` tab execution (was: unsupported type error);
  `save_session_with_merge_paths()` preserving base/output/read-only.

No changes were needed beyond the WIP: verification below is against the resumed state as-is
(minimal diff, reuse, stdlib first, no new deps).

## Gate evidence

- `CARGO_TARGET_DIR=/tmp/target-op-clivcs2 CARGO_BUILD_JOBS=2 cargo test -p autocompare-cli`
  → lib 37 + bcomp 8 + errors 5 + parity_hardening 4 + session 1 + media 5 + text 4 = **64 passed, 0 failed**.
- `cargo clippy -p autocompare-cli --all-targets -- -D warnings` → **zero warnings** (exit 0).
- Banned-API scan (`unwrap_used`/`expect_used` deny per workspace Cargo.toml): **zero hits in non-test
  code** (`unwrap_or`/`unwrap_err`-in-tests-with-allow only). `expect()` calls live under `#[cfg(test)]`
  modules carrying the explicit clippy allow.
- Only `/tmp/target-op-clivcs2` used for build artifacts; repo `target/` untouched by this leaf.

## 9-subcommand exit-code matrix (binary `/tmp/target-op-clivcs2/debug/autocompare`, 25/25)

| # | case | want | got |
|---|---|---|---|
| 1 | text same (diff mode) | 0 | 0 |
| 2 | text diff (diff mode) | 1 | 1 |
| 3 | text diff (`--bc-exit-code`) | 13 | 13 |
| 4 | folder diff | 1 | 1 |
| 5 | hex same | 0 | 0 |
| 6 | hex diff (diff mode) | 1 | 1 |
| 7 | hex diff (bc mode) | 11 | 11 |
| 8 | merge clean | 0 | 0 |
| 9 | merge conflict (diff mode) | 1 | 1 |
| 10 | merge conflict (bc mode) | 14 | 14 |
| 11 | table diff | 1 | 1 |
| 12 | sync one-sided dry run | 1 | 1 |
| 13 | sync content-`Different`-only (planner skips, no side wins) | 0 | 0 |
| 14 | picture diff | 1 | 1 |
| 15 | folder-merge dry run | 1 | 1 |
| 16 | audio diff | 1 | 1 |
| 17 | session run (saved text tab, diff) | 1 | 1 |
| 18 | text `--report html:` (diff) | 1 | 1 |
| 19 | git-mergetool 4-positional (`local remote base merged`) | 1 | 1 |
| 20 | `-vcs1= -vcs2=` passthrough (2-way text) | 1 | 1 |
| 21 | `-ancestor=` + 2 positionals → merge | 1 | 1 |
| 22 | `-ro merge --output` guard | 2 | 2 |
| 23 | `-ro sync --apply` guard | 2 | 2 |
| 24 | missing input file (trouble) | 2 | 2 |
| 25 | `--report html:` empty path (trouble) | 2 | 2 |

## --report wiring

- `--report-title EdgeTitle` content found verbatim in generated HTML.
- `--title1 L-pane --title2 R-pane` both found verbatim in generated HTML (VCS title path).
- `session` subcommand + `--report` writes report using the session tab title (covered by
  `test_session_report_generation_with_tab_title` and live case 17 variant).

## VCS passthrough notes

- Single-dash/slash normalization (`-ro`, `-automerge`, `-mergeoutput`, `-vcs1..4`, `-title1..4`,
  `-ancestor/-base/-center`, `@session`, `-qc/-bcexitcode`) covered by 8 `bcomp_test` cases, all green.
- 4-positional git-mergetool order (local remote base merged → `merge base local remote --output=merged`)
  verified live (case 19); dir-quad routes to `folder-merge` (unit-covered).
- Edge observation (not owned, no change): `plan_sync` always skips `Different` content entries
  ("content differs; no side wins", `crates/compare/src/sync.rs:222`, rule documented in
  `crates/compare/src/lib.rs:396`), so a sync pair differing only in bytes with no one-sided/newer
  entry dry-runs to exit 0. One-sided/newer entries still plan steps → exit 1 (case 12). Behavior is
  planner-owned; CLI mapping (`Outcome::for_diff(!steps.is_empty())`) is correct as-is.

## Files changed (owned only)

- `crates/cli/src/args.rs` (WIP resumed, verified)
- `crates/cli/src/runner.rs` (WIP resumed, verified)
- `crates/cli/src/session_cmd.rs` (WIP resumed, verified)
- `checks/cli-vcs2/round-1/REPORT.md` (this file)

RESULT: PASS
