# Wave AA-MERGE — REPORT (round-1)

Gap #7 model half: `TriplePaths` (base/left/right) se 3-pane merge-plan model.
Text side (`viewmodel.rs`) + tree side (`tree.rs`) land hue; `views.rs` paint
agle wave me (scope se bahar, is wave me paint API untouched).

## Before -> After (file:line)

- `crates/gui/src/viewmodel.rs:1564-1830` naya section "Three-pane merge plan":
  `ThreePaneKind` (Same/LeftChange/RightChange/BothSameChange/Conflict) +
  `ThreePaneHunk` (index/kind/base/left/right/conflicted) +
  `ThreePaneDecision` (hunk + `TakeChoice` reuse) + `ThreePanePlan`
  (`new`/`decide`/`merge_lines`/`summary`) + `ThreePaneSummary`
  (total/decided/pending/conflicted + `summary_line`) +
  `three_pane_hunks` (maximal same-shape runs, missing tails `None`, koi padding nahi) +
  `three_pane_merge_lines` (decided splice incl. Both=left+right, undecided
  non-conflict auto-resolve, undecided conflict marker block taaki downstream
  `merge_conflict_hunks`/`merge_pick_summary` chalte rahen) +
  `three_pane_summary` (unknown hunk picks kabhi decided nahi).
  Har item `#[allow(dead_code)]` wiring-pending (paint agla wave).
- `crates/gui/src/tree.rs` EOF naya section "Three-way tree merge plan":
  `ThreeWayTreeRow` (rel_path/is_dir/has_base/has_left/has_right) +
  `ThreeWaySide` (Left/Base/Right + `label()`) + `ThreeWayDecision`
  (rel_path + side) + `three_way_tree_rows` (teen snapshots ki union, sorted,
  har path ek baar) + `is_three_way_conflict` (files only; delete-vs-keep ya
  added-both-sides; dirs auto-keep) + `three_way_conflicts` +
  `ThreeWayTreeSummary` (total/decided/pending/conflicts/conflicted +
  `summary_line`) + `three_way_tree_summary` (unknown paths kabhi decided nahi).
- Geometry first: koi naya token nahi (pure index math, saturating, `.get()` only);
  typography second: summary lines existing honest-counts shape me;
  colors frozen: koi hue touch nahi (koi egui import nahi).

## Tests

- `cargo test -p autocompare-gui`: 289 passed, 0 failed (baseline 278 + 8 naye
  OWNS + 3 concurrent table-agent ke unstaged `views_table.rs` tests; fail 0).
- Naye viewmodel tests (5): `three_pane_hunks_split_by_change_shape`,
  `three_pane_hunks_coalesce_runs_and_cover_tails`,
  `three_pane_merge_lines_auto_and_decided`,
  `three_pane_plan_decide_dedup_and_reject_unknown`,
  `three_pane_summary_counts_progress_honestly`.
- Naye tree tests (3): `three_way_rows_union_sorted_with_presence`,
  `three_way_conflict_covers_delete_and_double_add`,
  `three_way_summary_counts_decisions_honestly`.
- `cargo test -p autocompare-gui three_`: 9 passed (8 naye + 1 pre-existing match).

## Harness

- `tools/gui-polish/run-geo-typo.sh checks/gui-merge-aa/round-1`: RESULT: PASS
  (PNG 9624 bytes, magic ok, distinct 256; `.size` values sirf ['14.0'] — koi typo change nahi).
- `rustfmt --edition 2024 --check` (OWNS files `tree.rs` + `viewmodel.rs`): clean.
- Banned grep (non-test + added lines `unwrap(`/`expect(`/`panic!`/`todo!`/...):
  ZERO-BANNED (`unwrap_or`/`unwrap_or_default` allowed hain, wahi use hote hain).
- Indexing scan (added lines): koi `[i]` slicing nahi (sirf `#[default]`/`#[test]`
  attributes); non-test code me `.get()`/iterators only.

## Scope note

- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: is waqt FAIL —
  eklauta error KEVL `views_table.rs:354` (`pad_table_row is never used`) hai, jo
  concurrent table-grid agent ka mid-work code hai (mere OWNS se bahar, haath nahi
  lagaya). `cargo check --all-targets` me poori crate me wahi 1 warning hai;
  mere dono OWNS files zero warnings (clippy output me `tree.rs`/`viewmodel.rs`
  ka 0 mention). Ye wave ka code agle paint wave tak `#[allow(dead_code)]` hai,
  isliye mere side se koi dead-code warning nahi.
- Working tree me concurrent agents ka kaam dikha (`views.rs` +112, `views_table.rs`
  +31, `checks/gui-table-aa/` untracked) — chhua nahi, commit me shaamil nahi.
  `app.rs` / `theme.rs` / `screenshot.rs` / `views.rs` / `views_table.rs` untouched.
- Koi filesystem persistence/I/O nahi; plan sirf run-time model. Apply agle wave/host
  ka kaam (OPEN). STATUS.md untouched, koi push nahi (local commit only).

## Files (sirf OWNS)

- `crates/gui/src/viewmodel.rs` (three-pane text plan model + 5 tests)
- `crates/gui/src/tree.rs` (three-way tree plan model + 3 tests)
- `checks/gui-merge-aa/GATES.md`, `checks/gui-merge-aa/REPORT.md`,
  `checks/gui-merge-aa/round-1/` (baseline-tests.out, tests.out, after.png,
  geo-typo-report.md, screenshot.out)
