# Wave M GAPS Report — parity models + patch

Date: 2026-11-19 UTC. Scope: sirf owned files
(`crates/gui/src/views_hex.rs`, `views_table.rs`, `tree.rs`,
`checks/parity-m/**`). `views.rs`/`app.rs`/theme/gutter/home/viewmodel/main/screenshot,
STATUS.md ko chhua nahi. No push. Clean-room own-words; comparable
product bahar se observe kiya, koi string/asset copy nahi.

Sources: `checks/local-run-l/REPORT.md` gap table (#3 nav, #4 gutter,
#10 merge, #15 find) + `checks/parity-l/REPORT.md` F-gaps (paint wiring
OPEN, gap 4/5/7/12-15 age waves ke paas).

## 1. F1 gutter-click jump — FIXED (model) / OPEN (paint, patch me)

Model (headless, no egui/IO):
- `views_hex.rs:563` `hex_hunk_at_offset`, `:581` `hex_next_hunk_from`,
  `:597` `hex_prev_hunk_from`, `:614` `hex_row_to_hunk` (row =
  `hex_row_start(row)` se offset; covering hunk jump target).
- `tree.rs:1071` `folder_next_changed`, `:1091` `folder_prev_changed`,
  `:1120` `folder_gutter_target` (changed flag par next/prev scan,
  wrap ke saath; khali par `None`).
Tests: `views_hex.rs:1242` hex_gutter (2-diff fixture, cover/next/prev/
row-click/wrap/khali), `tree.rs:999` folder_gutter (next/prev/wrap/
click-target/sab-same/khali). OPEN: gutter click se row/index paint
dega (orchestrator). Patch: `INTEGRATION.patch` hunk 3
(`App::hex_gutter_jump_to_row`), hunk 5 (folder next/prev wrappers),
views hunk (row-click wiring note).

## 2. F2 find-box — FIXED (model) / OPEN (paint, patch me)

Model (wrap, khali needle par `None`):
- `views_hex.rs:642` `hex_find_next` (from inclusive), `:667`
  `hex_find_prev` (from exclusive); slice-safe, saturating.
- `views_table.rs:392` `table_find_next`, `:419` `table_find_prev`
  (row-major, cell `contains`; `ignore_case` fold).
- `tree.rs:1144` `folder_find_next`, `:1169` `folder_find_prev`
  (rel_path `contains`; `ignore_case` fold).
Tests: `views_hex.rs:1145` hex_find (wrap dono taraf, khali/badi/
missing), `views_table.rs:815` table_find (case fold, wrap, khali),
`tree.rs:928` folder_find (wrap, case fold, khali). OPEN: text field +
cursor paint, file bytes dena (hex ke liye read wiring) orchestrator
karega. Patch: `table_find_next_cell` assoc fn (hunk 4); hex/folder
seedha owned fn call karte hain, isliye alag wrapper nahi.

## 3. F3 per-hunk merge — FIXED (model) / OPEN (apply wiring, patch me)

Model (pure choice, file apply OPEN):
- `views_hex.rs:708` `HunkSide` (Left/Right), `:733`
  `hex_resolve_hunk` (chosen side bytes; out-of-range skip, no panic).
- `views_table.rs:461` `TableMergeSide`, `:473` `table_resolve_row`
  (missing side par dusri side fallback; dono `None` par khali).
- `tree.rs:1211` `FolderMergeSide`, `:1222` `folder_resolve_row`
  (UpdateBoth rules: `=`/`!`/dir par `None`, baaki CopyToRight/Left).
Tests: `views_hex.rs:1168` (sides, out-of-range, insertion hunk),
`views_table.rs:839` (sides, orphan fallback, dono-None),
`tree.rs:951` (4 glyph directions, same/diff/dir `None`). OPEN:
poori-file apply + dir subtree merge (I/O) orchestrator karega.
Patch: `table_merge_row` (hunk 4); hex/folder seedha owned fn.

## 4. F4 paging — FIXED (model) / OPEN (paint, patch me)

Model (saturating, page size min 1, khali par pages 0):
- `views_hex.rs:751` `HexPager` (`new/page_count/last_top/next_page/
  prev_page/visible_range/bring_into_view/set_total/set_page_size`).
- `views_table.rs:496` `TablePager` (same set, header fixed).
- `tree.rs:1243` `RowPager` (same set, visible nodes).
Tests: `views_hex.rs:1206`, `views_table.rs:863`, `tree.rs:977`
(clamp, wrap-nahi, bring_into_view up/down/visible/khali). OPEN:
PageUp/Down + scrollbar paint, total_rows sync. Patch: 3 pager
fields + Default init (hunk 1-2), paint `App::hex_pager` chalaye
(views hunk note).

## 5. F5 folder size/time columns — CARRIED (parity-l se FIXED)

Koi naya code nahi: `tree.rs` `folder_column_defs` (Path flex + 4
fixed), `folder_cell_text` (`-` fallback), `pad_folder_cell`,
`folder_header_line`, `folder_format_row` parity-l me FIXED hain.
Is wave me sirf verify: `tree.rs` column tests green hain.
OPEN wahi hai jo parity-l ne kaha: paint wiring (views owner).

## 6. Metrics + gates

- `cargo test -p autocompare-gui`: 204 pass, 0 fail (baseline 193
  + 11 naye: F1x2/F2x3/F3x3/F4x3). Full suite green, koi sibling red nahi.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean
  (`?` operator, `#[allow(dead_code)]` test-only pattern par).
- `cargo fmt -p autocompare-gui -- --check`: clean.
- `checks/parity-m/INTEGRATION.patch`: 4294 bytes, 6 hunks (app hunk
  1-5 + views hunk 1). `git apply --check` PASS (HEAD ke against;
  sibling GEO/TYPO paint edits se rebase lag sakta hai — orchestrator
  dekhe). Patch me koi owned-file change nahi, sirf app/views wiring.
- Rounds used: 4/6 (F1 clippy, F2 clippy+fmt, F3 dead-code, F4 clean).
  MS guidelines: `forbid(unsafe_code)` barkarar, saturating ops,
  `.get()` har jagah, test ke bahar no unwrap/expect/panic.

## 7. OPEN (orchestrator ke paas)

- Teeno ki paint wiring: grid/column paint (parity-l F1-F3 ke saath),
  gutter click -> row/index, find text field, merge buttons + apply,
  pager keys/scrollbar. Patch hunks exact call sites dete hain.
- Gap #2/#8/#9/#11 (segment emphasis, weight, syntax, sessions) is
  wave ke bahar hain.
