# Wave AL-MERGEAPP — GATES (Gap #13: App wiring — per-hunk merge pick decisions)

OWNS: `crates/gui/src/app.rs` + `checks/gui-mergeapp-al/**` ONLY.
KEVL `views.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`views_hex.rs`/`home.rs`/`gutter.rs`/`theme.rs`/`screenshot.rs`
mat chhoona (read bhi avoid — sirf merge-pick intent model read-only use ok:
`viewmodel::MergeHunkPick` / `merge_conflict_hunks` / `merge_pick_summary` / `TakeChoice`
reuse, model fields/struct change nahi). Colors FROZEN (koi hue/token change nahi).
Geometry first, typography second — is wave me paint change nahi (App state + summary
strings only, existing wording pattern reuse). NO push (orchestrator pushes).
Dusre agent ki files kabhi touch mat karo. `git stash/push` kabhi nahi.

Scope: Gap #13 ka App-wiring half. AI-MERGEPICK wave ne per-hunk Take Left/Right
(+Base/Both) buttons se pick intent egui temp store me stage kiya tha
(`checks/gui-mergepick-ai/REPORT.md`). Is wave ka delta `app.rs` me hai: per-hunk
take-left/right (+Base/Both reuse) decision App state me persist ho
(`merge_picks: Vec<MergeHunkPick>`, last-staged wins), summary real picks se gine
(`merge_pick_summary` reuse → `counts_summary` Merge branch real decided/pending
dikhaye). File apply (I/O) agle wave ka hai — is wave me state wiring tak, koi
disk write nahi, koi `views.rs` signature change nahi.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (378 passed, 0 failed, recorded 2026-11-20, `main@df1e1f1`).
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail copy).

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/app.rs|checks/gui-mergeapp-al/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/app.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `app.rs` + `checks/gui-mergeapp-al/**`; KEVL-CLEAN; OWNS-CLEAN;
  NO-COLOR-ADD (App strings only, koi color/token change nahi).

## G3 — App pick decisions persist + summary real (headless unit-tested)

- CHECK: `cargo test -p autocompare-gui merge`
- EXPECT: green, kam se kam ye naye tests:
  + `merge_decide_persists_last_wins` (Left phir Right → Right; alag hunk judta hai)
  + `merge_decide_unknown_hunk_ignored` (bina model / galat index → false, vec unchanged)
  + `merge_pick_for_hunk_returns_last_staged` (unknown index `None`, khali `None`)
  + `merge_pick_summary_reflects_app_picks` (`merge_pick_summary` reuse: total 1,
    1 staged → resolved 1 / pending 0; bina pick pending 1)
  + `merge_summary_line_counts_real_picks` (no-model `no results yet`; model par
    decided/pending numbers real picks se)
  + `merge_swap_and_recompare_clear_picks` (swap / naya result stale intent inherit nahi karta)
  + `merge_adopt_dedups_and_prunes_unknown` (bridge adopt: duplicate me aakhri jeetti; unknown prune)
  + `merge_reconcile_clears_on_lines_change` / `merge_reconcile_keeps_picks_without_change`
    (global-Take lines-change par clear + status, nahi to adopt)
- WHAT: `App` me naya field `pub merge_picks: Vec<MergeHunkPick>` (+Default empty);
  methods `merge_hunks`, `merge_pick_for_hunk`, `decide_merge_hunk` (unknown ignore,
  upsert last-wins, bool return), `clear_merge_picks`, `merge_pick_summary`,
  `merge_summary_line`; `counts_summary` Merge branch `merge_summary_line` reuse;
  clear points: `swap_sides` Merge, `run_merge` validation-fail, result landing
  Ok/Err (fresh compare purani intent inherit nahi karta); paint call-site
  `views::paint_merge(ui, model)` unchanged; koi I/O nahi. Is wave ka delta
  paint bridge hai: `Tab::Merge` branch seed/adopt — App picks
  `merge_pick_id` temp store me seed (cards App truth dikhayen), paint ke
  baad staged `reconcile_merge_picks` (lines-change clear + status, nahi to
  adopt); KEVL `paint_merge` signature unchanged.

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/app.rs; echo fmt:$?`
  (alternative scoped: `cargo fmt --check -- crates/gui/src/app.rs`)
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/app.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/app.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/app.rs | grep -nE "\[[a-zA-Z_][a-zA-Z0-9_]*\s*\]" | grep -v "Vec\|String\|Option\|Result\|\[u8\]\|default" || echo NO-INDEXING`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test
  code (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi — `.get()` only;
  `&[u8]` signature + `#[default]` benign hits excluded; saturating arithmetic;
  `format!` only allocation).
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen
  (sirf apne OWNS files par `cargo fmt -- <files>` pattern).

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-mergeapp-al/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-mergeapp-al/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64.

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-mergeapp-al/REPORT.md checks/gui-mergeapp-al/round-1/after.png checks/gui-mergeapp-al/round-1/geo-typo-report.md`
  + `git add crates/gui/src/app.rs checks/gui-mergeapp-al/GATES.md checks/gui-mergeapp-al/REPORT.md checks/gui-mergeapp-al/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push.
