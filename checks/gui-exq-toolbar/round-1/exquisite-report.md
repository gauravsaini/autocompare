# Exquisite report (round-1)
- PNG: 10363 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x237, Grid x14, row-h x90, width calls x67, gutter x749
- TYPO: TypoRole x653, monospace x26, strong x39, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
- Wave TOOLBAR round-1 (views.rs only): two-row session action bar — row 1 actions reuse paint_toolbar_action_buttons (paint_session_toolbar live call-site ab isi par, shape same); row 2 jump cluster (Prev/Next arrows + N/M badge + 1:1 cursor right-dock, text nav-row precedent, 24 floor + TIGHT_GAP + BADGE_PAD, chrome/Small-weak roles, no color touches).
- Fixture: checks/gui-exq-toolbar/comparison/actionbar-narrow-180.json (180px avail: min 196 -> h-scroll, badge 0/3 + 2/3, blank cursor hidden, 3:1 shown) proved by views::tests::action_bar_* (5 naye).
- Gates: cargo test -p autocompare-gui 778 passed (baseline 773 + 5), clippy -D warnings 0, rustfmt clean, banned 0, harness PASS.
