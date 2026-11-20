# Exquisite report (round-1)
- PNG: 10363 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x253, Grid x14, row-h x98, width calls x67, gutter x772
- TYPO: TypoRole x674, monospace x26, strong x41, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
- Wave TREE4 round-1 (tree.rs only): grid column xs right-aligned side block (452px: 86+86+128+128+3x8) with left-stack fallback, path avail + truncate decision, guide row-span 24px, gutter prev/next arrows stacked in 16px LG cell (12+12), arrow typo = header-strong reuse. No new sizes, colors 33 unchanged.
- Fixture: checks/gui-exq-tree4/round-1/comparison/edge-deep-nesting-orphan-newer.json (5-deep chain guides 8..56 + orphan-newer dash-aligned) proved by tree::exq_tree4_tests (5 tests).
- Gates: cargo test -p autocompare-gui 798 green, clippy -D warnings 0, cargo fmt --check clean, banned 0 non-test, harness PASS.
