# Round 6 Edge Fixture: Multi-format Linked Report Bundles (HTML, Text, XML)

Beyond Compare generates linked folder comparison reports where a master summary index links to individual file comparison reports, with back-links from each child report back to the parent.

## Fixture Details
- Master bundle: `LinkedReportBundle::new("Project Compare", "index.ext", parent_input)`
- Multiple children with diverse status badges:
  - `Child 1`: "Modified" (+12, -3, ~2)
  - `Child 2`: "Left Only" (+0, -50, ~0)
  - `Child 3`: "Right Only" (+40, -0, ~0)
  - `Child 4`: "Identical" (+0, -0, ~0)
- Rendering across HTML, Plain Text (`render_text_bundle`), and XML (`render_xml_bundle`).

## Expected Outcomes
- Parent reports contain complete listings of children with status and stats.
- Child reports contain back-links to parent index.
- Full consistency across HTML, Plain Text, and XML outputs.

## Lock Tests
- `snapshot_tests::test_edge_multiformat_bundles_parity`
