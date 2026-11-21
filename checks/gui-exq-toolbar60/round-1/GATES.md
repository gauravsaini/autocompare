# GATES exq-toolbar60 round-1 (written BEFORE implementing per $unlazy)

Scope: `crates/gui/src/views.rs` ONLY + `checks/gui-exq-toolbar60/**`.
NEVER touch: `crates/gui/src/theme.rs`, `crates/sessions/**`,
`checks/gui-sessionpaint-by/**`, any other `checks/` dir, any other
`crates/gui` file. No new sizes/colors/tokens. No push.

Honest discrepancy (orchestrator brief vs code truth): `app.rs`
`ExqIcons50Mark` stages only Ring/Bars/Rails/Wedge (kinds 0..3) and
`exq_icons50_mark_kind` maps `index % 4` — there are NO staged
`paint_exq_icons50_mark` variants 4&5, and `app.rs` is outside this
leaf's OWNS. So row-2 Prev/Next get true vector chevrons (`<`/`>` line
marks) painted in `views.rs` reusing staged tokens only
(`exq_icons50_mark_xy` origin, `exq_icons50_mark_box` 12px box,
`exq_icons50_stroke_w` 2px stroke, `dim_for` ink) with the same
paint-after-response pattern as row-1. `toolbar_glyph_rects(4/5)` cell
data + `paint_toolbar_glyph` stay untouched (find-bar path intact).

## G1 baseline harness PASS on fresh round-1 dir
- CHECK: `tools/exquisite/run.sh checks/gui-exq-toolbar60/round-1` exits 0 and writes `after.png` + `exquisite-report.md` containing `RESULT: PASS`
- EXPECT: `HARNESS_BASELINE_PASS`

## G2 views.rs-only nav chevron wiring, hit-rects + clicks byte-identical
- CHECK: `git diff --stat` shows only `crates/gui/src/views.rs` (+ untracked `checks/gui-exq-toolbar60/**`); `git diff crates/gui/src/views.rs | grep -c paint_action_nav_row` >= 1; `grep -c "action_nav_prev_label()\|action_nav_next_label()" crates/gui/src/views.rs` unchanged vs base (labels untouched); `session_toolbar_button_size` body unchanged
- EXPECT: `OWNS_AND_LABELS_INTACT`

## G3 unit test proves Prev/Next vector marks, rects identical
- CHECK: `cargo test -p autocompare-gui toolbar60` exits 0 with `test result: ok`
- EXPECT: `NAV_CHEVRON_TEST_GREEN`

## G4 full suite green, fmt clean, banned scan zero
- CHECK: `cargo test -p autocompare-gui` exits 0; `cargo fmt --check -p autocompare-gui` exits 0; `grep -rnE "unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe" crates/gui/src/views.rs | grep -v "^.*//" | grep -viE "test|#\[|debug_assert" ` empty on non-test delta (`git diff -U0 | grep "^+" | grep -v "^+++" | grep -E "unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe[^_]"` empty)
- EXPECT: `SUITE_FMT_BANNED_CLEAN`

## G5 harness re-run PASS + kittest render documented
- CHECK: re-run `tools/exquisite/run.sh checks/gui-exq-toolbar60/round-1` exits 0 with `RESULT: PASS`; `cargo test -p autocompare-gui --test real_egui_shot` result recorded in REPORT.md with toolbar pixel box + dims (960x600 canvas)
- EXPECT: `HARNESS_RERUN_PASS_KITTEST_DOCD`

## G6 commit scope clean, conventional, no push
- CHECK: `git status --short` after commit shows no changes to `checks/gui-sessionpaint-by/**`, `crates/sessions/**`, `crates/gui/src/theme.rs`; `git log -1 --format=%s` matches `feat(gui): exq-toolbar60 .*`; `git status` shows working tree clean of strays
- EXPECT: `COMMIT_SCOPE_CLEAN`
