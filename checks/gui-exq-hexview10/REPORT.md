# HEXVIEW10 REPORT — hex panes plus ruler (Hinglish-Roman)

Round: `checks/gui-exq-hexview10/round-1` — harness `RESULT: PASS` (1 round, max6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

BC hex compare me do synced panes dikhte hain — ek taraf hex digits, dusri taraf ASCII — dono ek hi row window par scroll hote hain, kabhi alag nahi hote, aur upar offset ruler rehta hai. Ours me sirf hunk summary + sparse `??` grid tha: ruler nahi, scrollable sync panes nahi, narrow window me wrap nahi.

## Kya kiya (geometry FIRST, typo SECOND, colors FROZEN)

Geometry first (koi naya rhythm/token nahi, sab reuse):
- Ruler header: offset (10 chars, body address se barabar) + 16 column labels `00..0F` midline gap samet 49 chars (`HEX_HEX_PANE_CHARS`) + ASCII `|0123456789ABCDEF|` 18 chars — body cells (10/49/18) se char-barabar, isliye alag Grid hone par bhi columns align.
- Sync panes: dono panes ek hi vertical ScrollArea me (ek scroll = dono sync); wide par 3-column Grid (address | hex | ASCII, real bytes `format_hex_grid_cells_with_bytes` se), narrow <720px par stacked (har grid row = hex line + ASCII line, wahi shared scroll, `hex_layout_mode` reuse).
- Virtualized `show_rows` (`hex_grid_row_h` = 15.6 reuse) — lambi file me sirf dikhne wali rows banti hain; panes height `hex_panes_height` reuse, beech me `GEO_TIGHT_GAP`, spacing `[GEO_SECTION_GAP, GEO_TIGHT_GAP]` body precedent.
- Caller signature same (slices + hunks caller-owned, App/viewer struct me zero field edit) — wiring orchestrator ke paas pending.

Typo second (koi naya size nahi):
- Ruler + dono panes + stacked lines sab Mono (`hex_row_text_role` reuse); khali window note Small — label(13) > content(12) > hint(11) hierarchy wahi.

Colors frozen: koi Color32/Visuals/painter touch nahi (report me 33 hi hai, pichhle wave jaisa).

## Edge fixture (narrow stacked + tail)

`comparison/narrow-stacked-tail.json`: 700px narrow -> Stacked, 960 -> SideBySide; 48B + tail push = 49B -> 4 rows `[0,16,32,48]`, stacked visual 8; diff row 1 + tail row 3 markers; 300px me 19 rows; chrome reserve 52, grid avail 648, panes fit; tail row poori 80-char budget me. Naya headless test yehi prove karta hai (ruler shapes test alag).

## Natija

- PNG 10401B, magic ok, distinct 256 (blank nahi).
- Geometry signals up (Grid x16 me naye ruler+sync grids; margin/gap 346, row-h 129, gutter 1036) — sirf owned files se.
- 870 tests green, clippy 0, fmt clean, banned 0.
