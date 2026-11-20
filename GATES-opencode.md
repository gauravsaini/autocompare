# Gates: opencode-parallel-waves

OWNS: checks/opencode-study/**, checks/opencode-int/**, docs/study.md, capture/opencode-study/**

Scope: 2 opencode default-model subagents, zero-collision vs 7 live agyx, run-only + study-only, no source edits.

- [ ] G1: OP-STUDY study refresh lands local commit, REPORT with HIGH/MEDIUM/UNVERIFIED, no sibling OWNS touched
  CHECK: git log --oneline -5 -- checks/opencode-study docs/study.md capture/opencode-study
  EXPECT: OP-STUDY commit present
  EVIDENCE: pending

- [ ] G2: OP-INT integration re-verify lands local commit, layers ok + CLI help + tarball sha256, no source edits
  CHECK: git log --oneline -5 -- checks/opencode-int
  EXPECT: OP-INT commit present
  EVIDENCE: pending
