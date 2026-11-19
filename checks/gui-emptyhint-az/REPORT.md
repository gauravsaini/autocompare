# REPORT — Wave AZ-EMPTYHINT (micro)

## Summary

`tree.rs` me filter-pruned empty ko never-compared empty se alag wording
mili: naya `FolderEmptyState::FilterPruned` variant + `has_model` bool-param
wala `folder_empty_state_full` constructor. Legacy `NoResults`/`NoMatch`
arms aur legacy `folder_empty_state` behavior byte-identical frozen hai —
parallel wave ke `app.rs`/`views.rs` tests unhi substrings par hain.
`home.rs`/`app.rs` ko chhua nahi; unke liye neeche exact patch spec hai.

## Files changed (sirf OWNS)

- `crates/gui/src/tree.rs`:
  - `FolderEmptyState::FilterPruned` (naya variant, doc ke saath).
  - `folder_empty_hint` +1 arm: `FilterPruned` →
    `"folder: filters hide every row — clear filters"`.
  - `folder_empty_state_full(has_model, total_rows, visible_rows, display)`:
    `!has_model` → `NoResults`; `total == 0` → `NoResults`;
    `visible == 0 && display.is_active()` → `FilterPruned`; else `None`.
  - 2 naye tests (`folderfilter_tests` mod me, `sample_rows` reuse):
    `filter_pruned_wording_differs_from_never_compared`,
    `empty_state_full_routes_pruned_vs_never_compared`.
- `checks/gui-emptyhint-az/GATES.md`, `REPORT.md` (yeh file).

## Tests

- `cargo check -p autocompare-gui`: green (exit 0).
- `cargo test -p autocompare-gui`: **561 passed, 0 failed** (559 pehle +
  2 naye). Naye dono ok; padosi legacy
  (`empty_states_stay_honest`, `session_filter_helpers_wire_counts_and_hints`)
  bhi green — koi regression nahi.

## Patch spec 1/2 — `home.rs:798` (DO NOT APPLY — owner wave kare)

Current (`home.rs:796-806`, read-only dekha):

```rust
pub const fn folder_empty_hint(has_rows: bool, has_error: bool) -> &'static str {
    if has_rows {
        ""
    } else if has_error {
        "compare failed: fix the message above, then Compare again"
    } else {
        "type both folder paths, press Compare, then expand a folder"
    }
}
```

Exact addition (purana fn untouched rakho):

```rust
/// Filter-aware idle guidance: pruned empty gets its own wording so
/// filter-pruned aur never-compared kabhi ek hint share na karen.
pub const fn folder_empty_hint_full(
    has_rows: bool,
    has_error: bool,
    filters_active: bool,
) -> &'static str {
    if has_rows {
        ""
    } else if has_error {
        "compare failed: fix the message above, then Compare again"
    } else if filters_active {
        "filters hide every row — clear filters, then Compare"
    } else {
        "type both folder paths, press Compare, then expand a folder"
    }
}
```

Tests (owner wave `folder_empty_hint_never_leaves_view_blank` me jode):

```rust
assert_eq!(folder_empty_hint_full(true, false, false), "");
assert!(folder_empty_hint_full(false, true, true).contains("Compare again"));
assert!(folder_empty_hint_full(false, false, true).contains("hide every row"));
assert!(folder_empty_hint_full(false, false, false).contains("type both folder paths"));
```

## Patch spec 2/2 — `app.rs:2085` (DO NOT APPLY — owner wave kare)

Current (`app.rs:2075-2090`, read-only dekha) byte-identical rakho —
`folder_empty_state`/`folder_empty_hint` par maujooda tests
(`folder_empty_hints_separate_no_results_from_no_match`,
`..._sync_badges_...` wala pruned assert `NoMatch` + `"clear filters"`)
frozen hain. Sirf ye do methods jodo:

```rust
/// Filter-specific empty state: model-maujoodgi ke saath classify karta
/// hai — pruned case `FilterPruned` deta hai (`NoMatch` legacy generic
/// rehta hai). Paint-adoption par paint isi ko padhe.
pub fn folder_empty_state_full(&self) -> Option<tree::FolderEmptyState> {
    match self.folder_rows() {
        Some(rows) => {
            let visible = self.folder_session.visible_tree(rows).len();
            tree::folder_empty_state_full(
                true,
                rows.len(),
                visible,
                &self.folder_session.display,
            )
        }
        None => Some(tree::FolderEmptyState::NoResults),
    }
}

/// Filter-specific hint line: `folder_empty_state_full` ka paint roop.
/// Pruned par "filters hide every row — clear filters", warna legacy line.
pub fn folder_empty_hint_full(&self) -> &'static str {
    tree::folder_empty_hint(self.folder_empty_state_full())
}
```

Adoption par test updates (owner wave, same file):

- Pruned case (`set_folder_name_filter("zzz-no-such-file")` wala):
  `Some(NoMatch)` → `Some(FilterPruned)`,
  `"no rows match filters"` → `"hide every row"` (note: `"clear filters"`
  substring dono me hai — 6469 wala assert ab bhi green rehta hai).
- `None`-rows (never-compared) case unchanged: `NoResults` + `"no results yet"`.

## Notes / risks

- `views.rs:3956` `folder_paint_empty_hint` compile ke liye koi change
  nahi maangta (frozen `folder_empty_state` → ab bhi `NoMatch` deta hai);
  `_full` par migrate karna paint-owner wave ka faisla hai.
- Geometry/typo adoption (per-variant block/typo) bhi paint-owner
  (`views.rs`) ke paas hai — yeh wave sirf headless wording + classifier
  deti hai. Colors frozen, koi hue nahi.
- Full suite 561 green me parallel wave ke maujooda `app.rs`/`views.rs`
  tests shaamil hain — un files ko chhua nahi (`git status` me sirf OWNS
  paths).
- Commit conventional English, NO PUSH (orchestrator verify+push karega).
