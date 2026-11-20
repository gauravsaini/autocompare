# Exquisite report (round-1)
- PNG: 9765 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x229, Grid x14, row-h x90, width calls x66, gutter x720
- TYPO: TypoRole x636, monospace x26, strong x39, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
- Wave TEXT3 round-1 (views.rs only): center gutter now heads each contiguous diff block instead of every diff row (fixed 48px column kept, continuations stay blank click-to-select); header meta row and per-pane cursor plus 3-zone status keep HEADER_H + TIGHT_GAP geometry and Mono 12/1.30 with Small/Body chrome roles, no color touches.
- Fixture: checks/gui-exq-text3/comparison/edge-hunk-head.json (Changed,Changed,Equal,Inserted -> heads true,false,false,true) proved by views::tests::text_center_hunk_head_groups_contiguous_diffs.
- Gates: cargo test -p autocompare-gui --lib 761 passed, clippy -D warnings 0, rustfmt clean, banned 0 new, harness PASS.
RESULT: PASS
