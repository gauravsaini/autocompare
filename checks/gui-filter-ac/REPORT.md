# Wave AC-FILTER — REPORT: dual-tree display filters (Gap #19)

## 1. Summary
Dual-tree panes me display filters lag gaye: `FolderSessionState` ka display
filter ab dual paint ke tree build me apply hota hai, aur counts summary
filtered tree par aati hai. Paint API same hai (`views::paint_folder_dual`
untouched) — filter App+tree side se pre-filtered rows + filtered summary
dekar lagta hai. Single-list paint untouched — koi regression nahi.

## 2. Key metrics
- Tests: 299/299 green (`cargo test -p autocompare-gui`), is wave ke 3 naye
- Clippy `-p autocompare-gui --all-targets`: 0 warnings
- `cargo fmt --check -- crates/gui/src/app.rs crates/gui/src/tree.rs`: clean
- Banned tokens `app.rs` + `tree.rs` me: 0 (test blocks me bhi 0)
- Harness: `geo-typo-report.md` RESULT: PASS

## 3. Highlights
- `tree.rs`: `filtered_folder_rows()` (order-stable `filter_pass`) +
  `FolderSessionState::filtered_rows / filtered_counts /
  filtered_counts_summary`; purana `counts` / `visible_tree` untouched
- `app.rs`: `folder_counts_summary()` ab filtered counts par;
  naya `folder_dual_rows()` (filtered, order same); dual paint call
  `&rows` ki jagah `&dual_rows` + filtered summary deta hai
  (paint signature same); `sync_folder_dual` pehle se filtered
  visible-tree total par tha — ab paint usse milta hai
- 3 headless tests: session filtered order, filtered counts/summary,
  dual rows + summary + sync-total parity

## 4. GATES status (checks/gui-filter-ac/GATES.md)
- G1 tree.rs filtered dual build helpers: PASS (`session_filtered_*`)
- G2 App wiring dual filtered + summary: PASS (`folder_dual_rows_*`,
  `paint_folder_dual` call site)
- G3 full green + clippy + fmt + banned grep: PASS
- G4 harness proof round-1: PASS

## 5. Blockers / risks
- Koi blocker nahi. `views.rs` / KEVL files untouched.
- Dusre agents ki files untouched; `git stash/push` nahi chalaya.

## 6. Files (sirf OWNS)
- `crates/gui/src/tree.rs` (helpers + session methods + 2 tests)
- `crates/gui/src/app.rs` (filtered summary, dual rows, paint site, 1 test)
- `checks/gui-filter-ac/GATES.md`, `checks/gui-filter-ac/REPORT.md`,
  `checks/gui-filter-ac/round-1/`
