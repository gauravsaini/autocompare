# GATES — exq-copytake6 round-1

Wave: COPTAKE6 exquisite (geometry first, typography second, colors frozen).
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)

- `crates/gui/src/copy.rs`
- `crates/gui/src/take.rs`
- `checks/gui-exq-copytake6/**`

## FORBIDDEN (untouched)

`app.rs`, `views.rs`, `gutter.rs`, `tree.rs`, `home.rs`, `theme.rs`,
`viewmodel.rs`, `views_hex.rs`, `views_table.rs`, `screenshot.rs`,
`STATUS.md`, `PLAN.md`, `inbox/build-progress.json`.

## Gates

- `cargo test -p autocompare-gui --lib -- copy:: take::` -> PASS
  (30 passed, 0 failed; 10 naye copytake6). Default target dir (warm cache;
  `/tmp` isolated dir me transient ENOSPC tha, saaf karke harness default
  par chalaya), `CARGO_BUILD_JOBS=2`.
- Harness `tools/exquisite/run.sh checks/gui-exq-copytake6/round-1` ->
  RESULT: PASS round-1/6 (PNG 10401 bytes >= 8192, magic ok, non-blank;
  colors touches 33, unchanged — frozen). Full suite 840 passed, 0 failed.
- `cargo clippy -p autocompare-gui --lib -- -D warnings` -> PASS (0 warnings).
- `rustfmt --edition 2024 --check` on owned files -> PASS (clean).
- Banned grep (`unwrap(`/`expect(`/`panic!`/`todo!`/`unimplemented!`/
  `Color32`/`Visuals`) in owned files -> 0.
- Commit local only, pathspec `crates/gui/src/copy.rs`,
  `crates/gui/src/take.rs`, `checks/gui-exq-copytake6/**`. NEVER pushed.
