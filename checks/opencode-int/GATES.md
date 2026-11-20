# GATES — opencode-int (OP-INT integration re-verify, run-only, no code)

OWNS: `checks/opencode-int/**` only. Source me ZERO edit, `git add -A` nahi, push nahi.
Env: `CARGO_TARGET_DIR=/tmp/opencode-int-target`, `CARGO_BUILD_JOBS=2`. Target dir
se bahar likha kuch nahi: sirf read-only runs + `target/debug/*` binaries ka smoke.
`skills/ac-loop/SKILL.md` run-only padha (Check phase only, max 6 rounds, har round me
fresh logs + 1 PNG agar GUI build hota hai).

## G1 — xtask layers ok
- CHECK: `cargo run -p xtask -- layers`
- EXPECT: `layers: ok`, exit 0
- EVIDENCE: `round-1/xtask-layers.out`

## G2 — CLI --help (10 subcommands, goal me 9 likha — truth 10)
- CHECK: `target/debug/autocompare --help` + har subcommand `<cmd> --help` exit 0
- EXPECT: text folder hex merge table sync picture folder-merge audio session (10/10)
- EVIDENCE: `round-1/cli-help.out`, `round-1/cli-<cmd>.out`
- NOTE: goal me "9 subcommands" likha hai, binary me 10 hain (early-feedback-3 bhi 10).
  Goal ka number galat hai, binary sahi hai — gap nahi, note hai.

## G3 — GUI --screenshot smoke (1 PNG, 960x600, magic ok)
- CHECK: `target/debug/autocompare-gui --screenshot round-1/smoke.png`
- EXPECT: exit 0, PNG 960x600, magic `89504e47`
- EVIDENCE: `round-1/smoke.png`, `round-1/gui-screenshot.out`, `round-1/png-meta.out`

## G4 — dist tarballs + sha256 verify
- CHECK: `shasum -a 256 -c dist/autocompare-0.5.0-macos-arm64.tar.gz.sha256` (repo root se)
- EXPECT: OK, exit 0. Baaki 2 tarballs ke hash `round-1/sha256-all.out` me record.
- EVIDENCE: `round-1/sha256.out`, `round-1/sha256-all.out`

## G5 — TOP-10 gaps delta vs checks/early-feedback-3 (ISSUES delta, fix nahi)
- CHECK: `checks/early-feedback-3/ISSUES.md` D1–D8 se compare, koi naya band/khula gap?
- EXPECT: delta list REPORT.md me, koi source edit nahi
- EVIDENCE: `REPORT.md` section 3

## G6 — commit hygiene
- CHECK: `git status --porcelain -- checks/opencode-int`, commit pathspec sirf `checks/opencode-int`
- EXPECT: `git add -A` nahi, push nahi, message `checks(int): <scope>` English conventional
- EVIDENCE: `git log --oneline -3 -- checks/opencode-int`
