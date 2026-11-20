# Round 4 Comparison Edge Fixtures: JobRunner Cancellation & Bounded Progress

This directory contains edge fixtures validating the background JobRunner against Beyond Compare parity:
1. `job_cancel_scenario.json`:
   - Outlines the cancellation state transitions and queue boundary contracts.
2. Cancellation Protocol:
   - AtomicBool cancellation token (`AtomicBool::new(false)`).
   - Polling loop in worker checks cancellation flag before executing heavy operations.
   - Immediate abort returns `Err(Error::Cancelled)`.
3. Bounded Progress Queue:
   - Queue is bounded to 64 items (`MAX_PROGRESS_HISTORY = 64`).
   - UI thread polls `latest_progress()` or `drain_progress()` without lock contention or unbounded memory growth.
4. `verify.sh`:
   - Executes unit/integration verification of cancellation aborts, bounded queue polling, and folder merge cancellation.
