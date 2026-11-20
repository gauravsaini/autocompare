# HEXDENSE8 Report — cells plus gutter (round-1)

Wave: hex/table panes dense karo, cell padding even karo, gutter markers majboot karo. Geometry FIRST, typography SECOND, colors FROZEN.

## Kya mila (gap, apne shabdon me)

- Key/sheet bar me har column ke baad extra `add_space` tha + `item_spacing` bhi tha — matlab double gap (16px) jabki data Grid ka col gap 8px hai. Bar vs grid rhythm uneven.
- Find/goto action rows me koi item spacing set nahi tha (hex) ya 8px tha (table) — jabki live hunk-nav row 4px tight gap use karti hai. Action rows sparse.
- Hex gutter bar line-numbers-off par 6px patla ho jata tha, jabki offset ruler hamesha visible hai — patli strip me markers weak aur click mushkil.

## Kya badla (sirf OWNS files)

`crates/gui/src/views_table.rs`:
1. `paint_table_key_bar` — per-column trailing `add_space` hataya; ek hi gap source (`item_spacing.x = TABLE_KEY_COL_GAP` 8px). Bar rhythm ab Grid col gap jaisa even.
2. `paint_table_sheet_bar` — wahi mirror fix (key-bar parity barkarar).
3. `paint_table_find_controls` — gap 8px se `GEO_TIGHT_GAP` (4px) par; hunk-nav precedent reuse. Find bar + toolbar dono isi single source se chalte hain.

`crates/gui/src/views_hex.rs`:
4. `paint_hex_find_bar` / `paint_hex_goto_bar` — `item_spacing.x = GEO_TIGHT_GAP` (4px) set kiya; needle/buttons/badge compact, ek hi gap source.
5. `hex_gutter_bar_width` — ab hamesha wide bar (12.0) deta hai, flag chahe jo ho; ruler hamesha visible hai isliye narrow strip ka koi matlab nahi. Single source `gutter_bar_width(true)` — text wide-bar se drift nahi.

Koi naya size/token/color nahi — widths `HEX_*`/`TABLE_*` precedent reuse, sizes sirf `TypoRole` tokens se, `Color32/Visuals` ko haath nahi lagaya (harness count 33, pehle jitna hi).

## Edge fixture

`round-1/comparison/hexdense8-narrow-gutter.json` — 600px narrow window (< 720 breakpoint): layout Stacked, chrome reserve 52px (12 + 24 + 2x8), grid avail 548px, 7px char par panes fit, marker rows [1,3], y=100 click se hunk 0. Test `hexdense8_narrow_stacked_gutter_markers_stay_visible` isko inline mirror karke prove karta hai (marker rows, jump target, Mono/Body role parity).

Table side `tabledens8_bar_rhythm_stays_even_and_tight` — key/sheet gap == Grid col gap (even), action gap == row gap 4px (tight), roles Body/Mono (typo parity) lock karta hai.

## Natija

- Harness `round-1`: RESULT PASS (PNG 10401 bytes, magic ok, non-blank).
- Full suite: 858 passed, 0 failed. Clippy `-D warnings`: 0. rustfmt: clean. Banned (unwrap/expect/panic/todo, non-test): 0.
- Ek beech ka test failure aaya tha (naya test, y=150 galat row par tha — 75px rows par y=150 row 2 hai). Root cause test-side tha, source sahi tha; sabse chhota fix y=100 + fixture key update. Dobara single test green, phir full suite green.
