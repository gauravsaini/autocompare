# HEXTABLE10 REPORT — cells plus grid (Hinglish-Roman)

Round: `checks/gui-exq-hextable10/round-1` — harness `RESULT: PASS` (1 round, max6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

BC hex compare me byte-grid cells gutter overview/minimap aur ruler columns ke saath pixel-precise align hote hain — 16 columns me byte 7 aur 8 ke beech midline space hota hai, narrow view (<720px) me hex aur ASCII stacked visual lines me aate hain bina gutter markers aur row-height desync huye. BC data/table compare me column Grid SECTION_GAP (8px) column gap aur SPACE_SM (8px) cell padding ke saath align hota hai, aur headers strong weight me bina font size badhaye paint hote hain. Ours me exact column offsets, stacked row mapping aur table header strong weight contract locked nahi tha.

## Kya kiya (geometry FIRST, typo SECOND, colors FROZEN)

Geometry first (koi naya rhythm/token nahi, sab reuse):
- Hex byte-grid cells: 16 columns (0..15). Columns 0..7 start at `col * 3 * char_w`, columns 8..15 start at `(col * 3 + 1) * char_w` midline space ke karan (`hex_grid_byte_col_x`).
- Hex byte cell box width: 2 chars text (`hex_grid_byte_cell_text_w`) + do taraf `HEX_CELL_PAD_X` (`SPACE_SM`, 8.0px) reuse via `hex_grid_byte_cell_box_w`.
- ASCII cells: delimiter `|` ke baad character `(1 + col) * char_w` par (`hex_grid_ascii_col_x`).
- Gutter alignment: hex grid origin x starts immediately after chrome reserve (`hex_grid_origin_x` = 52.0px without line numbers). Grid row y aligns at `row * hex_grid_row_h()` (`hex_grid_row_y`).
- Stacked mode: narrow window (<720px) me 2 visual lines per grid row (`hex_stacked_visual_rows`), row height `hex_grid_row_h()` reuse (`hex_stacked_row_y`), fit gate `hex_stacked_panes_fit`.
- Table column Grid: col gap `TABLE_GRID_COL_GAP` == `GEO_SECTION_GAP` (8.0px), cell padding `TABLE_CELL_PAD_X` == `SPACE_SM` (8.0px).
- Table column layout math: `table_column_grid_col_x` computes cumulative x with section gaps; `table_column_grid_header_cell_rect` computes header cell box.

Typo second (koi naya size nahi):
- Table header weight name `"strong"`, weight value 600 (`TYPO_WEIGHT_SEMIBOLD` / `TypoRole::Heading::weight_value`), role `TypoRole::Mono` (12.0pt).
- Hex cells sab `TypoRole::Mono` (`hex_cell_text_role` reuse).

Colors frozen:
- Zero Color32/Visuals modification (report count 33, pichhle wave se unchanged).

## Edge fixture (hex-narrow)

`comparison/hex-narrow.json`: 640px narrow window stacks hex+ASCII (layout Stacked), 32 bytes (2 rows, row 1 diff); chrome reserve 52px (12 gutter + 24 minimap + 2x8 gaps), avail 588px; hex byte-cells col 0..15 aligned with midline gap at col 8 (x=175 at 7px, gap 28px vs 21px); gutter marker on row 1; table column Grid with SECTION_GAP col gap (8px), cell padding (8px), header weight strong via TypoRole (no new sizes). Headless tests `hex_exq_hextable10_edge_narrow_gutter_cells` aur `table_exq_hextable10_column_grid_and_cell_padding` isko prove karte hain.

## Natija

- PNG 10401B, magic ok, distinct 256 (non-blank).
- 886 tests passed, 0 failed.
- Clippy `-D warnings` clean (0 warnings on lib).
- rustfmt clean.
- Banned tokens (unwrap/expect/panic/todo/unimplemented/unsafe) 0.
