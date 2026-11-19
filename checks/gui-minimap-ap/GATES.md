# GATES — Wave AP-MINIMAP (Gap #5 minimap-click)

OWNS: `crates/gui/src/views.rs` + `checks/gui-minimap-ap/**`.
KEVL (kabhi mat chhoona): `app.rs`, `viewmodel.rs`, `tree.rs`,
`views_table.rs`, `views_hex.rs`, `home.rs`, `gutter.rs`, `theme.rs`,
`screenshot.rs` — `gutter_*` helpers sirf read-only call.

## Gap #5

Minimap/overview ruler par click karke us row/hunk par jump:
click-target geometry (ruler rect, row mapping) + bring-into-view,
accessibility label. Bahar se observe kiya gaya pattern apne shabdon
me: dayen kinare ek patli patti poore document ka sankshipt chitra
dikhati hai, us par click karne se mukhya drishya us sthan par kud
jata hai.

## Geometry first (tokens, koi naya magic nahi)

- Text overview ruler rect: `text_minimap_ruler_width(show) x avail_h`
  (`text_gutter_paint_width` ka alias — single source, 12.0/6.0).
- Hex minimap rect: `HEX_MINIMAP_W x HEX_MINIMAP_H` (pehle se, untouched).
- Row mapping pure: `text_minimap_row_at_y(y, height, total)`
  (fraction `y / height -> [0, total)`, kharab input par `None`;
  `gutter_row_at_y` semantics, arg order hex wali).
- Segment floor: `hex_minimap_seg_end` (pehle se, untouched).
- Bring-into-view: text = pending-scroll id + `scroll_to_rect(Center)`
  (pehle se); hex = `pager.bring_into_view` (pehle se). Naya scroll
  path nahi.

## Typography second (label text, koi naya size/weight nahi)

- `text_minimap_accessibility_label(selected, total) -> String`:
  empty par `"overview ruler: empty"`, bina selection par
  `"overview ruler: no selection, click to jump to nearest difference"`,
  selection par `"overview ruler: row N of M, click to jump ..."`.
- `hex_minimap_accessibility_label(selected, total_hunks) -> String`:
  same shape (`"minimap: ..."`).
- Paint me `resp.on_hover_text(label)` — default chrome hover, koi
  naya `TypoRole`/size nahi.

## Colors frozen

Koi naya hue/token nahi. Paint wahi reuse:
`track_for`, `DELETED_BG`/`INSERTED_BG`/`CHANGED_BG`, text tone marker.
Label me color param nahi.

## MS Rust guidelines

- `#![forbid(unsafe_code)]` (file top pehle se).
- Non-test code me `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/
  `unreachable` mana hai; indexing/slicing bhi deny — sirf
  `.get()`, `let ... else`, `is_some_and`, `saturating_*`.
- Clippy `--all-targets` zero warnings, `cargo fmt` sirf OWNS file par.

## DoD

1. Minimap/overview ruler click sahi row/hunk par jump karta hai
   (text: nearest-diff snap + next-frame scroll; hex: covering hunk
   select + pager window).
2. Ruler rect geometry + row mapping + a11y label unit-tested.
3. `cargo test -p autocompare-gui` green (full suite).
4. `cargo clippy -p autocompare-gui --all-targets` zero warnings.
5. `cargo fmt` OWNS file par clean + banned-token grep clean.
6. `tools/gui-polish/run-geo-typo.sh checks/gui-minimap-ap/round-1`
   headless PASS (`RESULT: PASS`).
7. `REPORT.md` + `round-1/` (after.png, geo-typo-report.md,
   tests.out, screenshot.out) repo me.
8. `git add` sirf OWNS paths + `git commit` (conventional English),
   no push, no stash.

## Verification (is order me)

1. `cargo test -p autocompare-gui`
2. `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
3. `cargo fmt -- --check crates/gui/src/views.rs`
4. `grep -nE 'unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!|unsafe' crates/gui/src/views.rs` (non-test hissa clean)
5. `tools/gui-polish/run-geo-typo.sh checks/gui-minimap-ap/round-1`
