# VIEWFILL7 REPORT — viewport fill plus rhythm (Hinglish-Roman)

Round: `checks/gui-exq-viewfill7/round-1` — harness 1 round (max6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

BC me text compare ka pane viewport ko poora bharta hai — chhoti file (3 line) ke neeche bhi khaali kala hissa nahi rehta, wahan halki hatched blank rows dikhti hain dono panes me aakhri line se neeche tak. Selected row ki patti dono panes ke saath beech ki tang gutter par bhi continuous lagti hai, beech me kati nahi. Ours me trailing blanks to the (wrap-on/wrap-off dono me `panes_h` tak), par selected row par center gutter blank rehta tha — band wahin toot-ta tha. Saans (row rhythm) aur hatch geometry pehle se thi, band missing tha.

## Kya kiya (geometry FIRST, typo SECOND, colors FROZEN)

Geometry first (koi naya size nahi, sab reuse):
- Center-gutter selection band: naya pure helper `text_center_gutter_band_fill(selected)` — selected par true (head/blank/continuation sab), bina selection false. Paint 4 jagah wahi gate lagata hai (wrap-on head band + wrap-on head-blank + wrap-on continuation + wrap-off head band + wrap-off blank): blank gutter ka allocate rect aur head arrows ke peeche full `row_height` band rect existing `CHANGED_BG` se bharta hai (selection ka purana rang, koi naya hue nahi). Trailing blanks kabhi selected nahi, wahan band nahi — viewport fill saaf rehta hai.
- Viewport fill lock: trailing blanks pehle se `text_trailing_blank_rows(panes_h, row_height, visible)` se `ceil(panes_h/row_h)` slots tak (wrap-on flat visuals par, wrap-off rows par) dono sides hatch samet — is wave me geometry wahi rakhi, sirf band joda taaki fill + selection ek saath BC jaisa lage.
- ROW_PAD breathing: band rects `row_height` (measured line + `ROW_PAD` 2.0) par hain — head arrows wali gutter bhi poori row height ka band leti hai, chhoti button-patti par saans nahi toot-ti.
- Filler hatch continuity: side panes ka hatch wahi `paint_filler_hatch` (step `TIGHT_GAP` 4.0, tick aadhi row height) — trailing blanks dono sides par wahi fn, isliye ticks ka step continuity bana rehta hai; center gutter me kabhi hatch nahi (wahan sirf band ya blank click).

Typo second (koi naya size/weight nahi):
- Koi font change nahi — band sirf background rect hai, `TypoRole`/Mono rhythm untouched; gutter labels aur copy arrows ka size wahi.

Colors frozen: koi naya Color32/Visuals nahi — band `CHANGED_BG` reuse (selection ka existing rang), hatch `dim` reuse; palette count badha nahi.

## Edge fixture (short-file fill + band)

`comparison/edge-viewfill7.json`: 600px viewport, 16px rhythm, 3 visible rows -> 35 trailing blanks (`ceil(600/16)=38-3`); band selected par true, unselected par false; gutter 48.0 (`28+12+4+4`); hatch step 4.0 par 48px me 12 ticks; `row_height_for_line(14.0)=16.0` (ROW_PAD 2.0 saans). Naya headless test `text_center_gutter_band_fills_on_selection_only` yehi gate lock karta hai; purane tests (`row_height_adds_fixed_pad`, trailing/filler/hatch) rhythm pehle se lock rakhte hain.

## Natija

- PNG 10401B, magic ok, distinct 256 (blank nahi) — `round-1/after.png`.
- 847 tests green (naya band test samet), clippy 0, fmt clean (scoped + owned-file `rustfmt --edition 2024 --check`), banned 0 (owned scope).
- Harness `RESULT: PASS` (1 round); colors 33 — pichhle wave jaisa, growth nahi.
