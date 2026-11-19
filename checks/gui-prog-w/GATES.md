# Wave W GATES — progress + cancel wiring (owns crates/gui/src/app.rs + checks/gui-prog-w/**)

Baseline: HEAD 973d81d, gui tests 249 green, clippy zero, fmt clean, PNG 9914B magic-ok 256.
Scope: app.rs only (views.rs / viewmodel.rs / theme.rs read-only). No hue/saturation/color-token changes.

- [ ] G1: 9x JobRunner::spawn sites wire cancel+progress, handle retained (no discard)
  CHECK: grep -n "JobRunner::spawn" crates/gui/src/app.rs (9 sites use `cancel`/`progress`, zero `_cancel`/`_progress`); grep -n "self.job = Some" crates/gui/src/app.rs
  EXPECT: 9 spawn closures name cancel+progress params (no underscore prefix); handle stored per run
- [ ] G2: progress fraction state shown via existing status field, throttled (no per-byte repaint spam)
  CHECK: grep -n "progress_fraction\|poll_job_progress\|progress_should_emit" crates/gui/src/app.rs; cargo test -p autocompare-gui prog
  EXPECT: fraction field + pure throttle gate (delta>=0.05 or msg change or done); status text reuses `status` field; prog tests green
- [ ] G3: cancel button + abort path per running job, no leaked threads, cancel checked at chunk boundaries
  CHECK: grep -n "cancel_active_job\|Cancel" crates/gui/src/app.rs; grep -n "is_cancelled\|Ordering::Relaxed\|operation cancelled" crates/gui/src/app.rs
  EXPECT: Cancel button next to spinner; cancel() on stored handle; each worker checks cancel at chunk bounds and reports "operation cancelled" via channel
- [ ] G4: headless tests for progress-state machine green
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: 0 failed; total >= 249 (baseline) with new prog_* tests listed
- [ ] G5: harness proof (geo-typo round + --screenshot PNG)
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-prog-w/round-1; python3 PNG magic+size+distinct on checks/gui-prog-w/after.png
  EXPECT: geo-typo-report.md RESULT: PASS; PNG magic-ok, size recorded, distinct>64
- [ ] G6: clippy zero + fmt clean + zero banned patterns in edited non-test code
  CHECK: cargo clippy -p autocompare-gui --all-targets 2>&1 | tail -2; cargo fmt --check -- crates/gui/src/app.rs; grep -rn "\.unwrap(\|\.expect(\|panic!\|todo!" crates/gui/src/app.rs | grep -v "unwrap_or\|#\[cfg(test)\]" || true (verify hits are pre-existing/test-only)
  EXPECT: clippy 0 warnings; fmt exit 0; zero new banned-pattern hits in added lines
- [ ] G7: REPORT.md + commit locally NO push
  CHECK: ls checks/gui-prog-w/REPORT.md; git log --oneline -1; git status --short
  EXPECT: REPORT.md present with evidence; commit exists on branch, no push (orchestrator verifies+pushes)
