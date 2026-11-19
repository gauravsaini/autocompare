# Wave U LAYOUT-EXQUISITE — REPORT (owns app.rs + checks/gui-layout-u/**)

Wiring-only wave: every text/table/folder/picture paint path consumes the
Wave-R split fraction at the app layer, with min-width floors, h-scroll
overflow, docked find-bar geometry, narrow-window behavior, and scroll
clamps. views.rs / theme.rs untouched (read-only; U-GEO owns paint rects
there). No color/token changes.

## GATES proof

- G1 split consumed (4 new branch reads + 7 pre-existing path sites):
  `app.rs:2674` text_fraction, `:2834` folder_fraction, `:3120`
  table_fraction, `:3151` picture_fraction; `paint_paths` x7 +
  `paint_picture_compare` x1 pass `&mut self.split_fraction` (Wave R).
  No `avail / 2` static-half math in app.rs non-test code.
- G2 min widths + h-scroll: `layout_row_min_width` = split floors +
  gap; `show_split_content` (`app.rs:1640`) wraps Text / Table /
  Folder-list / Picture + both triple rows in `ScrollArea::horizontal`
  with a `row_min` spacer, so narrow windows scroll instead of
  crushing/overlapping. Tests: `layout_pane_widths_follow_split_with_floor`
  (800 -> 396/396, 0 -> 180/180 floors), `layout_row_min_width_needs_hscroll_when_narrow`
  (800 -> row 800 no scroll; 100 -> row 368 scrolls).
- G3 find-bar/badge rects: `find_bar_geometry` pins bar_w == avail,
  badge_x == bar_w, prev/next hits >= 24.0 (test green). Wiring: text
  bar docked full-width via `ui.set_min_width(bar_geo.bar_w)` (`:2643`);
  paint rects inside the bar are U-GEO views-owned. App-owned find rows
  (folder/table/hex) use `find_nav_button` 24px-min hits (7 sites).
- G4 narrow <900px: `layout_is_narrow` (899 -> true, 900 -> false,
  test green); floors hold + h-scroll takes over, never overlap.
- G5 scroll clamps: `clamp_scroll_offset` ([0, content-viewport],
  fits -> 0.0, NaN -> 0.0, test green) + `clamp_scroll_after_layout`
  (reuses sync_*_pager totals+clamp) called on `set_split_fraction`,
  `drag_split_by`, and all 4 content branches. Test:
  `scroll_clamp_after_layout_keeps_pager_tops_valid`.
- G6 baselines: `cargo test -p autocompare-gui` 245 passed 0 failed
  (239 parallel-wave baseline + 6 new layout tests); harness
  `run-geo-typo.sh round-U-layout-1` RESULT: PASS (PNG 9914B magic-ok
  distinct-256, identical to baseline: default-width paint unchanged).
  app.rs fmt-clean; clippy reports 0 hits in app.rs.
- G7 hygiene: `git status` shows only `crates/gui/src/app.rs` +
  `checks/gui-layout-u/**` as mine (`views.rs` M is parallel U-GEO
  work, not touched here). Banned-pattern grep over app.rs non-test
  code: unwrap 0, expect 0, panic! 0, todo! 0 (one prose "never a
  panic" doc word only).

## Known parallel-wave interaction (evidence, not mine to fix)

- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` currently
  fails on U-GEO intermediate `views.rs` items (`GUTTER_W`,
  `find_bar_has_room` unused yet) + their `views.rs:2881` test assert.
  Proof of my scope: clippy output contains 0 references to `app.rs`;
  `cargo fmt --check` flags only `views.rs:2881`, my file clean.
- Test count 245 = 239 (incl. parallel U-GEO/U-TYPO additions) + 6 new
  layout tests here. All green.

## Files

- `crates/gui/src/app.rs`: layout section (consts + 6 pure fns + pager
  clamp method + 2 paint helpers), 4 branch wirings, triple-row wraps,
  6 new tests. No color/token edits, no new deps.
- `checks/gui-layout-u/GATES.md`, `REPORT.md` (this file),
  `round-U-layout-1/` (after.png 9914B, geo-typo-report.md PASS,
  tests.out 245 green, screenshot.out).
- STATUS.md untouched (orchestrator-only). No push (local commit only).
