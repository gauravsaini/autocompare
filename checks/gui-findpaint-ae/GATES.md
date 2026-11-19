# Wave AE-FINDPAINT — GATES (Gap #12 paint half: text find box + N/M badge)

OWNS: `crates/gui/src/views.rs` + `crates/gui/src/viewmodel.rs` + `checks/gui-findpaint-ae/**` ONLY.
KEVL `app.rs`/`tree.rs`/`views_table.rs`/`views_hex.rs`/`gutter.rs`/`theme.rs`/`screenshot.rs`
mat chhoona (read-only, grep-proof only). Colors FROZEN (koi hue/token change nahi).
Geometry first, typography second (sirf existing tokens: `FIND_BAR_H`/`FIND_NEEDLE_W`/
`BADGE_PAD` + small weak badge). NO push (orchestrator pushes). Dusre agent ki files
kabhi touch mat karo. `git stash/push` kabhi nahi.

Scope: Gap #12 ka paint half. Model half (Wave Z-FIND + Wave Q/S/T) landed hai:
`viewmodel::FindState` + `find_count_label`/`find_match_indices` + `App` side
`text_find_*` state (KEVL — verify-only). Is wave ka delta `views.rs` me hai:
`FindState` ko text paint me wire karna — find TextEdit + Next/Prev + count badge.
Table/folder find precedent reuse karo (needle field 120px + Prev/Next buttons +
count label; `table_cell_contains`/`folder_name_contains` jaisa case-flag matcher).
Purana App-wired `paint_text_find_bar` untouched rehta hai (compat).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (recorded count `round-1/baseline-tests.out` me)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views|viewmodel).rs|checks/gui-findpaint-ae/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|tree.rs|views_table.rs|views_hex|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/viewmodel.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `viewmodel.rs` + `checks/gui-findpaint-ae/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (sirf existing frozen palette reuse)

## G3 — FindState paint wiring + count badge unit-tested

- CHECK: `cargo test -p autocompare-gui find`
- EXPECT: green, kam se kam ye tests:
  + purane find tests same values par pass (`find_matches_case_insensitive_over_both_panes`,
    `find_state_cycles_next_prev_with_wrap`, `find_state_empty_query_has_no_position`,
    `find_count_label_formats_n_of_m`, `find_state_count_label_tracks_cursor`)
  + naya: case-flag matcher (`find_match_indices_cased` ignore_case true/false;
    whitespace-only needle par empty)
  + naya: `FindState::set_query_cased` + `set_ignore_case` recompute (cursor first hit par)
  + naya (views): `advance_text_find` — next/prev clicks se row milta hai, khali par None
  + naya (views): `find_bar_docks_top_with_right_badge_air` already PASS (GEO tokens reuse)
- WHAT: `viewmodel.rs` me case-flag matcher + `FindState::ignore_case` state;
  `views.rs` me `paint_text_find_state_bar` (FindState-owned bar) +
  `paint_text_with_find` (bar + `paint_text_with_prefs` delegation, selection sync) +
  headless `advance_text_find` helper; `#[allow(dead_code)]` wiring-pending hatana jahan
  paint use kare (clippy zero-warnings se proof)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views.rs crates/gui/src/viewmodel.rs && git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs crates/gui/src/viewmodel.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs crates/gui/src/viewmodel.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS files par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only;
  saturating arithmetic; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-findpaint-ae/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-findpaint-ae/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-findpaint-ae/REPORT.md checks/gui-findpaint-ae/round-1/after.png checks/gui-findpaint-ae/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs crates/gui/src/viewmodel.rs checks/gui-findpaint-ae/GATES.md checks/gui-findpaint-ae/REPORT.md checks/gui-findpaint-ae/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
