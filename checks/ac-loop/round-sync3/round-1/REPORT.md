# ac-loop round-sync3 round-1 — Folder Sync exec semantics

Scope: Update Left/Right/Both, Mirror/MirrorLeft dry-run vs apply parity,
cancellable JobRunner progress, consent gate, CLI render parity.
OWNS: crates/compare/src/sync.rs, crates/compare/src/jobs.rs,
crates/cli/src/render_sync.rs, checks/ac-loop/round-sync3/**.

## Fixture

- checks/ac-loop/round-sync3/round-1/comparison/edge-manifest.txt (1 edge
  fixture: nested left-only deep leaf, empty chain a/b/c, newer ties with
  pinned mtimes, right-only extra, non-empty keep dir, unix symlink note).

## Tests

- `CARGO_TARGET_DIR=/tmp/target-op-sync3 CARGO_BUILD_JOBS=2 cargo test -p autocompare-compare`
  = 154 passed, 0 failed (lib 26 incl. 9 new: 6 sync exec-semantics +
  3 JobRunner progress/cancel; integration sync 22, jobs 11 unchanged).
- New unit tests: update_left apply parity + dry-run read-only,
  both/mirror/mirror-left exec parity, requires_consent + apply_with_consent,
  apply progress callback sequence, precancelled plan/apply/prune/run -> Cancelled,
  prune symlink/root safety; spawn_sync_run parity + progress,
  spawn_sync_apply progress sequence, precancelled token -> Cancelled status.
- `cargo test -p autocompare-cli --lib` = 15 passed incl. 4 new
  render_sync format tests (measured while cli tree compiled; current tree
  is uncompilable from concurrent uncommitted report_adapter*.rs outside OWNS).
- `cargo test -p autocompare-cli --lib render_sync` = 4 passed
  (empty plan, steps/skipped/summary glyphs, truncated-on-stderr,
  report counts + color on/off).

## Gates

- clippy compare: `cargo clippy -p autocompare-compare --all-targets -- -D warnings`
  = zero warnings.
- clippy cli: 41 errors, all in report_adapter.rs / report_adapter_media.rs
  (concurrent uncommitted work outside OWNS); 0 in render_sync.rs.
- fmt owned: `cargo fmt -p autocompare-compare -p autocompare-cli -- --check`
  clean for sync.rs, jobs.rs, render_sync.rs; remaining diffs only in
  unowned runner.rs / session_cmd.rs / bcomp.rs, untouched per OWNS.
- banned-scan non-test: 0 (`unwrap(`/`expect(`/`panic!`/todo etc. all inside
  #[cfg(test)] mods starting sync.rs:1279, jobs.rs:599, render_sync.rs:90;
  `unsafe` only as forbid attribute + "unsafe path" strings; indexing only
  `#[default]` / `&[bool]`; clippy indexing_slicing deny corroborates).

## Parity (Update/Both/Mirror, dry-run vs apply, JobRunner)

- UpdateRight: copies newer-left + left-only, skips newer destination + right-only.
- UpdateLeft: copies newer-right + right-only, skips left-only; dry-run
  read-only, apply count == steps, re-plan converges.
- Both/TwoWay: newer wins both ways, one-sided copied, never deletes.
- Mirror: copies left wins + prunes right-only (dir prune suppresses subtree);
  MirrorLeft mirrors. Dry-run prunes/copies nothing; apply totals == steps.
- Consent: empty plan never needs it; Mirror always does; preview-only
  returns Unsupported and writes nothing.
- Progress: apply_sync_cancellable emits (idx,total,rel) per step + final
  complete; spawn_sync_run/apply emit per-file totals == steps, queue <= 64.
- Cancel: precancelled flag aborts plan/apply/prune/run with Cancelled.

RESULT: PASS
