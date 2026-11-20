# HEX8 Report — ruler alignment plus tall 1200px (round-1)

Wave: exq-hex8 leaf. Geometry FIRST, typography SECOND, colors FROZEN at 33 touches.

## Kya mila (gap, apne shabdon me)

- Beyond Compare hex compare parity gap: offset column width aur ASCII gap round-1 me fixed the, par ruler ticks aur byte-cell rhythm narrow aur varied scales par drift kar sakte the.
- Tall-file view lock missing tha: 1200px tall canvas height par panes height (1200 - 130 = 1070px) me kitni rows visible hongi (`1070 / 15.6 = 68`), uska dedicated constant aur helper lock nahi tha.

## Kya badla (sirf OWNS files)

`crates/gui/src/views_hex.rs`:
1. **Ruler tick alignment to byte columns**:
   - `hex_ruler_col_align_ok(char_w)`: Proves `hex_ruler_col_x(col, char_w)` aur `hex_grid_byte_col_x(col, char_w)` pixel-for-pixel match karte hain for all columns 0..15 across any character scale.
   - `hex_ruler_tick_step_w(char_w) == hex_byte_col_step_w(char_w)` locks ruler tick step to byte cell step so narrow and wide widths don't drift.
2. **Byte cell size from TypoRole Mono metrics**:
   - `hex_byte_cell_size_ok(char_w)`: Locks text_w to 14.0px (`2 * char_w`), box_w to 30.0px (`text_w + 2 * HEX_CELL_PAD_X`), and row_h to 15.6px from `TypoRole::Mono.line_height_px()`.
3. **Tall-file fixture lock at 1200px**:
   - `HEX_TALL_CANVAS_H`: 1200.0px.
   - `HEX_TALL_PANES_H`: 1070.0px (`1200.0 - 130.0` chrome reserve).
   - `hex_tall_1200_visible_rows()`: yields exactly 68 rows (`1070.0 / 15.6 = 68.589 -> 68`).
4. **Sync helper**:
   - `hex_ruler_rhythm_sync_ok(char_w)`: Combines ruler col alignment, byte cell size, tall 1200px visible rows, and offset plus gap sync.
5. **Headless test**:
   - `hex_exq_hex8_ruler_plus_tall_1200`: Proves edge fixture `hex-tall-1200.json`, ruler alignment across scales [5.0, 6.0, 7.0, 8.0, 10.0, 12.0], byte cell metrics, tall canvas row count (68), and Mono typo roles.
6. **Zero banned constructs**:
   - `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!` non-test code me 0.
   - Also fixed single test `.unwrap()` at line 5210 to `.unwrap_or(0.0)`.
7. **Colors FROZEN**:
   - `Color32::from_rgb` / `Visuals::` count 33 unchanged.

## Edge fixture

`checks/gui-exq-hex8/comparison/hex-tall-1200.json` aur `checks/gui-exq-hex8/round-1/comparison/hex-tall-1200.json`:
- `canvas_w`: 960.0
- `canvas_h`: 1200.0
- `panes_h`: 1070.0
- `visible_rows`: 68
- `row_h`: 15.6
- `byte_cell_text_w`: 14.0
- `byte_cell_box_w`: 30.0
- `byte_col_step_px`: 21.0
- `offset_addr_chars`: 10
- `offset_hex_digits`: 8
- `offset_widest_label`: "0xffffffff"

## Natija

- Headless test `hex_exq_hex8_ruler_plus_tall_1200` PASS.
- Full lib test suite passes.
- Exquisite harness `tools/exquisite/run.sh checks/gui-exq-hex8/round-1 crates/gui/src`: RESULT: PASS.
- Colors frozen at 33 touches.
