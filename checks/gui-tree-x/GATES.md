# Wave X-TREE — GATES: dual-tree folder panes (gap #19, kadam 1)

Scope: `crates/gui/src/tree.rs` + `crates/gui/src/views.rs` + `checks/gui-tree-x/**` only.
`app.rs` / `screenshot.rs` / `viewmodel.rs` read-only (dusre agent ke hain, chhoona mana hai).
App wiring baad me; ye wave sirf model + paint API deta hai (`paint_folder` purana API untouched).

## G1 — dual-tree model synced select/scroll (headless)
- CHECK: `cargo test -p autocompare-gui dual_tree`
- EXPECT: green; `select` dono pane me ek jaisa, `sync_scroll` clamp, `bring_into_view` shared top hilata hai, `visible_range` dono pane ek jaisa.

## G2 — per-row action menu stub labels (headless)
- CHECK: `cargo test -p autocompare-gui dual_row_action`
- EXPECT: green; file row par Compare + dono copy direction, dir par khaali, har label me `(stub)` saaf dikhe.

## G3 — dual paint geometry/typo tokens reuse (headless)
- CHECK: `cargo test -p autocompare-gui dual_pane`
- EXPECT: green; pane widths `split_picture_widths` se (sums to row, MIN_HALF floor), row height `row_height_for_line`, koi naya color/token nahi.

## G4 — full gui green + clippy zero warnings + fmt + banned grep
- CHECK: `cargo test -p autocompare-gui && cargo clippy --all-targets (zero warnings) && cargo fmt --check && grep -rn 'unwrap(\|\.expect(\|panic!\|todo!' crates/gui/src/tree.rs crates/gui/src/views.rs` (non-test code me empty)
- EXPECT: sab green; banned tokens sirf `#[cfg(test)]` blocks me.

## G5 — harness proof under checks/gui-tree-x/
- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-tree-x/round-1`
- EXPECT: `geo-typo-report.md` me `RESULT: PASS`, PNG magic-ok, `REPORT.md` filed, commit NO push (sirf OWNS paths).
