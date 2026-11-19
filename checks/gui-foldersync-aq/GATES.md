# GATES — Wave AQ-FOLDERSYNC

Wave: AQ-FOLDERSYNC. OWNS: `crates/gui/src/tree.rs`, `crates/gui/src/app.rs`, `checks/gui-foldersync-aq/**`.
KEVL (views.rs, viewmodel.rs, views_table.rs, views_hex.rs, home.rs, gutter.rs, theme.rs, screenshot.rs) ko chhoona mana hai, read bhi avoid.

Clean-room: Beyond Compare ko sirf bahar se observe kiya, apne shabdon me likha. Koi string copy nahi.

Order: Geometry first, typography second, colors frozen (koi hue change nahi).

Scope: folder compare me sync/copy direction actions ka App wiring + tree paint affordance headless — direction badges, selection model, dry-run summary real. Paint adoption agle wave ka hai, is wave me paint code nahi.

## Gates

1. Direction model (tree.rs): har file row + mode se ek direction — `sync_direction_for_row` wahi rules follow kare jo `sync_preview` use karta hai (`=` none, `!` manual skip, `>` L->R, `<` mode par, `L` L->R, `R` mode par copy/prune, dirs par dir-prune sirf Mirror+R). Unknown glyph none.
2. Direction badges (tree.rs): `sync_badge_for_row` har direction ke liye fixed text — `->`, `<-`, `X`, `=`, `?`. Pure, no egui, no I/O, no colors.
3. Selection model (tree.rs): `FolderSyncSelection` — BTreeSet based, select/toggle/remove/clear/contains/len/ordered list. Sirf real row paths select hote hain (helper `retain_real_rows`).
4. Dry-run summary real (tree.rs): `summarize_sync_preview` preview steps+skips se real counts gin-ta hai (copies L->R, copies R->L, deletes, skipped). `format_sync_summary` ek honest line deta hai. `sync_preview_for_selection` selection par scoped preview deta hai (dir-prune rules same).
5. Geometry tokens (tree.rs): badge min width aur summary metrics existing scale par — `SYNC_BADGE_CHARS`, `sync_badge_min_px` (`chars * MONO_ADVANCE + 2*PAD`), row height `folder_row_height` reuse (24.0), pad 8.0, gap 8.0, indent 12.0 frozen. Koi naya magic number nahi.
6. Typography tokens (tree.rs): badge + summary mono rehte hain — badge body 13 regular, delete badge strong, summary caption 12 regular, header strong reuse. `FolderTypo` reuse, theme import nahi (ownership disjoint).
7. App wiring (app.rs): `folder_sync_selection` field + `folder_sync_direction`, `folder_sync_badge`, `toggle/select/clear_folder_sync_row`, `folder_sync_selected_preview`, `folder_sync_summary_line`, `folder_sync_selected_summary_line`. Compare/swap/reopen par selection reset taaki stale paths jhooth na bolen. Paint code me koi change nahi.
8. Tests: naye unit tests tree + app dono me. Full `cargo test -p autocompare-gui` green.
9. Quality gates: `cargo clippy --all-targets` zero warnings, `cargo fmt -- <sirf OWNS files>` ke baad `cargo fmt --check` clean, banned grep clean (non-test me `unwrap|expect|panic|todo|unimplemented|unsafe` nahi), `tools/gui-polish/run-geo-typo.sh` headless PASS.

## DoD

- Sync direction wiring + summary real, unit tests, full gui tests green, harness PASS, commit (no push).

## Commands

- `cargo test -p autocompare-gui`
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
- `cargo fmt -- crates/gui/src/tree.rs crates/gui/src/app.rs && cargo fmt --check`
- `rg -n "unwrap\(|expect\(|panic!|todo!|unimplemented!|unsafe" crates/gui/src/tree.rs crates/gui/src/app.rs`
- `bash tools/gui-polish/run-geo-typo.sh checks/gui-foldersync-aq/round-1`
