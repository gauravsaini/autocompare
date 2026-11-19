# Parity Pcomp Report — Table Key Alignment + Numeric Tolerance + Hex Alignment

Date: 2026-11-19
Scope: `crates/diff`, `crates/compare` (owned, edited)
Read-only (reused via API, no edits): `crates/gui`, `crates/cli`, `crates/sessions`, `crates/reports`, `crates/vfs`, `crates/evidence`
Harness: `cargo test -p autocompare-diff -p autocompare-compare` green + `cargo clippy -p autocompare-diff -p autocompare-compare --all-targets -- -D warnings` clean

## Result

GREEN. 291 tests passed, 0 failed (5 ignored exhaustive sweeps, release-only).

| Crate | Test target | Passed |
|---|---|---|
| autocompare-compare | lib unit | 0 |
| autocompare-compare | tests/binary.rs | 14 |
| autocompare-compare | tests/folder.rs | 19 |
| autocompare-compare | tests/folder_merge.rs | 15 |
| autocompare-compare | tests/jobs.rs | 5 |
| autocompare-compare | tests/media.rs | 5 |
| autocompare-compare | tests/picture.rs | 7 |
| autocompare-compare | tests/sync.rs | 16 |
| autocompare-compare | tests/table.rs | 11 |
| autocompare-compare | tests/text.rs | 12 |
| autocompare-diff | lib unit (table/tolerance) | 52 |
| autocompare-diff | tests/api.rs | 33 (+5 ignored) |
| autocompare-diff | tests/audio.rs | 44 |
| autocompare-diff | tests/image.rs | 20 |
| autocompare-diff | tests/table.rs | 38 |

Clippy clean with workspace lints (`unsafe_code = "forbid"`, `unwrap_used/expect_used/panic/todo/unimplemented/unreachable/indexing_slicing = "deny"`).
Source check: no `unwrap()/expect()/panic!/todo!` in owned `src/` (only `unwrap_or` and the string literal `"unsafe path"` in pre-existing error text); whole workspace (`cli`, `gui` included) still builds — `TableOptions` was left `Copy` and untouched.

## Coverage

### 1. Table key-column alignment (`crates/diff/src/table.rs`, pure)

- `diff_table_by_key(a, b, key_columns)` — exact key-aligned diff returning `TableKeyDiff { key_columns, row_diffs, matching_rows, modified_rows, left_only_rows, right_only_rows }` with `KeyRowDiff::{Unchanged, Modified { differing_columns }, LeftOnly, RightOnly}`. Rows match by composite key regardless of order; duplicate keys pair off in first-unused order; empty key list falls back to the Myers row script.
- `detect_key_columns(table)` — single unique non-empty column first, then two-column composite pairs, else empty.
- `sort_table_rows(table, columns, ascending)` + `Table::sorted(...)` — numeric-aware (i64, then f64, then lexicographic).
- `Workbook/Sheet/diff_workbooks/SheetDiffStatus/WorkbookDiff` — name-matched multi-sheet compare.
- All of the above were already implemented but `#[allow(dead_code)]`-gated and unexported; they are now public API via `autocompare_diff::{...}` and covered by integration tests (reorder alignment, one-sided rows, unique/composite/no-key detection, numeric sort).

### 2. Numeric tolerance (`crates/diff/src/table.rs`, pure)

- `cells_equal_with_tolerance(a, b, tolerance)` — exact match wins; otherwise both cells must parse as finite `f64` (surrounding whitespace ignored) with `|a-b| <= tolerance`. `None`/negative/NaN/infinite tolerance means exact text. NaN and infinite cell values never match numerically.
- `TableCompareOptions { key_columns, numeric_tolerance, ignore_case, trim_space }`, `cells_equal_with_options`, `diff_cells_with_options` (missing cells compare as `""`), `diff_table_by_key_with_options` — tolerant key-aligned diff. With default flags it delegates bit-identically to `diff_table_by_key`; otherwise keys match normalised by `trim_space`/`ignore_case` (never by tolerance) and data cells compare tolerantly; empty key list compares positionally (row `i` vs row `i`) rather than minimal-edit.
- File layer: `TableKeyOptions` + `compare_table_files_by_key(left, right, &TableOptions, &TableKeyOptions)` in `crates/compare` — same read/decode/parse path as `compare_table_files`, alignment and equality from the tolerant engine. Tested with reordered CSVs, in/out-of-tolerance prices, and missing-file mapping.

### 3. Hex alignment modes Complete/Fast/None (`crates/diff/src/hex.rs` pure + `crates/compare/src/binary.rs` file layer)

- Pure: `HexAlignment::{None, Fast, Complete}`, `diff_hex(left, right, alignment) -> HexDiff { left_len, right_len, alignment, first_diff, differing_bytes, matching_bytes, ops, exact }`, `aligned_hex_rows` side-by-side view. `None` is positional; `Fast` is a windowed (512 B, 8 B anchor) resync heuristic; `Complete` is the optimal Myers edit script.
- File layer: `compare_binary_files_aligned(left, right, max_hunks, alignment) -> Result<BinaryComparison>` — reads both files whole (16 MiB cap each, see limitations), aligns with `diff_hex`, chunks differing runs into 64-byte hunks. Gap runs carry one-sided bytes; gap hunk offset is the left offset when present, else the right offset. `truncated` iff required chunks exceed `max_hunks`. Tested: identical files in all modes, insertion surfacing as one right-only gap in Complete/Fast vs positional noise + tail gap in None, anchor resync, zero-budget truncation, missing-file mapping.

## Limitations

1. Numeric comparison goes through `f64`: integers beyond 2^53 may conflate (`9007199254740993` vs `9007199254740992` compare equal even at tolerance 0).
2. Tolerance never applies to key matching: keys `"1"` and `"1.0"` are different keys. Case/space normalisation applies to keys; numeric tolerance does not.
3. Tolerant compare with empty key columns is positional, not minimal-edit (unlike the exact path's Myers fallback): a row insertion shifts all following rows to `Modified`.
4. `compare_binary_files_aligned` caps each file at 16 MiB (`ALIGNED_HEX_MAX_BYTES`); larger files return `Error::Unsupported`. Unbounded files must use positional `compare_binary_files`.
5. No column-name mapping: key columns are positional indices, and headers take no part in alignment. Column insert/delete without key coverage shows as cell shifts, not as column moves.
6. Beyond Compare behaviours not in scope: unimportant-text rules, date-format-aware compare, character-level alignment inside cells, and GUI session wiring — the CLI/GUI crates were read-only for this wave.
