# Wave AM-DIFFNAV — REPORT (text tab diff navigation: Next/Prev jump + N/M count)

## Summary

Text tab me diff navigation ka N/M count + bring-into-view kaam karta hai (find-bar precedent `checks/gui-tablefind-ai/REPORT.md` pattern reuse).
Nav row text window ke andar `views.rs` me paint hoti hai; position/count pure paint-side helpers me hai (KEVL model untouched).
`paint_text_with_prefs` toolbar find-bar geometry se paint hoti hai (full-width 28px row + Prev/Next small buttons + badge right-end small weak + 4px pad); Next/Prev existing `select_next/prev_diff` + `scroll_to` bring-into-view chalate hain.

## Metrics

- Tests: 393 passed (baseline) → 396 passed, 0 failed (full `cargo test -p autocompare-gui`)
- `diff_nav` focused: 3 passed; full suite green
- Clippy `--all-targets -- -D warnings`: 0 warnings
- `rustfmt --check` (sirf OWNS file): clean
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`...) non-test code me: zero
- Harness: `RESULT: PASS`; `after.png` 9624 bytes, magic `89504e470d0a1a0a`, distinct 256

## Highlights

- ✅ `views.rs`: pure `diff_nav_total` (sirf `is_diff_row` count, khali/equal-only par 0) + `diff_nav_position` (selected diff ki 1-based position, nahi to `None`) + `diff_nav_count_label` (N/M, find `count_label` mirror: `Some(pos) -> "pos/total"`, `None -> "0/total"`)
- ✅ `paint_text_with_prefs` toolbar wiring: `horizontal_wrapped` + min-size `avail x HEADER_H` (28.0 find-bar reuse) + `◀ Prev` / `Next ▶` small buttons + badge right-end `small().weak()` + `BADGE_PAD` (4.0); badge har frame `diff_nav_position/total` se recompute; Next/Prev existing `select_next/prev_diff` + `scroll_to` bring-into-view reuse (wrap model me, F3/Ctrl+N/P path untouched)
- ✅ Geometry first (nav row layout 28px + 4px pad reuse, koi naya token nahi), typography second (badge small weak, koi naya size/color nahi)

## Decisions

- Paint helper `views.rs` me rakha (OWNS) taaki KEVL `app.rs`/`viewmodel.rs` me koi edit na ho; model moves (`select_next/prev_diff`, `is_diff_row`, `find_count_label` shape) read-only reuse
- Badge/position semantics find precedent se mirror; nav row ke baad wali counts line (`rows/shown/diffs/sames`) untouched rakhi (honest totals barkarar)
- Geometry/typo values text/tablefind precedent se reuse (28.0/4.0, small+weak), koi naya token/color nahi

## Files changed (OWNS only, is commit me)

- `crates/gui/src/views.rs` (+~90: 3 pure fns + toolbar wiring + 3 tests)
- `checks/gui-diffnav-am/GATES.md`, `REPORT.md`, `round-1/`

## Note (sibling wave)

- Working tree me `crates/gui/src/app.rs` ka diff sibling merge-apply wave (dry-run/output-path/report fields) ka hai — is commit me staged nahi, untouched chhoda. `checks/gui-apply-am/` untracked bhi unka hai.
- NO push (orchestrator pushes).
