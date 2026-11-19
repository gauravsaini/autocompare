# Parity Hex+Table — Wave B Report (Agent B)

Scope: sirf headless pure models, koi app wiring nahi. Owned files:
`crates/gui/src/views_hex.rs`, `crates/gui/src/views_table.rs`.
`app.rs` / `views.rs` / `theme.rs` / `viewmodel.rs` ko chhua tak nahi.

## 1. Per-target counts

| Target | Tests | Status |
|---|---|---|
| `views_hex.rs` unit tests (`#[cfg(test)]`) | 12 | PASS (scratch crate me, repo wiring orchestrator ke paas) |
| `views_table.rs` unit tests (`#[cfg(test)]`) | 10 | PASS (same) |
| Scratch total (`/tmp/parity-check`, path-deps same engine) | 22 passed, 0 failed | PASS |
| `cargo test -p autocompare-gui` (repo, wiring se pehle) | 125 passed (122 baseline + parallel waves ke 3, unchanged by me) | PASS |
| `cargo clippy` scratch `--all-targets -- -D warnings` + workspace lint set (`unwrap_used`, `indexing_slicing`, …) | 0 warnings | PASS |
| `rustfmt --edition 2024 --check` owned files | clean | PASS |

Note: naye modules abhi `main.rs` me declare nahi (orchestrator ka kaam,
INTEGRATION.patch dekho), isliye repo ka count mere se badla nahi — koi
regression nahi. Wiring ke baad count 125 + 22 = 147 hona chahiye.

## 2. Coverage (naye models)

`views_hex.rs` — `HexCompareMode` (None/Fast/Complete ↔ `HexAlignment`
1:1 map, label round-trip), `HexCompareView` (empty / from_comparison /
build_from_slices, 16 MiB honest cap, 64-byte hunk chunking, truncated
flag, next/prev wrap selection, navigate_to_offset, status_line).
`views_table.rs` — `TableKeyView` (detect_key_columns preview, selector
toggle sorted, tolerance/case/trim options, positional fallback +
POSITIONAL_NOTE, to_compare_options/to_key_options mapping),
`TableKeySummary` (counts + line()).

## 3. 6 limitations (honest)

1. `build_from_slices` sirf headless test helper hai; asli file I/O +
   background thread driving orchestrator wiring me hoga (`app.rs:716`
   `run_hex` ke aas-paas).
2. Hunk views me sirf `left_len`/`right_len` hain, bytes nahi — paint ko
   hex dump bytes ke liye engine `BinaryHunk` dobara dekhna padega.
3. 16 MiB cap par compare skip hota hai; cap ke upar ke files ke liye
   unbounded positional fallback is model me nahi (engine ke
   `compare_binary_files` ka rasta wiring me chuna ja sakta hai).
4. Key matching me numeric tolerance kabhi apply nahi hota (engine
   semantics: keys `"1"` vs `"1.0"` alag keys) — yeh jaan-boojhkar parity hai.
5. `f64` ke through numbers compare hote hain, isliye 2^53 se bade
   integers conflate ho sakte hain (engine ki documented limitation).
6. Key selector me max-column cap nahi; composite detection engine ki
   tarah single + pair tak hi suggest karta hai (3+ column keys manual).

## 4. Integration (orchestrator ke liye)

`checks/parity-hex-table/INTEGRATION.patch` me `main.rs` ke liye unified
diff hai (sirf 2 `mod` lines). Baaki wiring points (direct edit nahi ki):

- `crates/gui/src/main.rs:14-23` — mod declarations lagao (patch).
- `crates/gui/src/app.rs:72,74` — `hex_model`/`table_model` ke saath
  `hex_view: views_hex::HexCompareView` / `table_view:
  views_table::TableKeyView` state rakho.
- `app.rs:716-731` (`run_hex`) — `compare_binary_files_aligned` + mode
  `hex_view.mode.to_alignment()` chalao, `HexCompareView::from_comparison`
  se model banao.
- `app.rs:773-792` (`run_table`) — `compare_table_files_by_key` +
  `table_view.to_key_options()` chalao, `table_view.refresh` se summary.
- Paint: `app.rs:1420-1428` (Hex) aur `app.rs:1440-1448` (Table) me
  mode selector + status_line (`views_hex.rs: status_line`,
  `views_table.rs: status_line`) jodo.
- Status counts: `app.rs:559-562` (hex), `app.rs:571-574` (table).
