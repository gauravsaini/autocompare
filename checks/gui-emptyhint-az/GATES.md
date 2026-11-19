# GATES — Wave AZ-EMPTYHINT (micro)

## Scope (OWNS only)

- `crates/gui/src/tree.rs`
- `checks/gui-emptyhint-az/**`

MAT CHHOONA (parallel wave chal rahi hai — sirf read kiya): `views.rs`,
`views_table.rs`, `screenshot.rs`, `app.rs`, `home.rs`,
`checks/opencode-waves/STATUS.md`.

## Goal

Filter-pruned empty vs never-compared empty ke liye alag wording:
`tree.rs:491` `folder_empty_hint` me pruned ke liye naya filter-specific
variant + ek bool-param (`has_model`) wala constructor. Legacy arms aur
legacy `folder_empty_state` behavior frozen (dusri waves ke tests un par hain).

## API contract (headless, pure — no egui, no I/O)

- `FolderEmptyState::FilterPruned` (naya; `NoResults`/`NoMatch` frozen).
- `folder_empty_hint` +1 arm: `FilterPruned` →
  `"folder: filters hide every row — clear filters"`. Baaki arms
  byte-identical.
- `folder_empty_state_full(has_model, total_rows, visible_rows, display)`:
  `!has_model` → `NoResults` (never-compared, legacy line);
  `total == 0` → `NoResults`; `visible == 0 && display.is_active()` →
  `FilterPruned`; warna `None`.

## Non-goals

- `home.rs:798` / `app.rs:2085` me koi edit nahi — sirf REPORT me exact
  patch spec.
- Koi paint/egui code nahi, colors frozen, geometry/typo adoption
  paint-owner wave (`views.rs`) ka kaam.
- Non-test me `unwrap`/`expect`/`panic`/`todo` mana; `forbid(unsafe_code)`
  barkarar.

## DoD

- [x] `cargo check -p autocompare-gui` green.
- [x] 2 naye unit tests green + poora suite 561 passed / 0 failed.
- [x] Legacy arms/wording untouched (app/views tests frozen substrings).
- [x] `REPORT.md` me home.rs + app.rs exact patch spec.
- [x] `git add` sirf OWNS paths + conventional commit. NO PUSH.
