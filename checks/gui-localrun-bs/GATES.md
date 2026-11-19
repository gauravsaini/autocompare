# BS-LOCALRUN GATES — round-1 (checks-only, crates/** ZERO edits)

## Scope

- OWNS: sirf `checks/gui-localrun-bs/`. NO-TOUCH: `crates/**`, `scripts/`, `tools/`, `skills/`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
- Focus: geometry + typography only. Colors out of scope (report me note only, spec nahi).
- Harness `tools/gui-polish/run-geo-typo.sh` read-only use (fix nahi, max 0 rounds edit).

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | `cargo build -p autocompare-gui` green | build ok, binary `target/debug/autocompare-gui` present |
| C2 | Headless fixtures: `fixture-home/text/folder/table.png` (filename-keyed `--screenshot`) | 4 files, har ek 960x600 PNG |
| C3 | PNG magic | har file ke pehle 8 bytes `89504e470d0a1a0a`, `file` bolega `PNG image data` |
| C4 | PNG size | har file `>= 8192` bytes |
| C5 | PNG distinct bytes | `len(set(bytes)) > 64` (non-blank proof) |
| C6 | Live window run (agar possible) | `cargo run` launch + `screencapture` PNG, PID + window frame log me; headless me fail ho to REPORT me reason + fixture-only fallback note |
| C7 | BC visual compare | BC app (`/Applications/Beyond Compare.app`) observe-only, koi click/edit nahi; geometry+typo notes REPORT me |
| C8 | REPORT map | shot map table (file, size, magic, kya hai) + gap table (ID/rank/observed/exact follow-up spec with file/fn) |
| C9 | Language | REPORT Hinglish-Roman, Devanagari MANA |
| C10 | Commit | `docs(checks): ...` local commit, PUSH MANA; STATUS.md untouched |

## Result (capture ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| C1 build | binary present, Finished dev profile | PASS |
| C2 fixtures 4x960x600 | home/text/folder/table, sab 960x600 PNG | PASS |
| C3 magic-ok | 7/7 `PNG image data, ... RGBA, non-interlaced` | PASS |
| C4 size>=8192 | min 9695 (fixture-text), max 1075844 (desktop) | PASS |
| C5 distinct>64 | sab 255-256 | PASS |
| C6 live run | PID 21062, frame 320,91 800x632, 3 live PNGs; tab-switch nahi (observe-only) | PASS |
| C7 BC compare | BCompare PID 23316 observe-only, Text Compare notes section 4 me | PASS |
| C8 report map+gap table | section 3 shot map, section 5 me P0x2/P1x4/P2x2 har row file/fn spec | PASS |
| C9 Hinglish-Roman | Devanagari nahi use hui | PASS |
| C10 commit no-push | docs(checks) local commit, push nahi; STATUS untouched | PASS |
