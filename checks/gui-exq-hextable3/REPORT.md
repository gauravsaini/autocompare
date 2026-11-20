# HEXTABLE3 REPORT — hex ascii plus table columns (Hinglish-Roman)

Round: `checks/gui-exq-hextable3/round-1` — harness `RESULT: PASS` (1 round, max6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

BC me hex window do synced panes me dikhta hai — ek taraf hex digits, dusri taraf ASCII — dono ek hi row window par scroll hote hain, kabhi alag nahi hote. Upar ruler offset, neeche Go To (64-bit) + byte edit. Table me key columns se pehle sort hota hai phir align, isliye reordered rows delete+insert nahi bante. Ours me sirf hunk summary tha, panes ka width-split, gutter, row height aur sort-before-align locked nahi tha.

## Kya kiya (geometry FIRST, typo SECOND, colors FROZEN)

Geometry first (koi naya rhythm nahi, sab reuse):
- Hex split lock: `HEX_RULER_CHARS=10`, `HEX_HEX_PANE_CHARS=49`, `HEX_ASCII_PANE_CHARS=16` (80-char budget `HEX_GRID_ROW_CHARS` ka split), beech me `hex_ascii_gutter_w()=SPACE_SM(8.0)`.
- Row height lock: `hex_grid_row_h()` = `TypoRole::Mono.line_height_px()` (15.6) — dono panes + ruler ek hi height par, desync nahi.
- Pane fit gate: `hex_pane_text_widths` / `hex_pane_outer_widths` (`hex_cell_outer_w` reuse) + `hex_panes_fit` — 960px canvas par fit, narrow par stacked (720 breakpoint reuse).
- Go To bar layout: `hex_goto_bar_min_w()` = 120 field + 4 pad + 2x8 gaps = 140 — canvas me clip nahi.
- Table sort-before-align: `table_sort_key_for_row` (case/trim fold, ragged `-` safe) + `table_sorted_row_indices` (stable, khali keys par identity) + `table_sort_rows_by_key`; grid fit `table_column_grid_fit` (`table_grid_total_w` reuse).

Typo second (koi naya size nahi):
- Row/pane/ruler sab Mono (`hex_row_text_role` = sync-pane reuse); labels Body-strong, badge Small — purani hierarchy wahi.
- Table me koi font change nahi (index math me font nahi hota).

Colors frozen: koi Color32/Visuals touch nahi (report me 33 hi hai, pichhle wave jaisa).

## Edge fixture (empty-vs-nonempty + 64bit)

`comparison/empty-vs-nonempty-64bit.json`: left empty 0 rows vs right 16B 1 row; `0xFFFFFFFFFFFFFFFF` + decimal MAX parse `Some(MAX)`, anchor row 1152921504606846975 col 15 (bina wrap), clamp MAX-at-MAX `None`, MAX-1 `Some`; pane 7px par 343/112 text, 359/128 outer, gutter 8, row 15.6, goto min 140; table `[2,10,1]` -> `[2,1,0]` (string sort), ragged dash, grid fit wide-true narrow-false. Dono naye headless tests yehi prove karte hain.

## Natija

- PNG 10401B, magic ok, distinct 256 (blank nahi).
- Geometry signals up (margin/gap 273, row-h 110, gutter 805) — sirf owned files se.
- 816 tests green, clippy 0, fmt clean, banned 0.
