# Exquisite report (round-1)
- PNG: 10363 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x235, Grid x14, row-h x90, width calls x67, gutter x749
- TYPO: TypoRole x653, monospace x26, strong x39, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
- Wave TREE2 round-1 (tree/gutter/copy only): indent guides per ancestor level (level*12+8 expander centers) + separated status cell (16px LG + 8px SM gap, status mono body-strong vs path regular) + center-gutter arrows per hunk (24px XL column, 16px LG box at segment midpoints, head-only visibility via copy_hunk_head; continuations blank click-to-select). Legacy flat offsets frozen.
- Fixture: checks/gui-exq-tree2/round-1/comparison/edge-deep-nest.json (depths 0..3 guides/status/path + 2-segment arrow centers + Changed,Changed,Equal,Inserted heads true,false,false,true) proved by tree/gutter/copy exq_tree2_tests.
- Gates: cargo test -p autocompare-gui 773 passed, clippy -D warnings 0, rustfmt owned-files clean, banned 0 non-test, harness PASS.
