# Wave Q REPORT (grid-q)

Scope: `crates/gui/src/views.rs` + `checks/gui-grid-q/**` only.
`app.rs` (sibling FIND agent ka hai) untouched, `STATUS.md` untouched, no push.

## Item-wise before/after

### (a) Hex byte grid: address + 8+8 + ASCII + midline gap
- Before: `views.rs:375-474` sirf hunk-list window (`format_hex_hunk_line` + pager),
  byte grid nahi tha.
- After: pure helpers `views.rs:326` (`hunk_covers_offset`), `:346`
  (`format_hex_grid_row`), `:375` (`hex_grid_rows_for_hunk`); paint wiring
  `views.rs:628-652` (visible hunk window se deduped 16-byte rows, cap 64).
- Shape: `0x00000010  ?? .. .. .. .. .. .. ..  .. .. .. .. .. .. .. ..  |....????....|`
  — `format_hex_offset` ruler, 16 cells, 8 ke baad extra space, ASCII col.
- Honest limit: model me byte values nahi (sirf offset/len), isliye covered
  byte `??`/`?` placeholder hai; paint note kehta hai "offsets only".
- Reuse: `HEX_BYTES_PER_ROW`, `format_hex_offset`, `HexPager::bring_into_view`,
  `TIGHT_GAP`, `text_row_height`; koi naya color/token nahi.

### (b) Table header bar + cell borders/padding
- Before: `views.rs:559-633` har row ek single string (`label | text`), header
  bar ya borders nahi the.
- After: pure helpers `views.rs:665` (`split_table_cells`), `:671`
  (`table_header_cells`), `:683` (`format_table_bordered_row`), `:696`
  (`format_table_header_rule`); paint wiring `views.rs:822-849` (header bar +
  `-` rule + bordered data rows `| a | b |`).
- Missing cell `-` (`views_table` convention); widths
  `views_table::table_column_widths` + `pad_table_cell` se (spec 228-245 reuse).
- Reuse: existing `TablePager` window/page buttons unchanged; glyph colors
  unchanged; header bar strong monospace, rule weak — koi naya hue nahi.

### (c) Minimap click/drag -> row -> covering hunk
- Before: `views.rs:914` (ab `:1160`) text minimap `Sense::hover` only — untouched.
- After: pure helpers `views.rs:447` (`hex_minimap_row_at_y`: fraction->row,
  `gutter_row_at_y` jaisa u64 rows ke liye), `:463` (`hex_minimap_jump_to_row`:
  `hex_row_to_hunk` -> selected -> `set_total` -> `bring_into_view`, wahi steps
  jo read-only `App::hex_gutter_jump_to_row` chalata hai); paint `views.rs:584-623`
  (`Sense::click_and_drag` rect, track + `CHANGED_BG` hunk marks, click/drag par jump).
- Koi `app.rs` edit nahi; text hover minimap jaisa tha waisa hai (count 1).

## Gates
- `cargo test -p autocompare-gui`: 218 passed, 0 failed (4 naye tests samet).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean.
- `cargo fmt -p autocompare-gui -- --check`: clean.
- Harness `checks/gui-grid-q/round-Q-1/harness.sh`: RESULT PASS (round 1;
  max 6 me se 1 use hua — pehle test-assert + clippy fix ke baad green).
- Guards: `Color32::from_rgb(` 0, `pub const` 8 (koi naya geometry token nahi),
  owned files ke bahar koi edit nahi.

Commit: local only, no push.
