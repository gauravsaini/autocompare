# Wave M TYPO-PAINT — REPORT (round M-1)

Scope: typography only, owned files only
(`theme.rs`, `gutter.rs`, `home.rs`). No geometry edits, no color
edits, no touch of views/app/viewmodel/hex/table/tree/main/
screenshot, no STATUS.md, no push.

## Baseline -> After

- `cargo test -p autocompare-gui`: 193 -> 195 passed, 0 failed.
  Owned: theme 12 + gutter 10 + home 6 = 28 green.
  (+2 vs Wave L baseline are sibling in-progress tests; owned
  count steady, values identical.)
- Harness `tools/gui-polish/run-geo-typo.sh
  checks/gui-exq-typo2/round-M-1`: PASS.
- PNG: 9914 bytes, magic ok, distinct 256 (not blank).
  Bit-identical to Wave L round-L-2 (paint untouched by design).
- Harness typo counts: monospace x33, strong x6, small x30,
  `.size` values ['14.0'], row_height x3 — unchanged, all live
  paint sites remain in views.rs/app.rs (out of OWNS).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean.
- `cargo fmt -p autocompare-gui -- --check`: clean.
- Owned raw-site scan: zero code uses of `monospace()`,
  `.strong()`, `.small()`, `.size(14.0)` in gutter.rs/home.rs.
  Only hits in owned files are comments (theme.rs:59,
  home.rs:252) plus the `rich()` applicator itself
  (theme.rs:213). Headless confirmed: no `RichText`/`Ui` in
  gutter.rs/home.rs.

## Role table (pane -> role -> owned call site)

| Pane | Role | Size/weight | Owned site |
|---|---|---|---|
| App header title | Heading | 18.0 strong | theme.rs:190 `text_style`, theme.rs:207 `rich` |
| Home card title | HomeTitle | 14.0 strong | home.rs:138 `home_card_title_role`, home.rs:122 `home_card_title_size` (now via `TypoRole::HomeTitle.size()`) |
| Home card desc | HomeDesc | 11.0 weak small | home.rs:143 `home_card_desc_role` |
| Home card button | HomeButton | 13.0 default | home.rs:148 `home_card_button_role`, home.rs:132 `home_card_button_size` (now via `TypoRole::HomeButton.size()`) |
| Diff panes / summaries / status | Mono | 12.0 regular-mono (max 13.0) | gutter.rs:100 `gutter_text_role`, gutter.rs:105 `gutter_text_size` (now via `TypoRole::Mono.size()`) |
| Hints / subtitles / counts | Small | 11.0 weak | theme.rs:190 `Small -> TextStyle::Small`, theme.rs:207 weak arm |
| Plain labels / buttons | Body | 13.0 default | theme.rs:190 `Body -> TextStyle::Body` |
| Line-height (all roles) | shared | measured mono row | gutter.rs:110, theme.rs:70 `TYPO_LINE_HEIGHT_SOURCE` |

## Diff (smallest, 3 delegations, same values)

- gutter.rs:105 `gutter_text_size()` returns
  `TypoRole::Mono.size()` (12.0) instead of `TYPO_MONO_SIZE`.
- home.rs:122 `home_card_title_size()` returns
  `TypoRole::HomeTitle.size()` (14.0) instead of
  `HOME_CARD_TITLE_SIZE`.
- home.rs:132 `home_card_button_size()` returns
  `TypoRole::HomeButton.size()` (13.0) instead of
  `HOME_CARD_BUTTON_BODY_SIZE`.
- No geometry, no color, no paint-file touch. Live paint
  adoption in views.rs/app.rs is left for the GEO/paint owner.

## Sibling note

Working tree shows sibling mid-edit files (app.rs, views.rs,
tree.rs, views_hex.rs, local-run-l REPORT). Not touched.
Gate: owned module tests (28/28) + full suite (195/195) +
harness PASS + clippy + fmt all green on owned diff.

## Rounds used: 1 of 6 (M-1 implement+verify). Green.
## Re-verify note

- One transient clippy red hit mid-run (`let...else` in a
  sibling-owned file while another agent was saving); re-run
  clean. Owned diff untouched. Final: tests 195/195, clippy
  clean, fmt clean, harness PASS.
