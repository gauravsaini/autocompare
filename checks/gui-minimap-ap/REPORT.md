# REPORT — Wave AP-MINIMAP (Gap #5 minimap-click)

## Summary

Text overview ruler aur hex minimap dono par click-to-jump ab poora hai:
click-target geometry (ruler rect + row mapping) + bring-into-view +
accessibility label. Saara kaam OWNS paths me hai —
`crates/gui/src/views.rs` + `checks/gui-minimap-ap/**`; KEVL files
untouched (`gutter_*` sirf read-only call).

## Kya badla (`crates/gui/src/views.rs` only)

- Naye pure helpers (geometry first, koi naya token/color nahi):
  - `text_minimap_ruler_width()` — `text_gutter_paint_width` ka alias
    (12.0 numbers ke saath / 6.0 bina); ruler rect isi se banta hai.
  - `text_minimap_row_at_y(y, height, total)` — fraction mapping
    `y / height -> [0, total)`, kharab input par `None`
    (`gutter_row_at_y` semantics, arg order hex jaisa).
  - `text_gutter_click_select()` ab isi helper se row nikalta hai, phir
    wahi nearest-diff snap (diff par stay, equal par aage/peeche diff).
  - `text_minimap_accessibility_label()` / `hex_minimap_accessibility_label()` —
    empty / no-selection / `row N of M` (`hunk N of M`) honest labels.
- Paint wiring (typography second — sirf hover text, koi naya size nahi):
  - Text ruler `resp.on_hover_text(text_minimap_accessibility_label(...))`
    + `debug_assert_eq!(rect.width(), bar_w)`.
  - Hex minimap `resp.on_hover_text(hex_minimap_accessibility_label(...))`
    + `debug_assert_eq!(rect.width(), HEX_MINIMAP_W)`.
  - Bring-into-view paths untouched (text: pending-scroll + Center;
    hex: `pager.bring_into_view`).
- Colors frozen: wahi `track_for`, `DELETED/INSERTED/CHANGED_BG`, text tone.

## Tests

- Naye unit tests: `text_minimap_ruler_geometry_and_row_mapping`,
  `minimap_click_labels_stay_honest`.
- `cargo test -p autocompare-gui`: 440 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero warnings.
- `cargo fmt -- --check crates/gui/src/views.rs`: clean.
- Banned-token grep (non-test): `unwrap/expect/panic/todo/unimplemented/
  unreachable/unsafe` — clean (`forbid(unsafe_code)` intact).

## Harness

- `tools/gui-polish/run-geo-typo.sh checks/gui-minimap-ap/round-1`:
  exit 0, `RESULT: PASS` (PNG 9624 bytes, magic ok, distinct 256).

## DoD

- [x] Minimap click sahi row/hunk par jump karta hai
- [x] Unit tests + full gui suite green
- [x] Harness PASS
- [x] Commit (no push)
