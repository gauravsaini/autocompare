# REPORT — Wave AF-MERGEPAINT (Gap #7 paint half: 3-pane merge paint)

## Summary

Merge tab me 3-pane paint live hai: har conflict hunk ke left/base/right panes
(model se real parse) + per-hunk decision display (real pending counts) paint hote
hain. Per-hunk Take buttons stub hain (label me "stub" saaf, no-op); global
Take Left/Right/Base/Both functional rehta hai (compat, `resolve_take` index 0).
`paint_merge` signature unchanged (KEVL `app.rs` call-site compat).

## Delta (OWNS only)

- `crates/gui/src/views.rs`:
  - Pure helpers (headless, unit-tested): `merge_pane_count`, `merge_hunk_sides`
    (marker-block section rule `take::resolve_take` jaisa, base-marker-less block
    me `=======` se pehle sab left), `format_merge_hunk_title`,
    `format_merge_hunk_decision`, `merge_three_pane_summary_line`.
  - Paint: `paint_merge_three_pane` (read-only reuse
    `viewmodel::merge_conflict_hunks` + `merge_pick_summary`; sirf existing
    tokens: `TIGHT_GAP`, small/strong/weak/monospace; koi naya color/token nahi)
    + `paint_merge` me call (empty-check ke baad, merged output se pehle).
  - Compat allows (sibling wave AF-APPFIND ne `app.rs` me Text tab ko
    `paint_text_with_find` par rewire kiya — uncommitted, parallel wave):
    `paint_text`, `text_find_badge_label`, `paint_text_find_bar` par
    `#[allow(dead_code)]` (behavior zero-change; tests + API compat alive).
- `checks/gui-mergepaint-af/`: `GATES.md`, `REPORT.md`, `round-1/`
  (`baseline-tests.out`, `after.png`, `geo-typo-report.md`, `tests.out`,
  `screenshot.out` — harness-generated).

## Gates evidence

- G1 baseline: `round-1/baseline-tests.out` — 324 passed, 0 failed (pre-edit).
- G2 OWNS-only: worktree me sibling wave (`app.rs` + `checks/gui-appfind-af/`,
  uncommitted) bhi dikhta hai — is commit me sirf OWNS paths staged
  (`views.rs` + `checks/gui-mergepaint-af/`); `app.rs` staged nahi.
  NO-COLOR-ADD, KEVL files untouched (mere diff me).
- G3 paint wiring: `cargo test -p autocompare-gui merge` — 19 passed;
  full suite — 333 passed, 0 failed (6 naye merge tests).
- G4 MS Rust: `clippy:0` (`--all-targets -- -D warnings`), `fmt:0`
  (`rustfmt --edition 2024 --check`), ZERO-BANNED (non-test),
  `forbid(unsafe_code)` kept, `.get()`-only + saturating, `format!` only alloc.
- G5 harness: `run-geo-typo.sh round-1` → `RESULT: PASS`; `after.png`
  9624 bytes, magic `89504e470d0a1a0a`, distinct 256.
- G6: ye file + artifacts; commit locally, NO push.

## DoD

3-pane merge paint dikhta hai (hunk cards: Left/Base/Right panes + pending
decision + stub Take row), 6 naye unit tests, full gui tests green (333),
harness PASS, commit (no push). Per-hunk Take wiring agle wave ka kaam hai.
