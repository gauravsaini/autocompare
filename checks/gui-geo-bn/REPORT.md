# REPORT: BN-GEO folder+text geometry tight (BC parity)

Round: checks/gui-geo-bn/round-1/ -> RESULT:PASS (1 round, no repeat needed)
Harness: PNG 9695 bytes (min 8192 ok), magic ok, distinct 255 (ok)
Signals: Grid x10, text_style_height x4, add_space literals [] (sab token reuse), widths 25 calls

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Header (paint_header) | 2 rows: title row (28px) + alag note label line, bottom gap 6 | 1 compact row: title + small one-line hint same row me, mode toggle right, gap 4 |
| Tab strip (paint_tabs) | horizontal_wrapped (wrap ho sakta tha), bottom gap 6 | single horizontal flow, item_spacing 4, bottom gap 4 |
| Path bar (paint_paths) | 2 rows: fields row + alag Compare row, gap 6 | 1 row: left field + Browse + handle + right field + Browse + Compare, min height 28, gap 4 |
| Triple path bar (paint_triple_paths) | 2 rows: 3 fields + browses, phir alag Compare row, gap 6 | 1 row: 3 fields + browses + Compare, min height 28, gap 4 |
| Text toolbar (paint_text_body) | horizontal_wrapped (double-row clutter) | single horizontal, item_spacing 4 |
| Find bars (paint_text_find_bar, paint_text_find_state_bar, table key/find) | horizontal_wrapped | single horizontal, item_spacing 4/8 |
| Error panel (paint_error) | horizontal_wrapped | single horizontal, item_spacing 4 |
| Folder grid (paint_folder) | Grid tha par per-row set_row_height missing (row overlap risk) | Grid + set_row_height(row_height) har row par, row_height = row_height_for_line(text_row_height(ui)) |
| Sync grid (paint_folder_sync) | Grid tha par per-row set_row_height missing | Grid + set_row_height(row_height) har row par |
| Table grid (paint_table) | Grid tha par per-row set_row_height missing | Grid + set_row_height(row_height) har row par |
| ScrollArea ids | 11 ScrollArea bina id ke (ek Compose me folder single + dual + sync = 3 ScrollArea clash -> debug red overlap P0) | 11/11 par unique id_salt (plan_lines_rows, hex_hunk_rows, merge_output_rows, table_single_rows, table_cell_rows, text_wrap_list, text_diff_rows, folder_tree_rows, folder_sync_rows, folder_dual_rows, picture_heatmap_frame) |

Colors: FROZEN, zero Color32/Visuals diff. Typography: sirf layout-ke-liye (hint small one-line, existing TypoRole reuse, koi naya .size nahi).

## Exact lines changed

crates/gui/src/views.rs (50 insert, 35 delete ke hisse me se):
- paint_header (~367-393): hint label second line hata kar same-row small hint, TAB_BOTTOM_GAP -> TIGHT_GAP
- paint_tabs (~511-526): horizontal_wrapped -> horizontal + item_spacing TIGHT_GAP, gap 6 -> 4
- paint_paths (~545-603): Compare button same row me merge, set_min_size HEADER_H, gap 6 -> 4
- paint_triple_paths (~614-692): Compare button same row me merge, set_min_size HEADER_H, gap 6 -> 4
- paint_lines (~702): id_salt plan_lines_rows
- paint_hex hunk list (~1287): id_salt hex_hunk_rows
- paint_merge lines (~1849): id_salt merge_output_rows
- paint_table single fallback (~2142): id_salt table_single_rows
- paint_table grid (~2167): id_salt table_cell_rows + loop me set_row_height
- paint_error (~2290): horizontal_wrapped -> horizontal + item_spacing
- paint_text_body toolbar (~2419): horizontal_wrapped -> horizontal + item_spacing; wrap list (~2552) id_salt text_wrap_list; rows list (~2738) id_salt text_diff_rows
- paint_folder grid (~4399/4411): id_salt folder_tree_rows + loop me set_row_height
- paint_folder_sync grid (~4771/4783): id_salt folder_sync_rows + loop me set_row_height
- paint_folder_dual (~5015): id_salt folder_dual_rows (set_row_height pehle se tha)
- paint_picture_compare (~5451): id_salt picture_heatmap_frame
- paint_text_find_bar (~5907), paint_text_find_state_bar (~5969): horizontal_wrapped -> horizontal + item_spacing

crates/gui/src/views_table.rs (6 lines):
- paint_table_key_bar: horizontal_wrapped -> horizontal + item_spacing TABLE_KEY_COL_GAP
- paint_table_find_bar: horizontal_wrapped -> horizontal + item_spacing TABLE_KEY_COL_GAP

## Gates status (sab green)

- cargo test -p autocompare-gui: 607 passed, 0 failed
- cargo clippy --all-targets -- -D warnings: exit 0
- rustfmt --edition 2024 --check (sirf OWNS rs files): clean
- banned grep (unwrap()/expect(/panic!/todo! non-test): zero
- colors grep (Color32::from_rgb / Visuals:: new): zero
- PNG magic+size+distinct: ok (9695B, 255 distinct)
- STATUS: untouched (git status clean)

## P0/P1 follow-up (dusre owners ke liye, is wave me NO-TOUCH tha)

- P0 (app.rs owner): live Folder tab ab bhi app.rs inline single-list paint karta hai (glyph + "< path" per row, Grid nahi) + uske baad paint_folder_dual + paint_folder_sync bhi chalte hain -> ek screen par 3 folder lists + 3 ScrollArea dikhte hain. views.rs me Grid + id_salt ready hai; App ko single-list hata kar paint_folder (Grid) par rewire karna hai taaki "< f.txt" wali loose rows khatam hon.
- P0 (app.rs owner): app.rs folder find/page toolbar ab bhi horizontal_wrapped hai (double-row wahi se aata hai live me) -> single horizontal par lao.
- P1 (screenshot.rs owner): fixture PNG manual x offsets (TEXT_LEFT_X / GUTTER_X consts) par hai; views.rs Grid geometry se judvaan check karo taaki fixture live se drift na kare.
- P1 (orchestrator): triple path bar ek row me tight hai par narrow window par clip ho sakta hai -> min-width scroll ya wrap-back decision chahiye (BC parity ke hisab se).
