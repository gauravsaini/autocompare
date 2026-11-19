# Wave Z-HEX — REPORT (round-1)

Gap #2 (hex byte-grid paint): `views_hex.rs` grid math
(address + 16-byte hex + ASCII, midline gap) ab `views.rs` hex painter
window me teen-column `egui::Grid` me paint hota hai. Monospace alignment,
GEO tokens reuse, colors frozen.

## Before -> After (file:line)

- `crates/gui/src/views.rs:451-476` nayi pure helper `format_hex_grid_cells`
  (addr, hex, ascii): `HEX_BYTES_PER_ROW` / `format_hex_offset` /
  `hunk_covers_offset` reuse, 8 ke baad midline gap extra space. Pure, no egui.
- `crates/gui/src/views.rs:478-488` `format_hex_grid_row` ab cells ka
  single-string join (`"{addr}  {hex} |{ascii}|"`) — shape byte-for-byte same,
  back-compat wrapper (logs/tests); Grid paint cells use karta hai.
- `crates/gui/src/views.rs:748-786` `paint_hex` byte-grid section `for row { ui.label }`
  se `egui::Grid::new("hex_byte_grid").num_columns(3).spacing([SECTION_GAP, TIGHT_GAP])`
  par gaya — har row me 3 monospace cells (addr, hex, `|ascii|`), GEO spacing reuse,
  koi naya token/color nahi.
- `views_hex.rs` me koi change nahi (grid math wahi single-source hai, read-only reuse).

## Tests

- `cargo test -p autocompare-gui`: 278 passed, 0 failed
  (baseline 275 + 1 naya + 2 sibling find-z agent ke; worktree shared hai).
- Naya: `hex_grid_cells_split_address_hex_ascii` (addr = `format_hex_offset`,
  hex me 2 `??`, ascii `"    ??          "`, join == Grid cells, empty row same width).
- Purane same shape/values par pass:
  `hex_grid_row_marks_covered_bytes_with_midline_gap`,
  `hex_grid_rows_for_hunk_align_and_cap`.

## Harness

- `tools/gui-polish/run-geo-typo.sh checks/gui-hex-z/round-1`: RESULT: PASS
  (PNG 9624 bytes, magic ok, distinct 256; Grid x2, mono x43, `.size` sirf ['14.0'] — koi typo change nahi).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero warnings.
- `rustfmt --edition 2024 --check` (OWNS files `views.rs` + `views_hex.rs`): clean.
- Banned grep (non-test `unwrap(`/`expect(`/`panic!`/`todo!`/...): ZERO-BANNED
  (`unwrap_or`/`unwrap_or_default` allowed hain, wahi use hote hain).
- `forbid(unsafe_code)` kept (`views_hex.rs:18`).

## Scope note

- Koi color/typo edit nahi; `app.rs` / `viewmodel.rs` / `theme.rs` / `screenshot.rs` /
  `tree.rs` untouched (worktree me dusre find-z agent ka `viewmodel.rs` + staged
  `checks/gui-find-z/**` dikha — haath nahi lagaya, commit me include nahi).
- `views_hex.rs` OWNS me tha par change ki zaroorat nahi padi (math reuse hi kaafi tha),
  isliye diff sirf `views.rs` + `checks/gui-hex-z/**`.
- `cargo fmt -- --check` workspace-level par sibling files par diff de sakta hai;
  OWNS files project rustfmt se clean hain.
- STATUS.md untouched, koi push nahi (local commit only).
