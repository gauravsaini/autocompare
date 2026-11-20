# REPORT — wave theme26 (light theme, round-1)

Scope: light-theme tokens (`theme.rs` `LIGHT_*` mirrors of the fixed diff accents) + `Visuals::light()` wiring (`app.rs` resolver, `main.rs` `--screenshot` default). Geometry and typography byte-identical by construction (same rects, fonts, sizes — only the palette remaps).

## Fixture shots

- `before.png`: dark baseline (11200 B), copied pre-edit.
- `after.png`: light re-baseline (11098 B), `--screenshot` default post-edit.
- `dark-ref.png`: dark reference (11200 B), rendered post-edit via `--screenshot-mode dark`.

## Exquisite re-run (post-edit, verbatim)

```
# Exquisite report (round-1)
- PNG: 11098 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x615, Grid x16, row-h x327, width calls x85, gutter x1754
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 38 (info only, must not grow for geo-typo waves)
RESULT: PASS
```

Geometry/typo signals identical to the dark baseline (x615/x16/x327/x85/x1754, same sizes/weights) — palette-only change.

## pxdiff before.png -> after.png (verbatim)

```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 390933 of 576000
diff_bbox: (0,0)-(959,595)
VERDICT: DIFFERENT
```

bbox covers the full panes (near-white pane bg + light chrome wash), same 960x600 canvas.

## dark-ref check (verbatim)

```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
```

`dark-ref.png` == `before.png` (11200 B both): explicit `--screenshot-mode dark` and `AUTOCOMPARE_DARK_REF=1` both reproduce the dark reference byte-identically.

## Screenshot theme behavior table (proven live against the built binary)

| invocation | result |
|---|---|
| `--screenshot after.png` (no flag) | light (11098 B, DIFFERENT vs dark) |
| `--screenshot dark-ref.png --screenshot-mode dark` | dark, IDENTICAL to `before.png` |
| `AUTOCOMPARE_DARK_REF=1 --screenshot x.png` | dark, IDENTICAL to `before.png` |
| `--screenshot-mode light` + env set | light (explicit flag wins) |

## Touched functions

- `theme.rs`: `deleted_bg_for`, `deleted_fg_for`, `inserted_bg_for`, `inserted_fg_for`, `changed_bg_for`, `changed_fg_for`, `accent_fg_for`, `error_bg_for`, `error_fg_for`, `light_diff_accents_aa` (+ 5 `LIGHT_*_RGB` + 5 `LIGHT_*` consts; module-doc color-model update).
- `app.rs`: `App::resolve_screenshot_mode`, `App::dark_ref_from_env`, `App::resolve_screenshot_mode_with_env` (+ tests `theme26_screenshot_mode_defaults_light`, `theme26_screenshot_mode_explicit_flag_wins`, `theme26_screenshot_mode_env_dark_ref`).
- `main.rs`: `--screenshot` path calls `App::resolve_screenshot_mode` (exit-2 on Err preserved).
- `theme.rs` test: `theme26_light_contrast` (+ edge fixture `comparison/theme-contrast-720.json`, min ratio 5.32).

## Color count 33 -> 38 (per-addition justification)

Each addition is a light-mirror of a dark token (same hue family, lightness remapped; inks reused where contrast allows):

1. `LIGHT_DELETED_BG` — mirror of `DELETED_BG` (ink reuses `LIGHT_RED`, pair 5.49).
2. `LIGHT_INSERTED_BG` — mirror of `INSERTED_BG` (ink reuses `LIGHT_GREEN`, pair 5.32).
3. `LIGHT_CHANGED_BG` — mirror of `CHANGED_BG` (wash for the amber family).
4. `LIGHT_CHANGED_FG` — mirror of `CHANGED_FG` (dark amber ink; pair 6.48; also reused for accent spans).
5. `LIGHT_ERROR_BG` — mirror of `ERROR_BG` (text reuses `LIGHT_RED`, pair 5.77).

No other `Color32::from_rgb`/`Visuals::` added. Dark values untouched (`tokens_keep_frozen_hues` still green).

## Gates evidence

- Full `autocompare-gui` suite: `test result: ok. 1054 passed; 0 failed` (incl. `theme26_light_contrast` + 3 resolver tests).
- `cargo fmt -p autocompare-gui -- --check`: clean.
- Banned-token grep on non-test delta (`unwrap\(|\.expect\(|panic!|todo!|unreachable!|unsafe`): empty.
- `git status`: only `crates/gui/src/theme.rs`, `crates/gui/src/app.rs`, `crates/gui/src/main.rs`, `checks/gui-exq-theme26/**`. Stray test-written PNGs under `checks/gui-sessionpaint-by/` (rewritten by a pre-existing `views.rs` test on every full run, few bytes of encoder nondeterminism) restored via `git checkout --` before commit.
