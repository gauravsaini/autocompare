# REPORT — Wave BK-SYNCWIRE (Folder-tab sync wiring, P0 BI-S1)

Spec: `checks/gui-mergesync-bi/REPORT.md` §spec BI-S1. `paint_folder_sync`
(`views.rs:4657-4815`, dead tha — Folder tab ise call nahi karta tha, sirf
inline + `paint_folder_dual` paint hota tha) ab Folder tab me wired hai: mode +
`FolderSyncSelection` + expanded + pager pass hota hai, taaki sync badges +
selection live Folder view me dikhen. AQ model + AR paint pehle land ho chuke
the; yeh wave sirf App-side rewiring hai. `views.rs` me ZERO edit.

## Kya badla (sirf OWNS)

1. **`crates/gui/src/app.rs` (Folder tab branch only, ~26 lines add)**:
   dual-tree block ke neeche naya BI-S1 block — `folder_dual_rows()` (filtered
   rows, order same) + `folder_counts_summary()` + `folder_session.sync_mode`
   copy karke locals me, phir `paint_folder_sync(ui, &sync_rows,
   sync_summary.as_str(), sync_mode, &mut folder_sync_selection, &mut
   folder_session.expanded, &mut folder_pager)`. Call ke baad
   `folder_expanded = folder_session.expanded.clone()` mirror (dual block
   wala precedent). Mode radios / dry-run / consent / Apply wala neeche ka
   preview section untouched hai — `paint_folder_sync` uska replacement nahi,
   badge+selection grid ka live roop hai.
2. **Borrow order**: pehle immutable copies (rows/summary/mode — `SyncModeChoice`
   `Copy` hai), phir ek scope me teen disjoint `&mut` (selection, expanded,
   pager), scope band karke mirror. Koi signature change nahi maanga gaya,
   isliye `views.rs` callsite threading ki zaroorat nahi padi.
3. **`checks/gui-syncwire-bk/`**: GATES.md + yeh REPORT + round-1
   (`after.png`, `after-folder.png`, `after-sync.png`, `geo-typo-report.md`,
   `tests.out`, `screenshot.out`).

## Kya nahi chhooa (KEVL)

- `views.rs`: koi paint edit nahi, signature bhi unchanged (diff me nahi).
  Working tree me dikh raha `views.rs` diff parallel agent (BI-M2 three-way
  paint) ka hai — read bhi zaroorat se zyada nahi kiya, commit me shaamil nahi.
- `tree.rs` / theme / screenshot / views_hex / views_table: read-only.
- `checks/opencode-waves/STATUS.md`: UNTOUCHED.
- Colors: koi hue/token baat nahi (hunk me koi color API nahi).

## Evidence

- `cargo test -p autocompare-gui`: **595 passed, 0 failed** (`round-1/tests.out`).
- `cargo clippy -p autocompare-gui --all-targets`: `app.rs` me **zero warning**;
  2 doc warnings (`views.rs:5718-5719`) parallel agent ke uncommitted diff ke
  hain, mere commit se bahar.
- `cargo fmt --check -p autocompare-gui`: exit 0 (koi reformat nahi; bare
  `cargo fmt` chalaya hi nahi).
- Banned scan (mere hunk par `unwrap|expect|panic|todo|unsafe|Color32|Visuals`):
  **ZERO new**. `#![forbid(unsafe_code)]` (`main.rs:12`) intact.
- Harness: `bash tools/gui-polish/run-geo-typo.sh
  checks/gui-syncwire-bk/round-1` → EXIT 0, `RESULT: PASS`
  (`round-1/geo-typo-report.md`).
- PNGs (dispatcher filename-route se, teeno alag scenes):
  `after.png` 9695 B distinct 255, `after-folder.png` 10068 B distinct 256,
  `after-sync.png` 10732 B distinct 256 — teeno `file` me `PNG image data, 960
  x 600, RGBA`, magic `89504e470d0a1a0a` ok, >= 8192 ok, non-blank ok. Teen
  alag sizes = koi fallback-copy nahi (AW2 wali `picture==text` class ka bug
  yahan nahi).

## Behavior (pehle vs ab)

- Pehle: Folder tab me per-row sync badge (`=>`, `<=`, `!`, `=` jaisa
  `sync_badge_for_paint`) + click-to-select + `N selected: <line>` summary
  kahin live nahi dikhta tha (dead paint).
- Ab: Folder tab me dual panes ke neeche sync grid live hai — glyph + badge +
  path/sides columns, click se selection toggle, neeche selected summary line.
  Mode switch (`set_folder_sync_mode`) consent reset karta hai (pehle se tha),
  badges `sync_direction_for_row(row, mode)` se mode-following hain.
- AQ (`FolderSyncSelection`, `select/toggle/clear`, `sync_preview_for_selection`)
  + AR (`paint_folder_sync`, `folder_sync_selected_line`,
  `folder_sync_header_cells`) ka headless contract ab live paint se jud gaya.

## Files (sirf OWNS)

- `crates/gui/src/app.rs` (Folder tab branch block only)
- `checks/gui-syncwire-bk/GATES.md` (yeh wave ke gates)
- `checks/gui-syncwire-bk/REPORT.md` (yeh file)
- `checks/gui-syncwire-bk/round-1/after.png`
- `checks/gui-syncwire-bk/round-1/after-folder.png`
- `checks/gui-syncwire-bk/round-1/after-sync.png`
- `checks/gui-syncwire-bk/round-1/geo-typo-report.md`
- `checks/gui-syncwire-bk/round-1/tests.out`
- `checks/gui-syncwire-bk/round-1/screenshot.out`

## Commit

- Message (English conventional): `feat(gui): wire folder sync badges into folder tab`
- Local only, NO push. Pathspec sirf OWNS (views.rs bahar).
