# Wave W REPORT: progress + cancel wiring (owns crates/gui/src/app.rs + checks/gui-prog-w/**)

HEAD 973d81d baseline: gui tests 249 green, clippy zero, fmt clean, PNG 9914B magic-ok 256.
Scope: app.rs only. views.rs / viewmodel.rs / theme.rs untouched (read-only).
Clean-room: reference tool observed from outside only; own words; no strings/assets copied. No hue/saturation/color-token changes (diff grep Color32/Visuals: 0).

## What changed (app.rs only, +425/-24 vs working tree)
- State: `job: Option<JobHandle<()>>`, `progress_fraction: Option<f32>`, `progress_message: Option<String>`; existing `status` field reused for display.
- Pure state machine (headless-tested): `progress_fraction_of` (None while total unknown, clamped 0..=1), `progress_should_emit` (emit on message change, None<->Some, delta>=0.05, or done; else quiet), `progress_status_text` (reuses `status`).
- Methods: `note_job_started` (one call per run path), `is_job_running`, `cancel_active_job` (signals handle, notes "cancel requested"), `poll_job_progress` (throttled fold into `status`, returns repaint flag; no ctx use).
- All 9 `run_*` sites (text/folder/hex/merge/table/sync/picture/folder_merge/audio) now keep the `JobHandle`, report chunk-boundary `JobProgress`, and check `cancel` at chunk bounds, sending "operation cancelled" through the channel so the thread always ends via the normal result path. Folder uses `compare_folders_cancellable` with the live flag.
- Completion clears `rx`+`job`+progress (no leaked threads). Paint (app.rs inline, no views.rs change): spinner row gains `ProgressBar` (default visuals, fraction only) + Cancel `small_button`; poll runs before repaint while busy.

## Evidence
- G1 wiring: `grep -c "note_job_started" = 9`; zero `_cancel`/`_progress` remain: `grep -n "_cancel, _progress" crates/gui/src/app.rs` empty.
- G2 throttle: `prog_throttle_gate_suppresses_byte_spam` green (same-msg same-fraction quiet, >=0.05 emits, msg change emits).
- G3 cancel: `prog_cancel_signals_running_job_and_status_notes_it` + `prog_cancel_path_without_job_reports_idle` green; `grep -c "operation cancelled" = 18` (2 chunk guards x 9 sites).
- G4 tests: `cargo test -p autocompare-gui` 257 passed 0 failed (baseline 249 + 5 prog_* + 3 from parallel wave's views/viewmodel edits, untouched by this wave). Evidence: checks/gui-prog-w/tests-summary.txt, prog-tests.txt.
- G5 harness: `tools/gui-polish/run-geo-typo.sh checks/gui-prog-w/round-1` exit 0, RESULT: PASS. `--screenshot` PNG 9914B magic-ok distinct 256 (baseline-exact). Evidence: round-1/geo-typo-report.md, checks/gui-prog-w/png-check.txt.
- G6 hygiene: `cargo clippy -p autocompare-gui --all-targets` 0 warnings; `cargo fmt --check -- crates/gui/src/app.rs` clean; banned-pattern grep `\.unwrap\(|\.expect\(|panic!|todo!` (excluding `unwrap_or`) in app.rs: 0 hits.
- G7: this file + commit locally, no push (orchestrator verifies + pushes).

## Drift note
A parallel wave edits views.rs/viewmodel.rs (staged + unstaged in tree) while this wave ran; one mid-task clobber of app.rs was detected (top-of-file edits lost, 3 run sites survived) and re-applied + re-verified. Final `git diff` for this wave is app.rs + checks/gui-prog-w/** only.
