# GATES — round-folder30 (unlazy, BEFORE implementation)
- Goal: Folder parity — tree expand/collapse + SyncPlanner Mirror/Update dry-run/apply --prune orphans + JobRunner cancel + 64 queue.
- Owns: ONLY crates/compare/src/folder.rs + crates/compare/src/sync.rs + crates/compare/src/jobs.rs + checks/ac-loop/round-folder30/** + crates/compare/tests/folder* + sync* + jobs*.
- Forbid: NEVER gui/cli, never copy assets, clean-room outside only.
- MS Rust: forbid unsafe, no unwrap/expect/panic/todo non-test, headless-first, cargo fmt.
- Gates:
  1. run-round.sh checks/ac-loop/round-folder30 -> capture PASS/FAIL (parity-report.md).
  2. Minimal diff fix in OWNS only; no gui/cli touches.
  3. Re-run -> RESULT: PASS.
  4. Edge fixture: nested-orphan-prune (deep orphan dir on losing side pruned in one step, descendants suppressed).
  5. clippy -D warnings + cargo fmt + banned greps (unwrap/expect/panic/todo outside tests) = 0.
- Done = commit "feat(compare): folder30 sync prune + jobs" OWNS only, hash > /tmp/wave-folder30.done, NO PUSH.
