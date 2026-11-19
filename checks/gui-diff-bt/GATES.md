# BT-DIFF GATES — P0-2 selective word highlight (views.rs only)

OWNS: `crates/gui/src/views.rs` + `checks/gui-diff-bt/**` ONLY.
KEVL NO-TOUCH: views_table.rs, theme.rs, gutter.rs, home.rs, tree.rs,
viewmodel.rs, app.rs, screenshot.rs, checks/opencode-waves/STATUS.md,
baaki checks dirs (read-only allowed). Sibling waves ke dirty files
(screenshot.rs, viewmodel.rs) ko haath nahi lagana, stage nahi karna.
Colors FROZEN: koi naya `Color32` nahi — sirf existing CHANGED_FG/BG,
DELETED/INSERTED, TRANSPARENT reuse. Geometry first, typography second.
NO `git push` (orchestrator pushes). Commit `feat(gui): ...` English.

Scope: BS-LOCALRUN P0-2 — BC me sirf changed WORD par selective highlight
+ khaali jagah hatched filler, ours me full-row bands. Fix `views.rs`
paint_text row/accent fns: highlight changed span tak, equal plain, filler
geometry (WIDTH/geometry, color nahi).

## G1 — Baseline green (before edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: ok, 600+ passed, 0 failed (record `round-1/baseline-tests.out`)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git diff --name-only` me sirf OWNS badle hon (sibling dirty
  pehle se hain — unko stage/commit nahi karna; staged check:
  `git diff --cached --name-only` sirf OWNS)
  + `git diff --cached --name-only | grep -E "views_table|theme.rs|gutter.rs|home.rs|tree.rs|viewmodel.rs|app.rs|screenshot.rs|STATUS" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+" | grep -E "Color32::from_rgb|from_rgba_premultiplied|Visuals::" || echo NO-COLOR-ADD`
- EXPECT: staged sirf `views.rs` + `checks/gui-diff-bt/**`; KEVL-CLEAN; NO-COLOR-ADD

## G3 — Selective highlight unit-tested (geometry + typo)

- CHECK: `cargo test -p autocompare-gui selective` + `cargo test -p autocompare-gui filler` + `cargo test -p autocompare-gui span`
- EXPECT: green, kam se kam ye tests:
  + `selective_row_is_plain_changed_and_equal` (Changed/Equal plain, Deleted/Inserted band)
  + `selective_side_palette_filler_empty_side_plain` (Deleted right / Inserted left filler plain)
  + `selective_span_format_accent_uses_changed_band` (accent CHANGED_FG/BG + 1px underline, plain passthrough, same mono size)
  + `selective_find_format_current_light_block` (current light block no-underline, other plain+line)
  + `filler_geometry_widths_and_hatch_step` (pad width saturating, hatch step TIGHT_GAP reuse)
  + purane span tests same values par pass (1 updated: inverted-block ab selective band)

## G4 — MS Rust + fmt clean (OWNS only)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -15 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: clippy 0 (sibling KEVL errors hon to views.rs 0 + note), fmt 0,
  ZERO-BANNED non-test, forbid kept, naye helpers pure (iterators + get, koi indexing nahi)

## G5 — Harness PASS + PNG gates

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-diff-bt/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-diff-bt/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: `RESULT: PASS`, magic `89504e470d0a1a0a`, size >= 8192, distinct > 64

## G6 — REPORT + artifacts, commit locally, NO push

- CHECK: `ls checks/gui-diff-bt/REPORT.md checks/gui-diff-bt/round-1/after.png checks/gui-diff-bt/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-diff-bt && git commit -m "feat(gui): ..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit, sirf OWNS staged, NO push, STATUS untouched
