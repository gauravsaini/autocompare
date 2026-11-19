# REPORT — Wave AF-APPFIND (Gap #12 FINAL wiring: App text tab → `paint_text_with_find`)

## 1. Summary

Text tab ka find box ab `FindState`-owned paint par wired hai: App me
`pub text_find: viewmodel::FindState` field rakha, Text tab paint
`views::paint_text_with_find(ui, model, &prefs, find)` call karta hai (bar +
`paint_text_with_prefs` delegation + `model.selected` sync andar hi). N/M badge
`FindState::count_label()` se aata hai, paint ke andar right-aligned dikhta hai.
Model me results hon par full FindState view; results na hon par halka legacy bar
(same GEO tokens/labels) + `paint_text` empty note + hint, taaki needle compare se
pehle type ho sake. Legacy `text_find_needle/cursor/ignore_case` + helpers compat
ke liye untouched hain (Wave Q/S/T tests unhi par hain): frame-start
`sync_text_find_state()` legacy ko FindState me seed karta hai, frame-end
`mirror_legacy_from_text_find()` needle/flag wapas mirror karta hai.
`App::text_find_badge()` ab `views::text_find_badge_label` par delegate karta hai
(badge parity by construction). KEVL files untouched (views.rs diff tree me dusre
parallel wave `checks/gui-mergepaint-af/` ka hai — is commit me include nahi).

## 2. Files changed (OWNS only)

- `crates/gui/src/app.rs`
  - Naya field `pub text_find: viewmodel::FindState` + Default init.
  - Naye headless helpers: `text_find_prefs()` (toggles → TextViewPrefs),
    `sync_text_find_state()` (legacy seed + stale refresh), 
    `mirror_legacy_from_text_find()`, `text_find_state_badge()`.
  - `text_find_badge()` ab `views::text_find_badge_label` delegate (parity).
  - Tab::Text paint: results par `paint_text_with_find` (split wrapper +
    `find_bar_geometry` dock + filter/copy status preserve); no-results par
    legacy `paint_text_find_bar` + `paint_text` empty + hint.
  - 3 naye tests: `text_find_state_prefs_follow_toggles`,
    `text_find_state_seeds_from_legacy_needle`,
    `text_find_state_badge_matches_legacy_badge`.
- `checks/gui-appfind-af/GATES.md` (G1–G6), `REPORT.md`, `round-1/` artifacts.

## 3. Tests

- Baseline: 324 passed, 0 failed (round-1/baseline-tests.out).
- After: 333 passed, 0 failed (mere 3 naye; +6 tree me dusre parallel wave ke —
  `views.rs` uncommitted uska hai, mera nahi).
- `cargo test -p autocompare-gui find`: 31 passed (purane find tests same values par).
- `rustfmt --edition 2024 --check crates/gui/src/app.rs`: clean (sirf OWNS file par
  `cargo fmt --` chalaya). Non-test me banned (`unwrap(`/`expect(`/`panic!`/`todo!`)
  zero; `forbid(unsafe_code)` kept.
- Clippy: `app.rs` se ZERO errors (mere 3 dead-code side effects genuine reuse se fix:
  badge delegate + empty-state legacy path). Bache hue 4 errors sab `views.rs`
  lines 1056–1059 me dusre active wave (`mergepaint-af`) ke stubs ke hain —
  KEVL hone se main chhu nahi sakta; unka wave complete hote hi green hoga.
- Harness `run-geo-typo.sh`: `RESULT: PASS`, PNG magic ok, 9624 bytes, distinct 256.

## 4. DoD check

- [x] Text find box App me live hai (results par FindState bar, empty par legacy bar)
- [x] `paint_text_with_find` App text-tab paint se called (allow dead-code KEVL me
      rehta hai — dusre agent ka file; wiring se function used hai)
- [x] N/M badge visible (`count_label`, parity tested)
- [x] Unit tests + full gui suite green
- [x] Harness PASS (headless proof)
- [x] Commit (no push), sirf OWNS paths staged
