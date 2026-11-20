# BX-SELFIX GATES — P0-2 selective highlight fixture visibility (views.rs + checks/gui-selfix-bx ONLY)

OWNS: `crates/gui/src/views.rs` + `checks/gui-selfix-bx/**` ONLY.
KEVL NO-TOUCH: app.rs, home.rs, viewmodel.rs, views_table.rs, theme.rs,
gutter.rs, tree.rs, screenshot.rs, checks/opencode-waves/STATUS.md (MANA),
baaki checks dirs (read-only allowed). Sibling dirty nahi hai (status clean
tha) — staged me sirf OWNS.
Colors FROZEN: koi naya `Color32` nahi — existing WHITE/BLACK reuse already,
koi rang change nahi. NO `git push` (orchestrator pushes). Commit `feat(gui):`
English. MS RUST: forbid(unsafe) kept, non-test me unwrap/expect/panic/todo
MANA.

Scope: BU-LOCALRUN2 P0-2 — selective highlight live hai (`f4fb03f`) par
default text fixture (`after.png` = `render_text_fixture_png`) abhi purane
full-band style me paint hota hai. Fixture bytes `screenshot.rs` (KEVL frozen)
+ `viewmodel` se bante hain — `views.rs` edit se fixture pixels NAHI badalte.
Is wave ka kaam: shift-vs-trigger hypothesis ko pixel se faisla karo, gates +
pixel evidence lock karo, screenshot wave ko exact follow-up spec do.

## G1 — Baseline (before edit)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-selfix-bx/round-1`
  + `cp round-1/after.png round-1/before.png`
  + `cargo test -p autocompare-gui 2>&1 | tail -3`
- EXPECT: after.png 9765B magic ok; full suite 641 passed + 1 KEVL fail
  (`app::tests::folder_double_click_stages_file_and_routes_to_text`,
  app.rs:7413 — pre-existing, clean HEAD par, KEVL MANA isliye no-fix);
  selective/filler/span filters green. Ye `round-1/tests.out` me record hai.

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git diff --name-only` me sirf OWNS badle hon
  + `git diff --cached --name-only | grep -E "views_table|theme.rs|gutter.rs|home.rs|tree.rs|viewmodel.rs|app.rs|screenshot.rs|STATUS" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+" | grep -E "Color32::from_rgb|from_rgba_premultiplied|Visuals::" || echo NO-COLOR-ADD`
  + banned: `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -15 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: staged sirf `checks/gui-selfix-bx/**` (+ views.rs agar change ho —
  is wave me views.rs me koi badlav zaroori nahi nikla, add no-op);
  KEVL-CLEAN; NO-COLOR-ADD; ZERO-BANNED; UNSAFE-FORBID-OK.

## G3 — Selective unit tests green (fixture geometry locks)

- CHECK: `cargo test -p autocompare-gui selective` + `... filler` + `... span`
  + full suite status record (`tail -3`)
- EXPECT: selective/filler/span filters 100% green (selective_row_is_plain,
  side_palette, span_format, find_format, filler geometry — sab live model
  lock karte hain); full suite me sirf wahi 1 pre-existing KEVL fail (G1 jaisa).

## G4 — Clippy zero + fmt clean (views.rs)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
- EXPECT: clippy 0, fmt 0 (views.rs untouched — vacuously clean; verify karke).

## G5 — Harness + PNG gates + pixel-scan (highlight rect visible)

- CHECK: harness rerun `zsh tools/gui-polish/run-geo-typo.sh checks/gui-selfix-bx/round-1`
  (exit 4 aata hai — G1 wale KEVL fail se, screenshot step se pehle after.png
  likha ja chuka hota hai)
  + `python3` pixel-scan `round-1/pixel-scan.out` me:
  (a) Changed row (y 172..196) me CHANGED_BG band + CHANGED_FG accent block
      right pane x≈597..641 (fixture scene ka word span `(18,27)`);
  (b) expected ax = 508 + text_width(prefix) = 597 — measured se match
      (shift hypothesis refuted, alignment ok);
  (c) `cmp before.png after.png` → pixel-diff EXPECT 0 (screenshot.rs frozen,
      views.rs se fixture invariant — ye proof hai, failure nahi).
  + supplementary: `autocompare-gui --screenshot round-1/after-accent.png`
    (accent scene: dono Changed rows, dono panes me word spans) + uska scan.
- EXPECT: magic `89504e470d0a1a0a`, size >= 8192, distinct > 64;
  accent block x 597..641 me VISIBLE; pixel-diff 0 documented.

## G6 — REPORT + commit, NO push

- CHECK: `ls checks/gui-selfix-bx/REPORT.md checks/gui-selfix-bx/GATES.md
  checks/gui-selfix-bx/round-1/{before,after,after-accent}.png`
  + `git add crates/gui/src/views.rs checks/gui-selfix-bx && git commit -m "feat(gui): ..."`
  + `git log --oneline -1 && git status --short`
- EXPECT: conventional English commit, sirf OWNS staged, NO push,
  STATUS untouched.
