# GATES — early-feedback-4 (2026-11-20)

Scope: read-only local run. Source me ZERO edit (mera add sirf `checks/early-feedback-4/`;
working tree me `crates/**` ke badlav dusre node ke hain, maine nahi chhue).
`$unlazy` order: GATES.md pehle, phir evidence, phir done. `skills/ac-loop/SKILL.md` read-only padha.
`CARGO_BUILD_JOBS=2` set tha; rebuild nahi chalaya (bane hue debug binaries ka smoke hi goal tha).

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

Usage lines round-3 se akshar-akshar same — CLI surface me koi drift nahi.
Top-level flags maujood (round-3 wale sab):
`--report`, `--ro/--ro1/--ro2`, `--automerge`, `--mergeoutput`,
`--vcs1..4`, `--title1..4`, `--bc-exit-code`, `--exit-code-mode`,
`--solo`, `--nobackups`, `--reviewconflicts`, `--save-session`, `--session`,
`--report-title` — PASS.

## Gate 2 — screenshots valid (3 PNGs, sab 960x600, magic ok) + drift note

| File | Bytes | Sha12 | Magic | Size | vs round-3 | Gate |
|---|---|---|---|---|---|---|
| home.png | 14991 | 36ebb7a8eccb | 89504e47 ok | 960x600 | DRIFT (15007B tha) | PASS-valid |
| fixture-text.png | 9765 | bde8ccf481c6 | 89504e47 ok | 960x600 | BYTE-SAME | PASS |
| fixture-folder.png | 10153 | c5b54d0989f8 | 89504e47 ok | 960x600 | DRIFT (10149B tha) | PASS-valid |

3/3 PASS-valid (magic+size ok). Text render byte-same — determinism barkarar.
Home/folder drift ka kaaran tay nahi: tree me dusre node ke uncommitted
`crates/**` badlav (19 files) + binaries 18:35 bane — committed-main ka nahi lagta.

## Gate 3 — BC outside-observe only

`osascript` process list me `BCompare` live — PASS.
Koi window click, koi AX dump, koi string/asset copy nahi — clean-room PASS.
Session types sirf `docs/study.md` headings + apne `--help` se confirm (9 sessions:
Folder Compare, Folder Merge, Folder Sync, Text Compare, Text Merge, Hex Compare,
Media Compare, Picture Compare, Table Compare + 1 session loader) — PASS, apne words me.

## Gate 4 — gaps listed (TOP-10, apne words)

ISSUES.md me G1–G10 geometry/typography deltas + N1 drift note,
har ek me file+line+expected — PASS.
Khud koi fix nahi kiya — PASS.

## Gate 5 — commit hygiene

Pathspec sirf `checks/early-feedback-4`, `git add -A` nahi, push nahi — PASS (neeche verify).
