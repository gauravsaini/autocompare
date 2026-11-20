# ac-loop round report (round-c1) — compare-depth wave

- CLI parity: PASS (log cli.log, report cli/parity-report.md — 10/10 subcommands)
- GUI geo-typo: PASS (log gui.log, report gui/geo-typo-report.md)
- BC live observe: BCompare (comparison/bc-process.txt, read-only)
- own sessions: text folder hex merge table sync picture folder-merge audio (comparison/own-help.txt)

## Is round me kya hua (Beyond Compare parity)

1. **Sync ab folder view jaisa dikhta hai waisa plan karta hai**: naya
   `plan_sync_with_compare` + `SyncPlanner::with_compare_options` — depth
   limit, content-compare, case-folding, include/exclude filter dry-run,
   plan, apply, cancel sab me lagte hain. Purana `plan_sync` default
   behavior same hai, isliye cli/gui crates bina change ke compile hote hain.
2. **Update Both / Mirror / dry-run / apply poora hai**: `SyncMode::Both`
   (NewerWins) dono taraf copy, `Mirror` (LeftToRight+prune) extra delete,
   `dry_run()` bina chhue preview, `apply()` report ke saath. Filter se
   bahar files ko Mirror prune nahi karta (c1-filter fixture).
3. **JobRunner background parity**: `spawn_sync_plan_with_compare`,
   progress latest+history+drain, cancel flag, join par structured error
   (spawn-fail ab misleading Cancelled nahi, Io hai). Folder-compare,
   sync-plan/apply, merge-plan/apply sab background me.
4. **Sessions ab compare-depth yaad rakhti hain**: `SessionTab` me
   `sync.mode`, `folder.depth_limit/content_compare/case_insensitive/
   include/exclude` typed helpers — kharab value par default, load kabhi
   fail nahi.
5. **Hostile input par crash nahi**: symlink cycle/links kabhi follow nahi,
   `../evil`/`/abs`/empty plan paths `unsafe path` se refuse, safe steps
   phir bhi lagte hain. `VfsPath` traversal/nul-byte ko structured
   `InvalidPath` deta hai.

## Edge fixtures (comparison/, 4 naye)

- `c1-deep/` + `expected-depth1.json`: depth_limit=1 par sirf top entries,
  `truncated: true`; statuses mtime-independent rakhe.
- `c1-filter/` + `expected-include-txt.json`: include=*.txt me `extra.log`
  na step, na skip, na prune.
- `c1-hostile-plan.json` + `expected-hostile-report.json`: 3 hostile paths
  refuse, safe copy lagta hai.
- `c1-symlink-cycle.sh` + `expected-cycle.json`: live self-loop par walk
  terminate, link me descend nahi (script PASS verify ho gaya).

## Verify

- `cargo test -p autocompare-compare -p autocompare-vfs -p autocompare-sessions`: 193 passed, 0 failed
- `cargo clippy ... --all-targets -- -D warnings`: zero errors
- `cargo fmt --check`: clean
- `CARGO_BUILD_JOBS=2` use hua; MS rust rules: unsafe forbid, non-test me
  unwrap/expect/panic/index-slicing deny (tests me allow-header hai)

RESULT: PASS
