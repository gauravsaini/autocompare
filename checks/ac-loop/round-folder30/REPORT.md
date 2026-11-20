# ac-loop round-folder30 — Folder parity (tree + Mirror/Update + jobs)

## Scope
- FolderCompare tree expand/collapse (`tree_nodes`, `expand_all`/`collapse_all`,
  `toggle_expanded`, visibility/depth/ancestor helpers).
- SyncPlanner Mirror/Update dry-run vs apply with `--prune` orphans.
- JobRunner cancel + 64-entry bounded progress queue.
- OWNS: `crates/compare/src/{folder,sync,jobs}.rs`,
  `checks/ac-loop/round-folder30/**`, `crates/compare/tests/{folder,sync,jobs}*`.
  NEVER gui/cli. Clean-room outside only.

## Harness
- `tools/parity-harness/run-round.sh checks/ac-loop/round-folder30`
  -> `RESULT: PASS` (text 1, text-same 0, folder 1, hex 1, merge 1,
  table 1, sync 1, picture 1, folder-merge 1, audio 1).
- No src fix needed: engine already correct; minimal diff = 1 edge test + 1 fixture.

## Edge fixture
- `comparison/nested-orphan-prune.txt`: 4-deep right-only subtree
  (`orphan/a/b/c/deep.txt` + mid leaves) under Mirror L2R prune.
- New test `mirror_prune_suppresses_nested_orphan_subtree`
  (`crates/compare/tests/sync.rs`): exactly 2 steps
  (CopyToRight keep.txt + DeleteRight orphan dir), zero steps/skips for any
  `orphan/*` descendant, dry-run read-only, apply `copied==1 deleted==1`
  via one `remove_dir_all`, plus Update-no-prune control (no deletes).

## Gates
- `cargo test -p autocompare-compare`: all suites ok
  (incl. new nested-orphan test 1 passed; sync integration 25 passed).
- `cargo clippy -p autocompare-compare --all-targets -- -D warnings`: 0 warnings.
- `cargo fmt -p autocompare-compare -- --check`: clean.
- banned non-test scan (`unwrap(`/`expect(`/`panic!`/`todo` outside `#[cfg(test)]`
  in folder.rs/sync.rs/jobs.rs): 0 (`unsafe` only as forbid attr + "unsafe path" strings).

RESULT: PASS
