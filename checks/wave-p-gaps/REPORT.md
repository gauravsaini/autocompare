# Wave P Gaps Recheck — post-paint delta (report-only)

Date: 2026-11-20 UTC. Repo: `/Users/ektasaini/autocompare`. HEAD: `5eb2a27` (Wave O paint `569dcd4` andar).
Scope: sirf `checks/wave-p-gaps/**`. `crates/**`, STATUS.md untouched. No push. Clean-room: BC bahar se dekha, apne shabdon me, koi string copy nahi.

Baseline: `checks/wave-o-gaps/REPORT.md` (19 rows vs Wave N head `fd7d6c0`), `checks/local-run-l/REPORT.md` (GAP-01..14), `checks/parity-m/REPORT.md` (F1-F5).
Wave O paint me kya hai: `app.rs:93-100,142-149` teen pager + 4 naye find/gutter fields; `app.rs:480` gutter-jump + pager bring-into-view; `app.rs:494,602,738` teen sync_fn; `app.rs:623,793` table/folder paged-find; `views.rs:344` gutter-row parser; `views.rs:375,559,1082` teeno paint me pager param + PageUp/Down + page buttons + visible_range window; `app.rs:2502,2637,2334` hex-gutter / table-find / folder-find boxes. Tests: 211 pass (O me 204 the).

## Delta table (Wave O status -> Wave P status)

| # | Gap (source) | O tha | P ab | Proof (file:line, live behavior) |
|---|---|---|---|---|
| 1 | Draggable splitter (EXQ1/L-01) | OPEN | OPEN | `views.rs:56` half_field_width, `:1263` picture_column_width static; koi draggable nahi |
| 2 | Hex byte-grid + ruler (EXQ2/L-09/M-F1c) | PARTIAL | PARTIAL | model `views_hex.rs` grid math hai; paint `views.rs:375-474` hunk list windowed hai, address+16-byte+ASCII grid + midline gap nahi |
| 3 | Table grid + header (EXQ3/L-10) | PARTIAL | PARTIAL | spec `views_table.rs:228-245` hai; paint `views.rs:559-633` paged hai par single-string row hi, header bar + cell border nahi |
| 4 | Picture slider/zoom/pan (EXQ4) | OPEN | OPEN | `views.rs:1341` column width + summary hi; slider/zoom/pan wiring nahi |
| 5 | Gutter click-jump (EXQ5/L-05/M-F1) | PARTIAL | PARTIAL (sudhra) | FIXED hissa: `app.rs:480` jump + `:484-485` bring_into_view, `views.rs:344` parser, `app.rs:2502-2525` Gutter-row box + Jump + Prev/Next change paged, `views.rs:403,409,417` selection bring_into_view. Bacha: minimap bar par click (`views.rs:914` `Sense::hover` hi, `gutter.rs:133` wiring note) |
| 6 | Folder columns size/time (EXQ6/M-F5) | PARTIAL | PARTIAL | `tree.rs:456` col defs + `:488` cell text model hai; paint `views.rs:1082-1160` + `app.rs:2362` windowed single list hi, size/time column paint nahi |
| 7 | 3-pane merge (EXQ7/L-14) | PARTIAL | PARTIAL | `app.rs:42-44` TriplePaths model; paint `views.rs:499-550` single list + Take idx0, 3-pane + bottom editor nahi |
| 8 | Sync/FolderMerge/Audio action UI (EXQ8) | PARTIAL | PARTIAL | `views.rs:1381` checklist paint hai; apply stub `app.rs:1411-1417` preview-only, real apply nahi |
| 9 | Font zoom Cmd+/- (EXQ9/L-06) | OPEN | OPEN | `theme.rs:55` 12.0 fixed, `:57` MAX 13.0; koi zoom_level/keybinding nahi |
| 10 | Headless PNG 5x7 glyphs (EXQ10) | OPEN | OPEN | `screenshot.rs:599-631` bitmap; layout proof hi, kerning nahi (carry) |
| 11 | Intra-line/cell spans (EXQ11) | PARTIAL | PARTIAL | `views.rs:977-997` pane_segments accents hain; poori-row `CHANGED_FG :626` + table `[old => new]` `:554-555`, per-char bg/weight nahi |
| 12 | Find box + count (EXQ12/L-08/M-F2) | PARTIAL | PARTIAL (sudhra) | FIXED hissa: `app.rs:623` table_find_next_paged + `:641` bring_into_view, `app.rs:2637-2644` table Find box + Next match, `app.rs:2334-2341` folder Find box + Next match, `app.rs:793` folder_find_next_paged. Bacha: text view me box nahi (`views.rs:712-731` F3/Ctrl+N/P hi), koi count/index badge nahi, hex find ka UI nahi (model `views_hex.rs:642,667` hi) |
| 13 | Per-hunk merge pick (EXQ13/M-F3) | PARTIAL | PARTIAL | model `views_hex.rs:708`, `views_table.rs:461`, `tree.rs:1211` hai; paint whole-file Take hi |
| 14 | Paging/virtual scroll (EXQ14/L-12/M-F4) | PARTIAL | FIXED | `views.rs:375` hex pager param + `:392` set_total + `:393-404` PageUp/Down + `:419-425` page buttons + `:443` window; `views.rs:559` table pager + `:568-577` keys + `:583-604` buttons/count + `:605` window; `views.rs:1082` folder pager + `:1096-1118` buttons + `:1135` window; `app.rs:494,602,738` sync fns; test `views.rs:2238-2253` + `app.rs:3468-3550` paged. Bacha sirf badi text-file virtual scroll (MAX_ROWS cap) — alag gap rakho |
| 15 | Progress/cancel (EXQ15) | OPEN | OPEN | channel `app.rs:15` + JobRunner spawn hai; paint me coarse note hi (`views.rs:562-567`), cancel button nahi |
| 16 | Toolbar Prev/Next (L-02) | FIXED | FIXED | `views.rs:627-632` text, `:400-417` hex hunk buttons; working, no regression |
| 17 | Margin copy arrows (L-03) | FIXED | FIXED | `views.rs:746-758` per-row arrows + `:758-770` Section/Line; working, no regression |
| 18 | Header align/line-height/chrome (L-04/07/11) | PARTIAL | PARTIAL | header bar pane split se offset; `views.rs:70` text_row_height default; `theme.rs:131-137` TypoRole + home 14.0 token hai par poori chrome hierarchy nahi |
| 19 | Folder dual-tree (L-13) | OPEN | OPEN | single list hi, synced trees/action menus nahi |

