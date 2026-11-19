# REPORT — Wave BM-TABWIRE (FolderMerge tab: three-way tree wiring)

## Summary

FolderMerge tab wiring land ho gaya: `Tab::FolderMerge` branch
(`app.rs:~5437`) ab flat checklist ke bajaye `views::paint_three_way_tree`
paint karta hai — rows `tree::three_way_tree_rows` se, per-conflict Take
Left/Base/Right buttons staged `Vec<tree::ThreeWayDecision>` me jaate hain,
header me `summary().summary_line()` aata hai. Callsite spec REPORT §4
(BK-MERGEMULTI) EXACT follow hua — `views.rs` me zero edit, `tree.rs`
read-only reuse. Khali union par flat checklist fallback rehta hai taaki
dono painters live rahen (koi dead_code nahi). Full suite 606 green, clippy
zero, fmt clean, harness PASS.

## Key Metrics

- Tests: 606 passed, 0 failed (2 naye wiring tests samet)
- Clippy `--all-targets -- -D warnings`: 0 warnings
- `rustfmt --edition 2024 --check crates/gui/src/app.rs`: clean
- Harness: `RESULT: PASS` (PNG 9695 bytes, magic ok, distinct 255)
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`) non-test me: zero; `forbid(unsafe_code)` kept
- Diff shape: `app.rs` me 256 insertions, 11 removals (sirf intended
  replacements: payload, clear sites, worker map, recv, callsite)

## Highlights

- ✅ App state: `folder_merge_base/left/right: Vec<FolderRow>` snapshots +
  `folder_merge_decisions: Vec<tree::ThreeWayDecision>` (naye compare / swap /
  error / path-change par clear, model ke saath)
- ✅ Worker payload: `CompareResult::FolderMerge` ab
  `Result<FolderMergeOutcome, String>` hai — plan ke saath teen snapshots
  (`snapshot_folder_presence`: iterative walk, symlink-safe, errors skip,
  sorted; worker kabhi nahi rokta). Saare send/recv sites `app.rs` me hi
  hain, isliye OWNS-safe
- ✅ Tab branch: model maujood + rows non-empty → `paint_three_way_tree`;
  khali union → flat checklist fallback (empty-plan honest rehta hai);
  model nahi → purana hint. Dono paint fns reachable, koi `dead_code` nahi
- ✅ Tests (headless, temp dirs): `folder_merge_snapshot_lists_sorted_presence`
  (sorted rows, dir mark, missing root khali) +
  `folder_merge_wiring_builds_tree_rows_and_stages_decisions` (delete-vs-keep
  + added-both conflicts, clean row, stage + summary `decided == 1`,
  swap par sab clear)

## Blockers / Notes

- 🚨 Sibling untracked dir `checks/gui-livecomp-bm/` dikha `git status` me.
  Maine usse touch nahi kiya; mera commit sirf OWNS paths stage karta hai.
  Orchestrator merge/push sambhale.
- G4 ka fmt command `rustfmt --edition 2024 <file>` hai (single OWNS file;
  bare `cargo fmt` nahi chalaya taaki sibling files untouched rahen).
- Harness `after.png` default fixture scene ka hai (live FolderMerge paint
  nahi — tab paint egui runtime me hota hai); is wave ka proof headless
  tests + full suite + harness PASS hai.
- Flat checklist ka fate: replace nahi, else-branch fallback rakha — khali
  union (teenon roots khali) par plan ka empty-state wahi dikhata hai, aur
  dono painters live rehte hain.

## Decisions Needed

- [ ] Next wave: staged decisions se real Apply wiring (dry-run gate +
  output path) — is wave me scope bahar tha; decisions sirf stage hoti
  hain, filesystem par kuch nahi likhti

## Verification

| Gate | Check | Result |
|------|-------|--------|
| G1 | full suite 606/0 | ok, `round-1/tests.out` |
| G2 | mere diff me OWNS-only + NO-COLOR-ADD + STATUS-CLEAN | ok (`app.rs` wiring only; `views.rs`/`tree.rs` zero edit) |
| G3 | `cargo test -p autocompare-gui folder_merge` (5 pass: 2 naye wiring + 3 existing) + full 606 | ok |
| G4 | clippy `-D warnings` 0 + fmt 0 + ZERO-BANNED + UNSAFE-FORBID-OK | ok |
| G5 | harness + PNG magic/size/distinct | PASS, 9695 / magic ok / 255 |
| G6 | commit, no push | feat commit, sirf OWNS paths staged |
