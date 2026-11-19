# Wave AB-DUALAPP — GATES: dual-tree App wiring (gap #19, kadam 2)

Scope: `crates/gui/src/app.rs` + `checks/gui-dualapp-ab/**` only.
KEVL read-only (chhoona mana hai): `views.rs` / `gutter.rs` / `tree.rs` /
`viewmodel.rs` / `views_table.rs` / `views_hex.rs` / `theme.rs` / `screenshot.rs`.
Model (`tree::DualTreeState`) + paint (`views::paint_folder_dual`,
`#[allow(dead_code)]` wala) kadam 1 me land ho chuka hai; ye wave sirf
App side wiring karta hai. Geometry first, typography second, colors frozen.

## G1 — App me DualTreeState field + results se build (headless)
- CHECK: `cargo test -p autocompare-gui folder_dual`
- EXPECT: green; `App::default()` me dual empty (koi select/pending nahi,
  window band), `sync_folder_dual` visible-tree total set karta hai,
  select dono pane me ek jaisa dikhta hai, swap/error par reset hota hai.

## G2 — folder view dual panes me paint hota hai (wiring)
- CHECK: `grep -n paint_folder_dual crates/gui/src/app.rs`
- EXPECT: Folder tab se `views::paint_folder_dual` call hota hai — same
  rows + counts summary, `folder_session.expanded` + `folder_dual` +
  `split_fraction` ke saath; legacy `folder_expanded` mirror hota hai.
  Single-list paint untouched (koi regression nahi).

## G3 — full gui green + clippy zero + fmt + banned grep
- CHECK: `cargo test -p autocompare-gui && cargo clippy -p autocompare-gui --all-targets` (zero warnings) `&& cargo fmt --check` (OWN files) `&& grep -rn 'unwrap(\|\.expect(\|panic!\|todo!' crates/gui/src/app.rs` (non-test code me empty)
- EXPECT: sab green; banned tokens sirf `#[cfg(test)]` blocks me.

## G4 — harness proof + REPORT + commit (NO push)
- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-dualapp-ab/round-1`
- EXPECT: `geo-typo-report.md` me `RESULT: PASS`, `REPORT.md` filed,
  `git add` sirf OWNS paths, `git commit` (conventional English), NO push,
  `git stash/push` kabhi nahi.
