# GEO Report — unify geometry tokens

Scope: `crates/gui/src/app.rs`, `crates/gui/src/views.rs`. Colors untouched.

## Tokens (before → after, px)

| Token | Before | After |
|---|---|---|
| Outer margin | implicit egui default (Frame central 8) | 12 via `OUTER_MARGIN`, `central_panel.inner_margin(Margin::same(12))` |
| Section gap | scattered `add_space(8.0)` x2 | 8 via `SECTION_GAP` |
| Tight gap | scattered `add_space(4.0)` x8 | 4 via `TIGHT_GAP` |
| Footer gap | `add_space(6.0)` before separator only | 6 via `FOOTER_GAP` after separator |
| Tab bottom gap | `add_space(2.0)` x2 | 2 via `TAB_BOTTOM_GAP` |
| Tree indent | `(depth as f32)*14.0` x2 | 14 via `TREE_INDENT` + `tree_indent(depth)` |
| Home cards | `set_width(260) set_height(75)` hard-coded, `vertical` left | `CARD_W/H` 260x75, `vertical_centered`, Grid spacing 12 |
| Path halves | `(avail/2).max(180)-75` inline | `half_field_width(avail)` |
| Path thirds | `(avail/3).max(130)-75` inline | `third_field_width(avail)` |
| Row height | `ui.text_style_height(Monospace)` x5 inline | `text_row_height(ui)` shared helper |
| Toolbar | 2 rows (actions / toggles) | 1 aligned row + separator |
| Footer | `add_space(6)+separator+label` | `add_space(SECTION_GAP)+separator+add_space(FOOTER_GAP)+label` |
| `add_space` raw values | `['2.0','4.0','6.0','8.0']` | `[]` (all via constants) |
| `Grid::new` | 1 | 1 |
| Width calls | 6 | 6 (via helpers, no new calls) |
| `text_style_height` | x5 sites | 1 def site (`text_row_height`) |
| Color touches | 17 | 17 (zero delta) |

## Screenshots

- Harness PNG `checks/gui-geo/after.png`: 9914 bytes → 9914 bytes (fixture from `screenshot.rs` read-only, unchanged shape; magic ok, distinct 256, non-blank).
- Geometry proven by code grep + headless unit tests, not pixels (fixture is static raster).

## Verification

- `tools/gui-polish/run-geo-typo.sh checks/gui-geo` → RESULT: PASS
- `cargo test -p autocompare-gui` → 115 passed
- `cargo clippy` owned files (`app.rs`/`views.rs`) → zero hits; full-workspace clippy still flags `theme.rs` assertions (typo-agent OWNS, not touched here)
- `cargo fmt --check` → clean
- New headless tests: `geometry_tokens_match_targets`, `half_fields_share_equal_width`, `third_fields_share_equal_width`, `tree_indent_scales_with_depth`

## Helpers added (`views.rs`, headless-tested)

`OUTER_MARGIN`, `SECTION_GAP`, `TIGHT_GAP`, `FOOTER_GAP`, `TAB_BOTTOM_GAP`,
`TREE_INDENT`, `CARD_W/H`, `half_field_width`, `third_field_width`,
`tree_indent`, `text_row_height`.
