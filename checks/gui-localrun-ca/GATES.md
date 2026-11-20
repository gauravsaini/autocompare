# CA-LOCALRUN3 GATES — round-3 (checks-only, crates/** ZERO edits)

## Scope

- OWNS: sirf `checks/gui-localrun-ca/`. NO-TOUCH: `crates/**`, `scripts/`, `tools/`, `skills/`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
- Focus: CA batch (P0-1 session chain 4 commits + P0-2 selwire + P1-1 pathmeta + P2 darkroot-docs) ke baad rebuild + recapture, bu2 round-2 (`checks/gui-localrun-bu2/round-2/`) se gap DELTA.
- Geometry + typography only. Colors out of scope (note only, spec nahi).

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | `cargo build -p autocompare-gui` green (CA HEAD par) | binary `target/debug/autocompare-gui` fresh, CA 7 commits included |
| C2 | Harness `tools/gui-polish/run-geo-typo.sh checks/gui-localrun-ca/round-3/` | exit 0, `RESULT: PASS` |
| C3 | Headless fixtures: `fixture-home/text/folder/table.png` (filename-keyed `--screenshot`) | 4 files, har ek 960x600 PNG |
| C4 | PNG magic | har file ke pehle 8 bytes `89504e470d0a1a0a`, `file` bolega `PNG image data` |
| C5 | PNG size | har file `>= 8192` bytes |
| C6 | PNG distinct bytes | `len(set(bytes)) > 64` (non-blank proof) |
| C7 | Live window run | direct binary launch + `screencapture` fullscreen + region crop, PID + frame log me; tab switch nahi (observe-only, bu2 jaisa) |
| C8 | REPORT map + delta table | shot map (file, size, magic, kya hai) + delta table (bu2 vs ca, ID/rank/observed/verdict/spec) |
| C9 | Language | REPORT Hinglish-Roman, Devanagari MANA |
| C10 | Commit | `docs(gui): ...` local commit, sirf OWNS pathspec stage, PUSH MANA; STATUS.md untouched |
| C11 | ZERO crates edits | `git status` me `crates/**` me zero content diff |

## Result

| Gate | Actual | Status |
|---|---|---|
| C1 build | rebuild green 12:42 (0.84s), CA 7 commits HEAD me | PASS |
| C2 harness | exit 0, `geo-typo-report.md` me `RESULT: PASS` (PNG 9765B, magic ok, distinct 256; Grid x14, mono x50) | PASS |
| C3 fixtures 4x960x600 | home/text/folder/table, sab 960x600 PNG | PASS |
| C4 magic-ok | 6/6 `PNG image data, ... RGBA, non-interlaced` | PASS |
| C5 size>=8192 | min 9765 (after/fixture-text), max 767963 (desktop) | PASS |
| C6 distinct>64 | sab 256 | PASS |
| C7 live run | PID 54672, frame 320,91 800x632 (bu2 jaisa), fullscreen + region 2 PNG; tab-switch nahi | PASS |
| C8 report delta | REPORT section 3 shot map, section 5 me P0x2/P1x4/P2x3 har row bu2 vs ca + verdict + exact file:line spec | PASS |
| C9 Hinglish-Roman | Devanagari use nahi hui | PASS |
| C10 commit no-push | docs(gui) local commit, sirf `checks/gui-localrun-ca` staged, push nahi; STATUS untouched | PASS |
| C11 crates zero | `git status --porcelain` me `crates/**` zero entries | PASS |
