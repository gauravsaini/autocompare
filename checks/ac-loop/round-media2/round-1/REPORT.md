# round-1 REPORT — table column mapping + sort-before-align + xlsx multi-sheet

## Assumptions (no interactive Q&A in this run)
- `crates/diff/src/lib.rs` untouched (outside OWNS): no new free-fn
  re-exports. New API is methods on already-exported `Table` only.
- `TableCompareOptions` gains no fields: `compare/src/table.rs` and
  `diff/tests/table.rs` use struct literals without `..Default`.
- `Cargo.toml` untouched (outside OWNS): no flate2, so deflated xlsx
  still returns `None` honestly (documented, not guessed).
- Harness: `tools/ac-loop/run.sh` needs cli/gui dirs; media rounds
  mirror round-m1 shape (`comparison/` fixtures + REPORT.md) instead.

## What changed (`crates/diff/src/table.rs` only)
- `Table::column_index(name)` — exact header lookup, empty never matches.
- `Table::select_columns(cols)` — mapping/reorder/select; OOR -> "".
- sort-before-align reuses existing `Table::sorted`; key diff reuses
  `diff_table_by_key_with_options` (numeric tolerance, no new fields).
- 4 unit tests (no unwrap/expect/panic, no indexing).

## Fixtures (`comparison/`)
- `table_map_left/right.csv` + `table_map_expected.txt`
  (shuffled layouts + rows; tol=0.1: matching=1, modified=1)
- `xlsx_multisheet_expected.txt` (stored-zip 2-sheet edge; sparse B2)

## Gates
- `cargo test -p autocompare-diff -p autocompare-compare`: all green
- `cargo clippy --all-targets` (both crates): 0 warnings
- `rustfmt --check` owned file: clean
- banned tokens (`unwrap(`/`expect(`/`panic!`): 0 in OWNS (1 comment mention)
- `allow(dead_code)` in OWNS: 0

## RESULT: PASS
