# GUI-Folder P0 Report (2026-11-20)

## Mission
Folder Compare P0: expand/collapse tree honest glyphs, display filters
(No Orphans / Left Newer / Right Newer / Flatten) honest counts,
double-click staging, sync preview Update Both/Mirror + Apply consent.

## Owned files changed (sirf ye)
- `crates/gui/src/tree.rs` — FolderDisplay filter, FolderCounts honest
  counts, folder_tree_filtered (flatten depth 0), toggle/expand_all/
  collapse_all, stage_compare_intent/clear_compare_intent,
  SyncModeChoice + sync_options_for + sync_preview + can_apply gate.
- `crates/gui/src/home.rs` — folder_empty_hint, sync_consent_hint,
  resolve_compare_tab (staged file -> Text tab, warna Folder).
- `checks/gui-folder/after.png` + ye REPORT.md.

## Rules follow
- `forbid(unsafe_code)`, no unwrap/expect/panic/todo non-test code.
- Engine reuse only: glyphs `viewmodel::build_folder_rows` se,
  sync mapping `SyncPlanner::options` jaisa (Both=NewerWins/no-prune,
  Mirror=LeftToRight/prune), symlink guard engine me rehta hai.
- Headless-first: saara state transition unit-testable, egui wiring nahi.

## Proof
- `cargo test -p autocompare-gui`: 90 passed, 0 failed.
  - `tree::`: 20 passed (expand/collapse, 4 filters, flatten,
    counts, staging, sync preview dono modes, consent gate,
    pruned-dir doomed children).
  - `home::`: 4 passed (9 sessions + empty hint + consent hint).
- `cargo fmt --check`: tree.rs/home.rs me koi diff nahi.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`:
  tree.rs/home.rs me koi error nahi. Baaki errors sab
  `viewmodel.rs` me hain (GUI-TEXT owned: TextFilter/FilterCounts/
  nav helpers abhi unwired) — uska agent theek karega.
- Screenshot: `cargo build -p autocompare-gui` (debug, 11 warnings
  jo pehle se dusre files me the), phir
  `./target/debug/autocompare-gui --screenshot checks/gui-folder/after.png`
  PNG magic `89504e470d0a1a0a` OK, 9846 bytes (before.png 9174 bytes).
  Note: `--screenshot` fixture abhi Text session render karta hai
  (screenshot.rs SHELL-owned, read-only mere liye); Folder-specific
  fixture uske agent ke paas hook ke roop me maanga hai.

## Needed hooks (dusre agents ke liye)
1. SHELL (app.rs/views.rs owned): Folder tab me FolderDisplay
   checkboxes + counts line, double-click -> stage_compare_intent +
   resolve_compare_tab se Text tab kholo, sync mode select
   (Update Both/Mirror) + consent checkbox + Apply button
   (can_apply gate). folder_empty_hint khaali view me dikhao.
2. TEXT (screenshot.rs owned): `--screenshot` me Folder fixture
   (tree + filter + preview) render karo taaki folder ka visual
   proof aaye; abhi sirf Text fixture hai.
3. viewmodel.rs read-only raha mere liye; koi hook nahi chahiye tha —
   FolderRow reuse kaafi tha.
