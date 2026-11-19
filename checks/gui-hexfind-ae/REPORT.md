# Wave AE-HEXFIND report (Gap #12 hex half: hex find model + count label)

## Scope
- OWNS: `crates/gui/src/views_hex.rs` + `checks/gui-hexfind-ae/**` ONLY.
- KEVL untouched: `views.rs` / `app.rs` / `tree.rs` / `viewmodel.rs` /
  `views_table.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` (grep-proof only).
- Gap #12 ka hex half: byte-slice `hex_find_next` / `hex_find_prev` (offset,
  wrap) pehle se tha — verify-only. Naya delta grid `(row, col)` hit,
  overlapping `hex_find_count`, aur N/M `hex_find_count_label` hai.
- `views.rs` paint wiring is scope se BAHAR (agla wave); naye helpers
  `#[allow(dead_code)]` file precedent ke saath wiring-pending hain.
- Colors FROZEN — koi paint token chhooa nahi (model-only wave).

## Design (apne shabdon me)
- Bahar se dekhe gaye hex find box jaisa vyavhaar: needle bytes badi input me
  kahin bhi ho sakte hain, aage/peeche le jaane par search ghoomkar shuru se
  milta hai, aur badge "kul me se kaunsa" batata hai.
- Is wave me offset layer ke upar grid layer rakha: `offset = row * 16 + col`
  (`HEX_BYTES_PER_ROW` reuse, koi naya magic number nahi). `next` from-inclusive
  hai, `prev` from-exclusive (pehle se selected hit par dobara rukne ke bajaye
  peeche jata hai) — dono andar purane offset search reuse karte hain.
- Count overlapping hai (ek-ek byte khisakkar) taaki badge koi hit na chhupaye;
  khali needle/haystack par count `0`, label `"0/0"`.
- Label shape text-side badge jaisa hai (`"2/5"`, `None` par `"0/5"`) taaki agle
  paint wave me dono tab ek jaisi badge dikhayen.

## Evidence
- Baseline (pre-edit): `round-1/baseline-tests.out` — 317 passed, 0 failed.
- After: `cargo test -p autocompare-gui` — 319 passed, 0 failed
  (+2 naye: `hex_find_positions_report_row_col_with_wrap`,
  `hex_find_count_and_label_formats_n_of_m`; purana
  `hex_find_wraps_forward_and_backward` same values par pass).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — exit 0.
- `rustfmt --edition 2024 --check crates/gui/src/views_hex.rs` — exit 0
  (`cargo fmt` sirf OWNS file par chalaya).
- Banned grep (non-test): ZERO-BANNED (`unwrap()`/`expect()`/`panic!`/`todo!`
  nahi; `forbid(unsafe_code)` kept; `.get()` + saturating only).
- Harness: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-hexfind-ae/round-1`
  — `RESULT: PASS`; `after.png` 9624 bytes, magic `89504e470d0a1a0a`,
  distinct 256 (model-only wave — PNG bit-same rehna expected tha).
- OWNS-CLEAN, KEVL-CLEAN, NO-COLOR-TYPO-ADD (GATES G2).

## Files changed
- `crates/gui/src/views_hex.rs` (+148): `hex_offset_to_row_col`,
  `hex_row_col_to_offset`, `hex_find_next_position`, `hex_find_prev_position`,
  `hex_find_count`, `hex_find_count_label` + 2 tests.
- `checks/gui-hexfind-ae/GATES.md`, `REPORT.md`, `round-1/` (artifacts).

## Open (agla wave)
- `views.rs` me hex find box paint + N/M badge wiring (is wave ke helpers use honge).
- `app.rs` hex tab state me cursor/badge advance (sibling wave coordination).
