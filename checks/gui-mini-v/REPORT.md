# Wave V MINI — REPORT (overview strip width + hits)

## Delta
RUN early-feedback #2: right-edge overview strip reference minimap se
sakar Photim — segment hits chhote lagte the. Fix: hex minimap paint
chauda + segment floor, mapping exact.

## Diff (views.rs only, app.rs/theme.rs read-only)
- `HEX_MINIMAP_W = 2.0 * GUTTER_W` (24.0): naya token, gutter scale
  par hi; paint `allocate_exact_size` isi ko deta hai (hit rect bhi
  double-wide: click/drag target bada).
- `hex_minimap_seg_end(y0, y1) = y1.max(y0 + ROW_PAD)`: sub-2px
  hunk band ek row-pad floor tak badhti hai (visible + clickable);
  badi band untouched.
- `hex_minimap_row_at_y` / `hex_minimap_jump_to_row` bodies untouched
  (mapping exact).
- No color/typo edits (diff me `Color32::from_rgb`/`Visuals::` 0).

## Gates evidence
1. Mapping exact: `hex_minimap_fraction_maps_to_row_and_jumps` ok.
2. Strip wider: `hex_minimap_strip_wider_than_gutter` ok (24 = 2*12).
3. Seg floor: `minimap_seg_floors_at_row_pad` ok.
4. Uniformity: `gutter_widths_stay_uniform` +
   `exquisite_geometry_tokens_reuse_scale` ok.
5. Hygiene: banned-pattern grep (non-test) 0, clippy 0, fmt clean,
   `git diff --stat` sirf views.rs + checks/gui-mini-v/**.
6. Harness: `round-V-1` RESULT: PASS, PNG 9914B magic-ok distinct-256.

## Metrics
- gui tests: 245 -> 249 (+2 mine; +2 sibling theme.rs, disjoint file).
- clippy 0, fmt clean, OWNS clean, commit NO push.
