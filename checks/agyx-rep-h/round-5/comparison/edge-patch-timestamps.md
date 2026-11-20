# Round 5 Edge Fixture: Patch Builder Timestamp Headers

Standard patch tools and Beyond Compare's patch generator include file modification timestamps in patch headers:
- Unified format:
  `--- a/old.rs\t2026-11-20 15:00:00.000000000 +0000`
  `+++ b/new.rs\t2026-11-20 15:10:00.000000000 +0000`
- Context format:
  `*** old.rs\t2026-11-20 15:00:00.000000000 +0000`
  `--- new.rs\t2026-11-20 15:10:00.000000000 +0000`

## Fixture Details
- `PatchReportBuilder::with_timestamps("2026-11-20 15:00:00 +0000", "2026-11-20 15:10:00 +0000")`
- Both Unified and Context formats tested.

## Expected Outcomes
- Timestamps separated from file path by a tab character `\t`.
- Existing default without timestamps continues to produce exact backwards-compatible header.

## Lock Tests
- `snapshot_tests::test_edge_patch_builder_timestamps`
