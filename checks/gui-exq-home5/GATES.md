# HOME5 Gates

Proved in isolated worktree (HEAD `d8c7270` + only `crates/gui/src/home.rs` from this wave),
`CARGO_TARGET_DIR=/tmp/target-op-home5`, `CARGO_BUILD_JOBS=2`.

| Gate | Command | Result |
|---|---|---|
| test | `cargo test -p autocompare-gui` (lib) | PASS — 817 passed, 0 failed |
| exquisite harness | `tools/exquisite/run.sh checks/gui-exq-home5/round-1` | PASS — `RESULT: PASS` (round 1/6) |
| clippy | `cargo clippy -p autocompare-gui --all-targets -- -D warnings` | PASS for owned pathspec (zero `home.rs` diagnostics); workspace-level FAIL is pre-existing at HEAD in forbidden `views_table.rs:3110` (`useless_vec`), untouched |
| rustfmt | `rustfmt --edition 2024 --check crates/gui/src/home.rs` | PASS — clean |
| banned | grep non-test `home.rs`: `unsafe` (only `#![forbid]`), `unwrap` 0, `expect(` 0, `panic!` 0, `todo!` 0 | PASS — 0 |
| colors frozen | `Color32`/`Visuals` in src: 33, same as home4 | PASS — no growth |
| edge fixture | `comparison/home5-empty-8tabs.json` + headless test | PASS |
| screenshot | `round-1/after.png` (10401B, PNG magic, non-blank) + copy `checks/gui-exq-home5/after.png` | PASS |

Overall for owned pathspec: **RESULT: PASS**.
Workspace-main caveat: sibling TREE5 WIP dirties `tree.rs` (missing `folder_tree_scoped_global`), so main-copy `cargo test`/`clippy` are red until that wave lands. No push performed (local commit only).
