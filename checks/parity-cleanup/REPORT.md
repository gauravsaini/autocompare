# Parity Cleanup — Wave J Agent A2 Slice-1 Report

Scope sirf owned files: `crates/gui/src/views_hex.rs`, `crates/gui/src/views_table.rs`,
`checks/parity-cleanup/**`. `app.rs`, `views.rs`, `main.rs`, `gutter.rs`, `viewmodel.rs`,
`theme.rs`, `STATUS.md` ko chhua nahi. Commit local hai, push nahi.

## 1. Slice-1 (2 sabse chhote items — verification, koi naya code nahi)

Dono items pehle se live nikle (Wave I + `10c8928` + `7dbd5f0` me land ho chuke).
Duplicate churn se bachne ke liye slice-1 me koi code change nahi — sirf used-where
verify, tests confirm, aur ye report refresh. Real use hai, stub kuch nahi.

Item 1 — `total()` / `line()` status_line paths me (views_table.rs):
- `line()` defined `views_table.rs:59`, real use `views_table.rs:204`
  (`TableKeyView::status_line` me `format!("table keyed: {}", self.summary.line())`).
- `total()` defined `views_table.rs:51`, real non-test use `crates/gui/src/app.rs:718`
  (`self.table_view.summary.total()` App status line me) + owned tests.
- In-file tests: `keyed_diff_counts_match_modify_and_orphans` (`line()` me "(keyed)"),
  `empty_keys_fall_back_to_positional_with_note` (`total()` arithmetic + "(positional)" +
  `status_line` me note), `empty_tables_suggest_no_key` (`total() == 0`).

Item 2 — `HEX_HUNK_LEN` hunk chunking me (views_hex.rs):
- Defined `views_hex.rs:30`, real use `views_hex.rs:350`
  (`chunk_run` me `(run_len - done).min(HEX_HUNK_LEN)` cap).
- `chunk_run` (`views_hex.rs:339`) ke do live callers: `build_from_slices`
  (`views_hex.rs:206`) aur `from_comparison` (`views_hex.rs:150`, `10c8928` se).
- In-file tests: `hunk_budget_clips_long_runs_honestly` (200B run → 4x64B hunks,
  `left_len == Some(HEX_HUNK_LEN)`), `from_comparison_chunks_overlong_engine_hunks`
  (over-long engine hunk bhi 64B me chunk + honest truncation).

## 2. Per-item (used-where ya allow-kyun, HEAD par verify)

| Item | Natija | File:line |
|---|---|---|
| `selected_hunk` | REAL USE — `status_line` guard me | views_hex.rs:236,304 |
| `chunk_run` | REAL USE — `from_comparison` + `build_from_slices` se call | views_hex.rs:150,206,339 |
| `HEX_HUNK_LEN` | REAL USE — `chunk_run:350` me `min` cap | views_hex.rs:30,350 |
| `is_over_cap` | live — `app.rs:529` + `build_from_slices` | views_hex.rs:231 |
| `build_from_slices` | test-only + `app.rs` tests; `#[allow(dead_code)]` + justification | views_hex.rs:179 |
| `from_label` | test-only round-trip; `#[allow(dead_code)]` + justification | views_hex.rs:79 |
| `to_compare_options` | REAL USE — `to_key_options`/`cells_equal`/`refresh` isko call karte hain | views_table.rs:163 |
| `to_key_options` | live — `app.rs:937` | views_table.rs:174 |
| `suggest_for` | test-only; `#[allow(dead_code)]` + justification | views_table.rs:135 |
| `apply_suggestion` | test-only; `#[allow(dead_code)]` + justification | views_table.rs:143 |
| `cells_equal` | test-driven; `#[allow(dead_code)]` + justification | views_table.rs:188 |
| `refresh` | test-driven; `#[allow(dead_code)]` + justification | views_table.rs:196 |
| `total()` / `line()` | live (slice-1 item 1, upar dekho) | views_table.rs:51,59,204 |

Kuch remove nahi kiya — har item ke paas ya live caller hai ya tests + allow hai.

## 3. Gates (HEAD `77edf6f`, B landed ke baad fresh)

- `cargo test -p autocompare-gui`: **189 passed, 0 failed** (owned: hex 14 + table 10 samet).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: owned files me
  **ZERO dead-code errors**. Bachi 3 errors sirf NOT-YET items, sab `viewmodel.rs` me
  (scope se bahar, chhua nahi):
  - `build_sync_summary` never used — `viewmodel.rs:1171`
  - `build_folder_merge_summary` never used — `viewmodel.rs:1208`
  - `build_audio_summary` never used — `viewmodel.rs:1250`
- `cargo fmt` owned: `rustfmt --edition 2024 --check` dono files par **clean**.

## 4. Limitations (honest)

1. `suggest_for`/`apply_suggestion`/`refresh`/`cells_equal` runtime me test-driven hain —
   file wiring parsed `Table` owned view tak nahi pahunchata.
2. `build_from_slices`/`from_label` sirf unit-test + `app.rs`-test fixture hain.
3. `from_comparison` ka 64B chunking defensive hai — engine contract par assert nahi karta.
4. `viewmodel.rs` ki 3 dead-code errors sibling scope ki hain; unko chhua nahi.
5. Naye tests headless hain (paint `views.rs` me hai jo scope se bahar hai).
6. Slice-1 me koi behavior change nahi — code pehle commits me land tha, ye commit report-only hai.
