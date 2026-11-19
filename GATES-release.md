# Gates: v0.5.0 installable binary for early feedback

OWNS: target/release/**, checks/release-smoke/**

Scope: Rebuild release binaries after HEAD fcfcf12, smoke validate, publish GitHub release with both binaries attached.

- [x] G1: Release binaries are fresh (built after HEAD commit) and executable.
  CHECK: node scripts/verify-release-fresh.mjs
  EXPECT: RELEASE_FRESH_OK
  EVIDENCE: exit=0; shell=/bin/sh; cwd=/Users/ektasaini/autocompare; path=88c67685bb82/56 entries; output=RELEASE_FRESH_OK

- [x] G2: CLI honors diff-convention exits and GUI binary starts headless check clean.
  CHECK: tools/parity-harness/run-round.sh checks/release-smoke
  EXPECT: RESULT: PASS
  EVIDENCE: exit=0; shell=/bin/sh; cwd=/Users/ektasaini/autocompare; path=88c67685bb82/56 entries; output=folder-merge=1 (want 1) | audio=1 (want 1)

- [x] G3: GitHub release v0.5.0 exists with both binaries attached.
  CHECK: node scripts/verify-release-published.mjs
  EXPECT: RELEASE_PUBLISHED_OK
  EVIDENCE: exit=0; shell=/bin/sh; cwd=/Users/ektasaini/autocompare; path=88c67685bb82/56 entries; output=RELEASE_PUBLISHED_OK
