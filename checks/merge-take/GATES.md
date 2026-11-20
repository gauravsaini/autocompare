# Wave MERGETAKE — GATES (3-way Take flow + conflict stepper)

OWNS: `crates/diff/**` + `crates/gui/src/take.rs` + `checks/merge-take/**` ONLY.
KEVL: `app.rs` / `viewmodel.rs` / `views*.rs` / `tree.rs` / `theme.rs` / `compare/*`
read-only (grep-proof only, no edits). Colors frozen. NO PUSH.

Scope: headless 3-way Take (Left/Center/Right, Center == Base alias), Myers/Patience
algorithm choice in diff (`MergeSession::new_with_algorithm`,
`merge3_with_algorithm`), conflict stepper + editable output buffer +
save-to-output text (BC trailing-newline parity) as thin `take.rs` wiring
(`TakeSide`, `TakeSession`, `join_take_output`) over the diff model.
Edge fixture: 5-line base/left/right with 2 conflicts (line2, line4).

## G1 — diff green (238 tests, incl. Center + algorithm parity)

- CHECK: `cargo test -p autocompare-diff`
- EXPECT: `test result: ok` on every suite (97 lib incl.
  `test_take_center_alias_and_algorithm_parity`)
- EVIDENCE: `round-1/tests-diff.out`

## G2 — gui green (782 tests, incl. 4 new take wiring tests)

- CHECK: `cargo test -p autocompare-gui`
- EXPECT: `test result: ok. 782 passed` (778 baseline + 4 new:
  side-mapping, session stepper+output, stepper take-all, trailing-newline)
- EVIDENCE: `round-1/tests-gui.out`
- NOTE: verified in clean HEAD worktree + OWNS files only, because the main
  working tree has a parallel wave's in-progress `tree.rs`/`sync.rs` rename
  (`SyncDirection` test breakage, outside OWNS, untouched).

## G3 — clippy 0, fmt clean (OWNS files only)

- CHECK: `cargo clippy -p autocompare-diff -p autocompare-gui -- -D warnings`
- EXPECT: `Finished` with 0 warnings on OWNS paths
- CHECK: `rustfmt --edition 2024 --check` on the 4 OWNS source files
- EXPECT: clean
- NOTE: main-tree `cargo check -p autocompare-gui` shows 1 pre-existing
  `unused import: SyncDirection` warning in `tree.rs` (parallel wave, KEVL).

## G4 — banned 0 in non-test OWNS code, `forbid(unsafe_code)` kept

- CHECK: `rg -n "unwrap\(|\.expect\(|panic!|todo!|unimplemented!"` on OWNS files
- EXPECT: hits only inside `#[cfg(test)]` modules (which carry explicit
  `#[allow]`); zero in non-test code; no new `allow(dead_code)`

## G5 — fixtures + headless edge test + screenshot

- CHECK: `ls round-1/` contains `base.txt`, `left.txt`, `right.txt`,
  `expected-take-{left,center,right}.txt`, `after.png` (valid PNG),
  `tests-diff.out`, `tests-gui.out`
- EXPECT: all present; `after.png` 960x600 PNG magic ok
- WHAT: 2-conflict edge fixture; take-left/right/center outputs verified
  against `MergeSession` semantics (left/right/center wins on both hunks)

## G6 — OWNS-only commit, NO PUSH

- CHECK: `git status --short` + `git diff --name-only`
- EXPECT: only `crates/diff/**`, `crates/gui/src/take.rs`,
  `checks/merge-take/**` staged/committed; working tree clean for OWNS paths
