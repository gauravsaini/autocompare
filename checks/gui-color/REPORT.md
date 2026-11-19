# GUI Color report (checks/gui-color)

Typed design-token palette, Dark/Light. Geometry + typography frozen
(no spacing/font/size moves; paint signals identical to the TYPO wave).

## Token table (theme.rs, single source)

Fixed diff accents (same both modes, own contrast):

| Token | RGB | Paint const |
|---|---|---|
| Deleted FG / BG | [255,170,170] / [120,32,32] | `DELETED_FG` / `DELETED_BG` |
| Inserted FG / BG | [170,240,180] / [24,90,46] | `INSERTED_FG` / `INSERTED_BG` |
| Changed FG / BG | [255,235,160] / [122,102,20] | `CHANGED_FG` / `CHANGED_BG` |
| Accent FG | [255,255,210] | `ACCENT_FG` |
| Error fill (text = Deleted FG) | [60,20,20] | `ERROR_BG` |

Per-mode chrome (`palette(dark)` / `*_for(dark)`, picked from
`ui.visuals().dark_mode` at each paint site):

| Role | Dark | Light |
|---|---|---|
| Base FG / BG | [230,230,230] / [27,27,27] | [20,20,20] / [248,248,248] |
| Dim (gutter, `=`) | [150,150,150] | [90,90,90] |
| Red marker | [255,100,100] | [180,20,20] |
| Green marker | [120,220,130] | [20,110,50] |
| Newer-side blue | [130,170,255] | [30,80,180] |
| Gutter track (decorative) | [32,32,32] | [225,225,225] |

Light markers keep the same hue families, darkened so AA holds on
light. One dim cannot pass on both base backgrounds (mid-gray fails
one side either way), hence the per-mode split.

## WCAG AA (theme_contrast_ok, >= 4.5)

- Deleted 5.72, Inserted 6.23, Changed 4.72 ok.
- Accent on changed/deleted/inserted BG: 5.48 / 10.11 / 8.06 ok.
- Error FG on error BG: 8.89 ok.
- Dark chrome on dark BG: base 13.80, dim 5.82, red 5.95, green 10.14, blue 7.50 ok.
- Light chrome on light BG: base 17.35, dim 6.49, red 6.48, green 5.98, blue 6.91 ok.
- Tests: `fixed_diff_pairs_satisfy_aa`,
  `palettes_satisfy_aa_per_mode`, `palette_selector_matches_mode`,
  `tokens_keep_frozen_hues` (theme.rs) + `chrome_selectors_follow_dark_mode`
  (views.rs). Full gui suite: 122 passed (was 117, +5 new).

## Paint mapping (views.rs, same hues)

- `kind_palette`: fixed diff tokens; Equal keeps theme `base_text`.
- `side_job_with_prefs` gutter: `TextPaneLayout.dim`
  (`dim_for(dark)` at the call site, computed once per paint).
- Separator `|`, table glyphs, merge conflict markers, folder glyphs:
  `dim/red/green/blue_for(dark)`.
- Thumbnail track: `track_for(dark)` (replaces the raw
  `Color32::from_rgb(32, 32, 32)`; zero `from_rgb` left in views.rs).
- `TextPaneLayout` gains one `dim: Color32` field (bundled, no new
  arg-count pressure); `glyph_color` takes `dark: bool`. No
  geometry/typo call touched.

## Harness + gates evidence

- `tools/gui-polish/run-geo-typo.sh checks/gui-color`: RESULT: PASS
  (PNG 9914 bytes, magic ok; see `geo-typo-report.md`).
- Paint signals unchanged vs TYPO wave: monospace x17, strong x5,
  small x10, `.size` values `['14.0']`, `text_style_height` x5,
  Grid x1, width calls x6.
- `cargo test -p autocompare-gui`: 122 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`:
  no warnings in `theme.rs` / `views.rs`.
- `cargo fmt --check`: clean for owned files.
- `git diff` on views.rs touches color lines only (verified: no
  add_space/size/monospace/strong/small/weak/width/grid/row-height
  hunks).
