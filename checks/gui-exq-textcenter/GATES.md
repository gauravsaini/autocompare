# GATES — exq-textcenter round-1

Wave: TEXTCENTER exquisite (geometry first, typography second, colors frozen).
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)

- `crates/gui/src/viewmodel.rs`
- `crates/gui/src/theme.rs`
- `checks/gui-exq-textcenter/**`

## FORBIDDEN (untouched)

`app.rs`, `views.rs`, `copy.rs`, `take.rs`, `gutter.rs`, `tree.rs`, `home.rs`,
`views_hex.rs`, `views_table.rs`, `screenshot.rs`, `STATUS.md`, `PLAN.md`,
`inbox/build-progress.json`.

## Gates

- `cargo test -p autocompare-gui` -> PASS (816 passed, 0 failed;
  10 naye: textcenter gutter/widths/positions/fill/hunk/spans + 2 edge fixtures
  + typo steps + geometry tokens). Isolated target:
  `CARGO_TARGET_DIR=/tmp/target-agy-textcenter`, `CARGO_BUILD_JOBS=2`.
- Harness `tools/exquisite/run.sh checks/gui-exq-textcenter/round-1` ->
  RESULT: PASS (PNG 10401 bytes >= 8192, magic ok, non-blank;
  colors touches 33, unchanged — frozen).
- `cargo clippy -p autocompare-gui --lib -D warnings` -> PASS (0 warnings).
- `cargo clippy -p autocompare-gui --all-targets -D warnings` -> RED,
  NOT OURS: `views_table.rs:3110` `useless_vec` from HEAD commit db4d8e9
  (hextable3 wave). File FORBIDDEN hai, fix unki wave karegi.
  Hamari files (`viewmodel.rs`, `theme.rs`) me 0 clippy hits.
- `rustfmt --edition 2024 --check` (dono owned files) -> CLEAN.
- Banned patterns in owned diff (`unwrap`/`expect`/`panic`/`todo`/
  `unsafe`/`Color32`/`Visuals`/`from_rgb`) -> 0.
- `after.png` harness ne regenerate kiya (round-1 me maujood).

## Evidence

- `checks/gui-exq-textcenter/round-1/exquisite-report.md` (RESULT: PASS)
- `checks/gui-exq-textcenter/round-1/after.png`
- `checks/gui-exq-textcenter/round-1/tests.out`
- `checks/gui-exq-textcenter/REPORT.md`
