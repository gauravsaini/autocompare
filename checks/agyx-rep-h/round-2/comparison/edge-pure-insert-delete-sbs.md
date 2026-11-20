# Round 2 Edge Fixture: Pure Insertions and Pure Deletions in Side-by-Side

Beyond Compare parity requires that a file with only insertions (e.g. newly added file / Right Only) or only deletions (e.g. deleted file / Left Only) formats properly in dual-pane Side-by-Side:
- In pure deletion: Left side displays deleted lines (`line-delete`), right side displays empty placeholder cells (`sbs-empty`).
- In pure insertion: Left side displays empty placeholder cells (`sbs-empty`), right side displays inserted lines (`line-insert`).
- Empty lines (`content: ""`): Must still receive proper `line-delete` or `line-insert` styling with line numbers, rather than being confused with empty padding cells.

## Fixture Details
- Pure Delete: 3 lines deleted, 0 added.
- Pure Insert: 3 lines inserted (including 1 blank line `""`), 0 deleted.

## Expected Outcomes
- Side-by-Side Left table cell has `line-delete` and Right cell has `sbs-empty` for deletions.
- Side-by-Side Left cell has `sbs-empty` and Right cell has `line-insert` for insertions.
- Blank inserted line retains `line-insert` class and its line number.

## Lock Tests
- `snapshot_tests::test_edge_pure_insertions_and_pure_deletions_sbs`
