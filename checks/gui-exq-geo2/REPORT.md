# Wave M GEO-PAINT REPORT (round M-1)

Scope: `crates/gui/src/views.rs` + `crates/gui/src/app.rs` geometry only.
No color edits. No typography edits. Clean-room own words.

## 1. Wave L study (checks/gui-exq-geo/REPORT.md)

Live tokens already: OUTER_MARGIN 12, SECTION_GAP 8.0, TIGHT_GAP 4.0,
FOOTER_GAP 6.0, TAB_BOTTOM_GAP 6.0 (2->6 done), TREE_INDENT 14.0,
CARD_W/H 240.0/75.0 (260->240 done), picture_column_width
`(avail-SECTION_GAP)/2` (TIGHT->SECTION done).
`app.rs:1832,1837,2385` already consume OUTER/TAB_BOTTOM/FOOTER tokens.

Bachi purani spacing (live paint paths me ab bhi 4px/12px):
header bottom, home grid gap, path rows bottom, toolbar bottom,
error-to-content bottom. Vertical rhythm tab(6) vs path(4) vs panes
uneven tha; grid gap outer-margin ko inner-gap ki tarah use karta tha.

## 2. Fix (round M-1, smallest GEO diff, 6 lines)

1. `views.rs:123` paint_header bottom: `TIGHT_GAP` -> `TAB_BOTTOM_GAP`.
   Before 4px, after 6px (+2, tab rhythm match).
2. `views.rs:135` home grid `.spacing([OUTER_MARGIN, OUTER_MARGIN])`
   -> `.spacing([SECTION_GAP, SECTION_GAP])`.
   Before [12,12], after [8,8] (-4/axis). Grid total
   3*240+2*12=744 -> 3*240+2*8=736px, 776px avail me fit.
3. `views.rs:234` paint_paths bottom: `TIGHT_GAP` -> `TAB_BOTTOM_GAP`.
   Before 4px, after 6px (+2, path-to-panes rhythm).
4. `views.rs:298` paint_triple_paths bottom: same 4px -> 6px (+2).
5. `app.rs:1874` toolbar bottom: `views::TIGHT_GAP`
   -> `views::TAB_BOTTOM_GAP`. Before 4px, after 6px (+2).
6. `app.rs:1974` error-to-content: `views::TIGHT_GAP`
   -> `views::TAB_BOTTOM_GAP`. Before 4px, after 6px (+2).

Koi color/typo touch nahi: diff me sirf gap tokens, no Color32,
no RichText/size/strong/small change (1 `.small()` context line hai,
edit nahi).

## 3. Proof

- `cargo test -p autocompare-gui`: 195 passed, 0 failed.
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-exq-geo2/round-M-1`: PASS.
- PNG `round-M-1/after.png`: 9914 bytes, magic `89504e470d0a1a0a` ok,
  distinct 256 (>64 ok). Fixture raster hai, egui token se bytes
  nahi badalte (Wave L note), proof static px math + fixture bytes.
- Harness metrics: add_space literals `[]`, Grid 1,
  text_style_height x3, width calls 9, monospace x33, strong x6,
  small x30, `.size` ['14.0'].
- Clippy owned clean (views.rs/app.rs me koi error nahi).
  Full `clippy --all-targets -- -D warnings` red hai sibling
  `tree.rs:1054` let-else (GAPS wave mid-edit), OWNS bahar, touch nahi kiya.
- `cargo fmt --check`: owned views.rs/app.rs clean (koi diff nahi);
  full tree me sibling `crates/compare` diffs hain, touch nahi kiya.

## 4. Rounds: 1/6. Next: sibling green hone par full green re-run.
