# Parity Gaps Report (Wave I agent B)

Date: 2026-11-19 UTC. Repo: /Users/ektasaini/autocompare.
Scope: gaps 5/6/12/13/14 from checks/local-run-exq/REPORT.md.
Owns: crates/gui/src/gutter.rs, viewmodel.rs, tree.rs, checks/parity-gaps/**.
Zero paint changes: no egui wiring, views.rs untouched (agent A owns paint).
Clean-room: bahar se observation only, koi string/asset copy nahi.

## 1. Per-feature counts

| Gap | Feature | New pure API (viewmodel/gutter/tree) | Tests |
|-----|---------|--------------------------------------|-------|
| 5 | gutter click-to-jump | gutter.rs:128 gutter_row_at_y, gutter.rs:144 gutter_segment_at_y, gutter.rs:170 gutter_jump_target | 4 |
| 12 | find box | viewmodel.rs:1300 find_match_indices, viewmodel.rs:1321 FindState (query/matches/cursor, next/prev wrap) | 3 |
| 13 | merge per-hunk pick | viewmodel.rs:1409 MergeHunk, viewmodel.rs:1422 merge_conflict_hunks, viewmodel.rs:1459 MergeHunkPick, viewmodel.rs:1470 apply_merge_picks, viewmodel.rs:1499 MergePickSummary, viewmodel.rs:1513 merge_pick_summary | 4 |
| 14 | truncation paging | viewmodel.rs:1542 PAGE_SIZE (5000), viewmodel.rs:1560 page_window, viewmodel.rs:1547 PageCursor (window/load_more/retarget) | 3 |
| 6 | folder columns | tree.rs:425 FolderColumnDef, tree.rs:436 FolderColumn (Path/LR-size/LR-time), tree.rs:488 folder_cell_text, FOLDER_SIZE_CHARS=10, FOLDER_TIME_CHARS=16 | 3 |

New tests B scope: 17 (gutter 5->9, viewmodel +10, tree 21->24).
Suite: baseline 125 -> 172 green final (delta +47 total;
+17 B scope, baaki sibling agents ke parallel landings).
Indent token: tree_indent_step 12.0 (SPACE_MD) frozen, colors frozen.

## 2. Wiring points (agent A / orchestrator)

1. Gutter click: views.rs gutter paint me click y par
   gutter_jump_target(&rows, total_rows, height, y) bulao, mile row par
   selection/scroll le jao. Segment highlight ke liye
   gutter_segment_at_y use karo.
2. Find box: har text view me ek FindState rakho; box edit par
   set_query(&rows, &q), Enter/Shift+Enter par next()/prev() aur row
   scroll; match_count()/position() se "n of m" label banao.
3. Merge cards: merge_conflict_hunks(&lines) se ek card per hunk,
   har card me Left/Right/Base/Both buttons MergeHunkPick stage kare;
   Apply-all par apply_merge_picks, summary line par merge_pick_summary.
   Single-hunk resolve resolve_take par hi rahe.
4. Paging: MAX_ROWS note wali har view me ek PageCursor rakho;
   page buttons page_window, load-more button load_more() chalaye.
   Paint window ab bhi model.rows slice kare, cursor sirf (start,end).
5. Folder columns: visible har node ke liye folder_column_defs() order
   me folder_cell_text(&node.row, col) paint karo; Path cell par
   tree_indent_for_depth(node.depth) offset lagao.

## 3. Limitations (6)

1. Gutter jump nearest-diff snap karta hai, click wali exact equal row
   par kabhi nahi rukta; pixel-perfect row jump ke liye paint ko
   gutter_row_at_y seedha bhi expose hai.
2. Find matching case-insensitive substring hai; regex/whole-word
   support nahi; har edit par O(rows) rescan hota hai.
3. Per-hunk pick sirf well-formed marker blocks par kaam karta hai;
   malformed (bina >>>>>>>) block skip hota hai, resolve_take
   MalformedConflict deta hai.
4. Paging cursor sirf (0, shown) prefix window track karta hai;
   arbitrary page jump paint ko page_window se khud slice karna padta hai.
5. Folder columns char-width spec hai, px measure nahi; mono paint me
   align hoga, proportional font me A ko measure karna hoga.
6. Ye models headless hain: B scope me koi paint/egui wiring nahi;
   A ke wiring bina user ko koi visible change nahi dikhega.

## 4. Gates

- cargo test -p autocompare-gui: final 172 passed, 0 failed
  (gutter 9, viewmodel 50, tree 24).
- clippy owned clean: gutter/viewmodel/tree me zero warnings
  (sort_by_key fix; wiring-pending items par targeted allow(dead_code)).
- fmt owned clean: cargo fmt --check me owned files me zero diff
  (sirf sibling-owned app.rs/views.rs me unke diffs hain).
- Disjoint hold: app.rs/main.rs/views*.rs/STATUS.md untouched by B.
