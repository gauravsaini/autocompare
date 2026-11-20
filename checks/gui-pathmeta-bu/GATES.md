# GATES — BU-PATHMETA (P1-1 path meta row)

OWNS: crates/gui/src/views.rs + crates/gui/src/app.rs + checks/gui-pathmeta-bu/**
KEVL NO-TOUCH: views_table.rs, theme.rs, gutter.rs, home.rs, tree.rs, viewmodel.rs, screenshot.rs, checks/opencode-waves/STATUS.md, baaki checks dirs.

Scope: BC path row me meta hai (timestamp, byte size, rules label, flags, history dropdown), ours me sirf fields + Browse. views.rs me compact meta row fn (geometry: alignment/spacing, TypoRole caption reuse), app.rs me paths state (history dropdown data, selection nahi badalni). Colors FROZEN. MS rust: forbid unsafe, non-test me unwrap/expect/panic/todo MANA.

## Gate list

- [x] G1: Meta helpers + paint fn OWNS me hai, selection mutate nahi hota.
  CHECK: `cargo test -p autocompare-gui path_meta -- --nocapture`
  EXPECT: sab path_meta/path_history tests PASS, 0 failed.
  EVIDENCE: 3 passed (2 views + 1 app path_meta filter); `path_` filter par 14 passed, 0 failed.

- [x] G2: Full gui suite green hai.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: 0 failed.
  EVIDENCE: 634 passed, 0 failed.

- [x] G3: Clippy zero warnings hai (OWNS scope; sibling dirt ko haath nahi).
  CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  EXPECT: exit 0. Sibling wave ka dirt mile to evidence me note, apni files se 0 warning.
  EVIDENCE: exit 0, zero warnings. Beech me meri test-indexing warning aayi thi, `.first().is_some_and(...)` se fix; `home.rs` ka sibling dirt OWNS bahar, untouched.

- [x] G4: Format sirf apne OWNS files par check hota hai.
  CHECK: `cargo fmt -- --check crates/gui/src/views.rs crates/gui/src/app.rs`
  EXPECT: exit 0. Fix ho to sirf `cargo fmt -- <apne OWNS files>`.
  EVIDENCE: PASS; `git diff` hunks sirf mere (views me pure additions, app me 8 hunks sab mere).

- [x] G5: Banned patterns non-test code me nahi hain.
  CHECK: `rg -n "unwrap\(|expect\(|panic!|todo!|unsafe" crates/gui/src/views.rs crates/gui/src/app.rs`
  EXPECT: sirf `#![forbid(unsafe_code)]` header lines match hon, bahar koi match nahi.
  EVIDENCE: sirf 2 header lines match; unwrap/expect/panic/todo usage zero.

- [x] G6: Geo-typo harness headless PASS deta hai.
  CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-pathmeta-bu/round-1`
  EXPECT: `round-1/geo-typo-report.md` me `RESULT: PASS` (max 6 rounds).
  EVIDENCE: round 1/6 me RESULT: PASS (PNG 9765 bytes, magic ok, distinct 256).

- [x] G7: Colors zero new hain.
  CHECK: `git diff -- crates/gui/src/views.rs crates/gui/src/app.rs | grep -c "Color32\|Visuals"`
  EXPECT: 0 (frozen palette, koi naya Color32/Visuals nahi).
  EVIDENCE: 0.

- [x] G8: PNG gate + STATUS untouched.
  CHECK: `file checks/gui-pathmeta-bu/round-1/after.png` + `git diff --name-only | grep STATUS` khali
  EXPECT: PNG magic ok + size >= 8192 + distinct > 64; STATUS me zero diff.
  EVIDENCE: PNG 960x600 RGBA, magic `89504e470d0a1a0a`, 9765 bytes, distinct 256; STATUS grep khali. `home.rs` sibling dirt stage nahi kiya.

## DoD

- Path row ke neeche compact meta row dikhta hai (left meta left, history dropdown beech, right meta right-dock; dropdown sirf labels, selection untouched).
- History data App state me capped/dedupe rehta hai, compare par record hota hai.
- G1–G8 sab PASS. REPORT.md Hinglish-Roman me, commit `feat(gui): ...` sirf OWNS paths `git add`, PUSH MANA.
