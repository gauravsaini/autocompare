# GATES — exq-take7 round-1

Wave: TAKE7 exquisite (geometry first, typography second, colors frozen).
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)

- `crates/gui/src/take.rs`
- `checks/gui-exq-take7/**`

## FORBIDDEN (untouched)

`app.rs`, `views.rs`, `views_table.rs`, `views_hex.rs`, `home.rs`, `tree.rs`,
`gutter.rs`, `copy.rs`, `theme.rs`, `viewmodel.rs`, `dist/**`, `skills/**`,
`checks/localrun/**`, `checks/gui-exq-copytake6/**`, `checks/gui-exq-viewfill9/**`,
`checks/gui-exq-gutter11/**`.

## Gates

- `cargo test -p autocompare-gui --lib -- take::` -> PASS
  (28 passed, 0 failed; 4 naye take7 tall-marker). `CARGO_BUILD_JOBS=2`.
- Harness `tools/exquisite/run.sh checks/gui-exq-take7/round-1 crates/gui/src` ->
  RESULT: PASS round-1/6 (PNG 10401 bytes >= 8192, magic ok, non-blank;
  colors touches 33, unchanged — frozen). `CARGO_TARGET_DIR=/tmp/target-exq-take7`.
- `cargo clippy -p autocompare-gui --lib -- -D warnings` -> PASS (0 warnings).
- `rustfmt --edition 2024 --check` on `crates/gui/src/take.rs` -> PASS (clean).
- Banned grep (`unwrap(`/`expect(`/`panic!`/`todo!`/`unreachable!`/
  `Color32`/`Visuals`) in owned non-test code -> 0.
- Commit local only, pathspec `crates/gui/src/take.rs`,
  `checks/gui-exq-take7/**`. NEVER pushed.
