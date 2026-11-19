# Wave BK-MERGEMULTI — GATES (BI-M2: three-way tree paint)

OWNS: `crates/gui/src/views.rs` (naya paint fn + tests only) + `checks/gui-mergemulti-bk/**` ONLY.
KEVL `app.rs` me koi wiring MANA (callsite spec REPORT §4 me), `tree.rs` READ-ONLY
(model reuse: `three_way_tree_rows`, `is_three_way_conflict`, `three_way_tree_summary`;
model field/struct change nahi). Baaki sab READ-ONLY.
`checks/opencode-waves/STATUS.md` ko CHHOONA MANA. Colors FROZEN (koi hue/token
change nahi). Geometry first, typography second (sirf existing tokens:
`TIGHT_GAP` + status role header + mono rows + small weak notes). NO push
(orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
`git stash/push` kabhi nahi.

Scope: BI-M2 ka paint half. Model half landed hai (`tree.rs:2485-2656` headless).
Is wave ka delta `views.rs` me hai: naya `paint_three_way_tree` (three-way tree
rows + per-conflict Take Left/Base/Right + header me `summary().summary_line()`),
pure helpers ke saath. FolderMerge tab (`app.rs:~5403`) abhi flat checklist
(`paint_folder_merge_checklist`, `views.rs:5569`) paint karta hai — tab wiring
agle wave ka hai; is wave me paint fn `#[allow(dead_code)]` pending painter hai
(`paint_folder_sync` precedent).

## G1 — Baseline green (before harness)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 2`
- EXPECT: green (601 passed, 0 failed: 595 prior + 6 naye)
- EVIDENCE: `round-1/tests.out` (harness-generated, post-edit full suite)

## G2 — OWNS-only diff, KEVL + colors + STATUS untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views\.rs|checks/gui-mergemulti-bk/)" || echo OWNS-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
  + `git status --short checks/opencode-waves/STATUS.md | grep -q . && echo STATUS-TOUCHED || echo STATUS-CLEAN`
- EXPECT: mere diff me sirf `views.rs` (pure additive, zero removed lines) +
  `checks/gui-mergemulti-bk/**`; `app.rs` ka uncommitted diff parallel BI-S1 agent
  ka hai (chhooa nahi, commit me shaamil nahi); NO-COLOR-ADD (sirf existing frozen
  palette reuse: `red_for`/`dim_for`); STATUS-CLEAN

## G3 — three-way paint intent unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui three_way`
- EXPECT: green, kam se kam ye naye tests:
  + `three_way_presence_marks_cover_sides` (`BLR` / `B-R` / `-L-`)
  + `three_way_row_line_marks_dirs` (dir par `/` mark)
  + `three_way_decision_for_row_returns_last_staged` (duplicate me aakhri jeetta hai; unknown `None`)
  + `stage_three_way_decision_upserts_last_wins` (same path overwrite, naya path judta hai)
  + `three_way_row_decision_text_covers_states` (pending vs clean vs `take <side>` intent)
  + `three_way_header_line_uses_summary` (conflict count + `(unresolved)` + resolve par clean)
- WHAT: pure helpers (`three_way_presence_marks`, `three_way_row_line`,
  `three_way_decision_for_row`, `stage_three_way_decision`,
  `three_way_row_decision_text`, `three_way_header_line` — sab `tree::three_way_*`
  reuse) + `paint_three_way_tree` (header summary line, presence marks + path +
  decision display per row, conflict rows par real Take Left/Base/Right buttons jo
  staged decisions me likhte hain; signatures naye hain, purani signatures untouched)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()`/iterators
  only; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-mergemulti-bk/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-mergemulti-bk/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-mergemulti-bk/REPORT.md checks/gui-mergemulti-bk/round-1/after.png checks/gui-mergemulti-bk/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-mergemulti-bk/GATES.md checks/gui-mergemulti-bk/REPORT.md checks/gui-mergemulti-bk/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`), sirf OWNS paths staged
  (`app.rs` ka parallel diff + `checks/gui-syncwire-bk/` staged nahi), NO push
