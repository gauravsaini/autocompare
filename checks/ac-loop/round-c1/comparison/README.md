# round-c1 comparison fixtures (compare-depth wave)

C1 me folder depth + sync-filter + hostile-input parity ke liye 4 edge fixtures:

1. `c1-deep/` — depth_limit=1 walk. `top.txt` size-differ (mtime-independent
   `Different`), `sub/` left-only dir with children. Expected:
   `expected-depth1.json` (entries `[sub, top.txt]`, `truncated: true`).
2. `c1-filter/` — include=`*.txt` + Mirror prune. `extra.log` right-only lekin
   filter se bahar: na step, na skip, na prune. Expected:
   `expected-include-txt.json`.
3. `c1-hostile-plan.json` — `../evil`, `/abs`, `` jaise hostile rel_paths.
   Expected: `expected-hostile-report.json` (`copied: 1`, 3x `unsafe path`,
   koi crash/write bahar nahi).
4. `c1-symlink-cycle.sh` + `expected-cycle.json` — live self-loop tree banata
   hai, CLI `folder` walk terminate hota hai, link me descend nahi hota.

Statuses mtime-independent rakhe gaye hain taaki checkout par stable rahen;
mtime-sensitive cases (NewerLeft/Right, Same) pinned-mtime Rust tests me
cover hain: `crates/compare/tests/sync.rs`, `folder.rs`, `jobs.rs`.
