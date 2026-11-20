# Wave T TYPO — REPORT (round T-2)

Scope: typography only. Koi geometry edit nahi (widths/margins/padding/
alignment untouched — harness geometry signals T-1 se identical), koi
color edit nahi (Color32/Visuals touches 29 -> 29, zero new). CR sec-8
ke 3 typo gaps wired paint me fix: flat hierarchy, label-vs-input
confusion, dim status contrast — sab weight/size/family se, color frozen.

## Baseline (round-T-1) -> After (round-T-2)

- `cargo test -p autocompare-gui`: 674 passed -> 677 passed (+3: hex
  find-label role, table section-label role, status zone roles).
- Harness `tools/gui-polish/run-geo-typo.sh`: PASS -> PASS.
- PNG bytes: 9765 -> 9765 (byte-identical, `cmp` clean). Expected:
  default headless scene me find/key rows nahi aate — hierarchy wired
  paint paths + unit-test proof se cover hai, default scene regression-free.
- Harness counts: monospace x50 -> x49 (labels/badges role-rich me gaye),
  strong x26 -> x32 (3 wired paint-side `.strong()` + doc precedent),
  small x52 -> x41 (`.small()` chains Small-role rich me gaye),
  `.size` values ['14.0'] same, text_style_height x4 same (koi naya size),
  Grid x14 same, widths 28 same, add_space [] same.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean
  (final window; beech me sibling diff/vfs waves ke transient red the —
  OWNS me zero, unko touch nahi kiya).
- `cargo fmt -p autocompare-gui -- --check`: clean.
- MS guidelines: `#![forbid(unsafe_code)]` barkarar; added lines me zero
  unwrap/expect/panic/todo/unsafe (grep clean); colors frozen.

## 3 gaps — kya badla (file:line)

1. Flat hierarchy — section labels ek step upar:
   - `views_hex.rs`: `hex_find_label_text_role()` (Body 13) + paint-side
     `.strong()`; badge `TypoRole::Small.rich` (11-weak, same pixels).
   - `views_table.rs`: `table_section_label_text_role()` (Body 13) +
     `.strong()` — `Key:` aur `Find:` dono isi ko chalate hain.
   - Natija: label(13-strong) > content(12-mono) > hint/badge(11-weak).
2. Label-vs-input confusion — family se farak:
   - Dono needle `TextEdit` (hex + table) par `.font(Monospace)`: typed
     values/hint Mono me, labels proportional-strong me. Color zero.
3. Dim status contrast — weight se contrast (color frozen):
   - `(both tables are empty)`: `.weak()` -> `TypoRole::Body.rich`
     (13 default, weak hata).
   - Positional note: Small-weak -> `TypoRole::Body.rich`.
   - `viewmodel.rs`: `status_verdict_text_role()` (Body) +
     `status_side_text_role()` (caption == Small, tabular) — verdict >
     sides hierarchy, paint wiring-pending (views.rs FORBIDDEN, untouched).
   - Counts/badges/page-label Small rahe (tabular intent, bottom step).

## Style table (wired paint)

| Pane | Role | Family | Weight | Size | Refs |
|---|---|---|---|---|---|
| Find/Key section label | Body + paint `.strong()` | proportional | strong | 13.0 | views_hex.rs:hex_find_label_text_role, views_table.rs:table_section_label_text_role |
| Needle input values | TextStyle::Monospace | monospace | default | 12.0 | views_hex.rs:paint_hex_find_bar, views_table.rs:paint_table_find_controls |
| Badges/hints/suggested | Small | proportional | weak | 11.0 | Small.rich call sites (pixels same) |
| Empty/positional status | Body | proportional | default | 13.0 | paint_table_cell_grid, paint_table_key_bar |
| Verdict (wiring-pending) | Body | proportional | default | 13.0 | viewmodel.rs:status_verdict_text_role |
| Sides (wiring-pending) | Small/caption | proportional | weak | 11.0 | viewmodel.rs:status_side_text_role |

## Rounds used: 2 of 6 (T-1 baseline, T-2 implement+verify). Green.

---

# Wave L TYPO — REPORT (round L-2)

Scope: typography only. No geometry edits (widths/margins/padding/
alignment untouched), no color edits (no Color32/Visuals touched).
Paint file `views.rs` untouched by design: this round centralizes the
per-pane type hierarchy in tokens + one egui applicator.

## Baseline (round-L-1) -> After (round-L-2)

- `cargo test -p autocompare-gui`: 182 passed -> 193 passed (+11:
  text_style mapping, rich applicator, home/gutter role tests).
- Harness `tools/gui-polish/run-geo-typo.sh`: PASS -> PASS.
- PNG bytes: 9914 -> 9914 (bit-identical, `cmp` clean). Expected:
  paint untouched; hierarchy now lives in tokens for GEO wave to adopt.
- Harness counts: monospace x33 -> x33, strong x5 -> x6 (one
  `rich()` applicator), small x30 -> x30, `.size` values ['14.0'] ->
  ['14.0'], text_style_height x4 -> x3 (doc reword; code call site in
  `views.rs:72` unchanged, token string + test string intact).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean.
- `cargo fmt -p autocompare-gui -- --check`: clean.
- MS guidelines: `#![forbid(unsafe_code)]` kept in all 3 files; zero
  unwrap/expect/panic/todo added in non-test code (grep count 0);
  all new fns headless (no `Ui`/`Context` needed).

## Style table (pane -> family / weight / size / line-height)

Line-height is shared everywhere: measured mono row height
(`theme.rs:70` TYPO_LINE_HEIGHT_SOURCE). Letter-spacing is 0.0
everywhere (`theme.rs:67`, applied via `extra_letter_spacing` in
`rich()`; egui has no tracking).

| Pane | Role | Family | Weight | Size | Refs |
|---|---|---|---|---|---|
| App header title | Heading | proportional (`TextStyle::Heading`) | strong | 18.0 | theme.rs:49, theme.rs:190 |
| Home card title | HomeTitle | proportional (`TextStyle::Body` + explicit size) | strong | 14.0 | theme.rs:60, home.rs:138 |
| Home card desc | HomeDesc | proportional (`TextStyle::Small`) | weak | 11.0 | theme.rs:53, home.rs:143 |
| Home card button | HomeButton | proportional (`TextStyle::Button`) | default | 13.0 | theme.rs:51, home.rs:148 |
| Diff panes / summaries / status | Mono | monospace (`TextStyle::Monospace`) | regular-mono | 12.0 (max 13.0) | theme.rs:55, gutter.rs:100 |
| Hints / subtitles / counts | Small | proportional (`TextStyle::Small`) | weak | 11.0 | theme.rs:53 |
| Plain labels / buttons | Body | proportional (`TextStyle::Body`) | default | 13.0 | theme.rs:51 |

Applicator: `TypoRole::rich()` at theme.rs:207 (family + weight +
size + letter-spacing in one headless builder). Style map:
`TypoRole::text_style()` at theme.rs:190. Gutter text-size/role/
line-height passthroughs at gutter.rs:100,105,110. Home role
passthroughs at home.rs:138,143,148.

## Rounds used: 2 of 6 (L-1 baseline, L-2 implement+verify). Green.
