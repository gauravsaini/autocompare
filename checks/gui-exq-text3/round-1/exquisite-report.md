# Exquisite report (round-1)
- PNG: 10401 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x435, Grid x16, row-h x184, width calls x70, gutter x1085
- TYPO: TypoRole x896, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
- Wave TEXT3 round-3 (views.rs + gutter.rs): center gutter two-arrow rows now carry a 6px leading inset (36px content in 48px column) so hunk markers read centered in wrap-on and wrap-off paint; wrap-marker alignment locked headlessly (marker at number-field right edge, same width as blank gutter, trail pad preserved); trailing blanks stay even at 960 rhythm (448/448 panes, no orphan blank). No color touches.
- Fixture: checks/gui-exq-text3/comparison/text-wide-960.json (960px, 48px gutter, 448/448 panes, wrap 80, heads true,false,false,true, trailing 436/17.6/9->16 even) proved by views::tests::text3_wide_960_center_wrap_trailing + gutter center-row-marker inset test.
- Gates: cargo test -p autocompare-gui --lib 909 passed, clippy --lib -D warnings 0, banned 0 new, harness PASS.
