# GUI-FOLDER P0 proof (2026-11-20)

## Mission
Folder Compare P0: expand/collapse tree, display filters, double-click
compare intent, sync plan preview with mode select + explicit Apply consent.
Headless-first: all transitions in `tree.rs` / `home.rs`, unit-tested.

## Changes (owned files only)
- `crates/gui/src/tree.rs`
  - Reused parallel-leaf additions already in working tree (FolderDisplay,
    counts, sync_preview, stage_compare_intent) and fixed 3 clippy errors
    in them (`field_reassign_with_default`, `needless_arbitrary_self_type` x2).
  - Added `FolderSessionState`: expanded + display + staged + sync_mode +
    consented in one testable struct. Mode switch resets consent.
- `crates/gui/src/home.rs`
  - Added `resolve_compare_tab`: staged file -> Text tab, none -> Folder.
  - Restored `sync_consent_hint` doc comment clobbered during edit.

## Proof
- `cargo test -p autocompare-gui`: 90 passed, 0 failed.
  - New: `tree::session_tests::*` (5), `home::tests::staged_file_routes_to_text_otherwise_folder` (1).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`:
  tree.rs + home.rs clean (0 hits). Remaining errors all in
  `viewmodel.rs` (TEXT-leaf scope, read-only for this agent) + fmt diff
  only in `viewmodel.rs`.
- `rustfmt --check` on tree.rs + home.rs: clean.
- Screenshots: `before.png` (00:56 baseline) vs `after.png` (regenerated
  01:05 via debug build `--screenshot`; fixture renderer unchanged, no
  regression). Folder UI wiring pending host hooks below.

## Needed host hooks (app.rs / views.rs NOT owned, not touched)
1. Hold `FolderSessionState` in App folder tab; `toggle()` on dir click.
2. Paint 4 filter checkboxes from `FolderDisplay` + counts line from
   `FolderCounts::summary()` (+ visible count).
3. Double-click file node -> `stage_double_click()` -> open
   `resolve_compare_tab(&staged)`; directory click stages nothing.
4. Sync preview: mode select labels from `SyncModeChoice::label()`,
   options via `sync_options_for` + `plan_sync` dry-run, consent checkbox
   via `sync_consent_hint`, Apply button gated by `can_apply_now`.

## MS rules
No unsafe, no unwrap/expect/panic/todo in non-test code. Engine reuse
only (glyphs + SyncOptions from autocompare-compare); no re-implementation.
