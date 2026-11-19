# Wave O Gaps Recheck — report-only

Date: 2026-11-20 UTC. Repo: /Users/ektasaini/autocompare. HEAD: `fd7d6c0` (Wave N `d8126f4` applied).
Scope: sirf `checks/wave-o-gaps/**`. `crates/**`, STATUS.md untouched. No push. Clean-room: BC bahar se dekha, apne shabdon me, koi string copy nahi.

Baseline: `checks/local-run-exq/REPORT.md` (15 gaps), `checks/local-run-l/REPORT.md` (GAP-01..14), `checks/parity-m/REPORT.md` (F1-F5), `checks/local-run-orch/round-orch-1/geo-typo-report.md` (PASS).

Wave N HEAD me kya hai: `app.rs:93-95,137-139` teen pager fields; `app.rs:470` `hex_gutter_jump_to_row`; `app.rs:535` `table_find_next_cell`; `app.rs:618-629` folder next/prev wrappers; `views.rs:365` pager comment (consume nahi); `views.rs:627-632` text Prev/Next toolbar; `views.rs:746-758` per-row copy arrows; `theme.rs` GEO/TYPO tokens. Tests: 204 pass.

## Status table (FIXED = paint me working, PARTIAL = model hai paint pending, OPEN = kuch nahi)

| # | Gap (source) | Status | Evidence (file:line) | Owner (OPEN/PARTIAL ke liye) |
|---|---|---|---|---|
| 1 | Draggable splitter (EXQ1/L-01) | OPEN | `views.rs:198` half_field_width, `:1171` picture_column_width static; koi draggable nahi | GEO |
| 2 | Hex byte-grid + ruler (EXQ2/L-09/M-F1c) | PARTIAL | model `views_hex.rs` grid math + `hex_row_to_hunk`; paint `views.rs:371-390` list hi hai | PAINT-O (current) |
| 3 | Table grid + header (EXQ3/L-10) | PARTIAL | spec `views_table.rs:228-245`; paint `views.rs:468-476` single-string row | PAINT-O |
| 4 | Picture slider/zoom/pan (EXQ4) | OPEN | `views.rs:948-961` summary + heatmap hi | PARITY |
| 5 | Gutter click-jump (EXQ5/L-05/M-F1) | PARTIAL | `app.rs:470` jump fn, `app.rs:618-629` folder wrappers; paint `views.rs:800` `Sense::hover` hi, click nahi | PAINT-O |
| 6 | Folder columns/size-time (EXQ6/M-F5) | PARTIAL | `tree.rs` column defs; rewire pending `views.rs:37`, paint `views.rs:953` single list | PAINT-O |
| 7 | 3-pane merge (EXQ7/L-14) | PARTIAL | `TriplePaths app.rs:42-44`; `paint_merge views.rs:414` single list, Take idx0 par | PARITY |
| 8 | Sync/FolderMerge/Audio action UI (EXQ8) | PARTIAL | `paint_folder_merge_checklist views.rs:1381` hai; apply stub `app.rs:1411-1417` preview-only | PARITY |
| 9 | Font zoom Cmd+/- (EXQ9/L-06) | OPEN | `theme.rs:55` 12.0 fixed, `TYPO_MONO_SIZE_MAX 13.0`; koi zoom_level nahi | TYPO |
| 10 | Headless PNG 5x7 glyphs (EXQ10) | OPEN | `screenshot.rs:599-631` bitmap; layout proof hi, kerning nahi (carry) | INFRA |
| 11 | Intra-line/cell spans (EXQ11/L-GAP intra) | PARTIAL | accents hain, per-cell span nahi `views.rs:401-404` poori row CHANGED_FG | TYPO/PAINT-O |
| 12 | Find box + count (EXQ12/L-08/M-F2) | PARTIAL | models `app.rs:535`, hex/tree find fns; UI nahi, sirf F3/Ctrl+N/P `views.rs:584-594` | PAINT-O |
| 13 | Per-hunk merge pick (EXQ13/M-F3) | PARTIAL | `views_hex.rs:708-733`, `views_table.rs:461-473`, `tree.rs:1211-1222` models; paint whole-file | PARITY |
| 14 | Paging/virtual scroll (EXQ14/L-12/M-F4) | PARTIAL | `app.rs:93-95,137-139` pagers, zero paint use (rg empty); notes `views.rs:441,473,563` | PAINT-O (current) |
| 15 | Progress/cancel (EXQ15) | OPEN | channel `app.rs:16`; sirf coarse note `views.rs:562-567` | PARITY |
| 16 | Toolbar Prev/Next (L-02) | FIXED | text `views.rs:627-632`, hex `views.rs:351-357` working buttons | — |
| 17 | Margin copy arrows (L-03) | FIXED | per-row arrows `views.rs:746-758` + Section/Line `views.rs:631-640`, `copy_line` wired | — |
| 18 | Header align / line-height / chrome (L-04/07/11) | PARTIAL | header bar pane split se offset; `text_row_height views.rs:71` default; `TypoRole theme.rs:131-137` + home 14.0 token hai | GEO/TYPO |
| 19 | Folder dual-tree (L-13) | OPEN | single list hi, synced trees/action menus nahi | PARITY |

## BC clean-room note

BC instance `/Applications/Beyond Compare.app` bahar se dekha; L-table ke bahar koi naya P0 nahi dikha. Upar ke OPEN/PARTIAL hi bache hain. Koi string/asset copy nahi.

## Gates

- `cargo test -p autocompare-gui`: 204 pass, 0 fail (read-only run).
- `git status`: sirf `checks/wave-o-gaps/**` touched. STATUS.md untouched, no push.
