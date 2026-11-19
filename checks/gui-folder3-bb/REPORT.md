# REPORT — Wave BB-FOLDER3 (micro)

## Summary

`tree.rs` me unedited-identical-folders ke liye teesra empty variant mila:
naya `FolderEmptyState::AllSame` + `folder_all_same` helper + rows-aware
`folder_empty_state_full_rows` classifier (az-wave model-aware pattern par,
wahi style me 3rd arm joda). Do identical folders compare par ab honest hint
hai — "folders match, all same, nothing to sync" — jo filter-pruned
("hide every row — clear filters") aur never-compared ("no results yet —
compare first") dono se alag hai. Legacy `folder_empty_state`, az-wave
`folder_empty_state_full` aur `home.rs`/`app.rs`/`views.rs` ko chhua nahi;
unke liye neeche exact patch spec hai.

## Files changed (sirf OWNS)

- `crates/gui/src/tree.rs`:
  - `FolderEmptyState::AllSame` (naya variant, doc ke saath).
  - `folder_empty_hint` +1 arm: `AllSame` →
    `"folder: folders match — all same, nothing to sync"`.
  - `folder_all_same(rows)`: non-empty + sab glyph `'='`.
  - `folder_empty_state_full_rows(has_model, rows, visible, display)`:
    `!has_model` → `NoResults`; khaali model → `NoResults`;
    `visible == 0 && all_same` → `AllSame`;
    `visible == 0 && active` → `FilterPruned`; else `None`.
  - 2 naye tests (`folderfilter_tests` mod me, `sample_rows` reuse):
    `identical_wording_differs_from_pruned_and_never_compared`,
    `empty_state_full_rows_routes_identical_vs_pruned_vs_never_compared`.
- `checks/gui-folder3-bb/GATES.md`, `REPORT.md` (yeh file),
  `round-1/after.png` (harness shot).

## Tests

- `cargo test -p autocompare-gui`: **568 passed, 0 failed** (566 pehle +
  2 naye). Naye dono ok; padosi legacy
  (`empty_states_stay_honest`, `filter_pruned_wording_...`,
  `empty_state_full_routes_pruned_vs_never_compared`,
  `session_filter_helpers_...`) bhi green — koi regression nahi.
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings.
- Banned grep apne diff par zero new
  (`unwrap` / `expect(` / `panic!` / `todo!` / `unsafe` — koi match nahi).
- Harness `tools/gui-polish/run-geo-typo.sh`: RESULT: PASS
  (PNG 9627 B, magic ok, distinct 256 — default text fixture route).
- `rustfmt --edition 2024 --check crates/gui/src/tree.rs`: clean.

## Patch spec 1/2 — `views.rs:3963` rows-aware paint hint (DO NOT APPLY — owner wave kare)

Current (`views.rs:3963-3973`, read-only dekha) byte-identical rakho —
glyph-blind legacy path frozen hai (`folder_paint_empty_hint_stays_honest`
us par hai). Sirf ye naya fn jodo:

```rust
/// Rows-aware paint hint: identical folders ko prune se alag wording.
/// Legacy `folder_paint_empty_hint` frozen hai; `_full_rows` adopt kare to
/// diff-seeking filter par identical model `AllSame` deta hai. Paint me
/// `rows` model se aate hain isliye `has_model` hamesha `true`.
pub fn folder_paint_empty_hint_full_rows(
    rows: &[FolderRow],
    visible_rows: usize,
    display: &crate::tree::FolderDisplay,
) -> &'static str {
    crate::tree::folder_empty_hint(crate::tree::folder_empty_state_full_rows(
        true,
        rows,
        visible_rows,
        display,
    ))
}
```

Adoption par test update (owner wave, same file): single `'='` row +
`Different` filter wala case `Some(NoMatch)` → `Some(AllSame)` aur
`"clear filters"` → `"all same"` ho jayega — owner wave faisla kare ki
legacy paint fn rehta hai ya migrate hota hai.

## Patch spec 2/2 — `app.rs:2077` session `_full_rows` wiring (DO NOT APPLY — owner wave kare)

Current (`app.rs:2077-2088`, read-only dekha) byte-identical rakho —
maujooda `folder_empty_state`/`folder_empty_hint` par tests
(`folder_empty_hints_separate_no_results_from_no_match`,
`..._sync_badges_...` wala pruned assert `NoMatch` + `"clear filters"`)
frozen hain. Sirf ye do methods jodo:

```rust
/// Rows-aware empty state: identical folders `AllSame` dete hain
/// (glyph-blind legacy `folder_empty_state` prune/all-same me farak
/// nahi karta). Paint-adoption par paint isi ko padhe.
pub fn folder_empty_state_full_rows(&self) -> Option<tree::FolderEmptyState> {
    match self.folder_rows() {
        Some(rows) => {
            let visible = self.folder_session.visible_tree(rows).len();
            tree::folder_empty_state_full_rows(
                true,
                rows,
                visible,
                &self.folder_session.display,
            )
        }
        None => Some(tree::FolderEmptyState::NoResults),
    }
}

/// Rows-aware hint line: `folder_empty_state_full_rows` ka paint roop.
/// Identical par "all same, nothing to sync", warna legacy line.
pub fn folder_empty_hint_full_rows(&self) -> &'static str {
    tree::folder_empty_hint(self.folder_empty_state_full_rows())
}
```

## Notes / risks

- `folder_empty_state_full` (az-wave, glyph-blind) ab effectively legacy hai:
  identical rows par woh `FilterPruned` deta hai (glyph dekh hi nahi sakta) —
  uske tests byte-identical rakhe hain, paint ko sahi wording ke liye
  `_rows` adopt karna hoga (specs upar).
- `AllSame` sirf khaali ped par milta hai (`visible == 0` gate) — "None
  matlab view khaali nahi" contract barkarar hai. Bina filter identical
  browse par `None` (rows dikhti hain, hint nahi) — hint diff-seeking filter
  (jaise `Changed` status) par identical model me nikalta hai, jahan
  "clear filters" bolna misleading hota (clear karne par bhi kuch karne ko
  nahi milega).
- Gap closure: `checks/localrun-ax2/REPORT.md` §10.5 ka teesra-variant gap
  headless side band (wording + classifier); paint adoption owner wave par.
- Geometry/typo adoption (per-variant block/typo) bhi paint-owner
  (`views.rs`) ke paas hai — yeh wave sirf headless wording + classifier
  deti hai. Colors frozen, koi hue nahi.
- Full suite 568 green me parallel waves ke maujooda `app.rs`/`views.rs`
  tests shaamil hain — un files ko chhua nahi (`git status` me sirf OWNS
  paths).
- Commit conventional English, NO PUSH (orchestrator verify+push karega).
