# GATES — early-feedback-5 (2026-11-20)

Scope: read-only local run. Source me ZERO edit (mera add sirf
`checks/early-feedback-5/`; working tree me `crates/**` ke badlav dusre
node ke hain, maine nahi chhue). Isolated target `target/agent-local5`.
`CARGO_BUILD_JOBS=2` set tha. Test/clippy stagger: is run me nahi chalaye
(goal CLI+GUI smoke tha). Push: never.

## Gate 1 — CLI smoke (task wale 10 commands + bonus session)

Isolated build: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-local5
cargo build -p autocompare-cli -p autocompare-gui` — exit 0, 3m 27s.

`target/agent-local5/debug/autocompare --help` exit 0 — PASS.
Har subcommand `<cmd> --help` exit 0:

| # | Cmd | Usage line | Exit | Gate |
|---|---|---|---|---|
| 1 | (top) --help | `autocompare [OPTIONS] [COMMAND]` | 0 | PASS |
| 2 | text | `autocompare text [OPTIONS] <LEFT> <RIGHT>` | 0 | PASS |
| 3 | folder | `autocompare folder [OPTIONS] <LEFT> <RIGHT>` | 0 | PASS |
| 4 | hex | `autocompare hex [OPTIONS] <LEFT> <RIGHT>` | 0 | PASS |
| 5 | merge | `autocompare merge [OPTIONS] <BASE> <LEFT> <RIGHT>` | 0 | PASS |
| 6 | table | `autocompare table [OPTIONS] <LEFT> <RIGHT>` | 0 | PASS |
| 7 | sync | `autocompare sync [OPTIONS] <LEFT> <RIGHT>` | 0 | PASS |
| 8 | picture | `autocompare picture [OPTIONS] <LEFT> <RIGHT>` | 0 | PASS |
| 9 | folder-merge | `autocompare folder-merge [OPTIONS] <BASE> <LEFT> <RIGHT>` | 0 | PASS |
| 10 | audio | `autocompare audio [OPTIONS] <LEFT> <RIGHT>` | 0 | PASS |
| + | session (bonus) | `autocompare session [OPTIONS] <PATH>` | 0 | PASS |

Usage lines round-4 se akshar-akshar same — CLI surface me koi drift nahi.

## Gate 2 — screenshot valid (gui.png, 960x600, magic ok, byte-same)

| File | Bytes | Sha12 | Magic | Size | vs round-3/4 | Gate |
|---|---|---|---|---|---|---|
| gui.png | 9765 | bde8ccf481c6 | 89504e47 ok | 960x600 | BYTE-SAME | PASS |

`file` = `PNG image data, 960 x 600, 8-bit/color RGBA`. Python struct
check `960x600` ok. `cmp -s` round-3 `fixture-text.png`, round-4
`fixture-text.png`, round-3 `gui.png` — teenon se BYTE-SAME.
Isolated-target rebuild ke baad bhi ZERO drift — PASS.

## Gate 3 — BC outside-observe only

`/Applications/Beyond Compare.app` maujood — PASS.
`osascript` process list me `BCompare` live — PASS.
Koi window click, koi AX dump, koi string/asset copy nahi — clean-room PASS.
Session types sirf `docs/study.md` headings + apne `--help` se confirm
(9 sessions + 1 loader) — PASS, apne words me.

## Gate 4 — gaps listed (TOP-10, apne words, label sahit)

ISSUES.md me G1–G10 geometry/typography deltas, har ek me
file+line+expected+label (HIGH/MEDIUM/UNVERIFIED) — PASS.
Khud koi fix nahi kiya — PASS.

## Gate 5 — commit hygiene

Pathspec sirf `checks/early-feedback-5`, `git add -A` nahi, push nahi — PASS.
