# REPORT — Wave SYNCGUI (round-1)

Scope: GUI sync execution — Update Left/Right/Both + Mirror, conflict
nav, live log (BC parity). Thin GUI: `tree.rs` wires existing tree
fill + filter state through the compare ToolSpec, zero business logic
in gui. Headless-first: every transition unit-tested without a window.

OWNS: `crates/compare/**`, `crates/gui/src/tree.rs`,
`checks/sync-gui/**`. No other paths touched.

## Key metrics

- compare: 11 suites ok, 156 passed, 0 failed
  (`cargo test -p autocompare-compare`), incl 2 new ToolSpec tests.
- gui: 787 passed, 0 failed (`cargo test -p autocompare-gui`),
  incl 4 new (4-mode preview, conflict nav, live log, session
  cursor+log+reset).
- Clippy: 0 warnings
  (`cargo clippy -p autocompare-compare -p autocompare-gui --all-targets -- -D warnings`).
- `cargo fmt --check`: clean.
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`/`unimplemented!`/bare
  `unsafe`, comments/strings/test-mods stripped): clean.
  Negative control (known-dirty snippet) correctly flagged.
- Screenshot: `round-1/sync-tree-plan.png`, 11416 bytes, PNG magic ok.

## What was built

compare (`crates/compare/src/sync.rs` + `lib.rs` re-exports) —
ToolSpec registry, single source of truth next to the engine rules:
- `gui_sync_modes()` — `[UpdateLeft, UpdateRight, Both, Mirror]`
  picker order; `gui_sync_options(mode)` — engine `SyncOptions`.
- `gui_action_for` / `gui_skip_reason_for` — per-row verdict over
  `(glyph, is_dir, opts)`, mirroring `plan_file` / `plan_dir`.
- `is_sync_conflict_reason` + `sync_conflict_next` / `sync_conflict_prev`
  over raw skipped lists (same predicate as `SyncPlan`).
- `SyncLiveLog` — plan summary + per-step/skip lines, per-step
  apply lines, report summary + failures; `lines()` for paint.

gui (`crates/gui/src/tree.rs` only) — thin wiring:
- `SyncModeChoice` extended to 4 modes with `all()`, `label()`,
  `engine_mode()`; `sync_options_for` delegates to `gui_sync_options`.
- `sync_preview` + `sync_direction_for_row` delegate per-row to the
  ToolSpec (doomed-subtree suppression stays: structural, not rules).
- Conflict nav: `sync_preview_conflicts`, `sync_preview_conflict_count`,
  `sync_preview_next_conflict`, `sync_preview_prev_conflict`.
- Live log: `SyncReportView` (+`to_plan`/`to_report` bridges),
  `sync_live_log_lines`, `sync_log_visible_lines`, `sync_log_typo`.
- `FolderSessionState`: `sync_conflict` cursor + `live_log` with
  next/prev/clear/log helpers; `set_sync_mode` resets consent +
  cursor + log together (no stale paths).

Edge fixture: `round-1/edge-orphan-newer.json` (two-folder
orphan/newer + same/differ controls, expected plan per mode),
covered headless by
`toolspec_gui_verdicts_match_engine_plans_on_orphan_newer_edge`.

## Blockers

None. Note: working tree holds pre-existing dirty files outside OWNS
(`crates/sessions`, `dist`, other `checks/`); untouched, uncommitted
by this wave.

## Gate evidence

- G1 compare tests: 11x `test result: ok`, 0 failed.
- G2 gui tests: `787 passed; 0 failed`.
- G3 clippy: `Finished`, zero warnings.
- G4 fmt: `FMT_CLEAN` via `cargo fmt --check`.
- G5 banned: `BANNED_CLEAN` (checker + negative control above).
- G6 screenshot: `SCREENSHOT_OK`, 11416 bytes.
- G7 this report: `RESULT: PASS`.

RESULT: PASS
