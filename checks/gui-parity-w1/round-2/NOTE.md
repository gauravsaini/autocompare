# Round 2 — Home flex cards + CTA parity (2026-11-20)

## Study
- `capture/opencode-study/round-1-session-types.md`: 9 session types
  (3 folder-family, 2 text-family, 4 specialist) — apna `home_model()`
  9 cards isi order par, 3x3 grid.
- BC launcher pattern: session tiles row bharte hain (fixed tang cards
  nahi). Hamara live `paint_home` fixed `CARD_W` = 240 use karta tha,
  jabki 960-canvas fixture 306px cards dikhata tha — live vs fixture
  mismatch (fixture live se ~27% chauda).

## Build (geometry FIRST, typography/colors untouched)
1. `views::home_paint_card_width(avail)`: flex fill — `(avail - 2*8)/3`,
   floor 240. 936 inner width par 306.67 (fixture 306 truncation parity);
   tang window par floor pakadta hai, grid scroll karta hai.
   `paint_home` ab `set_width(card_w)` — pehle fixed 240 tha.
2. Fixture `HOME_CTA_W` 150 -> 224 = live `home::HOME_CARD_CTA_W`
   (240 - 2*8 pad) parity. Hit area 150*28=4200 -> 224*28=6272
   (contract `home_card_cta_hit_area` = 6272 se exact match).
3. Fixture CTA card me centered (offset (306-224)/2 = 41) — live
   `vertical_centered` parity; pehle left-aligned +12 tha.
4. Naya test `home_fixture_mirrors_live_paint`: fixture consts live
   contract se lock (card flex, CTA, height ceiling 100 >= 99.6).

## Check
- `cargo test -p autocompare-gui`: 701 -> 703 (2 naye: flex floor,
  fixture-live mirror). Sab green.
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings.
- `cargo fmt -p autocompare-gui --check`: clean.
- Screenshot: `/tmp/w1-home.png` 960x600, 14991 bytes.
  Grid: x = 12 + col*314 (306 + 8 gap), 3 cols = 934 + 12 + 12 + 2 slack.
  Card: y = 90 + row*108, h = 100 (99.6 ceiling). CTA 224x28 centered.
  Text scene `/tmp/w1-text.png` 960x600 unchanged shape (regression nahi).

## Bacha hua gap
- BC Home launcher ka exact layout UNVERIFIED (screenshot permission
  blocked, pehle se noted) — hamara 3x3 + recent-list layout apna
  design hai, BC-clone nahi (clean-room rule).
- 720px BC window par hamare 240-floor cards 3-col me overflow karenge
  (736 > 696 avail) — grid scroll hota hai, 2-col fallback nahi hai.
  Agli wave ka candidate.
