# Wave U LAYOUT-EXQUISITE — GATES (owns crates/gui/src/app.rs + checks/gui-layout-u/**)

Scope: app.rs wiring only. views.rs / theme.rs READ-ONLY (no edits).
Colors frozen: NO hue/saturation/token changes. MS rust rules: forbid unsafe,
no unwrap/expect/panic/todo in non-test code. Commit locally, NO push.

- [x] G1: Split fraction consumed in every text/table/folder/picture paint path
  CHECK: grep -n "self.split_fraction()" crates/gui/src/app.rs (non-test)
  EXPECT: reads in Text + Table + Folder + Picture branches (>=4 sites);
  no `avail / 2` static-half math in app.rs non-test code

- [x] G2: Min pane widths with clamp + overflow to h-scroll, never crush
  CHECK: cargo test -p autocompare-gui layout_row_min_width (new test)
  EXPECT: green; row min = split floors + gap, narrow avail keeps floors
  CHECK: grep -n "ScrollArea::horizontal" crates/gui/src/app.rs
  EXPECT: wraps Text + Table + Folder-list + Picture content paints

- [x] G3: Find-bar / badge layout rects (docked top full-width, badge right end, prev/next >=24px)
  CHECK: cargo test -p autocompare-gui find_bar_geometry (new test)
  EXPECT: green; bar_w == avail, badge_x == bar_w, prev_hit/next_hit >= 24.0
  CHECK: grep -n "set_min_width(bar_geo" crates/gui/src/app.rs; grep -n "find_nav_button(ui" crates/gui/src/app.rs
  EXPECT: text bar docked full-width at app layer (paint rects U-GEO views-owned);
  folder/table/hex find nav buttons use 24px-min hits (app-owned rows)

- [x] G4: Narrow-window <900px behavior (min-scroll, never overlap)
  CHECK: cargo test -p autocompare-gui layout_is_narrow (new test)
  EXPECT: green; 899.0 -> true, 900.0 -> false, panes keep floors + h-scroll

- [x] G5: Scroll-offset clamps after layout change
  CHECK: cargo test -p autocompare-gui clamp_scroll_offset (new test)
  EXPECT: green; offset clamped to [0, content-viewport], fits -> 0.0, NaN -> 0.0
  CHECK: grep -n "clamp_scroll_after_layout" crates/gui/src/app.rs
  EXPECT: called on set_split_fraction + drag_split_by + content branches

- [x] G6: Baselines hold + new tests green, clippy zero, fmt clean
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -2
  EXPECT: 231 baseline + new layout tests green, 0 failed
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings; cargo fmt --check -p autocompare-gui
  EXPECT: zero warnings; fmt clean
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-layout-u/round-U-layout-1
  EXPECT: exit 0, geo-typo-report.md contains RESULT: PASS (PNG size recorded)

- [x] G7: Hygiene — owned files only, zero banned patterns, local commit NO push
  CHECK: git status --short
  EXPECT: only crates/gui/src/app.rs + checks/gui-layout-u/**
  CHECK: grep -rn "unwrap\|expect\|panic\|todo" crates/gui/src/app.rs | grep -v test/adapter guard
  EXPECT: 0 hits in non-test code (tests may use .unwrap_or_default-style guards only if pre-existing)
  CHECK: git log --oneline -1; git status --short
  EXPECT: English-only conventional commit, local only (orchestrator verifies + pushes)
