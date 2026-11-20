# REPORT — SWARM-EXQ-L (exquisite hex/table parity, round-1)

## Kya kiya (round-1)

1. **views_hex me 64-bit goto + byte-edit headless judvaan**: `parse_hex_goto` (0x hex / decimal, `_ , space tab` separators, u64 full range, invalid/overflow par `None`), `clamp_hex_goto` (total_len gate), `hex_nibble_value` / `apply_hex_nibble` / `edit_byte_at` (`ByteEditBuffer::set_byte` semantics bina history ke, `.get_mut()` + `try_from` only), `paint_hex_goto_bar` (find-bar geometry mirror: wahi bar height/field chaudai/badge pad; label Body-strong, value Mono, badge Small).
2. **views_table me xlsx/tsv source + sheet selector**: `TableSourceKind` (`csv`/`tsv`/`tab`/`xlsx` extension se, engine `detect_table_delimiter`/`Workbook::from_xlsx` ka GUI-side judvaan — parse yahan nahi), `table_delimiter_for_source`, `table_source_badge_label`, `table_sheet_clamp`/`table_sheet_label` (OOB par `None`), `paint_table_sheet_bar` (key-bar mirror: wahi height/gap/pad, marker `key_column_marker` reuse, label Body-strong, names Mono+strong-selected, badge Small).
3. **viewmodel me zone-role helpers (typography second)**: `hex_goto_zone_*`, `table_key_zone_*`, `table_sheet_zone_*`, `zone_badge_role` + `zone_weight`/`zone_family`/`zone_line_height_px`/`zone_letter_spacing_em`/`zone_is_tabular` aliases — sab existing `TypoRole` reuse, koi naya size/token/color nahi. Hierarchy lock: label(13) > value(12-mono-tabular) > badge(11).
4. **Tests + harness**: 4 naye tests (`hex_goto_parses_*`, `hex_goto_clamps_*`, `table_source_kind_*`, `exq_l_zone_roles_*`); `cargo test` 678 -> 682 green; clippy zero; fmt clean; `run-geo-typo.sh checks/gui-exq-l/round-1` RESULT PASS (PNG 9765 B, Grid x14, row_h x4, touches 29 same).

## Ponytail (shortest diff)

- Reuse only: find-bar/key-bar consts, `key_column_marker`, `TypoRole`, `GUTTER_*` rhythm; koi naya dep/token/color nahi.
- Wiring-pending: paint fns `#[allow(dead_code)]`, App/viewer struct me zero field edit; caller locals par chalte hain.
- MS rust: `forbid(unsafe_code)` barkarar, non-test me `.get()`/saturating/`try_from` only, banned/colors-grep zero.

## Agla step (agar round-2 chahiye)

- Orchestrator jab hex/table tab caller wire kare tab `paint_hex_goto_bar` / `paint_table_sheet_bar` ko existing find/key bars ke saath single toolbar me jodna (duplicate bar nahi), phir screenshot me goto/sheet rows photo-proof karna.
