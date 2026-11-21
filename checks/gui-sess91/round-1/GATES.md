# Gates: sess91 round-1 — sessions persistence hardening

OWNS: crates/sessions/**, checks/gui-sess91/**

Scope: Harden SessionDocument/WorkspaceDocument/AppSettings persistence (schema_version + migration, clamp active_tab_index, atomic file saves, JSON round-trip) with zero GUI changes.

- [x] sess91:G1: Session, Workspace, and Settings JSON round-trip + migration tests pass.
  CHECK: cargo test -p autocompare-sessions --quiet && echo SESS_TESTS_OK
  EXPECT: SESS_TESTS_OK
  EVIDENCE: exit 0, SESS_TESTS_OK; 34 unit + 11 corrupt + 20 migration + 12 round_trip = 77 passed, 0 failed (checks/gui-sess91/round-1/tests.out, 108 lines)

- [x] sess91:G2: Formatting is clean for the sessions crate.
  CHECK: cargo fmt -p autocompare-sessions -- --check && echo FMT_OK
  EXPECT: FMT_OK
  EVIDENCE: exit 0, FMT_OK

- [x] sess91:G3: Strict clippy lints pass for the sessions crate.
  CHECK: cargo clippy -p autocompare-sessions --all-targets -- -D warnings && echo CLIPPY_OK
  EXPECT: CLIPPY_OK
  EVIDENCE: exit 0, CLIPPY_OK, zero warnings

- [x] sess91:G4: Zero banned constructs in non-test production code.
  CHECK: node checks/gui-sess91/round-1/check-banned.mjs
  EXPECT: BANNED_0
  EVIDENCE: exit 0, BANNED_0; negative control: fixture with o.unwrap() -> BANNED_HITS 1, exit 1

- [x] sess91:G5: Working tree touches only owned paths (no strays, no GUI).
  CHECK: node checks/gui-sess91/round-1/check-owns.mjs
  EXPECT: OWNS_OK
  EVIDENCE: exit 0, OWNS_OK; negative control: crates/gui/__STRAY_PROBE__ -> STRAY_HITS 1, exit 1, probe removed
