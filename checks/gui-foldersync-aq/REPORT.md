# REPORT — Wave AQ-FOLDERSYNC (round-1)

Scope: folder compare me sync/copy direction actions ka App wiring + tree paint affordance headless — direction badges, selection model, dry-run summary real. Paint adoption agle wave ka hai, is wave me koi paint code nahi.

OWNS: `crates/gui/src/tree.rs`, `crates/gui/src/app.rs`, `checks/gui-foldersync-aq/**`. KEVL files untouched (read bhi avoid).

## Key metrics

- Tests: 448 passed, 0 failed (`cargo test -p autocompare-gui`), isme 8 naye (6 tree + 2 app).
- Clippy: OWNS files me zero warnings. Repo-wide 17 warnings sab `crates/gui/src/views.rs` (KEVL, parallel agent ka WIP) me hain — is wave ne us file ko chhooa nahi.
- `cargo fmt --check`: clean (sirf OWNS files par `cargo fmt --` chalaya).
- Banned grep (non-test): clean — sirf do `#![forbid(unsafe_code)]` guard lines match hoti hain, koi `unwrap()/expect()/panic!/todo!/unimplemented!` nahi.
- Harness: `tools/gui-polish/run-geo-typo.sh checks/gui-foldersync-aq/round-1` → RESULT: PASS.

## Kya bana (headless only)

tree.rs (additive, 535 lines):
- `SyncRowDirection` (None/CopyToRight/CopyToLeft/DeleteRight/Manual) + `badge()` (`=`, `->`, `<-`, `X`, `?`) + `label()` + `action()`.
- `sync_direction_for_row` — wahi per-row faisla jo `sync_preview` leta hai (dirs: sirf Mirror+R prune).
- `sync_badge_for_row` — badge text helper.
- `FolderSyncSelection` — BTreeSet multi-select (select/toggle/remove/clear/contains/ordered/retain_real_rows).
- `SyncSummaryCounts` + `summarize_sync_preview` + `format_sync_summary` — real counts line (`dry-run: N L->R, ...`).
- `sync_preview_for_selection` — scoped preview, wahi dir-prune rules.
- Geometry: `SYNC_BADGE_CHARS=2`, `sync_badge_min_px` (advance+pad math, row 24.0/pad 8.0/indent 12.0 frozen reuse). Typography: `sync_badge_typo` (mono caption, delete strong), `sync_summary_typo` (mono caption regular). Koi color change nahi.

app.rs (additive, 163 lines):
- Naya field `folder_sync_selection` + Default init.
- Methods: `folder_sync_direction`, `folder_sync_badge`, `select/toggle/clear_folder_sync_row` (sirf real rows), `folder_sync_selected_preview`, `folder_sync_summary_line`, `folder_sync_selected_summary_line` (`N selected: <real line>`).
- Reset: fresh compare Ok/Err, swap, reopen-restore par selection clear (stale paths kabhi jhooth na bolen). Mode switch par selection rehta hai. Paint code untouched.

## Blockers / notes

- Koi blocker nahi. Note: working tree me `crates/gui/src/views.rs` (KEVL) me parallel agent ke uncommitted changes hain + `checks/gui-wrap-aq/` untracked hai — dono ko chhooa nahi. Is commit me sirf OWNS paths hain.
- Clippy repo-wide zero nahi (17 pre-existing views.rs warnings); OWNS zero hai. Gate evidence upar metrics me.

## Gate evidence

- `cargo test -p autocompare-gui` → ok, 448 passed.
- `cargo clippy -p autocompare-gui --all-targets` → warnings sirf views.rs me, tree.rs/app.rs me koi nahi.
- `cargo fmt -- crates/gui/src/tree.rs crates/gui/src/app.rs && cargo fmt --check` → clean.
- Banned grep → clean (forbid guards ke siva).
- `bash tools/gui-polish/run-geo-typo.sh checks/gui-foldersync-aq/round-1` → PASS (report round-1/ me).
