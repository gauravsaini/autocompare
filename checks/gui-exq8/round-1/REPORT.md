# gui-exq8 round-1 — Text panes remaining-height fill (exq8b retry)

Scope: leaf-3.2 wave8b. OWNS-only: `crates/gui/src/views.rs`,
`crates/gui/src/viewmodel.rs`, `checks/gui-exq8/**`. No touch on
`app.rs`, `home.rs`, `tree.rs`, `views_hex.rs`, `gutter.rs`,
`views_table.rs`, `theme.rs`, `crates/diff/**`, `crates/cli/**`,
`crates/sessions/**`, or other checks dirs (home1 landed, uxkey +
clivcs2 active, exq7 WIP preserved — read-only).

## Root cause (outside observation + code reading, no decompilation)

Symptom: own 9765B PNG vs BC 909131B PNG; bottom ~50% black waste.
`capture/live-observe-orch-3.md` pointers confirmed in code:
`paint_text_body` and `paint_merge` read `ui.available_height()`
*inside* the panes `horizontal`, after the list column already
consumed layout — so the row shrank to content (`rows x row_h`)
instead of filling the viewport. The outer horizontal scroll wrapper
(`app.rs:show_split_content`, NO-TOUCH) then sizes to that short
content, leaving the central panel's lower half empty.

## Fix (geometry FIRST, +55/-5 lines, no new deps)

- `viewmodel.rs`: new `TEXT_PANES_MIN_H = 100.0` (names the legacy
  inline `.max(100.0)` floor) + `text_panes_height(avail_h)` — pure,
  finite-guarded remaining-height budget. Single source for paint.
- `views.rs` `paint_text_body`: budget captured *before* the panes
  row; list column `set_min_height(panes_h)`; minimap/`gutter_geometry`
  pin to the same `panes_h` (was: leftover `available_height()`).
- `views.rs` `paint_merge`: same pattern for merge output list +
  conflict overview ruler.
- Already-covered (verified, untouched): center gutter copy arrows
  (`TEXT_CENTER_GUTTER_W`, `text_gutter_has_copy`), split 20-80%
  draggable (`clamp_split_fraction`, `paint_split_handle`,
  `SPLIT_MIN/MAX_FRACTION`), toolbar (`paint_session_toolbar`),
  file-info bar (`paint_path_meta_row`), 960 (460/460) + narrow-720
  (340/340) via `dual_pane_widths`, floors keep narrow windows
  scrolling instead of clipping.
- Typography SECOND (verified, untouched): panes paint
  `TypoRole::Mono` (mono family, same `FontId`, size never changes —
  emphasis is inversion + 1px underline only), line-height from the
  shared measured `text_row_height` + `ROW_PAD` (`row_height_for_line`),
  every row `set_row_height(row_height)`. Colors frozen (no hue/token
  touched).

## Edge fixture (1 added)

`viewmodel.rs::text_panes_height_edge_guards`: 600→600 passthrough,
99.9/0/-50/NaN/+inf→100.0 floor, result always finite+positive.

## Gates

- `CARGO_TARGET_DIR=/tmp/target-op-guiexq8b CARGO_BUILD_JOBS=2 cargo test -p autocompare-gui`: **743 passed, 0 failed** (baseline 742 + 1 new).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: **zero warnings** (exit 0).
- `cargo fmt --all --check`: own files clean; only diffs are pre-existing `theme.rs` (uxkey wave, out of OWNS scope, untouched).
- Banned-scan on own diff (`unsafe|decompil|include_bytes|extern crate|new use`): **zero hits**; `Cargo.toml` untouched (stdlib only, const/token reuse = ponytail).
- `cargo run -p xtask -- secrets`: 1 finding in `checks/release-w1/REPORT.md` — pre-existing, outside OWNS, untouched.
- Screenshot: `views::tests::session_strip_scenes_prove_png` passes (PNG contract green); live visual re-capture left to the screenshot-owning wave (app.rs NO-TOUCH here).

Commits: code `0cb98ad` (2 files, +55/-5, pathspec-only, NO PUSH).
Counts: tests 743/0, clippy 0, fmt-own 0, banned 0, secrets-own 0.

RESULT: PASS
