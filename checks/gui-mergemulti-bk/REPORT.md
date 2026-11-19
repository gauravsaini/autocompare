# REPORT — Wave BK-MERGEMULTI (BI-M2: three-way tree paint)

## Summary

Three-way tree paint land ho gaya: naya `paint_three_way_tree` (`views.rs`, BI-M2
spec BI-M2 ke mutabiq) three-way rows + per-conflict Take Left/Base/Right + header
me `summary().summary_line()` paint karta hai. Model `tree.rs:2485-2656` se reuse
hai (read-only, koi model change nahi). FolderMerge tab wiring is wave me MANA tha
— paint fn `#[allow(dead_code)]` pending painter hai, callsite spec §4 me. Full
suite 601 green, clippy zero, fmt clean, harness PASS.

## Key Metrics

- Tests: 595 (prior) → 601 (6 naye, sab `three_way_*` paint helpers par)
- Clippy `--all-targets -- -D warnings`: 0 warnings (doc-continuation lint fix samet)
- `rustfmt --edition 2024 --check crates/gui/src/views.rs`: clean
- Harness: `RESULT: PASS` (PNG 9695 bytes, magic ok, distinct 255)
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`) non-test me: zero; `forbid(unsafe_code)` kept
- Diff shape: `views.rs` me pure additive (zero removed lines); purani signatures untouched

## Highlights

- ✅ Pure helpers (headless, unit-tested): `three_way_presence_marks` (`BLR`/`B-R`/`-L-`
  fixed 3-char), `three_way_row_line` (dir par `/` mark), `three_way_decision_for_row`
  (last-staged wins, `three_way_tree_summary` dedup rule ka mirror),
  `stage_three_way_decision` (upsert — same path overwrite, naya path judta hai),
  `three_way_row_decision_text` (pending vs clean vs `take <side>` intent),
  `three_way_header_line` (`three_way_tree_summary().summary_line()` reuse)
- ✅ `paint_three_way_tree`: header status-role summary line, har row par presence
  marks (conflict red / clean dim, frozen palette) + mono path + decision display,
  conflict rows par 3 real buttons (Take Left / Take Base / Take Right —
  `ThreeWaySide::label()` reuse, koi stub nahi); click staged decisions me jata hai,
  model rows untouched
- ✅ KEVL clean: `app.rs` wiring nahi chhooa, `tree.rs` read-only reuse,
  `STATUS.md` untouched, colors frozen (koi naya hue/token nahi), geometry/typo
  existing tokens only

## Blockers / Notes

- 🚨 Parallel wave agent ka uncommitted diff `app.rs` me hai (BI-S1 sync wiring) +
  untracked `checks/gui-syncwire-bk/` dir dikha `git status` me. Maine unhe touch
  nahi kiya; mera commit sirf OWNS paths stage karta hai. Orchestrator merge/rebase sambhale.
- GATES G4 ka fmt command `rustfmt --edition 2024 --check <file>` hai (single-file;
  bare `cargo fmt` nahi chalaya taaki sibling files untouched rahen).
- Harness `after.png` default fixture ka hai (live FolderMerge paint nahi — wiring
  agle wave me); is wave ka proof headless tests + harness PASS hai.

## §4 — Callsite spec (agle wave ka kaam, is wave me MANA)

- Jagah: `app.rs` `Tab::FolderMerge` branch (`~5403-5414`): abhi `if let Some(list) =
  self.folder_merge_model.as_mut()` par `views::paint_folder_merge_checklist(ui, list)`
  paint hota hai, warna "no folder-merge plan yet" hint.
- Agle wave me: App par teen snapshots (base/left/right `Vec<FolderRow>`) + staged
  `Vec<tree::ThreeWayDecision>` state jodo (naye fields, `folder_merge_model` ke
  saath ya uski jagah — agle wave ka OWNS `app.rs` hai); rows
  `tree::three_way_tree_rows(&base, &left, &right)` se banao, phir
  `views::paint_three_way_tree(ui, &rows, &mut decisions)` call karo; flat checklist
  hatao yaElse-branch me rakho (agle wave ka faisla). Is wave me `app.rs` me ZERO edit.
- Contract: `paint_three_way_tree(ui, rows, decisions)` signature stable hai —
  wiring sirf call + state hai, paint fn me koi change expected nahi.

## Decisions Needed

- [ ] Next wave (FolderMerge wiring): App state shape (snapshots kahan se aayenge —
  existing `folder_merge_model` plan se ya naye base/left/right fields se) + flat
  checklist ka fate (replace vs fallback) — is wave me scope bahar tha

## Verification

| Gate | Check | Result |
|------|-------|--------|
| G1 | full suite 601/0 | ok, `round-1/tests.out` |
| G2 | mere diff me OWNS-only + NO-COLOR-ADD + STATUS-CLEAN | ok (views.rs additive only) |
| G3 | `cargo test -p autocompare-gui three_way` (9 pass: 6 naye + 3 model) + full 601 | ok |
| G4 | clippy `-D warnings` 0 + fmt 0 + ZERO-BANNED + UNSAFE-FORBID-OK | ok |
| G5 | harness + PNG magic/size/distinct | PASS, 9695 / magic ok / 255 |
| G6 | commit, no push | feat commit, sirf OWNS paths staged |
