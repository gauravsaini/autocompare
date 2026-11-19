# REPORT — Wave AL-MERGEAPP (Gap #13: App wiring — per-hunk merge pick decisions)

## Summary

App-side merge decision wiring live: `merge_picks: Vec<MergeHunkPick>` App state me persist hota hai,
`decide_merge_hunk` per-hunk take-left/right (+Base/Both reuse) last-wins upsert karta hai,
summary real picks se ginta hai (`merge_pick_summary` reuse). Paint signature unchanged
(`views::paint_merge(ui, model)` same call), sirf App seed/adopt bridge temp-store se sync karta hai.
File apply (I/O) scope bahar hai — koi disk write nahi. Clean-room: Beyond Compare bahar se
observe only, apne shabdon me likha. Full suite 393 green, clippy zero, harness PASS.

## Key Metrics

- Tests: 378 (baseline pre-edit, `main@df1e1f1`) → 393 (9 naye App tests, 0 fail)
- Clippy `-p autocompare-gui --all-targets`: 0 warnings
- `rustfmt --edition 2024 --check crates/gui/src/app.rs`: clean (scoped `cargo fmt -- <file>` only)
- Harness: `RESULT: PASS` (PNG 9624 bytes, magic ok `89504e470d0a1a0a`, distinct 256)
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`) non-test me: zero; `forbid(unsafe_code)` kept
- Colors: NO-COLOR-ADD; KEVL-CLEAN; OWNS-CLEAN

## Highlights

- ✅ Naya field `pub merge_picks: Vec<MergeHunkPick>` (+Default empty, swap/fresh-compare par clear)
- ✅ Methods (headless-tested): `merge_hunks`, `merge_pick_for_hunk` (last-staged wins, unknown `None`),
  `decide_merge_hunk` (unknown ignore → false, upsert last-wins → true), `clear_merge_picks`,
  `merge_pick_summary` (no-model par zero summary), `merge_summary_line` (real decided/pending)
- ✅ `counts_summary` Merge branch ab `merge_summary_line` reuse karta hai (purani static line hata)
- ✅ Clear points: `swap_sides` Merge, `run_merge` validation-fail, result landing Ok/Err par picks clear —
  fresh compare purani intent inherit nahi karta
- ✅ Paint bridge: frame start par App picks temp me seed, `paint_merge` ke baad staged wapas adopt
  (dedup last-wins + unknown-index prune), lines badalne par clear + status — KEVL signature unchanged
- ✅ Geometry first, typography second — App strings only, koi hue/token change nahi

## Blockers / Notes

- 🚨 Koi blocker nahi. OWNS-only diff (`crates/gui/src/app.rs` + `checks/gui-mergeapp-al/**`);
  KEVL files read bhi avoid kiye (sirf `viewmodel::` intent API reuse), `git checkout --` dusre paths par nahi.
- GATES G4 note follow kiya: bare `cargo fmt` nahi, sirf `cargo fmt -- crates/gui/src/app.rs` pattern.
- NO push — commit local only, orchestrator pushes.

## Decisions Needed

- [ ] Agle wave: staged `merge_picks` se file apply (I/O via `apply_merge_picks`) + Apply-all UX

## Verification

| Gate | Check | Result |
|------|-------|--------|
| G1 | baseline 378/0 (`main@df1e1f1`, `round-1/baseline-tests.out`) | ok |
| G2 | OWNS-only + KEVL-CLEAN + NO-COLOR-ADD | ok |
| G3 | `cargo test -p autocompare-gui merge` (9 App merge tests) + full 393/0 | ok |
| G4 | clippy 0 + fmt 0 + ZERO-BANNED + UNSAFE-FORBID-OK + NO-INDEXING | ok |
| G5 | harness + PNG magic/size/distinct | PASS, 9624 / magic ok / 256 |
| G6 | commit, no push | feat commit, working tree clean |
