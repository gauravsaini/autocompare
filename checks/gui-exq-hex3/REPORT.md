# HEX3 REPORT — ruler plus grid (round-1)

## Kya badla (`crates/gui/src/views_hex.rs`, checks/gui-exq-hex3/**)

1. **Ruler tick spacing symmetric via SPACE tokens (geometry):**
   - `hex_ruler_tick_spacing()` reuses `GEO_SECTION_GAP` (`SPACE_SM`, 8.0px).
   - `hex_ruler_tick_sym_pad()` reuses `GEO_TIGHT_GAP` (`SPACE_XS`, 4.0px).
   - `hex_ruler_col_x(col: usize, char_w: f32) -> Option<f32>`: 16 column positions
     align perfectly with `hex_grid_byte_col_x` using the shared midline helper.
2. **Nibble group gap from midline helper (geometry):**
   - `hex_nibble_group_gap_w(char_w: f32) -> f32` single source derived from
     `hex_midline_gap_w(char_w)`.
   - `hex_grid_byte_col_x` updated to shift column 8..15 via `hex_nibble_group_gap_w(char_w)`.
3. **Row height locked to Mono line-height at 960px wide (geometry/typo):**
   - `HEX_CANVAS_W` (960.0px) reference width locks `hex_wide_row_h()` to
     `TypoRole::Mono.line_height_px()` (15.6px).
   - `hex_wide_960_visible_rows()` yields 30 fully visible rows in 470px panes height.
   - Grid availability: 908.0px (`960.0 - 52.0px chrome reserve`).
   - Grid origin: `(52.0px, 19.6px)`.

## Geometry / typo / colors

- Geometry: SPACE tokens (`SPACE_XS`, `SPACE_SM`) reuse, naya magic number zero.
- Typo: `TypoRole::Mono` (12.0 * 1.3 = 15.6px), zero new point sizes.
- Colors: FROZEN — `Color32/Visuals` touches 33 (exact baseline, zero growth).

## Headless proof

- `views_hex::tests::hex_exq_hex3_round3_ruler_ticks_and_grid_wide_960`: pass
- Edge fixture: `checks/gui-exq-hex3/comparison/hex-wide-960.json`

## Harness

- `tools/exquisite/run.sh checks/gui-exq-hex3/round-1 crates/gui/src` → **RESULT: PASS**
- PNG: 10401 bytes (≥ 8192), magic ok, distinct 256.
