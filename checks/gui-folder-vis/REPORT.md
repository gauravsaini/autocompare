# VIS-FOLDER Report (2026-11-20)

## Mission
Folder tab wiring (app.rs) + Folder screenshot fixture (screenshot.rs),
headless tree/home logic reuse, koi duplication nahi.

## Owned files me changes
- `crates/gui/src/app.rs`
  - `App.folder_session: tree::FolderSessionState` naya field
    (legacy `folder_expanded` mirror rakha, back-compat).
  - Headless methods: `folder_counts_summary` (FolderCounts::summary),
    `toggle_folder_dir`, `folder_visible_tree`, `stage_folder_double_click`
    (stage + `resolve_compare_tab` se Text routing), `set_folder_sync_mode`
    (`SyncModeChoice::label`), `set_folder_consent`,
    `folder_sync_preview`, `folder_can_apply` (`can_apply_now`),
    `folder_sync_options_label` (`sync_options_for` + plan_sync parity note).
  - Folder tab UI: 4 filter checkboxes (FolderDisplay), counts line,
    dir click toggle (single click), file double-click stage + Text routing,
    khaali view me `folder_empty_hint`, sync preview panel (mode select,
    dry-run options, preview steps, consent checkbox +
    `sync_consent_hint`, Apply button `can_apply_now` gate; Apply sirf
    status likhta, filesystem write nahi — preview only).
  - 5 naye headless tests.
- `crates/gui/src/screenshot.rs`
  - `render_fixture_png` ab dispatcher: file naam me `folder` ho to
    Folder fixture, warna Text fixture (purana behavior same).
  - `render_folder_fixture_png` naya: tab bar (Folder highlight), filter
    line, `count_folder_rows` counts line, `folder_tree_filtered` tree,
    `sync_preview` panel + `SyncModeChoice::label` + `sync_consent_hint`
    — saare strings headless logic se, mock nahi.
  - 3 naye tests (text PNG, folder PNG, dispatcher routing).
- `tree.rs` / `home.rs`: koi change nahi (logic reuse hi kaafi tha).

## Proof
- `cargo test -p autocompare-gui`: 106 passed, 1 failed —
  fail sirf `views::tests::prefs_line_numbers_off_blanks_gutter_and_narrows`
  (TEXT scope, views.rs read-only mere liye; uska agent live fix kar raha;
  mere fail karne se pehle 2 the, usne 1 theek kiya).
  Mere tests sab green: `app::` 5/5, `screenshot::` 3/3, `tree::` + `home::` sab pass.
- `cargo build -p autocompare-gui`: OK. `cargo clippy`: meri owned files
  me zero warnings; 6 warnings sab views.rs me (5 TEXT scope + 1
  `paint_folder` dead jo mere Folder-tab rewrite ka side-effect hai —
  views.rs owner usko hatayega; hook neeche).
- `cargo fmt --check`: meri files clean (fmt repo-wide chalaya, dusri
  files ke hunks commit me nahi lunga).
- Screenshots: `before-text.png` 9914 bytes, `after-folder.png` 10300 bytes,
  dono PNG magic OK, distinct 256 (non-blank).
- `checks/round-D-1`: console `RESULT: PASS`, visual `RESULT: PASS` green.

## Hooks (dusre agents ke liye)
1. TEXT/VIEWS owner: `views::paint_folder` ab Folder tab se call nahi hota
   (wiring app.rs me hai) — dead-code warning hatao (function hatao ya
   reuse karo). `views::tests::prefs_line_numbers_off...` fail ho raha,
   theek karo.
2. `tools/parity-harness/run-visual.sh` me folder-shot support nahi dala
   (owns ke bahar hai) — folder visual proof `checks/round-D-1/after-folder.png`
   + `folder-visual.md` me hai; harness owner chahe to `--screenshot` dusra
   shot jod sakta, dispatcher ready hai.
