# Wave AN-THREE — GATES (Gap #7: 3-pane layout, left/base/right aligned)

OWNS: `crates/gui/src/views.rs` + `checks/gui-three-an/**` ONLY.
KEVL `app.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`views_hex.rs`/`home.rs`/`gutter.rs`/`theme.rs`/`screenshot.rs`
mat chhoona (read bhi avoid karo, grep-proof only). Merge-plan model reuse read-only
(`viewmodel::merge_conflict_hunks` + `merge_pick_summary`, `TakeChoice` reuse — model fields/struct change nahi).
Colors FROZEN (koi hue/token change nahi). Geometry first, typography second
(sirf existing tokens: `SECTION_GAP`/`TIGHT_GAP`/`MIN_THIRD` + small/weak/strong/monospace).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
`git stash/push` kabhi nahi.

Scope: Gap #7 ka layout pass. Model half (AA-MERGE) + paint half (AF-MERGEPAINT)
+ pick half (AI-MERGEPICK) landed hain. Kami: `paint_merge_three_pane` teen
verticals bina fixed widths ke paint karta hai — har pane content-sized hai,
isliye cards ke beech left/base/right columns misaligned hain. Is wave ka delta
`sirf views.rs` me hai: pure `three_pane_widths(avail)` helper (2-pane
`split_picture_widths` precedent: `2 * SECTION_GAP` subtract, `MIN_THIRD` floor,
leftover aakhri pane me taaki row bhare) + paint har pane par fixed width set
karta hai (`set_width`), isliye teenon columns har card me aligned rehte hain.
Splitter ratio 1:1:1 default (equal thirds, path-row `third_field_width`
precedent). Typo pehle se locked hai (small strong headers, mono lines, small
weak decisions) — koi typo change nahi, sirf reuse. Signatures unchanged
(KEVL `app.rs` call-site compat). Koi stub nahi, koi App wiring nahi (agli wave).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (404 passed, 0 failed, recorded 2026-11-20, `main@6a0f9e8`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-three-an/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `checks/gui-three-an/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (sirf existing frozen palette reuse;
  koi naya hue/token nahi)

## G3 — 3-pane layout unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui three_pane`
- EXPECT: green, kam se kam ye naye tests:
  + `three_pane_widths_split_equal_thirds` (wide avail par teen barabar, sum row bharta hai)
  + `three_pane_widths_floor_narrow_panes` (narrow avail par har pane `MIN_THIRD`, scroll crush nahi)
  + `three_pane_widths_cover_full_row` (teen widths + `2 * SECTION_GAP` == avail, koi gap/overflow nahi)
  + purane merge tests same values par pass (AF/AI tests untouched)
- WHAT: `views.rs` me pure helper (`three_pane_widths`, `MIN_THIRD` floor reuse)
  + `paint_merge_three_pane` (har pane `set_width`, separators beech me, headers/typo same);
  `paint_merge`/`paint_merge_three_pane` signature unchanged; global Take untouched

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "\[[a-zA-Z_][a-zA-Z0-9_]*\s*\]" | grep -v "Vec\|String\|Option\|Result" || echo NO-INDEXING`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only;
  saturating arithmetic; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-three-an/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-three-an/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-three-an/REPORT.md checks/gui-three-an/round-1/after.png checks/gui-three-an/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-three-an/GATES.md checks/gui-three-an/REPORT.md checks/gui-three-an/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
