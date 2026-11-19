# Gates: v0.5.0 installable binary for early feedback

OWNS: target/release/**, checks/release-smoke/**

Scope: Rebuild release binaries after HEAD fcfcf12, smoke validate, publish GitHub release with both binaries attached.

- [ ] G1: Release binaries are fresh (built after HEAD commit) and executable.
  CHECK: node scripts/verify-release-fresh.mjs
  EXPECT: RELEASE_FRESH_OK
  EVIDENCE: pending

- [ ] G2: CLI honors diff-convention exits and GUI binary starts headless check clean.
  CHECK: tools/parity-harness/run-round.sh checks/release-smoke
  EXPECT: RESULT: PASS
  EVIDENCE: pending

- [ ] G3: GitHub release v0.5.0 exists with both binaries attached.
  CHECK: node scripts/verify-release-published.mjs
  EXPECT: RELEASE_PUBLISHED_OK
  EVIDENCE: pending
