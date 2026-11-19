# Wave U report: release rebuild + local-run early feedback (report-only)

HEAD a3ea288 (task stated bf71315; tree is one commit ahead). Zero source edits; owns checks/gui-run-u/** only.
Clean-room: reference tool observed from outside only (process running); no strings/assets copied. No color changes.

## Results
- G1 release rebuild: `cargo build --release -p autocompare-gui` exit 0, wall 50.35s, size 11074592B vs baseline 11074624 (delta -32B). Evidence: checks/gui-run-u/build-size.txt
- G2 headless PNG: `--screenshot` exit 0, 9914B vs baseline 9914 (exact), magic-ok, distinct-bytes 256. Folder variant 10300B magic-ok. Evidence: checks/gui-run-u/png-check.txt, after.png, folder.png. Headless path: crates/gui/src/main.rs:37-43, canvas 960x600 per crates/gui/src/screenshot.rs:44-46
- G3 geo-typo: tools/gui-polish/run-geo-typo.sh checks/gui-run-u/round-U-run-1 exit 0, RESULT: PASS (PNG ok, Grid x1, row-height x3, monospace x36). Evidence: checks/gui-run-u/round-U-run-1/geo-typo-report.md
- G4 tests: workspace 601 passed 0 failed (baseline 579, +22); gui 245 passed 0 failed (baseline 231). Gui count drifted 233->239->245 across runs (see drift note). Evidence: checks/gui-run-u/tests-summary.txt
- G5 smoke: both fixtures render headless (text 7-row diff + tabs/gutter/status; folder tree + sync preview). Reference process confirmed running (outside-only). Top-3 own-words deltas: (1) toolbar toggle row packs controls tighter than reference, needs breathing room; (2) right-edge overview strip is narrower than reference minimap, segment hits look small; (3) raster glyphs at fixed pitch read chunkier than reference font smoothing, row height 24px feels tall. No window launch attempted (headless-first).
- G6 clippy/fmt: clippy default exit 0, 0 errors; 6 dead_code warnings (FIND_BAR_H, FIND_NEEDLE_W, BADGE_PAD, GUTTER_W, find_bar_has_room, paint_text_find_bar) + 1 const-assertion warn in test target. `cargo fmt --check` FAILS, diffs only in crates/gui/src/app.rs, views.rs (uncommitted, other wave). Evidence: checks/gui-run-u/clippy-warnings.txt, clippy-fmt.txt
- G7 ownership: `git status` shows my adds only under checks/gui-run-u/ plus others' uncommitted crates/gui edits and checks/gui-layout-u/ (untouched). Banned-pattern grep: 0 hits in non-test code (all hits inside #[cfg(test)] mods, e.g. crates/sessions/src/session.rs:350-357).

## Drift note
Tree changed under test: crates/gui diff grew 297->311 insertions between runs; gui count moved with it. Baselines compared against latest observed; committed HEAD a3ea288 unaffected (no source edits staged).
