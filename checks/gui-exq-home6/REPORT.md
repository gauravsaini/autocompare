# HOME6 Exquisite Report — narrow plus cards & CTA width & toolbar hits

## 1. Summary
- `crates/gui/src/home.rs` me HOME6 contract add kiya:
  1. Card min-w from SPACE scale at 720: `HOME6_CARD_MIN_W` = 10 * `SPACE_XL` = 240.0. Narrow 720px par avail content width 696.0 (`720 - 2 * 12`), cards wrap from 3 cols to 2 cols (504 <= 696, 3 cols need 768 > 696). 9 cards yield 5 rows. Card flexes to 336.0 (`(696 - 24) / 2`).
  2. CTA width from card inner width: `home6_card_cta_width(card_w)` derives directly from `card_w - 2 * HOME6_CARD_PAD` (224.0 at min-w, 320.0 at 720 narrow flexed card).
  3. Toolbar button 24px hits symmetric: 96x24 button (`HOME6_TOOLBAR_BUTTON_W` x `HOME6_TOOLBAR_BUTTON_H`) expands with 4px hit-slop (`SPACE_XS`) on all 4 edges to 104x32 hit area (3328 px²).
  4. Edge fixture comparison: `checks/gui-exq-home6/comparison/home-narrow-720.json` added.
  5. Headless test `home6_narrow720_cards_cta_toolbar_edge` proves narrow cards, CTA width derived from card inner width, symmetric toolbar hit box, and SPACE token scaling.
- Geometry first, typography second (TypoRole reuse), colors frozen at 33 touches.

## 2. Harness
- `tools/exquisite/run.sh checks/gui-exq-home6/round-1 crates/gui/src` -> `RESULT: PASS` (round 1/6).
- PNG: 10401 bytes (min 8192 ok), magic ok, distinct 256.
- Geometry: SPACE tokens `['LG', 'MD', 'SM', 'XL', 'XS']`, margin/gap consts x569, Grid x16, row-h x297, width calls x81, gutter x1540.
- Typo: TypoRole x1038, monospace x26, strong x45, .size `['14.0']`, weights MEDIUM/REGULAR/SEMIBOLD, line-heights BODY/CHROME/MONO/SOURCE.
- Colors frozen: Color32/Visuals touches 33 (must not grow).

## 3. Tests
- `home6_narrow720_cards_cta_toolbar_edge`: passes.
- All 54 home tests pass; 1004 library tests pass in isolated worktree.
- Rustfmt clean; zero banned keywords in non-test code.
