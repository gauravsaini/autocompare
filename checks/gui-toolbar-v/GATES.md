# Wave V TOOLBAR — GATES (owns crates/gui/src/app.rs + checks/gui-toolbar-v/**)

Scope: app.rs toolbar row only. views.rs / theme.rs READ-ONLY (no edits).
Colors frozen: NO hue/saturation/token changes. MS rust rules: forbid unsafe,
no unwrap/expect/panic/todo in non-test code. Commit locally, NO push.

- [x] G1: Toolbar breathing room via GEO consts, single aligned row
  CHECK: grep -n "toolbar_item_gap\|toolbar_group_gap" crates/gui/src/app.rs
  EXPECT: helpers defined + used in toolbar row near app.rs:2482; item_gap reuses views::TIGHT_GAP, group_gap reuses views::SECTION_GAP, zero new float literals in toolbar paint
  CHECK: sed -n '2482,2520p' crates/gui/src/app.rs | grep -c "ui.horizontal"
  EXPECT: 1 (single aligned row, no second row, no horizontal_wrapped)

- [x] G2: Narrow-safe, no wrap/overlap
  CHECK: cargo test -p autocompare-gui toolbar_ (new tests)
  EXPECT: green; item_gap/group_gap finite positive, group_gap > item_gap, narrow avail keeps single-row intent (h-scroll predicate true below min)
  CHECK: grep -n "horizontal_wrapped\|vertical(" crates/gui/src/app.rs | sed -n '1,20p' (toolbar region audit)
  EXPECT: toolbar row uses ui.horizontal only, no wrap call in toolbar region

- [x] G3: Baselines hold + harness PASS
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -2
  EXPECT: 245 baseline + new toolbar tests green, 0 failed
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: zero warnings
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: clean
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-toolbar-v/round-V-1
  EXPECT: exit 0, geo-typo-report.md contains RESULT: PASS
  CHECK: target/debug/autocompare-gui --screenshot checks/gui-toolbar-v/after.png
  EXPECT: exit 0, PNG magic 89504e470d0a1a0a ok, size recorded

- [x] G4: Hygiene — owned files only, zero banned patterns, local commit NO push
  CHECK: git status --short
  EXPECT: only crates/gui/src/app.rs + checks/gui-toolbar-v/**
  CHECK: grep -n "unwrap\|expect(\|panic\|todo!" crates/gui/src/app.rs | grep -v "^\s*[0-9]*:\s*//" | grep -iv "unwrap_or\|test" ; echo BANNED:$?
  EXPECT: 0 non-test hits (only pre-existing unwrap_or guards)
  CHECK: git log --oneline -1; git status --short | head
  EXPECT: English-only conventional commit, local only (orchestrator verifies + pushes)
