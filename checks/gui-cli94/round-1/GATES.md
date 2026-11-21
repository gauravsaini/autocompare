# Gates: leaf-3.1 cli94 round-1

OWNS: crates/cli/**, checks/gui-cli94/**

Scope: CLI + VCS round-1 — 9 subcommands, report CLI, bcomp VCS flags (-ro, -automerge, -mergeoutput), exit codes 0/1/2, no GUI touch.

- [x] leaf-3.1:G1: All 9 CLI subcommands + report + bcomp VCS flags pass fixture tests with exit codes 0/1/2 and BC mode.
  CHECK: cargo test -p autocompare-cli
  EXPECT: test result: ok.
  EVIDENCE: checks/gui-cli94/round-1/tests.out + REPORT.md (2026-11-21, base 7c89620)

- [x] leaf-3.1:G2: Workspace fmt clean for owned crate and no other-leaf paths touched.
  CHECK: cargo fmt --check -p autocompare-cli && echo fmt-clean
  EXPECT: fmt-clean
  EVIDENCE: checks/gui-cli94/round-1/tests.out + REPORT.md (2026-11-21, base 7c89620)

- [x] leaf-3.1:G3: Prod (non-test) code has zero banned constructs (unsafe, unwrap/expect/panic/todo in prod).
  CHECK: python3 checks/gui-cli94/round-1/check_banned.py
  EXPECT: BANNED_ZERO
  EVIDENCE: checks/gui-cli94/round-1/tests.out + REPORT.md (2026-11-21, base 7c89620)

- [x] leaf-3.1:G4: Binary end-to-end — text same/diff exit codes, bcomp -ro guard, and --report write work via built binary.
  CHECK: sh checks/gui-cli94/round-1/check_e2e.sh
  EXPECT: E2E_OK
  EVIDENCE: checks/gui-cli94/round-1/tests.out + REPORT.md (2026-11-21, base 7c89620)
