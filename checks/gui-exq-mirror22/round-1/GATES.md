# GATES mirror22 (write BEFORE implementing per $unlazy)

## CHECK: live center gutter width is 20px on LIVE wrap-off path
- CHECK: `text_center_gutter_width()` returns 20.0 and live wrap-off head call-site in `paint_text_body` uses `ui.set_width(text_center_gutter_width())` (views.rs ~3923).
- EXPECT: headless assert `text_center_gutter_width() == 20.0`, column math `text_center_column_x_for_avail` / `hit_band` symmetric around center; no new width constant.

## CHECK: live hunk-head arrows yellow only on changed rows (no pill frame)
- CHECK: live wrap-off + wrap-on head sites call `text_center_hunk_head` gate, label via `text_center_marker_label` (Some only for Changed/Deleted/Inserted, None for Equal), ink via `text_center_marker_fg()` (CHANGED_FG), click via frameless label/button (no `small_button` pill frame).
- EXPECT: Changed/Deleted/Inserted => `Some("=>")`, Equal => None; `text_center_marker_fg() == CHANGED_FG`; multi-row hunk heads only at [2,500,1000]-style indices.

## CHECK: live gutter track continuous full-height (band + floor)
- CHECK: every live center cell (head/continuation/trailing, wrap-on + wrap-off) paints track fill via existing `track_for(dark)` when unselected and `CHANGED_BG` band via `text_center_gutter_band_fill(true)` when selected; trailing blanks use `text_trailing_blank_rows_balanced` floor.
- EXPECT: unselected Equal rows still show 20px track (not transparent); selected rows show CHANGED_BG band; tall-scroll (1200 rows) gutter math stable across scroll windows.

## CHECK: live bottom bar mirrors status20 (per-pane + legend, honest fallback)
- CHECK: live bottom bar painted in views.rs (`paint_text_pane_status` + `paint_text_legend_bar`) remains; per-pane labels via `text_pane_cursor_labels` (L-/R- fallback, no fake numbers) + `text_pane_detail_label` (label-only when preview empty); legend via `text_legend_segments` (4 static segments) + `format_text_legend_line`.
- EXPECT: empty model => L-/R- labels only; legend line has 4 segments; no new colors/sizes.

## CHECK: MS Rust + colors frozen
- CHECK: `cargo test -p autocompare-gui`, `cargo fmt --check`, banned-token grep (`unwrap|expect|panic|todo|unreachable|indexing` non-test delta) empty.
- EXPECT: suite green incl. `mirror22_live_gutter_path`; fmt clean; colors `Color32::from_rgb|Visuals::` == 33.
