# Parity-Picture Report (Wave J-B, agent B)

Date: 2026-11-19 UTC. Repo: /Users/ektasaini/autocompare. Scope: sirf
`crates/gui/src/app.rs`, `crates/gui/src/views.rs` edit hue;
`checks/parity-picture/**` me ye report. Gap 4 (picture: summary-lines only)
aur gap 8 (sync/folder-merge/audio: plain summary lines) dono band.

## 1. Kya bana

- Picture run path structured hai: `PictureCompareModel::from_diff`
  diff fields + CLI-worded lines rakhta hai; view params alag
  `PictureViewState` me (mode/overlay/zoom/pan, sab clamped).
- Sync/folder-merge/audio run paths per-file checklist models dete hain
  (`SyncChecklist`, `FolderMergeChecklist`, `AudioChecklist`), plain
  `Vec<String>` summary khatm.
- Picture paint model-driven hai: side-by-side columns ya overlay slider,
  zoom slider, pan nudge + reset, phir zoomed heatmap aur summary lines.
- Sync/merge/audio paint per-file checkbox rows paint karte hain
  (glyph + path + action label), phir skips/conflicts aur counts line.
- Spacing gutter/tree tokens se (`TREE_INDENT`, `SECTION_GAP`,
  `TIGHT_GAP`); colors frozen diff palette se, koi naya hue nahi.

## 2. File:line refs

Models (`crates/gui/src/app.rs`):

- `PictureMode` (all/label): 1095
- `PictureViewState` (clamped setters, ratio/factor, describe): 1121
- `PictureCompareModel::from_diff`, `status_summary`: 1200
- `SyncCheckRow` (glyph/label/display_line): 1269
- `SyncChecklist::from_plan`, `summary`, toggles: 1310
- `FolderMergeCheckRow` (glyph/label/is_conflict): 1378
- `FolderMergeChecklist::from_plan`, `summary`: 1429
- `AudioCheckKind`/`AudioCheckRow`/`AudioChecklist::from_diff`: 1494/1506/1519
- `App` picture setters (`set_picture_mode`, `set_picture_overlay`,
  `set_picture_zoom`, `nudge_picture_pan`, `reset_picture_view`): ~1600
- Run paths (`run_sync`, `run_picture`, `run_folder_merge`, `run_audio`):
  965/982/1002/1019; paint arms: ~2340

Paint (`crates/gui/src/views.rs`):

- `picture_column_width`, `clamp_overlay_mix`, `clamp_zoom_pct`: 1088/1093/1098
- `zoom_label`, `overlay_label`, `pan_label`: 1103/1109/1118
- `picture_status_line`, `picture_display_size`: 1124/1134
- `paint_picture_compare`: 1145
- `sync_row_text`, `sync_skip_text`, `sync_row_color`: 1284/1290/1296
- `paint_sync_checklist`: 1308
- `folder_merge_row_color`, `paint_folder_merge_checklist`: 1360/1376
- `audio_row_text`, `audio_row_color`, `paint_audio_checklist`:
  1434/1445/1461

## 3. Tests

- `cargo test -p autocompare-gui`: 189 passed, 0 failed.
- Delta: Wave-I poll-24 baseline 172 thi; is checkout par pre-change
  175 mili (beech me +3 land hue). Is change ke baad 189 (+14, sab
  isi wave ke: app.rs me 8, views.rs me 6).
- Naye tests: picture view clamps/describe, model-from-diff mapping,
  view setters + swap reset, picture status line, sync rows/toggles,
  sync status line, merge conflicts, audio verdicts (app.rs);
  column floor, slider clamps/labels, status/display-size, sync
  text/colors, merge colors, audio text/colors (views.rs).

## 4. Gates

- `cargo test -p autocompare-gui`: green (189/0).
- `cargo clippy -p autocompare-gui --all-targets`: owned files
  (app.rs, views.rs) me zero warnings. 3 warnings `viewmodel.rs`
  me hain (`build_sync_summary`, `build_folder_merge_summary`,
  `build_audio_summary` ab koi caller nahi rakhta) — ye file owned
  nahi hai, isliye chhui nahi; owner allow ya reuse kare.
- `rustfmt --check` owned files par clean.
- Koi `unwrap`/`expect`/`panic` non-test code me nahi; koi nayi
  dependency nahi; `forbid(unsafe_code)` barkarar.

## 5. Limitations (6)

1. Overlay slider sirf model value badalta hai; do images ka asli
   pixel-blend paint nahi hota — heatmap frame hi dikhta hai.
2. Zoom heatmap ko scale karta hai, par side-by-side columns me
   left/right source images paint nahi hoti (heatmap-only display).
3. Pan positive offsets par scroll karta hai; negative pan sirf readout
   me dikhta hai, image shift nahi karta.
4. Checklist checkboxes selection stage karte hain; Apply ab bhi CLI
   operation hai — GUI se koi filesystem write nahi hota (dry-run).
5. `viewmodel.rs` ke 3 summary builders ab dead hain (clippy warn,
   §4); hatana ya reuse karna owner ke scope me hai.
6. Headless PNG harness me naye paint paths render nahi hue; paint
   coverage unit-tested mapping tak hai, pixel-proof tak nahi.

Gates: tests green, owned clippy/fmt clean. Commit locally, NO push.
