# GATES — Wave BI-MERGESYNC (research-only audit, zero code edits)

Wave: BI-MERGESYNC. OWNS: `checks/gui-mergesync-bi/**` ONLY.
KEVL (mat chhoona, read-only grep ok): `crates/**`, `checks/opencode-waves/STATUS.md`,
dusre agents ki working-tree files (`crates/gui/src/views_hex.rs` ka uncommitted
diff parallel agent ka hai — haath nahi lagaya). Colors FROZEN. NO push.

## G1 — Read-only proof (crates me zero edit)

- CHECK: `git status --short -- crates/ && git diff --stat -- crates/`
- EXPECT: mere naam par koi crates diff nahi (jo dikhe woh parallel agent ka,
  mere commit me shaamil nahi)
- EVIDENCE: commit pathspec sirf `checks/gui-mergesync-bi/`; REPORT me scope note

## G2 — Harness PASS proof

- CHECK: `bash tools/gui-polish/run-geo-typo.sh checks/gui-mergesync-bi/round-1`
- EXPECT: EXIT 0 + `round-1/geo-typo-report.md` me `RESULT: PASS`
- EVIDENCE: `round-1/geo-typo-report.md` (PNG 9695 B, magic ok, distinct 255),
  `round-1/after.png`, `round-1/tests.out`, `round-1/screenshot.out`

## G3 — Spec table + rank (REPORT me)

- CHECK: REPORT me har gap ke liye fn + lines + change + OWNS file + P0/P1/P2
- EXPECT: 8 gaps (BI-M1..M4, BI-S1..S4), har ek me exact follow-up spec
- EVIDENCE: `REPORT.md` "Gap spec table"

## G4 — PNG magic on round-1 PNGs

- CHECK: `file round-1/*.png` + magic bytes `89504e470d0a1a0a` + distinct count
- EXPECT: `merge-ref.png` (8829 B, magic ok, distinct 256),
  `folder-sync-ref.png` (10068 B, magic ok, distinct 256) — dono existing copies
- EVIDENCE: REPORT "Harness evidence" + `round-1/merge-ref.png`,
  `round-1/folder-sync-ref.png`

## G5 — Commit, no push

- CHECK: `git log --oneline -1` + `git status --short` (sirf OWNS paths staged the)
- EXPECT: `docs(checks): ...` conventional message, local commit only
- EVIDENCE: commit hash REPORT me nahi, log me; `git push` kabhi nahi chalaya

## G6 — STATUS.md untouched

- CHECK: `git diff --name-only | grep opencode-waves/STATUS.md || echo UNTOUCHED`
- EXPECT: UNTOUCHED
- EVIDENCE: gate command output

## DoD

- REPORT + GATES + round-1 (harness PASS + 2 PNG copies magic-ok), commit (no push).
