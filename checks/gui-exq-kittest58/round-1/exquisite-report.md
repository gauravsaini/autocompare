# Exquisite report (round-1)
- PNG: 10946 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x717, Grid x16, row-h x374, width calls x85, gutter x1979
- TYPO: TypoRole x1146, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 35 (info only, must not grow for geo-typo waves)
RESULT: PASS

## Dual-theme real-egui shots (exq-kittest58 round-1, test-only)
- dark `real-egui.png`: 51926 bytes (min 8192 ok), magic ok, 960x600, distinct-RGB 503 (min 500 ok), base 10, ratio 50 (min 10 ok)
- light `real-egui-light.png`: 52669 bytes (min 8192 ok), magic ok, 960x600, distinct-RGB 549 (min 500 ok), base 10, ratio 54 (min 10 ok)
- same fixtures (`t_left.txt`/`t_right.txt`), same 960x600 canvas, shared `capture_shot` helper; shots live in target tmp only, tracked copies refresh on EXQ_KEEP_PNG=1
- RESULT: PASS
