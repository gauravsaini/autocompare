# REPORT: Wave AO-SNAPSHOT snapshot parity

Scope: `crates/gui/src/screenshot.rs` OWNS only. KEVL files untouched (no read).
Colors frozen, geometry first, typography second.

## Kya kiya

- Dispatcher: filename lowercase me `hex`/`table`/`merge`/`folder`/`home` route, warna text. `foldermerge` -> `merge` (merge family). Pehle sirf folder/text tha.
- Naye fixtures (same 960x600 chrome, 24px rows, frozen hues):
  - hex: do 460px pane + gutter, 8 rows offset+hex+ascii, 1 changed row
  - table: single 936px pane, header + 14 rows, column guides 200/400/600/760
  - merge: 3 equal panes 309px (12/325/638, gap 4), 7 rows base/left/right
  - home: 3 cards 304px (gaps 12, total 960) + recent list 5 rows
- Shared helpers: `paint_chrome` (title 36px + tab 32px + path row + toolbar) + `paint_status` (30px strip y570). Public consts: SNAP_W/H, ROW_H, OUTER_PAD, PANE_GAP, TEXT_PANE_W/X, GUTTER_X/W, MERGE_PANE_W/X0/X1/X2.
- Tests (9 naye): 4 fixture PNG, dispatcher_all_tabs (6 alag bytes + foldermerge==merge), geometry_shared_tokens, merge_equal_thirds, home_cards_fit.

## Geometry (before -> after)

| Token | Before | After |
|---|---|---|
| Canvas | 960x600 (text/folder me hardcoded) | 960x600 via SNAP_W/H consts, sab fixtures |
| Row height | 24 ad-hoc | ROW_H 24 const, sab fixtures + tests |
| Text panes | x12 w460 + x476 w460, gutter x940 w14 (implicit) | TEXT_LEFT_X/RIGHT_X/PANE_W + GUTTER_X/W consts, test me gap 4 prove |
| Merge panes | n/a | 309/309/309 equal thirds, gaps 4, test me prove |
| Home cards | n/a | 304*3 + 12*2 + 12*2 = 960, test me prove |
| Padding | 12/76/380 etc scattered | OUTER_PAD 12 + paint_chrome shared, widths text_width se fit prove |
| Colors | frozen | zero delta (sirf purane BG/FG/ACCENT reuse, koi naya hue nahi) |

## Verification

- `cargo test -p autocompare-gui screenshot`: 16 passed
- `cargo test -p autocompare-gui`: 431 passed, 0 failed
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: exit 0, screenshot.rs hits 0
- `cargo fmt -- crates/gui/src/screenshot.rs --check`: exit 0
- banned grep: unwrap(/expect(/panic!/todo!/unimplemented!/unreachable! hits 0; unsafe sirf forbid line me
- `tools/gui-polish/run-geo-typo.sh checks/gui-snapshot-ao/round-1`: RESULT: PASS (9624 bytes, magic ok, distinct 256)
- round-1 PNGs: text 9624, folder 10068, hex 10030, table 9973, merge 8829, home 9011 — sab magic ok, >= 8192

## Notes

- KEVL files ko chhua nahi, read bhi avoid kiya; naye fixtures self-contained hain (folder fixture ke purane tree/home importsurbed nahi).
- Non-test code me koi unwrap/expect/panic/todo/unsafe/indexing nahi; sab saturating + get_mut + map_err.
