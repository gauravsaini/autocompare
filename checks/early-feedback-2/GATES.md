# GATES — early-feedback-2 (2026-11-20)

Scope: read-only local run. Source me ZERO edit (`git status` me sirf `checks/early-feedback-2/` untracked).
`$unlazy` order: GATES.md pehle, phir evidence, phir done. `skills/ac-loop/SKILL.md` read-only padha.

## Gate 1 — CLI smoke (10/10 subcommands + top --help)

`target/debug/autocompare --help` exit 0 — PASS.
Har subcommand `<cmd> --help` exit 0 — PASS (10/10):

| # | Cmd | Usage line | Gate |
|---|---|---|---|
| 1 | text | `autocompare text [OPTIONS] <LEFT> <RIGHT>` | PASS |
| 2 | folder | `autocompare folder [OPTIONS] <LEFT> <RIGHT>` | PASS |
| 3 | hex | `autocompare hex [OPTIONS] <LEFT> <RIGHT>` | PASS |
| 4 | merge | `autocompare merge [OPTIONS] <BASE> <LEFT> <RIGHT>` | PASS |
| 5 | table | `autocompare table [OPTIONS] <LEFT> <RIGHT>` | PASS |
| 6 | sync | `autocompare sync [OPTIONS] <LEFT> <RIGHT>` | PASS |
| 7 | picture | `autocompare picture [OPTIONS] <LEFT> <RIGHT>` | PASS |
| 8 | folder-merge | `autocompare folder-merge [OPTIONS] <BASE> <LEFT> <RIGHT>` | PASS |
| 9 | audio | `autocompare audio [OPTIONS] <LEFT> <RIGHT>` | PASS |
| 10 | session | `autocompare session [OPTIONS] <PATH>` | PASS |

Top-level flags maujood: `--report`, `--ro/--ro1/--ro2`, `--automerge`, `--mergeoutput`,
`--vcs1..4`, `--title1`, `--bc-exit-code`, `--solo`, `--nobackups`, `--reviewconflicts` — PASS.

## Gate 2 — screenshots valid (3 PNGs, sab 960x600, magic ok)

| File | Bytes | Sha12 | Magic | Gate |
|---|---|---|---|---|
| home.png | 15007 | ac9946b7acdc | 89504e47 ok | PASS |
| fixture-text.png | 9765 | bde8ccf481c6 | 89504e47 ok | PASS |
| fixture-folder.png | 10149 | f9c6a846ef24 | 89504e47 ok | PASS |

3/3 PASS. `fixture-text.png` sha `gui-localrun-early` wale `after.png` se match
(deterministic render — PASS, gap nahi). `fixture-folder.png` sha bhi purane
`fixture-folder.png` (`f9c6a846ef24`) se match — deterministic — PASS.

## Gate 3 — BC outside-observe only

`osascript` process list me `BCompare` live — PASS.
Koi window click, koi AX dump, koi string/asset copy nahi — clean-room PASS.
Session types sirf `docs/study.md` headings + `--help` se confirm (9 sessions) — PASS.

## Gate 4 — gaps listed (TOP-10, apne words)

ISSUES.md me 10 geometry/typography deltas, har ek me file+line+expected — PASS.
Khud koi fix nahi kiya — PASS.

## Gate 5 — commit hygiene

Pathspec sirf `checks/early-feedback-2`, `git add -A` nahi, push nahi — PASS (verify neeche).
