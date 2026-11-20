# round-1 REPORT — leaf-1.3 Table .xlsx deflate follow-up

Scope (OWNS only): `crates/diff/src/table.rs`, `crates/diff/tests/table.rs`,
`checks/table-xlsx/**`. `Cargo.toml`/`Cargo.lock`, `crates/gui/**`,
`crates/cli/**`, `crates/sessions/**` untouched. Clean-room: outside
observation only; no decompilation, no asset/wording copying.

## What changed

- `read_zip_entries` now handles ZIP method 8 (raw DEFLATE) via
  `flate2::read::DeflateDecoder`, keeping the existing hand-rolled
  local-header scanner (no new dependencies; `zip` 2.2 + `flate2` were
  already in `crates/diff/Cargo.toml`). Method 0 unchanged; other
  methods skipped without failing the workbook.
- Hardening, all total and without panic: data-descriptor flag (GP bit 3)
  stops the scan (local sizes unreliable), truncation stops the scan and
  keeps complete entries, corrupt/over-cap deflate payloads are skipped
  while healthy sheets survive, directory/empty names skipped.
- Zip-bomb caps: `MAX_XLSX_ENTRIES = 1024`,
  `MAX_XLSX_ENTRY_BYTES = 16 MiB`, `MAX_XLSX_TOTAL_BYTES = 64 MiB`.
- 4 unit tests (`table.rs`): deflated sheet decodes, garbage/truncation
  is `None`, corrupt-deflate + unsupported-method skipped (mixed healthy
  sheet survives), data-descriptor stops safely. No unwrap/expect/panic,
  no indexing.
- 2 integration tests (`tests/table.rs`): deflated workbooks decode via
  `Workbook::from_xlsx` and sort-before-align + tolerance 0.1 gives
  matching=1 modified=1; truncated/garbage/corrupt-deflate are `None`.

## Fixtures (`comparison/`)

- `xlsx_deflate_left.xlsx` / `xlsx_deflate_right.xlsx`: real-DEFLATE
  (every entry verified `compress_type=8`) minimal workbooks; rows
  (1,9.99)(2,5.00) vs (1,10.04)(2,7.50).
- `xlsx_deflate_expected.txt`: matching=1 modified=1 left_only=0
  right_only=0 under key-col-0 sort-before-align, tolerance 0.1.
- Disk fixtures verified end-to-end via a temporary integration test
  (decode + key diff counts as expected), temp file removed afterwards.

## Gates (all `CARGO_TARGET_DIR=/tmp/target-op-table1 CARGO_BUILD_JOBS=2`)

- `cargo test -p autocompare-diff`: ok — lib 96 passed (4 new),
  `tests/table.rs` 40 passed (2 new); all other targets green.
- `cargo clippy -p autocompare-diff --all-targets -- -D warnings`: 0 errors
  (fixed 3 `collapsible_if` via let-chains).
- `cargo fmt -p autocompare-diff -- --check`: clean.
- Banned-token scan of non-test `table.rs`
  (`unwrap(`/`expect(`/`panic!`/`todo!`/`unimplemented!`/`unreachable!`):
  0 hits; indexing/slicing covered by the denied workspace lint (clippy
  clean). `tests/table.rs` unwrap/expect only under its pre-existing
  `#![allow(...)]` header (test code).

## Limitations

1. No central-directory parsing: archives relying on the data descriptor
   stop at that entry instead of recovering trailing entries.
2. Unsupported methods (bzip2/LZMA/etc.) are skipped, not decoded.
3. Numbers still go through `f64` (documented 2^53 conflation limit).

## RESULT: PASS
