# GATES — exq-hex31 round-1 (write BEFORE implementing per $unlazy)

## Intent (clean-room, own words)
Hex compare shows two synchronized grids: an address ruler on the left,
sixteen byte cells with a visual split after the eighth cell, and an ASCII
strip on the right. Wide windows must show all three side by side with no
horizontal scroll; narrow windows stack ASCII under hex with one shared
scroll. This wave locks that: gutter widths from shared spacing tokens,
row height from the shared mono metric, column alignment from char metrics,
ASCII gap from spacing tokens, plus a 1200px wide no-scroll lock and a
700px narrow stacked edge.

## CHECKs (geometry first, typography second, colors frozen)
- CHECK: `tools/exquisite/run.sh checks/gui-exq-hex31/round-1` baseline before edit
  EXPECT: RESULT PASS (PNG >=8192B, magic ok, non-blank); geometry/typo signals reported
- CHECK: offset gutter width reuses SPACE tokens only (`GEO_TIGHT_GAP` pad, `GEO_SECTION_GAP` gap, `HEX_CELL_PAD_X`==`SPACE_SM`)
  EXPECT: `hex31_offset_gutter_space_ok()` true; no new width/rhythm token
- CHECK: row height reuses `TypoRole::Mono.line_height_px` (15.6)
  EXPECT: `hex31_row_h_mono_ok()` true (`hex_grid_row_h`==mono, `hex_even_row_h`/`hex_offset_row_h` same source)
- CHECK: hex dump columns align ruler-vs-grid (`hex_ruler_col_x`==`hex_grid_byte_col_x` all 16 cols, midline gap exactly 1 char)
  EXPECT: `hex31_columns_align_ok(7.0)` true
- CHECK: ASCII pane gap is `SPACE_SM` and `hex_ascii_pane_x` leaves exactly that gap past hex outer
  EXPECT: `hex31_ascii_gap_ok(7.0)` true
- CHECK: 1200px wide canvas shows all panes with no h-scroll (`SideBySide`, panes total <= grid avail)
  EXPECT: `hex31_wide1200_no_hscroll_ok(7.0)` true; fixture `comparison/hex-wide-1200.json` needs_hscroll false
- CHECK: narrow700 edge (700px < 720 breakpoint) stacks, stacked lines fit, 36 visible rows
  EXPECT: `hex_layout_mode(700.0)`==Stacked, `hex31_narrow700_stacked_ok(7.0)` true, `hex31_narrow700_visible_rows()`==36; fixture `comparison/hex-narrow-700.json`
- CHECK: typography reuses `TypoRole` only (Mono grid/ruler/offset/ASCII, Body labels; no new size/token)
  EXPECT: `hex31_typo_roles_ok()` true; no `.size(`, no `Color32`/`Visuals` growth (stays 38 total per report: 33 in views_hex +5 elsewhere)
- CHECK: `cargo test -p autocompare-gui --lib views_hex` + `cargo fmt` + banned scan
  EXPECT: tests PASS incl. new `hex31_*` tests; `cargo fmt --check` clean; `unwrap()/expect(/panic!/todo!/unreachable!/unsafe/indexing` 0 in non-test code
- CHECK: harness re-run `tools/exquisite/run.sh checks/gui-exq-hex31/round-1`
  EXPECT: RESULT PASS; OWNS only `crates/gui/src/views_hex.rs` + `checks/gui-exq-hex31/**`
