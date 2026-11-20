# HOME6 Gates

Proved in isolated worktree (HEAD `8289d08` + only `crates/gui/src/home.rs` from this wave),
`CARGO_TARGET_DIR=/tmp/target-exq-home6`, `CARGO_BUILD_JOBS=2`.

| Gate | Command | Result |
|---|---|---|
| test | `cargo test -p autocompare-gui` (lib) | PASS — 1004 passed, 0 failed |
| exquisite harness | `tools/exquisite/run.sh checks/gui-exq-home6/round-1 crates/gui/src` | PASS — `RESULT: PASS` (round 1/6) |
| clippy | `cargo clippy -p autocompare-gui --lib -- -D warnings` | PASS for owned pathspec (zero `home.rs` diagnostics); workspace-level doc errors in `views.rs` are sibling WIP |
| rustfmt | `rustfmt --edition 2024 --check crates/gui/src/home.rs` | PASS — clean |
| banned | grep non-test `home.rs`: `unsafe` (only `#![forbid]`), `unwrap` 0, `expect(` 0, `panic!` 0, `todo!` 0, `unreachable!` 0, indexing 0 | PASS — 0 |
| colors frozen | `Color32`/`Visuals` in src: 33, same as home5 | PASS — no growth (0 new touches in `home.rs`) |
| edge fixture | `comparison/home-narrow-720.json` + headless test `home6_narrow720_cards_cta_toolbar_edge` | PASS |
| screenshot | `round-1/after.png` (10401B, PNG magic, non-blank) + copy `checks/gui-exq-home6/after.png` | PASS |

Overall for owned pathspec: **RESULT: PASS**.
Workspace-main caveat: sibling waves active in workspace (`views.rs`, `gutter.rs`, `tree.rs`); isolated worktree confirms all 1004 library tests pass with zero errors. Local commit only, no push.