## BC clean-room note

BC instance `/Applications/Beyond Compare.app` (PID 23316) bahar se dekha. L-table ke bahar koi naya P0 nahi dikha. Upar wale OPEN/PARTIAL hi bache hain. Koi string/asset copy nahi.

## Bache hue OPEN (owner wave ke saath)

- OPEN: #1 splitter (GEO), #4 picture zoom/pan (PARITY), #9 font zoom (TYPO), #10 PNG glyphs (INFRA), #15 progress/cancel (PARITY/GUI), #19 dual-tree (PARITY).
- PARTIAL: #2 hex grid (GUI/GEO), #3 table grid (GUI/GEO), #5 minimap-click (GEO), #6 folder cols paint (GUI), #7 3-pane (PARITY), #8 apply wiring (PARITY), #11 intra spans (TYPO), #12 text-find + count badge (GUI/PARITY), #18 header/chrome (GEO/TYPO).

## Top-5 agle fix

1. Text-view find box + count badge (GUI/PARITY): `views.rs:712` F3 loop ke paas TextEdit + `table_find_from` jaisa cursor + `N/M` label; model `table_find_next_cell` reuse.
2. Minimap click-to-jump (GEO): `views.rs:914` hover rect par click/drag + `gutter_geometry` fraction se row idx + `hex_gutter_jump_to_row` reuse.
3. Draggable splitter (GEO): `views.rs:56,1263` static half ke beech drag handle + `App` me split fraction `0.20..=0.80` clamp.
4. Hex byte-grid paint (GUI/GEO): `views_hex.rs` grid math ko `views.rs:443-474` window me address + 8+8 bytes + ASCII teen column me paint karo.
5. Table header + column paint (GUI/GEO): `tree.rs:456-534` defs jaise `table` header bar + cell border/padding `views.rs:559` window me.

## Gates

- `cargo test -p autocompare-gui`: 211 pass, 0 fail (read-only run; O me 204 the, +7 Wave O paint tests).
- `git status`: sirf `checks/wave-p-gaps/**` touched. STATUS.md untouched, no push.
