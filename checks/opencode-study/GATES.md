# Gates: opencode-study refresh (2026-11-20)

OWNS: checks/opencode-study/**, capture/opencode-study/**, docs/study.md.
Baaki repo ko touch karna mana hai.

- [x] G1: Study refresh local commit me land, REPORT me
  HIGH/MEDIUM/UNVERIFIED labels, sibling OWNS ko touch nahi
  CHECK: git log --oneline -3 -- checks/opencode-study docs/study.md capture/opencode-study
  EXPECT: docs(study) commit maujood
  EVIDENCE: REPORT.md niche

- [x] G2: Har round me 1+ observation note + 1 fixture idea
  CHECK: ls capture/opencode-study
  EXPECT: round-1-session-types.md, round-2-view-tools.md,
  round-3-geometry-search.md (har ek me fixture idea)
  EVIDENCE: 3 notes, 3 fixture ideas

- [x] G3: Clean-room + commit hygiene
  CHECK: git show --stat HEAD
  EXPECT: sirf OWNS paths, pathspec commit, NO PUSH, koi source
  edit nahi, koi cargo test nahi, decompile/strings nahi
  EVIDENCE: commit stat niche
