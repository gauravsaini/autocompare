# REPORT — Wave AD-APPLY (Gap #8, folder Sync one-direction apply)

Date: 2026-11-20 UTC. Repo: `/Users/ektasaini/autocompare`, branch `main`.
OWNS: `crates/gui/src/app.rs` + `checks/gui-apply-ad/**` only. No push.
Clean-room: Beyond Compare bahar se observed, apne shabdon me; koi string/asset copy nahi.

## Kya bana

Gap #8 ka pehla real wiring — Sync apply preview-only se aage, sirf ONE
safe direction (left-to-right copy of new/changed files), dono UI surfaces par:

- Shared safe core (`app.rs`, headless-tested): `mirror_copy_options()`
  (LeftToRight + prune false — is combo se `plan_sync` sirf `CopyToRight`
  steps de sakta hai), `mirror_copy_subset()` (wider plan aaye to bhi sirf
  `CopyToRight` rakhta hai), `plan_mirror_copy()` (engine `plan_sync`,
  read-only), `apply_mirror_copy()` (engine `apply_sync`, subset filter karke),
  `describe_mirror_report()`. `crates/compare` me zero edit.
- Folder tab: Mirror preview ke neeche dry-run checkbox (default ON) +
  last-report line; Apply dabane par dry-run me sirf ginati batata hai
  (koi write nahi), consent + uncheck par engine plan se safe subset apply
  karta hai. Deletes kabhi nahi.
- Sync tab: checklist-driven Apply — `SyncChecklist::copy_right_plan()`
  (sirf checked `CopyToRight` file rows), dry-run checkbox (default ON) +
  explicit confirm checkbox + gated "Apply checked copies" button. Har real
  Apply ke baad confirm reset (dobara tick chahiye — jaanboojhkar friction).
- Stale-report honesty: swap sides / recompute (Ok ya Err) purani report +
  confirm saaf karte hain; dry-run pref rehta hai.

## Naye/touched API (sab `app.rs`, sab headless-tested)

- `App::mirror_copy_options / mirror_copy_subset / plan_mirror_copy /`
  `apply_mirror_copy / describe_mirror_report / folder_should_write /`
  `run_folder_mirror_apply / set_folder_sync_dry_run`
- `App::{sync_dry_run, sync_confirmed, sync_last_report}` (default true/false/None) +
  `set_sync_dry_run / set_sync_confirmed / run_sync_checklist_apply`
- `SyncChecklist::copy_right_plan` + free helpers `sync_apply_enabled`,
  `sync_dry_run_summary`
- Paint (existing tokens only — `TIGHT_GAP`, `monospace`/`small`/`strong`,
  koi `Color32`/`Visuals` touch nahi): Folder Apply section + Sync-tab
  Apply section (`app.rs` me, `views.rs` untouched).

## Gates evidence

- G1 core: `mirror_copy_options_are_left_to_right_without_prune`,
  `mirror_copy_subset_keeps_only_copy_to_right` pass.
- G2 temp dirs: `mirror_copy_applies_left_to_right_in_temp_dirs` (new+upd copy,
  right-only bachta, `deleted == 0`) +
  `sync_apply_copies_new_and_changed_left_to_right` (2 copied, same-size +
  `touch_newer` deterministic mtime, koi sleep nahi) +
  `sync_checklist_apply_end_to_end_confirmed` (dry-run me no-write, confirm
  par 1 copied, confirm reset) pass.
- G3 UI gate: `folder_mirror_apply_defaults_to_dry_run_and_needs_consent`,
  `sync_copy_right_plan_keeps_only_checked_copies`,
  `sync_apply_gate_needs_confirm_and_steps`,
  `sync_dry_run_default_reports_without_writes`,
  `sync_apply_without_confirm_writes_nothing`,
  `apply_reports_clear_on_swap` pass.
- G4: `cargo test -p autocompare-gui` → 317 passed, 0 failed.
- G4 clippy: `cargo clippy -p autocompare-gui --all-targets` → zero warnings.
- G4 fmt: `cargo fmt -- crates/gui/src/app.rs` + `cargo fmt --check` clean (sirf own file).
- G4 banned: `unwrap(|.expect(|panic!|todo!` grep `app.rs` me empty
  (sirf `#![forbid(unsafe_code)]` line me `unsafe` shabd aata hai).
- G5 harness: `tools/gui-polish/run-geo-typo.sh checks/gui-apply-ad/round-1`
  → RESULT: PASS (fresh `round-1/` proof is commit me hai).

## Process note (duplicate worker)

Is wave me do AD-APPLY workers ek hi working tree me sakriya mile: ek ne
Folder-tab wiring + core likha, dusre ne Sync-tab wiring + apne GATES/REPORT
likhe (dusre ke GATES me "Folder Apply stub rahega" non-goal tha jo file ki
asliyat se match nahi karta tha, isliye GATES/REPORT ko combined reality par
rewrite kiya). Dusre worker ka failing fixture (`changed.txt` 8 vs 9 bytes →
engine `Different` skip) usi ne khud `touch_newer` se fix kiya; reviewer ne
verify kiya. Dono halves ek hi safe core share karte hain, 317 tests green,
isliye combined state ko ek commit me finalize kiya (delete karne se green
kaam nasht hota). KEVL files + `crates/compare` untouched; `stash/push` nahi.

## Limitations

1. `Different` (content-differ, same size + content-compare off) entries
   hamesha skipped — engine rule; Apply unhe copy nahi karta.
2. `NewerLeft` grade mtime par hai (content-compare default off); UI me
   "changed" ka matlab newer-mtime wali same-size files hai.
3. Deletes/prune is kadam me nahi — Mirror preview me dikhne wale
   `DeleteRight` steps kabhi apply nahi hote (subset filter + prune:false).
4. Agla kadam: recompare ke baad auto-refresh, progress/parinat UI polish.
