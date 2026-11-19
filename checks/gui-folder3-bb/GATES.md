# GATES — Wave BB-FOLDER3 (micro)

## Scope (OWNS only)

- `crates/gui/src/tree.rs`
- `checks/gui-folder3-bb/**`

MAT CHHOONA (parallel waves chal rahi hain — sirf read kiya): `views.rs`,
`views_table.rs`, `theme.rs`, `screenshot.rs`, `app.rs`, `home.rs`,
`checks/opencode-waves/STATUS.md`.

## Goal

Unedited-identical-folders case ke liye distinct empty hint: do identical
folders compare par "same, kuch diff nahi" jaisa honest hint — filter-pruned
aur fully-empty (never-compared) variants se alag wording. `tree.rs`
`folder_empty_hint` me naya `AllSame` arm + rows-aware classifier
`folder_empty_state_full_rows` + `folder_all_same` helper. Legacy arms,
legacy `folder_empty_state` aur az-wave `folder_empty_state_full` behavior
frozen (dusri waves ke tests un par hain).

## API contract (headless, pure — no egui, no I/O)

- `FolderEmptyState::AllSame` (naya; `NoResults`/`NoMatch`/`FilterPruned` frozen).
- `folder_empty_hint` +1 arm: `AllSame` →
  `"folder: folders match — all same, nothing to sync"`. Baaki arms
  byte-identical.
- `folder_all_same(rows)`: non-empty aur har glyph `'='` → `true`,
  warna `false` (khaali slice par `false`).
- `folder_empty_state_full_rows(has_model, rows, visible_rows, display)`:
  `!has_model` → `NoResults`; `rows` khaali → `NoResults`;
  `visible == 0 && folder_all_same(rows)` → `AllSame` (identical hi honest
  kaaran hai, filter chahe laga ho); `visible == 0 && display.is_active()` →
  `FilterPruned`; warna `None`.

## Non-goals

- `app.rs` / `views.rs` / `home.rs` me koi edit nahi — sirf REPORT me exact
  patch spec (paint/session-owner waves ke liye, DO NOT APPLY).
- Koi paint/egui code nahi, colors frozen, geometry/typo adoption
  paint-owner wave (`views.rs`) ka kaam.
- Non-test me `unwrap`/`expect`/`panic`/`todo` mana; `forbid(unsafe_code)`
  barkarar.

## DoD

- [x] `cargo test -p autocompare-gui` full green (568 passed / 0 failed).
- [x] `cargo clippy -p autocompare-gui --all-targets` zero warnings.
- [x] `rustfmt --check` sirf OWNS file par clean.
- [x] Banned grep apne diff me zero new (`unwrap`/`expect(` /`panic!`/`todo!`/`unsafe`).
- [x] 2 naye unit tests green + legacy arms/wording untouched.
- [x] `REPORT.md` me paint/session adoption patch spec.
- [x] Harness `run-geo-typo.sh` RESULT: PASS + `round-1/after.png` PNG magic.
- [x] `git add` sirf OWNS paths + conventional commit. NO PUSH.
