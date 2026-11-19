# GATES — Wave AR-SYNCPAINT

Wave: AR-SYNCPAINT. OWNS: `crates/gui/src/views.rs`, `checks/gui-syncpaint-ar/**`.
KEVL (mat chhoona, read bhi avoid): `app.rs`, `viewmodel.rs`, `tree.rs`, `views_table.rs`, `views_hex.rs`, `home.rs`, `gutter.rs`, `theme.rs`, `screenshot.rs` — sirf folder sync model (`SyncModeChoice`, `sync_badge_for_row`, `sync_direction_for_row`, `FolderSyncSelection`, `sync_preview_for_selection`, `summarize_sync_preview`, `format_sync_summary`, `sync_badge_min_px`, `sync_badge_typo`, `sync_summary_typo`) read-only use ok.

Clean-room: Beyond Compare ko sirf bahar se observe kiya, apne shabdon me likha. Koi string copy nahi.

Order: Geometry first, typography second, colors frozen (koi naya hue nahi; selection `selectable_label` precedent par, badge weight se farak).

Scope: folder rows me sync direction badges + selection highlight ka paint adoption — `checks/gui-foldersync-aq/REPORT.md` precedent (model AQ me bana, paint is wave me). `app.rs` call sites KEVL hain, isliye existing `paint_*` signatures unchanged; naya additive painter + pure helpers.

## Gates

1. Badge source single: `sync_badge_for_paint` sirf `tree::sync_badge_for_row` ka thin wrapper — har glyph x dono modes par model se parity (test-locked). Paint me badge text ka koi duplicate literal nahi.
2. Geometry reuse, koi naya token nahi: badge column width = `tree::sync_badge_min_px()`, grid columns = `folder_grid_columns()+1`, row rhythm `row_height_for_line`, page bar `HEADER_H`, gaps `SECTION_GAP`/`TIGHT_GAP`, indent `TREE_INDENT`.
3. Typography mapping second: badge mono caption via `tree::sync_badge_typo` (sirf delete strong, baaki regular), summary mono caption via `tree::sync_summary_typo`, header mono weak (existing). Koi naya size nahi, koi `.color()` nahi.
4. Selection highlight: path cell `selectable_label` (dual painter precedent), click se `FolderSyncSelection` toggle (sirf real rows paint hote hain, stale path kabhi nahi). Neeche selected summary line (`N selected: <tree::format_sync_summary line>`).
5. Additive paint: naya `paint_folder_sync` (rows + summary + mode + selection + expanded + pager); `paint_folder`/`paint_folder_dual` signatures untouched taaki KEVL `app.rs` call sites na tooten. App rewiring (mode+selection pass) orchestrator ke paas pending — REPORT me note.
6. Tests: naye headless unit tests (badge parity, width lock, grid/header shape, delete-strong-only, sizes, selection probe). Full `cargo test -p autocompare-gui` green.
7. Quality gates: `cargo clippy -p autocompare-gui --all-targets` zero warnings, `cargo fmt -- <sirf OWNS files>` ke baad `cargo fmt --check` clean, banned grep clean (non-test me `unwrap(|expect(|panic!|todo!|unimplemented!|unsafe` nahi), `tools/gui-polish/run-geo-typo.sh` headless PASS.
8. Note: tree me parallel agent ka uncommitted `home.rs` WIP abhi build tod raha hai (missing `saved_at_secs`) — KEVL hai isliye fix nahi karunga; verify uske green hote hi retry hoga, REPORT me blocker darj rahega.

## DoD

- Badges/selection paint dikhte hain (naya painter + tests), full gui tests green, harness PASS, commit (no push).

## Commands

- `cargo test -p autocompare-gui`
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
- `cargo fmt -- crates/gui/src/views.rs && cargo fmt --check`
- `rg -n "unwrap\(|expect\(|panic!|todo!|unimplemented!|unsafe" crates/gui/src/views.rs`
- `bash tools/gui-polish/run-geo-typo.sh checks/gui-syncpaint-ar/round-1`
