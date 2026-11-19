# Wave AC-FILTER — GATES: dual-tree display filters (Gap #19)

Scope: `crates/gui/src/app.rs` + `crates/gui/src/tree.rs` +
`checks/gui-filter-ac/**` only.
KEVL read-only (chhoona mana hai): `views.rs` / `viewmodel.rs` /
`views_table.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs`.
Paint API same: `views::paint_folder_dual` ko chhoona nahi — filter App+tree
side se pre-filtered rows + filtered summary dekar lagta hai
(`viewmodel::folder_tree` unfiltered hi rahega; input pehle se filtered).
Geometry first, typography second, colors frozen.

## G1 — tree.rs me filtered dual build helpers (headless)
- CHECK: `cargo test -p autocompare-gui tree::`
- EXPECT: green; `filtered_folder_rows` order rakhkar `filter_pass` lagata hai,
  `FolderSessionState::filtered_rows / filtered_counts / filtered_counts_summary`
  session display se aate hain; purana `counts` / `visible_tree` untouched.

## G2 — App wiring: dual panes + summary filtered (paint API same)
- CHECK: `cargo test -p autocompare-gui folder_` + `grep -n paint_folder_dual crates/gui/src/app.rs`
- EXPECT: green; dual paint call me `&rows` ki jagah `folder_dual_rows()`
  (filtered), summary `folder_counts_summary()` filtered tree par;
  `sync_folder_dual` total filtered visible-tree se milta hai;
  single-list paint untouched; `views.rs` me koi diff nahi.

## G3 — full gui green + clippy zero + fmt + banned grep
- CHECK: `cargo test -p autocompare-gui && cargo clippy -p autocompare-gui --all-targets` (zero warnings) `&& cargo fmt -- <OWN files>` + `cargo fmt --check -- <OWN files>` `&& grep -rn 'unwrap(\|\.expect(\|panic!\|todo!' crates/gui/src/app.rs crates/gui/src/tree.rs` (non-test code me empty)
- EXPECT: sab green; banned tokens sirf `#[cfg(test)]` blocks me.

## G4 — harness proof + REPORT + commit (NO push)
- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-filter-ac/round-1`
- EXPECT: `geo-typo-report.md` me `RESULT: PASS`, `REPORT.md` filed,
  `git add` sirf OWNS paths, `git commit` (conventional English), NO push,
  `git stash/push` kabhi nahi.
