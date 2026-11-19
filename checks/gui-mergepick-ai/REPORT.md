# REPORT — Wave AI-MERGEPICK (Gap #13: per-hunk merge pick UI)

## Summary

Per-hunk merge pick UI live: har conflict hunk card par real Take Left / Take Right
(plus Take Base / Take Both) buttons, click par pick intent temp store me stage hota hai,
decision display pending se "take <side>" me badalta hai, summary real picks se ginta hai.
Koi "(stub)" label nahi bacha merge paint me. Full suite 363 green, clippy zero, harness PASS.

## Key Metrics

- Tests: 355 (baseline pre-edit) → 363 (8 naye: 6 pick-intent + 2 purane AF samet green)
- Clippy `--all-targets`: 0 warnings
- `rustfmt --edition 2024 --check crates/gui/src/views.rs`: clean
- Harness: `RESULT: PASS` (PNG 9624 bytes, magic ok, distinct 256)
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`) non-test me: zero; `forbid(unsafe_code)` kept

## Highlights

- ✅ Pure helpers (headless, unit-tested): `merge_pick_id` (lines-hash-scoped temp id —
  naya compare purani intent inherit nahi karta), `merge_pick_choice_label`,
  `format_merge_pick_intent`, `merge_pick_for_hunk` (last-staged wins),
  `stage_merge_pick` (upsert, `apply_merge_picks` dedup ka mirror),
  `format_merge_hunk_decision_with_pick` (pending vs intent)
- ✅ `paint_merge_three_pane`: summary `merge_pick_summary(lines, &picks)` se (real pending),
  har card par 4 real buttons → `stage_merge_pick`, intent display per-card
- ✅ Signatures unchanged (`paint_merge`, `paint_merge_three_pane`) — KEVL `app.rs` compat
- ✅ Colors frozen (koi naya hue/token nahi), geometry/typo existing tokens only

## Blockers / Notes

- 🚨 Parallel wave agent `app.rs` + `views_table.rs` me kaam kar raha hai (uncommitted diff
  dikha `git status` me). Maine un files ko touch nahi kiya; mera commit sirf OWNS paths
  stage karta hai. Orchestrator merge/rebase sambhale.
- GATES G4 ka fmt command `rustfmt --edition 2024 --check <file>` hai (single-file;
  bare `cargo fmt` nahi chalaya taaki sibling files untouched rahen).

## Decisions Needed

- [ ] Next wave (App wiring): temp-store picks ko `ThreePanePlan::decide` / `MergeSummary`
  se jodna + Apply-all (`apply_merge_picks`) — is wave me scope bahar tha

## Verification

| Gate | Check | Result |
|------|-------|--------|
| G1 | baseline 355/0 (`main@bbfad4b`) | ok, `round-1/baseline-tests.out` |
| G2 | OWNS-only + KEVL-CLEAN (mere diff me) + NO-COLOR-ADD | ok |
| G3 | `cargo test -p autocompare-gui merge` (25 pass) + full 363 | ok |
| G4 | clippy 0 + fmt 0 + ZERO-BANNED + UNSAFE-FORBID-OK | ok |
| G5 | harness + PNG magic/size/distinct | PASS, 9624 / magic ok / 256 |
| G6 | commit, no push | feat commit, working tree (mere paths) clean |
