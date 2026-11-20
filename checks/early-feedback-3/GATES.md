# GATES — early-feedback-3 (2026-11-20)

Scope: read-only local run. Source me ZERO edit (`git status` me `crates/**` ke
badlav dusre node ke hain, maine nahi chhue; mera add sirf `checks/early-feedback-3/`).
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

Usage lines round-2 se akshar-akshar same — CLI surface me koi drift nahi.
Top-level flags maujood (round-2 wale sab + kuch aur dikhe):
`--report`, `--ro/--ro1/--ro2`, `--automerge`, `--mergeoutput`,
`--vcs1..4`, `--title1..4`, `--bc-exit-code`, `--exit-code-mode`, `--solo`,
`--nobackups`, `--reviewconflicts`, `--save-session`, `--session`, `--report-title` — PASS.

## Gate 2 — screenshots valid (3 PNGs, sab 960x600, magic ok, round-2 se byte-same)

| File | Bytes | Sha12 | Magic | Size | Gate |
|---|---|---|---|---|---|
| home.png | 15007 | ac9946b7acdc | 89504e47 ok | 960x600 | PASS |
| fixture-text.png | 9765 | bde8ccf481c6 | 89504e47 ok | 960x600 | PASS |
| fixture-folder.png | 10149 | f9c6a846ef24 | 89504e47 ok | 960x600 | PASS |

3/3 PASS. Teenon sha round-2 se byte-match — render deterministic, visual drift ZERO.
Route note: `--screenshot <naam>` file-naam se fixture chunta hai
(`home`→home, `folder`→folder, default→text) — `crates/gui/src/screenshot.rs` routed fn.

## Gate 3 — BC outside-observe only

`osascript` process list me `BCompare` live — PASS.
Koi window click, koi AX dump, koi string/asset copy nahi — clean-room PASS.
Session types sirf `docs/study.md` headings + apne `--help` se confirm (9 sessions:
Folder Compare, Folder Merge, Folder Sync, Text Compare, Text Merge, Hex Compare,
Media Compare, Picture Compare, Table Compare + 1 session loader) — PASS, apne words me.

## Gate 4 — gaps listed (TOP deltas vs round-2, apne words)

ISSUES.md me round-2 ke mukable delta list, har ek me file+line+expected — PASS.
Khud koi fix nahi kiya — PASS.

## Gate 5 — commit hygiene

Pathspec sirf `checks/early-feedback-3`, `git add -A` nahi, push nahi — PASS (neeche verify).
