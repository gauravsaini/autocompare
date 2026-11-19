# Wave AA-MERGE — GATES (Gap #7 model half: 3-pane merge plan)

OWNS: `crates/gui/src/tree.rs` + `crates/gui/src/viewmodel.rs` + `checks/gui-merge-aa/**` ONLY.
KEVL `views.rs` / `views_table.rs` / `app.rs` / `theme.rs` / `screenshot.rs` mat chhoona
(read-only, grep-proof only). `views.rs` paint is scope se BAHAR (agla wave) —
ye wave sirf model + pure helpers + headless tests deta hai.
Colors FROZEN (koi hue change nahi). Geometry first, typography second.
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: `TriplePaths` (base/left/right) se 3-pane merge-plan model.
Text side (`viewmodel.rs`): left/base/right line hunks + per-hunk take-side
decision list (`TakeChoice` reuse) + decision summary helper.
Tree side (`tree.rs`): left/base/right presence rows + per-row take-side
decision list + decision summary helper. Koi egui, koi I/O, koi filesystem nahi.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `278 passed; 0 failed` (recorded 2026-11-20, `main@3548ca1`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(tree|viewmodel).rs|checks/gui-merge-aa/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|views_table.rs|app.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/tree.rs crates/gui/src/viewmodel.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `tree.rs` + `viewmodel.rs` + `checks/gui-merge-aa/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (koi naya hue nahi)

## G3 — 3-pane merge-plan decisions unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui merge_plan; cargo test -p autocompare-gui three_`
- EXPECT: green, kam se kam ye naye tests:
  + viewmodel text side: hunks split (same/left-only-change/right-only-change/both-same/conflict),
    per-hunk decisions se `merge_lines` (decided splice, undecided conflict par marker block),
    dedup (last pick wins), unknown hunk reject, summary counts (total/decided/pending/conflicted)
  + tree side: `three_way_tree_rows` union-sorted presence rows,
    `is_three_way_conflict` (delete-vs-keep + added-both-sides),
    decisions per rel_path (dedup last-wins, unknown path reject), summary counts
  + paint wiring agle wave me: naye types `#[allow(dead_code)]` wiring-pending,
    existing `views.rs` API untouched

## G4 — MS Rust guidelines + fmt clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/tree.rs crates/gui/src/viewmodel.rs -- --check; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/tree.rs crates/gui/src/viewmodel.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -12 crates/gui/src/tree.rs | grep -q "forbid(unsafe_code)" && head -20 crates/gui/src/viewmodel.rs | grep -q "use crate::take" && echo UNSAFE-FORBID-OK`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/tree.rs crates/gui/src/viewmodel.rs | grep -nE "\[[a-zA-Z_][a-zA-Z0-9_]*\s*\]" | grep -v "Vec\|String\|Option\|Result" || echo NO-INDEXING`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing/slicing nahi, `.get()` only)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-merge-aa/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-merge-aa/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-merge-aa/REPORT.md checks/gui-merge-aa/round-1/after.png checks/gui-merge-aa/round-1/geo-typo-report.md`
  + `git add crates/gui/src/tree.rs crates/gui/src/viewmodel.rs checks/gui-merge-aa/GATES.md checks/gui-merge-aa/REPORT.md checks/gui-merge-aa/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
