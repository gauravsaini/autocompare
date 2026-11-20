# CHROME9 REPORT — toolbar plus tabs (round-1)

## Kya badla (sirf `crates/gui/src/app.rs`, views FROZEN)

1. **Mode tabs ScrollArea me (width/positioning):** `paint_tabs` ke 10 fixed buttons ab
   `ScrollArea::horizontal` (`chrome_mode_tabs`) me hain — tang window par sikudte nahi,
   scroll karte hain. Session strip jaisa behavior. Gate `chrome_mode_tabs_need_hscroll`
   existing `LAYOUT_NARROW_BREAKPOINT` (900) reuse karta hai.
2. **View toggles left-docked (alignment/padding):** toggles row ka leading
   `group_gap + separator + group_gap` (16px + line) hata diya. Word Wrap / Line Numbers /
   Syntax Highlighting ab Swap/Reload ke left edge se shuru hote hain — BC dense
   2-row toolbar jaisa. Item gap `toolbar_item_gap` (TIGHT_GAP 4) same hai.
3. **Toolbar h-scroll gate (width):** `chrome_toolbar_needs_hscroll` sirf
   `views::action_bar_needs_hscroll` ka reuse hai — row 1/row 2 apne 200px floor
   par rehte hain. Koi naya token/size/color nahi.

## Geometry / typo / colors

- Geometry: gaps/margins sirf existing tokens (`TIGHT_GAP`, `SECTION_GAP`,
  `TAB_BOTTOM_GAP`, `OUTER_MARGIN`); naya size zero.
- Typo: koi naya `.size`/weight/role nahi — `TypoRole` reuse hi raha
  (report: TypoRole x759, `.size ['14.0']` unchanged).
- Colors: frozen — `Color32/Visuals` touches 33, baseline (chrome6) jaisa, growth zero.

## Headless proof

- `chrome9_mode_tabs_hscroll_uses_narrow_breakpoint`: 700/899 true, 900/1280 false.
- `chrome9_toolbar_hscroll_mirrors_action_bar_floor`: 180/700 mirror, NaN/0 false.
- Edge fixture: `comparison/narrow-700.json` (700px narrow path).

## Harness

- `tools/exquisite/run.sh checks/gui-exq-chrome9/round-1` → **RESULT: PASS**
- PNG: 10401 bytes, magic ok, distinct 256 (non-blank).
