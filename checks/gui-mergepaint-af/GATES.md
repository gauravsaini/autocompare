# Wave AF-MERGEPAINT — GATES (Gap #7 paint half: 3-pane merge paint)

OWNS: `crates/gui/src/views.rs` + `checks/gui-mergepaint-af/**` ONLY.
KEVL `app.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`views_hex.rs`/`gutter.rs`/`theme.rs`/`screenshot.rs`
mat chhoona (read-only, grep-proof only). Merge-plan model reuse read-only
(`viewmodel::merge_conflict_hunks` + `merge_pick_summary`, `TakeChoice` reuse).
Colors FROZEN (koi hue/token change nahi). Geometry first, typography second
(sirf existing tokens: `SECTION_GAP`/`TIGHT_GAP`/`HEADER_H` + small/weak/strong/monospace).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
`git stash/push` kabhi nahi.

Scope: Gap #7 ka paint half. Model half (Wave AA-MERGE) landed hai:
`ThreePanePlan` + `merge_conflict_hunks`/`merge_pick_summary` headless.
`App` side `MergeSummary` + `paint_triple_paths` + `run_merge` KEVL hain
(verify-only). Is wave ka delta `views.rs` me hai: `paint_merge` me
3-pane section — har conflict hunk ke left/base/right panes (marker block
se real parse) + per-hunk decision display (real pending counts) + per-hunk
Take buttons stub (label me "stub" saaf). Global Take Left/Right/Base/Both
functional rehta hai (compat, `resolve_take` index 0). `paint_merge`
signature unchanged (KEVL `app.rs` call-site compat).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (324 passed, 0 failed, recorded 2026-11-20, `main@305e4f2`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-mergepaint-af/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `checks/gui-mergepaint-af/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (sirf existing frozen palette reuse:
  `red_for`/`dim_for`, `CHANGED_BG` nahi — marker red already frozen)

## G3 — 3-pane merge paint unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui merge`
- EXPECT: green, kam se kam ye naye tests:
  + `merge_hunk_sides_splits_left_base_right` (full marker block)
  + `merge_hunk_sides_without_base_marker_puts_left_before_sep` (no `|||||||`)
  + `merge_hunk_sides_out_of_range_is_empty` (start/end bahar)
  + `merge_three_pane_summary_line_counts_pending` (total/pending/conflicted wording)
  + `merge_hunk_title_and_decision_labels` (hunk index + pending wording, stub shabd nahi — paint label me hai)
  + `merge_pane_count_is_three`
  + purane merge tests same values par pass (`merge_pick_summary` downstream compat)
- WHAT: `views.rs` me pure helpers (`merge_hunk_sides`, `format_merge_hunk_title`,
  `format_merge_hunk_decision`, `merge_three_pane_summary_line`, `merge_pane_count`)
  + `paint_merge_three_pane` (hunks + summary read-only reuse, teen panes real,
  per-hunk decision real pending, per-hunk Take buttons stub-labelled no-op);
  `paint_merge` signature unchanged; global Take functional untouched

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views.rs -- --check; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "\[[a-zA-Z_][a-zA-Z0-9_]*\s*\]" | grep -v "Vec\|String\|Option\|Result" || echo NO-INDEXING`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only;
  saturating arithmetic; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-mergepaint-af/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-mergepaint-af/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-mergepaint-af/REPORT.md checks/gui-mergepaint-af/round-1/after.png checks/gui-mergepaint-af/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-mergepaint-af/GATES.md checks/gui-mergepaint-af/REPORT.md checks/gui-mergepaint-af/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
