# GATES — Wave BD-FOLDERHOME (folder AllSame paint wiring + home CARD_H verify)

## Scope (OWNS only)

- `crates/gui/src/views.rs` (spec 1/2 fn + 1-line paint adopt + 1 naya test)
- `crates/gui/src/app.rs` (spec 2/2 do methods + 1 naya test)
- `checks/gui-folderhome-bd/**`

MAT CHHOONA (sibling wave parallel — sirf read kiya): `tree.rs`,
`theme.rs`, `views_table.rs`, `views_hex.rs`, `screenshot.rs`,
`home.rs`, `checks/opencode-waves/STATUS.md`. Colors FROZEN.

## Goal (sequential, isi order me)

1. P0-1 Folder AllSame wiring: `checks/gui-folder3-bb/REPORT.md` patch
   spec 1/2 (`views.rs` me `folder_paint_empty_hint_full_rows`) + 2/2
   (`app.rs` me `folder_empty_state_full_rows` +
   `folder_empty_hint_full_rows`) EXACT adopt, legacy glyph-blind path
   frozen rakhte hue + spec wali test update.
2. P1-3 Home CARD_H: contract min-height (99.6/100) ko `views.rs` home
   block paint me lana (purana 75 hatana). Single-token adoption, koi
   layout redesign nahi.

## API contract (pure — no egui, no I/O)

- `views::folder_paint_empty_hint_full_rows(rows, visible, display)`:
  `tree::folder_empty_state_full_rows(true, rows, visible, display)` ka
  hint roop. Legacy `folder_paint_empty_hint` byte-identical frozen
  (us par `folder_paint_empty_hint_stays_honest` hai); paint callsite
  (`paint_folder` khali-view branch) ab `_full_rows` padhta hai.
- `App::folder_empty_state_full_rows()` / `folder_empty_hint_full_rows()`:
  spec 2/2 verbatim — `folder_session.visible_tree(rows).len()` se
  visible, `None` model par `NoResults`.
- P1-3: koi naya token nahi — `paint_home` pehle se
  `home_paint_card_height()` (= 99.6, `CARD_H` 75 se upar),
  `home_paint_cta_size()` (= 120x28), `HOME_GRID_COLUMNS` par hai
  (commit `1a7a572`). Is wave me home block me zero diff — verify only.

## Non-goals

- `tree.rs` classifier/wording me koi edit nahi (FROZEN, BB-FOLDER3 ka hai).
- Paint callsite ke bahar koi wiring nahi: `Clear filters` button gate
  (`display.is_active()`) byte-identical; AllSame par bhi Clear dikhta
  hai (filter hatane par rows wapas aati hain — harmless).
- Koi layout redesign, koi naya hue; non-test me
  `unwrap`/`expect`/`panic`/`todo` mana; `forbid(unsafe_code)` barkarar.

## DoD

- [x] `cargo test -p autocompare-gui` full green (578 passed / 0 failed).
- [x] `cargo clippy -p autocompare-gui --all-targets` zero warnings
      (legacy fn par `allow(dead_code)` test-lock, repo precedent par).
- [x] `rustfmt --edition 2024 --check` sirf OWNS files par clean
      (bare `cargo fmt` MANA — nahi chalaya).
- [x] Banned grep apne diff me zero new
      (`unwrap` / `expect(` / `panic!` / `todo!` / `unsafe`).
- [x] 2 naye unit tests green + 4 legacy folder-empty tests untouched green.
- [x] Harness `run-geo-typo.sh` RESULT: PASS + `round-1/after.png` PNG magic.
- [x] `git add` sirf OWNS paths + conventional commit. NO PUSH.
