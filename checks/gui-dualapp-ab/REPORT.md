# Wave AB-DUALAPP — REPORT: dual-tree App wiring (gap #19, kadam 2)

## 1. Summary
`paint_folder_dual` (kadam 1 ka `#[allow(dead_code)]` API) ab Folder view
se wired hai: `App` me `folder_dual: tree::DualTreeState` field, compare
results se build/sync, aur har Folder paint me dual-panes call.
Single-list paint untouched — koi regression nahi.

## 2. Key metrics
- Tests: 294/294 green (`cargo test -p autocompare-gui`), is wave ke 5 naye
- Clippy `-p autocompare-gui --all-targets`: 0 warnings
- `cargo fmt --check -- crates/gui/src/app.rs`: clean (sirf OWNS file)
- Banned tokens `app.rs` me: 0 (test blocks me bhi 0)
- Harness: `geo-typo-report.md` RESULT: PASS, PNG 9624 bytes magic-ok

## 3. Highlights
- `app.rs` field + Default: `folder_dual: DualTreeState::new(20)`
  (`folder_pager` wali page size mirror)
- `sync_folder_dual()` (visible-tree total, select/pending rehta hai) +
  `reset_folder_dual()` (page size rehta hai, select/pending/total saaf);
  `clamp_scroll_after_layout` me dual sync joda
- Lifecycle: Ok results par sync; Err/validation-fail/swap par reset
- Paint: Folder tab me single-list ke baad `views::paint_folder_dual`
  (same rows + `folder_counts_summary`, `folder_session.expanded` canonical,
  legacy `folder_expanded` mirror-back); `views.rs` untouched
  (`#[allow(dead_code)]` ab satisfied, attribute KEVL ka hai — chhua nahi)
- 5 headless tests: default shape, sync total, select, swap-reset, stub-reset

## 4. GATES status (checks/gui-dualapp-ab/GATES.md)
- G1 App DualTreeState + results build: PASS (`folder_dual_*` tests)
- G2 folder view dual paint wiring: PASS (`paint_folder_dual` call site)
- G3 full green + clippy + fmt + banned grep: PASS
- G4 harness proof round-1: PASS

## 5. Blockers / risks
- Koi blocker nahi. Known limitation (agli kadam ke liye): dual paint
  `viewmodel::folder_tree` (unfiltered) se tree banata hai, jabki counts
  `folder_session` filters se aate hain — Hide-orphans jaise filters dual
  panes me nahi lagte. Filter parity future wave me.
- Dusre agents ki files untouched; `git stash/push` nahi chalaya.

## 6. Files (sirf OWNS)
- `crates/gui/src/app.rs` (+99 lines: field, sync/reset, lifecycle, paint, tests)
- `checks/gui-dualapp-ab/GATES.md`, `checks/gui-dualapp-ab/REPORT.md`,
  `checks/gui-dualapp-ab/round-1/`
