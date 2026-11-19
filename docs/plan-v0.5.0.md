# Plan v0.5.0 — GUI covers all 9 session types

Method: craft-anything clean-room Study -> Build -> Check -> Ready.
No original code, resources, strings, assets, names or wording are read or copied.
Safe Rust only (`forbid(unsafe_code)`), never crash on hostile input, honest limits.

## Study (HIGH confidence, 2026-11-19)

- Original: Beyond Compare 5.2.6.32774, bundle `com.ScooterSoftware.BeyondCompare`,
  document types folders/disks/all files, settings `.bcpkg`, snapshots `.bcss`.
- Session types observed via the running app's own Session menu (accessibility API):
  Folder Compare, Folder Merge, Folder Sync, Text Compare, Text Merge,
  Hex Compare, Media Compare, Picture Compare, Table Compare.
- autocompare CLI already covers all 9: `text`, `folder`, `hex`, `merge` (= Text Merge),
  `table`, `sync`, `picture`, `folder-merge`, `audio` (= Media Compare).
- Gap: `autocompare-gui` has only Text / Folder / Picture tabs (3/9).
  Hex, Merge, Table, Sync, Folder-Merge, Audio have no GUI surface.

## Build (this release)

Extend the thin GUI client over the same engine, no engine changes:
1. `Tab` enum: Text, Folder, Hex, Merge, Table, Sync, Picture, FolderMerge, Audio.
2. Two-path tabs keep `Paths`; Merge / FolderMerge get `TriplePaths { base, left, right }`.
3. Pure view-model builders in `viewmodel.rs` (headless-tested, no egui, no I/O):
   `build_hex_summary`, `build_merge_summary`, `build_table_rows`,
   `build_sync_summary`, `build_folder_merge_summary`, `build_audio_summary`.
4. Painters in `views.rs`: 9-tab bar, triple-path inputs, generic line list,
   merge view with conflict highlighting, table rows with `[old => new]` cells.
5. `app.rs` runners call the existing `autocompare-compare` / `autocompare-diff`
   functions; errors become the red panel, models clear on failure.

Out of scope: file dialogs (paths stay typed by design), sync/folder-merge apply
buttons (plan/dry-run view only; `--apply` stays CLI), audio playback, heatmaps
beyond Picture tab.

## Check

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (new headless view-model tests for all 6 builders)
- Live runs: each new tab driven via its builder against fixture pairs;
  CLI/GUI wording matches (`first difference at`, `[old => new]`, `step(s) planned`).

## Ready

- Conventional commits: `feat(gui): ...`, `docs: ...`, `chore: ...`
- Update README (GUI tab list), `docs/study.md` (GUI parity note), `TIMELINE.md` (v0.5.0 entry)
- Push `main` to `origin` (https://github.com/gauravsaini/autocompare)
