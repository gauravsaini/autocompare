# HEX9 Report — narrow 720px plus ruler (round-1)

Wave: exq-hex9 leaf. Geometry FIRST, typography SECOND, colors FROZEN at 33 touches.

## Kya mila (gap, apne shabdon me)

- Beyond Compare hex compare parity gap: narrow 720px canvas par ruler ticks aur byte-cell rhythm loose the — ruler tick x positions for cols 0..16, byte cell geometry from TypoRole Mono (text_w 14.0, box_w 30.0, row_h 15.6), aur narrow height (720 - 130 = 590px, 37 visible rows) ka dedicated constant aur sync helper lock nahi tha.
- Narrow-720 hex-side edge fixture missing tha to prove tick xs match grid byte cols and visible row count locks at 37.

## Kya badla (sirf OWNS files)

`crates/gui/src/views_hex.rs`:
1. **Narrow 720px canvas constants**:
   - `HEX_NARROW_CANVAS_W`: 720.0px.
   - `HEX_NARROW_CANVAS_H`: 720.0px.
   - `HEX_NARROW_PANES_H`: 590.0px (`720.0 - 130.0` chrome reserve).
   - `hex_narrow_720_visible_rows()`: yields exactly 37 rows (`590.0 / 15.6 = 37.82 -> 37`).
2. **Narrow ruler tick alignment and xs at 720**:
   - `hex_narrow_ruler_tick_xs(char_w)`: returns tick positions [0.0, 21.0, 42.0, 63.0, 84.0, 105.0, 126.0, 147.0, 175.0, 196.0, 217.0, 238.0, 259.0, 280.0, 301.0, 322.0] matching `hex_grid_byte_col_x` for all cols 0..16.
   - `hex_narrow_ruler_col_align_ok(char_w)`: proves tick alignment and SideBySide mode at 720.
3. **Byte cell size from TypoRole Mono metrics**:
   - text_w 14.0, box_w 30.0 at 7px char, row_h 15.6px locked to TypoRole::Mono line height.
4. **Sync helper**:
   - `hex_narrow_ruler_sync_ok(char_w)`: Combines narrow ruler tick alignment, byte cell size, narrow 720px visible rows (37), and offset plus gap sync.
5. **Headless test**:
   - `hex_exq_hex9_narrow_plus_ruler`: Proves edge fixture `hex-narrow-720.json`, ruler tick xs [0..322], alignment across scales, byte cell metrics, narrow canvas row count (37), and Mono typo roles.
6. **Zero banned constructs**:
   - `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!` non-test code me 0. No slice indexing in non-test code (`.get()` used).
7. **Colors FROZEN**:
   - `Color32::from_rgb` / `Visuals::` count 33 unchanged.

## Edge fixture

`checks/gui-exq-hex9/comparison/hex-narrow-720.json` aur `checks/gui-exq-hex9/round-1/comparison/hex-narrow-720.json`:
- `canvas_w`: 720.0
- `canvas_h`: 720.0
- `panes_h`: 590.0
- `visible_rows`: 37
- `row_h`: 15.6
- `byte_cell_text_w`: 14.0
- `byte_cell_box_w`: 30.0
- `byte_col_step_px`: 21.0
- `offset_addr_chars`: 10
- `offset_hex_digits`: 8
- `offset_widest_label`: "0xffffffff"
- `grid_avail_w`: 668.0
- `ruler_tick_xs`: [0.0, 21.0, 42.0, 63.0, 84.0, 105.0, 126.0, 147.0, 175.0, 196.0, 217.0, 238.0, 259.0, 280.0, 301.0, 322.0]

## Natija

- Headless test `hex_exq_hex9_narrow_plus_ruler` PASS.
- Full lib test suite passes.
- Exquisite harness `tools/exquisite/run.sh checks/gui-exq-hex9/round-1 crates/gui/src`: RESULT: PASS.
- Colors frozen at 33 touches.
