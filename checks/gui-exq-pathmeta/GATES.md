# GATES — EXQ-PATHMETA (path metadata strip + segmented status)

OWNS: crates/gui/src/screenshot.rs + crates/gui/src/viewmodel.rs + checks/gui-exq-pathmeta/**
KEVL NO-TOUCH: views.rs, app.rs, theme.rs, gutter.rs, home.rs, tree.rs, checks/opencode-waves/STATUS.md, baaki checks dirs.

Scope: BC per-pane path bar me dropdown + folder icons + second metadata line
(timestamp/bytes/format/rules) + bottom 4-segment status (cursor/format/result/edit) hai;
ours me single path field + Browse + single summary line tha (local6 TOP-10 #4,7).
Is wave me sirf headless half: viewmodel me pathmeta strip + segmented status model
(format/join/history/roles, honest-empty, clip via existing clip_to_width),
screenshot fixture me per-pane strip paint ([ ] icon + clipped path + v caret + Browse,
neeche second meta line) + bottom 4-zone segmented status (dividers SEG_DIV*).
Live paint (views.rs/app.rs) agle wave ka OWNS hai — haath nahi lagaya. Colors FROZEN.
MS rust: forbid unsafe, non-test me unwrap/expect/panic/todo MANA.

## Gate list

- [x] G1: viewmodel pathmeta helpers pure + headless-tested hain.
  CHECK: `cargo test -p autocompare-gui pathmeta -- --nocapture`
  EXPECT: sab pathmeta tests PASS, 0 failed (meta join, history honest, segmented line, roles, edge empty-vs-long).

- [x] G2: Full gui suite green hai.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: 0 failed.

- [x] G3: Clippy zero warnings hai.
  CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  EXPECT: exit 0.

- [x] G4: Format OWNS files par clean hai.
  CHECK: `cargo fmt -- crates/gui/src/viewmodel.rs crates/gui/src/screenshot.rs` ke baad `git diff` sirf OWNS hunks.
  EXPECT: rustfmt clean; sibling dirt (gutter/copy/tree/views) ko haath nahi.

- [x] G5: Banned patterns non-test code me nahi hain.
  CHECK: `rg -n "unwrap\(|expect\(|panic!|todo!|unsafe" crates/gui/src/viewmodel.rs crates/gui/src/screenshot.rs | grep -v forbid`
  EXPECT: koi match nahi.

- [x] G6: Exquisite harness PASS deta hai.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-pathmeta/round-1`
  EXPECT: `round-1/exquisite-report.md` me `RESULT: PASS` (PNG magic ok, >=8192B, distinct>64).

- [x] G7: Colors zero new hain.
  CHECK: `git diff -- crates/gui/src/viewmodel.rs crates/gui/src/screenshot.rs | grep -c "Color32\|Visuals"`
  EXPECT: 0 (frozen palette, sirf existing dim/text/panel reuse).

- [x] G8: Edge fixture + headless proof hai.
  CHECK: `checks/gui-exq-pathmeta/round-1/comparison/edge-pathmeta.md` + `pathmeta_edge_empty_vs_long_clips_honestly` test
  EXPECT: empty left honest-empty ("", history), long right ellipsis-clip (budget fit), cursor-only segmented honest.

## DoD

- Fixture `after.png` me per-pane strip dikhta hai ([ ] icon + path + v + Browse, neeche meta line) + bottom 4-zone status (dividers).
- History dropdown label honest hai (empty par "history", koi fake path nahi).
- G1–G8 sab PASS. REPORT.md Hinglish-Roman, commit `feat(gui): ...` sirf OWNS paths `git add`, PUSH MANA.
