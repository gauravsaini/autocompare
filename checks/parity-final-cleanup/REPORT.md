# Parity Final Cleanup Report (Wave K, Agent A)

Date: 2026-11-19 UTC. Repo: /Users/ektasaini/autocompare. Scope: sirf
`crates/gui/src/viewmodel.rs` edit hua; `checks/parity-final-cleanup/**`
me ye report. Baaki GUI files (`views_hex.rs`, `views_table.rs`, `app.rs`,
`views.rs`, `main.rs`, `gutter.rs`, `tree.rs`, `theme.rs`, `STATUS.md`)
chhui nahi.

## 1. Kya hataaya (per-item)

1. `build_sync_summary` — REMOVED. Repo-wide grep me iska koi caller
   nahi mila (sirf khud ke 2 unit tests). Replacement pehle se wired hai:
   `app.rs` me `SyncChecklist::from_plan` + `run_sync` usi ko use karta hai.
2. `build_folder_merge_summary` — REMOVED. Koi bahari caller nahi
   (sirf 2 khud ke tests). Replacement: `FolderMergeChecklist::from_plan`
   + `run_folder_merge`.
3. `build_audio_summary` — REMOVED. Koi bahari caller nahi (sirf 3 khud
   ke tests). Replacement: `AudioChecklist::from_diff` + `run_audio`.
4. `build_picture_summary` — KEPT. Ye obsolete nahi hai: `app.rs` me
   `PictureCompareModel::from_diff` isko andar call karta hai, isliye
   hatana breakage laata.

## 2. Saboot (grep repo-wide)

- Hatane se pehle grep `build_sync_summary|build_folder_merge_summary|
  build_audio_summary|PictureCompareModel`: 3 builders ke hits sirf
  `viewmodel.rs` ke andar (definition + khud ke tests) aur purani docs/
  reports me; `app.rs`/`views.rs` me koi call nahi.
- Hatane ke baad grep: code me zero references; sirf retired-note
  comments (`viewmodel.rs` me 2 jagah) aur purani reports/docs me zikr.
- Isliye bahar ke callers ko haath lagane ki zaroorat nahi padi; koi
  re-export/shim ki need nahi hui (zero caller = zero migration).

## 3. Counts

- Functions removed: 3 (`build_sync_summary`, `build_folder_merge_summary`,
  `build_audio_summary`) + unke doc comments/section headers.
- Tests removed: 7 (sync 2, folder-merge 2, audio 3) — sab removed
  functions ke khud ke tests the; replacement tests `app.rs`/`views.rs`
  me untouched rakhe.
- Imports cleaned: 5 naam (`SyncAction`, `SyncPlan`, `TreeMergeAction`,
  `TreeMergePlan`, `AudioDiff`) top-level `use` se hataye; local test
  `use` lines tests ke saath gayin.
- Lines: `viewmodel.rs` me 19 insertions, 255 deletions.
- Tests: `cargo test -p autocompare-gui` 189 passed se 182 passed (delta
  -7 = hataye hue obsolete tests; 0 failed).
- Clippy: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  pehle 3 dead-code errors, ab ZERO errors worktree-wide (sibling red
  nahi tha, isliye full gate green).
- Fmt: `cargo fmt -p autocompare-gui -- --check` clean (owned file).
- Report size: ye file 15KB se bahut neeche.

## 4. Orchestrator ke liye caller-migration note

- Koi caller migration pending nahi hai — teenon builders ke zero
  external callers the, isliye koi file update karne ko nahi bachi.
- Naya code rule: sync/folder-merge/audio ke liye `Vec<String>` summary
  builder wapas na laaya jaaye; seedha structured types use hon:
  `SyncChecklist::from_plan`, `FolderMergeChecklist::from_plan`,
  `AudioChecklist::from_diff`, picture ke liye `PictureCompareModel::
  from_diff` (jo `build_picture_summary` reuse karta hai).
- Agar bhavishya me koi bahari crate ko plain lines chahiye hon to
  checklist ke `summary()`/`display_line()` se banaya jaaye, naya
  duplicate builder `viewmodel.rs` me na joda jaaye.

## 5. File refs

- `crates/gui/src/viewmodel.rs`: retired-note comment (~1164),
  imports (~42-46), test-note (~2074).
- Replacement models (untouched, Wave J-B): `app.rs` me
  `PictureCompareModel::from_diff`, `SyncChecklist::from_plan`,
  `FolderMergeChecklist::from_plan`, `AudioChecklist::from_diff`;
  run paths `run_sync`/`run_picture`/`run_folder_merge`/`run_audio`.

## 6. Limitations (6)

1. Ye cleanup sirf owned file tak seemit hai; `app.rs`/`views.rs` ke
   checklist models ke andar ka dead code (agar ho) is scope se bahar hai.
2. Hataye hue 7 tests ke exact wording-assertions ab repo me nahi hain;
   CLI-wording parity ab checklist `summary()` tests se cover hoti hai.
3. `build_picture_summary` ab bhi `viewmodel.rs` me hai kyunki
   `from_diff` usko use karta hai — picture summaries ka single owner
   ab bhi yehi function hai.
4. Purani reports/docs (`parity-picture/REPORT.md`, `parity-cleanup/
   REPORT.md`, `docs/plan-v0.5.0.md`) me builders ka zikr bacha hai;
   history ke liye chheda nahi.
5. Headless PNG harness me naye paint paths render nahi hue; coverage
   unit-test mapping tak hai, pixel-proof tak nahi.
6. Clippy worktree-wide abhi green hai, par sibling agent (`ses-cleanup-j2`
   `views_table.rs` par) ke mid-edit par dobara red ho sakta hai; us
   waqt gate dobara chalana padega.

Gates: tests green (182/0), owned + worktree clippy zero, fmt clean.
Commit locally, NO push, `STATUS.md` untouched.
