# Round 4 Edge Fixture: Context Filtering Boundaries (ReportFilter::Context)

Beyond Compare allows filtering reports to show only changes plus N surrounding lines of unchanged context (`ReportFilter::Context(N)`).

## Fixture Details
- 10-line hunk with:
  - Lines 1-3: Unchanged context
  - Line 4: Delete
  - Line 5: Insert
  - Lines 6-8: Unchanged gap
  - Line 9: Modify
  - Line 10: Unchanged
- Testing `ReportFilter::Context(0)` (diffs only), `ReportFilter::Context(1)` (1 line context before/after diffs), and `ReportFilter::All`.

## Expected Outcomes
- `Context(0)`: Only lines 4, 5, 9 included.
- `Context(1)`: Lines 3, 4, 5, 6, 8, 9, 10 included; lines 1, 2, 7 omitted.
- `All`: All 10 lines included.

## Lock Tests
- `snapshot_tests::test_edge_context_filter_boundaries`
