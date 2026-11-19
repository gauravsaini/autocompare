# Wave BM-TABWIRE — GATES (FolderMerge tab: three-way tree wiring)

OWNS: `crates/gui/src/app.rs` (FolderMerge tab branch wiring + App state +
tests only) + `checks/gui-tabwire-bm/**` ONLY.
KEVL: `views.rs` me koi paint edit MANA (callsite threading only —
`paint_three_way_tree` signature stable, koi change expected nahi);
`tree.rs` READ-ONLY (model reuse: `three_way_tree_rows`,
`three_way_conflicts`, `three_way_tree_summary`; model field/struct change
nahi); `screenshot.rs` aur baaki sab READ-ONLY.
`checks/opencode-waves/STATUS.md` ko CHHOONA MANA. Colors FROZEN (koi
hue/token change nahi). Geometry first, typography second (koi geometry/token
change nahi — sirf callsite threading, existing tokens reuse). NO push
(orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
`git stash/push` kabhi nahi.

Scope: REPORT §4 callsite spec EXACT follow. `Tab::FolderMerge` branch
(`app.rs:~5437`) ab flat checklist ke bajaye `views::paint_three_way_tree`
paint karta hai — rows `tree::three_way_tree_rows` se, decisions App-owned
staged `Vec<tree::ThreeWayDecision>` me. Model `tree::three_way_*` reuse.

## G1 — Baseline green (before harness)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 2`
- EXPECT: green (606 passed, 0 failed — 2 naye wiring tests samet)
- EVIDENCE: `round-1/tests.out` (harness-generated, post-edit full suite)

## G2 — OWNS-only diff, KEVL + colors + STATUS untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/app\.rs|checks/gui-tabwire-bm/)" || echo OWNS-CLEAN`
  + `git diff -- crates/gui/src/app.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
  + `git status --short checks/opencode-waves/STATUS.md | grep -q . && echo STATUS-TOUCHED || echo STATUS-CLEAN`
- EXPECT: mere diff me sirf `app.rs` (FolderMerge branch + App state +
  headless tests) + `checks/gui-tabwire-bm/**`; `views.rs`/`tree.rs` me zero
  edit; NO-COLOR-ADD; STATUS-CLEAN

## G3 — FolderMerge wiring intent unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui folder_merge`
- EXPECT: green, kam se kam ye naye tests:
  + `folder_merge_snapshot_lists_sorted_presence` (snapshot rows sorted,
    dir par `is_dir`, missing root khali)
  + `folder_merge_wiring_builds_tree_rows_and_stages_decisions` (snapshots
    se `three_way_tree_rows`, `del.txt` delete-vs-keep + `new.txt`
    added-both conflicts, `keep.txt` clean, decision stage + summary
    `decided == 1`, swap par model + snapshots + decisions sab clear)
- WHAT: App state (`folder_merge_base/left/right` + `folder_merge_decisions`),
  worker payload (`FolderMergeOutcome`), tab branch
  (`paint_three_way_tree` jab rows hon, khali union par flat checklist
  fallback taaki dono painters live rahen), clear sites (path-change, swap,
  require-fail, recv-error) — sab headless covered

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/app.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/app.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/app.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naya helper headless fn (koi indexing nahi,
  `.get()`/iterators only; walk errors skip, worker kabhi nahi rokta)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tabwire-bm/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tabwire-bm/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tabwire-bm/REPORT.md checks/gui-tabwire-bm/round-1/after.png checks/gui-tabwire-bm/round-1/geo-typo-report.md`
  + `git add crates/gui/src/app.rs checks/gui-tabwire-bm/GATES.md checks/gui-tabwire-bm/REPORT.md checks/gui-tabwire-bm/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`), sirf OWNS paths staged
  (`checks/gui-livecomp-bm/` untracked sibling — staged nahi), NO push
