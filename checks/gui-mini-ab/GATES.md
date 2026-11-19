# Wave AB-MINI — GATES (Gap #5 bacha hissa: text-gutter minimap click-to-jump)

OWNS: `crates/gui/src/views.rs` + `crates/gui/src/gutter.rs` + `checks/gui-mini-ab/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only).
Colors FROZEN (koi hue add nahi — sirf existing palette/tokens, koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch nahi.

Scope: text gutter hover rect (`paint_text_with_prefs`, `Sense::hover`) par click/drag jump.
Fraction se row idx (`gutter_row_at_y` pattern), nearest-diff snap (`gutter_jump_target` — hex side ke
`hex_gutter_jump_to_row` ka text analog, same snap contract), selection + next-frame scroll sync,
selection marker (viewport indicator) synced. Hex minimap untouched (click/drag wahan pehle se wired hai).

## G1 — Baseline green (edit se pehle)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: all pass, 0 fail
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views|gutter).rs|checks/gui-mini-ab/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/gutter.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `gutter.rs` + `checks/gui-mini-ab/**`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD

## G3 — Mapping exact (pure helpers, headless tests)

- CHECK: `cargo test -p autocompare-gui gutter_ 2>&1 | tail -n 3` + `cargo test -p autocompare-gui text_gutter_ 2>&1 | tail -n 3`
- EXPECT: all pass — `gutter_y_for_row` (row→y inverse, marker geometry) + `text_gutter_click_select`
  (`gutter_jump_target` contract: diff par stay, equal par nearest-diff snap, bahar/empty par `None`)

## G4 — Paint wiring (click/drag jump + indicator sync)

- CHECK: `grep -n "click_and_drag" crates/gui/src/views.rs` (text gutter rect) + `grep -n "text_gutter_pending_scroll" crates/gui/src/views.rs`
- EXPECT: text gutter rect `Sense::click_and_drag` par hai; pressed + `interact_pointer_pos` + `text_gutter_click_select`
  se `model.selected` set hota hai; pending-scroll key agle frame list ko us row par laata hai;
  selection marker (`gutter_y_for_row` + `ROW_PAD` height) jump ke baad synced dikhta hai

## G5 — Hygiene (MS guidelines + zero warnings)

- CHECK: `cargo clippy -p autocompare-gui --all-targets 2>&1 | tail -n 3` (zero warnings)
  + `cargo fmt --check -- crates/gui/src/views.rs crates/gui/src/gutter.rs` (clean; full `cargo fmt --check` nahi — sirf OWNS files)
  + `grep -rn 'unwrap\|expect\|panic\|todo' crates/gui/src/views.rs crates/gui/src/gutter.rs | grep -v '^\s*//' | grep -v '#\[test\]' || echo BANNED-CLEAN` (non-test code me zero; test module lines allowed nahi — test fns me bhi avoid, sirf `#[cfg(test)]` block ke andar jo hai wo test hai)
- EXPECT: clippy zero, fmt clean, non-test banned zero
- NOTE: banned grep sirf non-test code par lagu; `#[cfg(test)] mod tests` ke andar asserts me `unwrap_or_default` jaisa existing pattern chalta hai, naya `unwrap` wahan bhi avoid karo

## G6 — Harness proof (headless-first round, real PNG)

- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-mini-ab/round-1`
- EXPECT: `RESULT: PASS`, PNG magic-ok, size ≥ 8192
