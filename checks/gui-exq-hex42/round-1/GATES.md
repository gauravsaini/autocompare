# GATES — exq-hex42 round-1 (written BEFORE implementing per $unlazy)

## Intent (clean-room, own words)
The hex compare view shows an address gutter on the left, sixteen byte
cells with a single midline split after the eighth cell, and an ASCII
strip on the right. Wide windows must show all three side by side with no
horizontal scroll; narrow windows under the 720px breakpoint stack ASCII
under hex on one shared scroll. Round-2 locks the live dump columns: the
offset gutter reuses shared spacing tokens, hex columns align ruler
against grid from char metrics, the ASCII pane keeps one spacing-token
gap, the 1200px canvas holds no h-scroll, and the 700px narrow edge stays
stacked.

## CHECKs (geometry first, typography second, colors frozen)
- CHECK: `tools/exquisite/run.sh checks/gui-exq-hex42/round-1` baseline before edit
  EXPECT: RESULT PASS (PNG >=8192B, magic ok, non-blank); geometry/typo signals reported
- CHECK: offset gutter reuses SPACE tokens only (pad `GEO_TIGHT_GAP`/`SPACE_XS`, gap `GEO_SECTION_GAP`/`SPACE_SM`, cell pad `SPACE_SM`)
  EXPECT: `hex42_offset_gutter_space_ok()` true; no new width or rhythm token
- CHECK: hex dump columns align ruler-vs-grid from char metrics (even 3-char step, shared 1-char midline gap)
  EXPECT: `hex42_columns_align_ok(7.0)` true; false on bad char width
- CHECK: ASCII pane gap is `SPACE_SM` and pane x leaves exactly that gap past the hex outer box
  EXPECT: `hex42_ascii_gap_ok(7.0)` true; false on bad char width
- CHECK: 1200px wide canvas shows all panes with no h-scroll (`SideBySide`, panes total inside grid avail, ASCII end inside canvas)
  EXPECT: `hex42_wide1200_no_hscroll_ok(7.0)` true; fixture `comparison/hex-wide-1200.json` needs_hscroll false
- CHECK: narrow700 edge (700px < 720 breakpoint) stacks, stacked lines fit, 36 visible rows
  EXPECT: `hex_layout_mode(700.0)`==Stacked, `hex42_narrow700_stacked_ok(7.0)` true, `hex31_narrow700_visible_rows()`==36; fixture `comparison/hex-narrow-700.json`
- CHECK: typography reuses `TypoRole` only (Mono grid/ruler/offset/ASCII, Body labels; no new size or token)
  EXPECT: `hex42_typo_roles_ok()` true; no `.size(`, no `Color32`/`Visuals` growth
- CHECK: `cargo test -p autocompare-gui views_hex` + `cargo fmt` + banned scan
  EXPECT: tests PASS incl. new `hex_exq_hex42_live_hex_columns`; `cargo fmt --check` clean; `.unwrap()`/`.expect(`/`panic!`/`todo!`/`unreachable!`/`unsafe` code 0 in non-test code (`forbid` line only)
- CHECK: harness re-run `tools/exquisite/run.sh checks/gui-exq-hex42/round-1`
  EXPECT: RESULT PASS; OWNS only `crates/gui/src/views_hex.rs` + `checks/gui-exq-hex42/**`
