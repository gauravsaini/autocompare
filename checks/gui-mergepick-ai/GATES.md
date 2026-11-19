# Wave AI-MERGEPICK — GATES (Gap #13: per-hunk merge pick UI)

OWNS: `crates/gui/src/views.rs` + `checks/gui-mergepick-ai/**` ONLY.
KEVL `app.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`views_hex.rs`/`gutter.rs`/`theme.rs`/`screenshot.rs`
mat chhoona (read-only, grep-proof only). Merge-plan model reuse read-only
(`viewmodel::merge_conflict_hunks` + `merge_pick_summary` + `MergeHunkPick`,
`TakeChoice` reuse — model fields/struct change nahi). Decision ka App wiring
agli wave ka hai; is wave me pick intent local state (egui temp store) + display tak.
Colors FROZEN (koi hue/token change nahi). Geometry first, typography second
(sirf existing tokens: `SECTION_GAP`/`TIGHT_GAP` + small/weak/strong/monospace).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
`git stash/push` kabhi nahi.

Scope: Gap #13 ka paint half. Model half landed hai:
`merge_conflict_hunks` + `MergeHunkPick`/`apply_merge_picks`/`merge_pick_summary`
headless. Wave AF-MERGEPAINT ne `paint_merge_three_pane` me per-hunk Take buttons
stub-label no-op chhode the ("(stub)" shabd ke saath). Is wave ka delta `views.rs`
me hai: har hunk card par real Take Left / Take Right buttons (plus Take Base /
Take Both bhi real — koi stub nahi), click par pick intent egui temp store me
stage hota hai, decision display pending se "take <side>" intent me badalta hai,
summary line real staged picks se ginata hai. Global Take `paint_merge` me
functional rehta hai (compat, `resolve_take` index 0). `paint_merge` aur
`paint_merge_three_pane` signature unchanged (KEVL `app.rs` call-site compat).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (355 passed, 0 failed, recorded 2026-11-20, `main@bbfad4b`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-mergepick-ai/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `checks/gui-mergepick-ai/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (sirf existing frozen palette reuse:
  `red_for`/`dim_for` marker red already frozen; koi naya hue/token nahi)

## G3 — per-hunk pick intent unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui merge`
- EXPECT: green, kam se kam ye naye tests:
  + `merge_pick_for_hunk_returns_last_staged` (duplicate picks me aakhri jeetta hai; unknown index `None`)
  + `stage_merge_pick_upserts_last_wins` (doosri pick same hunk overwrite karti hai; nayi hunk judti hai)
  + `merge_pick_choice_labels_are_take_sides` (Left->"left", Right->"right", Base->"base", Both->"both")
  + `merge_pick_intent_and_decision_with_pick_labels` (pick bina pending wording; pick ke saath "take left"/"take right" intent)
  + `merge_pick_summary_reflects_staged_picks` (staged picks se pending ghat-ta hai, `merge_pick_summary` reuse)
  + purane merge tests same values par pass (`format_merge_hunk_decision` pending wording compat; AF tests untouched)
- WHAT: `views.rs` me pure helpers (`merge_pick_id`, `merge_pick_choice_label`,
  `merge_pick_for_hunk`, `stage_merge_pick`, `format_merge_pick_intent`,
  `format_merge_hunk_decision_with_pick`) + `paint_merge_three_pane` (temp-store
  picks, summary `merge_pick_summary(lines, &picks)` se, har card par real
  Take Left/Right/Base/Both buttons jo `stage_merge_pick` karte hain, decision
  display intent dikhata hai); `paint_merge`/`paint_merge_three_pane` signature
  unchanged; global Take functional untouched; koi "(stub)" label nahi

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "\[[a-zA-Z_][a-zA-Z0-9_]*\s*\]" | grep -v "Vec\|String\|Option\|Result" || echo NO-INDEXING`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only;
  saturating arithmetic; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-mergepick-ai/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-mergepick-ai/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-mergepick-ai/REPORT.md checks/gui-mergepick-ai/round-1/after.png checks/gui-mergepick-ai/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-mergepick-ai/GATES.md checks/gui-mergepick-ai/REPORT.md checks/gui-mergepick-ai/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
