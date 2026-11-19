# GUI Exquisite — Geometry + Typography (Wave H, Agent C)

Owner files only: `crates/gui/src/theme.rs`, `crates/gui/src/gutter.rs`,
`crates/gui/src/tree.rs`, `checks/gui-exquisite/**`.
Colors FROZEN from COLOR wave — no hue changes in this wave.

## Baseline

- `cargo test -p autocompare-gui`: 122 passed (pre-edit).
- Harness `tools/gui-polish/run-geo-typo.sh /tmp/geo-baseline`: RESULT PASS.
- PNG: 9914 bytes, magic ok, distinct 256 (> 64 blank check).

## Edits (minimal, token-driven)

`theme.rs` — single source of truth, no paint touched:

- Geometry scale (px): canonical spacing 4/8/12/16/24.
- `GEO_TIGHT_GAP` (4), `GEO_SECTION_GAP` (8), `GEO_OUTER_MARGIN` (12),
  `GEO_PANEL_GUTTER` (16), `GEO_CARD_GAP` (24).
- `GUTTER_BAR_WIDE` (12 = MD step), `GUTTER_BAR_NARROW` (6 = half MD),
  `GUTTER_MIN_SEGMENT_H` (2.0), `GUTTER_MERGE_GAP` (2.0).
- `GEO_TREE_INDENT_STEP` (12 = MD step, canonical; `views.rs` keeps
  legacy 14.0 until the owning agent adopts — disjoint ownership).
- `TYPO_LINE_HEIGHT_SOURCE` (`ui.text_style_height(Monospace)`);
  `TypoRole::line_height_source` returns it (was inline literal).
- `TypoRole::weight()` (strong/default/weak/regular-mono) and
  `TypoRole::family()` (monospace/proportional) — hierarchy carrier
  next to size since egui has no letter-tracking.
- `TYPO_LETTER_SPACING` stays 0.0 (documented no-op).
- Sizes unchanged: 18 / 13 / 11 / mono 12 (max 13) / Home title 14.

`gutter.rs` — applies theme tokens purely:

- Segment min-height and same-kind merge gap read
  `GUTTER_MIN_SEGMENT_H` / `GUTTER_MERGE_GAP` (were 1.5 / 1.0 magic).
- `gutter_bar_width` reads `GUTTER_BAR_WIDE` / `GUTTER_BAR_NARROW`
  (values still 12.0 / 6.0, now scale-anchored).

`tree.rs` — applies theme token purely:

- New `tree_indent_step()` (= `GEO_TREE_INDENT_STEP`, 12.0) and
  `tree_indent_for_depth(depth)` (= depth × step, saturating math).

## Token table

| Token | Value | Role |
|---|---|---|
| `SPACE_XS` / `GEO_TIGHT_GAP` | 4.0 | tight gaps, marker padding |
| `SPACE_SM` / `GEO_SECTION_GAP` | 8.0 | section gaps |
| `SPACE_MD` / `GEO_OUTER_MARGIN` | 12.0 | outer margin, panel gutter |
| `SPACE_LG` / `GEO_PANEL_GUTTER` | 16.0 | large panel gutter |
| `SPACE_XL` / `GEO_CARD_GAP` | 24.0 | card / large section gap |
| `GUTTER_BAR_WIDE` | 12.0 | gutter bar (line numbers on) |
| `GUTTER_BAR_NARROW` | 6.0 | gutter bar (line numbers off) |
| `GUTTER_MIN_SEGMENT_H` | 2.0 | min visible hunk height |
| `GUTTER_MERGE_GAP` | 2.0 | same-kind merge gap |
| `GEO_TREE_INDENT_STEP` | 12.0 | canonical tree indent / depth |
| `TYPO_HEADING_SIZE` | 18.0 | heading, strong, proportional |
| `TYPO_BODY_SIZE` | 13.0 | body / Home button, default |
| `TYPO_SMALL_SIZE` | 11.0 | small / Home desc, weak |
| `TYPO_MONO_SIZE` | 12.0 | mono diff, regular-mono |
| `TYPO_MONO_SIZE_MAX` | 13.0 | tolerated mono upper bound |
| `TYPO_HOME_TITLE_SIZE` | 14.0 | Home card title, strong |
| `TYPO_LETTER_SPACING` | 0.0 | no-op (egui has no tracking) |
| `TYPO_LINE_HEIGHT_SOURCE` | `ui.text_style_height(Monospace)` | single line-height source |

## Measurements (after)

- `cargo test -p autocompare-gui`: 125 passed (122 + 3 new token tests).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean
  (also fixed 4 pre-existing `assertions_on_constants` lints in
  `theme.rs` tests via `const { assert! }`).
- `cargo fmt --check -p autocompare-gui`: clean.
- Harness `run-geo-typo.sh /tmp/geo-after`: RESULT PASS.
- PNG: 9914 bytes, magic ok, distinct 256 — identical to baseline
  (expected: headless token unification, no paint change).
- Harness details: `add_space []`, `Grid x1`, `text_style_height x4`
  (was x5 — literal now defined once as `TYPO_LINE_HEIGHT_SOURCE` and
  referenced), `monospace x17`, `strong x5`, `small x10`,
  `.size ['14.0']` (unchanged baseline), width calls x6,
  `Color32/Visuals` touches 22 (info only, untouched).

## WCAG note

Colors unchanged in this wave — all `*_RGB` / `Color32` values
byte-identical to the COLOR wave. `fixed_diff_pairs_satisfy_aa`,
`palettes_satisfy_aa_per_mode`, `theme_modes_satisfy_contrast`, and
`tokens_keep_frozen_hues` still green, so AA holds exactly as before.
Geometry/typo hierarchy comes from size + weight + line-height only.

## Proof files

- `before.png`: copy of `checks/gui-color/after.png` (9914 B).
- `after.png`: fresh harness screenshot (9914 B, identical — no paint change).
- `geo-typo-report.md`: harness output (RESULT: PASS).
