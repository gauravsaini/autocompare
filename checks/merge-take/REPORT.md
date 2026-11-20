# REPORT — Wave MERGETAKE (3-way Take flow + conflict stepper)

## Summary

3-way merge Take flow live on the headless model: `Take Left/Center/Right`
(Center == Base ancestor alias, BC parity), Myers/Patience algorithm choice
in `autocompare-diff`, conflict stepper, editable output buffer with undo,
save-to-output text with BC trailing-newline parity, thin `take.rs` wiring
over the diff model. Edge fixture (2 conflicts) + valid screenshot PNG in
`round-1/`. RESULT: PASS.

## Key Metrics

- Tests: diff 237 (96+33+44+20+4+40 +1 new) → 238 total lines counted as
  97+33+44+20+4+40 suites green; gui 778 → 782 (+4 new take wiring tests)
- Clippy `-- -D warnings` on diff+gui: 0 warnings on OWNS paths
- `rustfmt --edition 2024 --check` on 4 OWNS source files: clean
- Banned (`unwrap(`/`expect(/panic!/todo!`) non-test OWNS: zero;
  `forbid(unsafe_code)` kept everywhere
- Harness: `after.png` 10363 bytes, PNG magic ok, 960x600 RGBA

## Highlights

- ✅ `crates/diff/src/merge3.rs`: `merge3_blocks_with_algorithm` /
  `merge3_with_algorithm` (Myers default preserved, Patience selectable)
- ✅ `crates/diff/src/merge.rs`: `TakeChoice::Center` const alias == `Base`
  (+ `is_center`), `MergeSession::new_with_algorithm`, `take_center` /
  `take_all_center`, algorithm-parity headless test
- ✅ `crates/diff/src/lib.rs`: `merge3_with_algorithm` public export
- ✅ `crates/gui/src/take.rs` (thin wiring only, old marker API intact):
  `TakeSide` Left/Center/Right + labels, `join_take_output` (BC parity),
  `TakeSession` (stepper `step_to_first/next/prev`, `take_left/center/right`,
  `take_all`, `clear`, `output_lines/text`, editable `TextBuffer` + `sync_buffer`)
- ✅ `checks/merge-take/round-1/`: base/left/right fixtures (2 conflicts),
  expected take-left/center/right outputs, `after.png`, test evidence tails
- ✅ OWNS-only commit, NO PUSH

## Blockers / Notes

- 🚨 Main working tree currently has a parallel wave's in-progress rename in
  `crates/compare/src/sync.rs` + `crates/gui/src/tree.rs` (`SyncDirection` →
  `SyncRowDirection`, tests half-migrated, `cargo test -p autocompare-gui`
  red there, plus 1 pre-existing `unused import` warning in `tree.rs`).
  Outside OWNS — untouched. Full-suite evidence above was measured in a clean
  HEAD worktree (`/tmp/mergetake-verify`, since removed) with only the 4 OWNS
  source files copied over: diff 97+33+44+20+4+40 ok, gui 782 ok, clippy 0.

## Decisions Needed

- [ ] Next wave (paint): wire `TakeSession` into merge paint cards
  (Take Left/Center/Right buttons + stepper prev/next + editable output pane
  + save-to-output apply). Headless contract ready; no paint changes here.

## Verification

| Gate | Check | Result |
|------|-------|--------|
| G1 | `cargo test -p autocompare-diff` | ok, 97+33+44+20+4+40 (`tests-diff.out`) |
| G2 | `cargo test -p autocompare-gui` | ok, 782 (`tests-gui.out`, clean worktree) |
| G3 | clippy `-D warnings` + rustfmt | 0 warnings, FMT-CLEAN |
| G4 | banned scan + unsafe-forbid | 0 non-test, kept |
| G5 | fixtures + `after.png` PNG check | present, magic ok |
| G6 | OWNS-only commit, no push | done (see commit) |

RESULT: PASS
