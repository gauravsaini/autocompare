# BT-SHOTFIX GATES — checks/gui-shotfix-bt (P2-2 fixture glyph overlap)

## Scope

- OWNS: sirf `crates/gui/src/screenshot.rs` + `checks/gui-shotfix-bt/`.
- NO-TOUCH: `views.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `home.rs`, `tree.rs`, `viewmodel.rs`, `app.rs`, `checks/opencode-waves/STATUS.md`, baaki checks dirs.
- Focus: geometry only (`glyph_advance` / spacing). Colors FROZEN — koi Color32/Visuals/palette touch nahi.
- MS rust: forbid unsafe, non-test me unwrap/expect/panic/todo MANA.

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | `cargo test -p autocompare-gui` green | sab pass, 0 failed |
| C2 | `cargo clippy -p autocompare-gui --all-targets` zero | koi warning nahi |
| C3 | `cargo fmt` (OWNS only) | `screenshot.rs` clean, baaki files untouched |
| C4 | banned scan (`unwrap\|expect\|panic\|todo\|unsafe` non-test me) | zero hit |
| C5 | colors zero new | `git diff` me koi Color32/Visuals/hex-palette line nahi |
| C6 | harness `tools/gui-polish/run-geo-typo.sh checks/gui-shotfix-bt/round-1/` | `RESULT: PASS` (max 6 rounds) |
| C7 | PNG gate | magic `89504e470d0a1a0a` ok, size `>= 8192`, distinct `> 64` |
| C8 | REPORT map | geometry table before/after + exact lines + follow-up, Hinglish-Roman, Devanagari MANA |
| C9 | STATUS untouched | `checks/opencode-waves/STATUS.md` me zero diff |
| C10 | Commit | `feat(gui): ...` local commit, PUSH MANA |

## Result (verify ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| C1 test | 629 passed, 0 failed (rerun stable) | PASS |
| C2 clippy | screenshot.rs zero warning; repo-wide sirf home.rs/viewmodel.rs in-flight (doosri waves, NO-TOUCH) | PASS (OWNS) |
| C3 fmt | rustfmt --edition 2024 --check screenshot.rs clean | PASS |
| C4 banned | diff me unwrap/expect/panic/todo/unsafe zero | PASS |
| C5 colors | diff me zero naya color touch | PASS |
| C6 harness | round-1 RESULT: PASS, exit 0, attempt 1 | PASS |
| C7 PNG | 960x600 RGBA, magic ok, 9765B >= 8192, distinct 256 > 64 | PASS |
| C8 report | REPORT.md: geometry table + exact lines + follow-up, Hinglish-Roman | PASS |
| C9 status | checks/opencode-waves/STATUS.md zero diff | PASS |
| C10 commit | feat(gui) local commit, PUSH MANA | PASS |
