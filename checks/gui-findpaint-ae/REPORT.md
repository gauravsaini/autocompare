# REPORT — Wave AE-FINDPAINT (Gap #12 paint half: text find box + N/M badge)

## 1. Summary

`viewmodel::FindState` / `count_label` (Z-FIND model half, pehle se landed) ab
`views.rs` text paint me wired hai: `paint_text_find_state_bar` (find TextEdit,
Ignore-case checkbox, Prev/Next buttons, right-aligned N/M count badge) aur
`paint_text_with_find` (bar + existing `paint_text_with_prefs` delegation,
Prev/Next se `model.selected` sync). Table/folder find precedent reuse hua:
wahi 120px needle width, wahi button labels, wahi case-flag matcher semantics
(`table_cell_contains` / `folder_name_contains` jaisa `find_text_contains`).
Purana App-wired `paint_text_find_bar` compat ke liye untouched hai.

## 2. Files changed (OWNS only)

- `crates/gui/src/viewmodel.rs`
  - `find_match_indices_cased(rows, query, ignore_case)` naya pure helper;
    `find_match_indices` ab iska `ignore_case = true` wrapper (purane callers/tests unchanged).
  - `FindState` me `ignore_case` + `rows_len` fields; naye methods `set_query_cased`,
    `set_ignore_case`, `refresh_if_stale` (stale compare par honest counts).
  - `#[allow(dead_code)]` wiring-pending comments Wave AE wording par update.
- `crates/gui/src/views.rs`
  - `advance_text_find` (headless nav step: click na ho to `None`, taaki F3 diff-nav
    selection se ladai na ho), `paint_text_find_state_bar`, `paint_text_with_find`.
  - Sirf existing GEO tokens (`FIND_BAR_H`, `FIND_NEEDLE_W`, `BADGE_PAD`) aur small
    weak badge typo; koi naya token/color nahi.
- `checks/gui-findpaint-ae/GATES.md` (G1–G6), `REPORT.md`, `round-1/` artifacts.

## 3. Tests

- Baseline: 317 passed, 0 failed (round-1/baseline-tests.out).
- After: 324 passed, 0 failed (5 naye: `find_match_indices_cased_honors_the_case_flag`,
  `find_state_case_flag_recomputes_with_cursor_on_first_hit`,
  `find_state_refresh_resets_only_on_new_model`, `find_state_badge_matches_model_label`,
  `advance_text_find_moves_only_on_click`).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: 0 warnings.
- `rustfmt --edition 2024 --check` OWNS files: clean. Non-test me banned
  (`unwrap(`/`expect(`/`panic!`/`todo!`) zero; `forbid(unsafe_code)` kept.
- Harness `run-geo-typo.sh`: `RESULT: PASS`, PNG magic ok, 9624 bytes, distinct 256.

## 4. DoD check

- [x] Text find box (TextEdit) + Next/Prev + count badge paint me wired
- [x] Count badge `FindState::count_label` se aata hai (badge parity tested)
- [x] Unit tests + full gui suite green
- [x] Harness PASS (headless proof)
- [x] Commit (no push), sirf OWNS paths staged
