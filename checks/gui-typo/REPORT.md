# GUI Typo report (checks/gui-typo)

Headless typography system only. No paint edits in this wave
(`views.rs` / `app.rs` read-only); GEO wave applies these contracts.

## Scale table (theme.rs, single source)

| Role | Size px | Family | Weight | Line-height source | Paint site |
|---|---|---|---|---|---|
| Heading | 18 | proportional | strong | `ui.text_style_height(Monospace)` | `ui.heading` |
| Body | 13 | proportional | regular | same | buttons, plain labels |
| Small | 11 | proportional | weak | same | hints, counts, `Show:` |
| Mono diff | 12 (max 13) | monospace | regular | same (row height) | diff panes, summaries |
| HomeTitle | 14 | proportional | strong | same | `paint_home` title (only `.size(14.0)`) |
| HomeDesc | 11 | proportional | small weak | same | `paint_home` description |
| HomeButton | 13 | proportional | regular (body) | same | `paint_home` `Open X` button |

Constants: `TYPO_HEADING_SIZE` 18.0, `TYPO_BODY_SIZE` 13.0,
`TYPO_SMALL_SIZE` 11.0, `TYPO_MONO_SIZE` 12.0,
`TYPO_MONO_SIZE_MAX` 13.0, `TYPO_HOME_TITLE_SIZE` 14.0,
`TYPO_WRAP_PANE_CHARS` 80, `TYPO_FULL_PANE_CHARS` 200,
`TYPO_LETTER_SPACING` 0.0, `TypoRole::{Heading,Body,Small,Mono,HomeTitle,HomeDesc,HomeButton}`.

Letter-spacing: egui has no tracking; documented as no-op
(`TYPO_LETTER_SPACING = 0.0`). Hierarchy rides on size +
strong/weak + measured line-height. No dep added.

## px / height measurements (grep + runtime source)

- `geo-typo-report.md`: PNG 9914 bytes (min 8192 ok), magic ok,
  distinct 256 (ok). `RESULT: PASS`.
- Typography signals (unchanged paint): monospace x17, strong x5,
  small x10, `.size` values `['14.0']` (single Home title),
  `row_height` x5.
- Geometry context (GEO-owned): `Grid::1`, widths 6 calls
  (`half`/`third` field helpers + `CARD_W` 260.0 / `CARD_H` 75.0),
  `text_style_height` via shared `text_row_height(ui)` helper in
  `views.rs` (one def, 5 `show_rows` call sites: lines, merge,
  table, text, folder).
- Gutter px: `gutter_bar_width(true)` 12.0, `(false)` 6.0.
  New `gutter_bar_width_for_font(size, shown)` scales linearly from
  mono 12 (`13px -> 13.0 / 6.5`), falls back on NaN/inf/<=0.
- Pane char widths (viewmodel, headless): wrap-on caps at 80
  (`TYPO_WRAP_PANE_CHARS`), wrap-off keeps full width (200 in the
  documented dense case, `TYPO_FULL_PANE_CHARS`). Gutter chars 4
  shown / 1 hidden.

## Home cards (home.rs contract, paint applied by GEO wave)

- Title: strong 14 (`HOME_CARD_TITLE_SIZE`, `home_card_title_size()`).
- Description: small weak (`HOME_CARD_DESC_SMALL_WEAK`,
  `home_card_desc_is_small_weak()`).
- Button: body scale 13, default size (`HOME_CARD_BUTTON_BODY_SIZE`,
  `home_card_button_size()`).

## Files touched (TYPO owns only)

- `crates/gui/src/theme.rs`: typo constants + `TypoRole` + tests.
- `crates/gui/src/gutter.rs`: `gutter_bar_width_for_font` + tests.
- `crates/gui/src/home.rs`: Home card typo constants/fns + test.
- `crates/gui/src/viewmodel.rs`: pane-width docs via theme consts,
  typo + gutter re-exports for GEO/integration consumption.
- No `Color32` / `Visuals` / palette touches in owned files
  (one doc mention only). No `views.rs` / `app.rs` edits.

## Verification

- `cargo test -p autocompare-gui`: 115 passed.
- `cargo clippy -p autocompare-gui -- -D warnings`: clean.
- `cargo fmt --check -p autocompare-gui`: clean.
- `tools/gui-polish/run-geo-typo.sh checks/gui-typo`: `RESULT: PASS`.
