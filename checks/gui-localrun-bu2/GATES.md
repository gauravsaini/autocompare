# BU-LOCALRUN2 GATES — round-2 (checks-only, crates/** ZERO edits)

## Scope

- OWNS: sirf `checks/gui-localrun-bu2/`. NO-TOUCH: `crates/**`, `scripts/`, `tools/`, `skills/`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
- Focus: BT batch (glyph fix + selective highlight + 3-zone status model + CTA contract) ke baad rebuild + recapture, round-1 (`checks/gui-localrun-bs/round-1/`) se gap DELTA.
- Geometry + typography only. Colors out of scope (note only, spec nahi).

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | `cargo build -p autocompare-gui` green (BT HEAD par) | binary `target/debug/autocompare-gui` fresh, BT 4 commits included |
| C2 | Headless fixtures: `fixture-home/text/folder/table.png` (filename-keyed `--screenshot`) | 4 files, har ek 960x600 PNG |
| C3 | PNG magic | har file ke pehle 8 bytes `89504e470d0a1a0a`, `file` bolega `PNG image data` |
| C4 | PNG size | har file `>= 8192` bytes |
| C5 | PNG distinct bytes | `len(set(bytes)) > 64` (non-blank proof) |
| C6 | Live window run | direct binary launch + `screencapture` fullscreen + region crop, PID + frame log me; tab switch nahi (observe-only, round-1 jaisa) |
| C7 | REPORT map + delta table | shot map (file, size, magic, kya hai) + delta table (round-1 vs round-2, ID/rank/observed/spec) |
| C8 | Language | REPORT Hinglish-Roman, Devanagari MANA |
| C9 | Commit | `docs(checks): ...` local commit, sirf OWNS pathspec stage, PUSH MANA; STATUS.md untouched |
| C10 | ZERO crates edits | `git status` me `crates/**` me zero content diff |

## Result (capture ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| C1 build | force rebuild 10:50:56 (touch + build, 4.69s compile), BT 4 commits HEAD me | PASS |
| C2 fixtures 4x960x600 | home/text/folder/table, sab 960x600 PNG | PASS |
| C3 magic-ok | 6/6 `PNG image data, ... RGBA, non-interlaced` | PASS |
| C4 size>=8192 | min 9765 (fixture-text), max 762897 (desktop) | PASS |
| C5 distinct>64 | sab 256 | PASS |
| C6 live run | PID 40485, frame 320,91 800x632 (round-1 jaisa), fullscreen + region 2 PNG; tab-switch nahi | PASS |
| C7 report delta | REPORT section 3 shot map, section 5 me P0x2/P1x4/P2x2 har row round-1 vs round-2 + spec | PASS |
| C8 Hinglish-Roman | Devanagari use nahi hui | PASS |
| C9 commit no-push | docs(checks) local commit, sirf `checks/gui-localrun-bu2` staged, push nahi; STATUS untouched | PASS |
| C10 crates zero | `git status --porcelain` me `crates/**` zero entries (mtime touch only, content diff zero) | PASS |
