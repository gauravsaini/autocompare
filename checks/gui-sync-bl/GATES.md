# GATES — Wave BL-SYNCMODEGATE (BI-S2 mode radio + BI-S3 ek gate, P1)

Wave: BL-SYNCMODEGATE. Spec: `checks/gui-mergesync-bi/REPORT.md` §spec
BI-S2 (mode paint me chuna jata hai) + BI-S3 (consent gate ek jagah).
OWNS: `crates/gui/src/views.rs` + `crates/gui/src/app.rs` +
`checks/gui-sync-bl/**`. KEVL (mat chhoona): `tree.rs` model READ-ONLY
(reuse only: `set_sync_mode`, `can_apply`, `can_apply_now`,
`SyncModeChoice::label`, `home::sync_consent_hint`),
`screenshot.rs` / `views_hex.rs` / `views_table.rs` / theme READ-ONLY,
`checks/opencode-waves/STATUS.md` UNTOUCHED. Colors FROZEN. NO push.

## G1 — BI-S2 mode radio paint me + set_sync_mode wiring

- CHECK: `grep -n "paint_sync_mode_row\|sync_mode_choices" crates/gui/src/views.rs`
  + `grep -n "set_folder_sync_mode(picked)" crates/gui/src/app.rs`
- EXPECT: radio row `views.rs` me (picture tab `radio_value` precedent),
  `paint_folder_sync` return `SyncModeChoice`, Folder branch pick ko
  `set_folder_sync_mode` se wire kare (mode-switch consent reset reuse)
- EVIDENCE: `views.rs` me `sync_mode_choices()` + `paint_sync_mode_row()`;
  `paint_folder_sync` top par radio, grid + selected line `picked` par,
  return `picked`; `app.rs` Folder branch me purana inline radio block hata
  kar `if picked != ... { self.set_folder_sync_mode(picked); }`

## G2 — BI-S3 dono apply paths ek gate par

- CHECK: `grep -n "sync_tab_can_apply\|sync_consent_hint" crates/gui/src/app.rs`
- EXPECT: Sync branch me `sync_consent_hint(confirmed, copy_steps)` paint +
  Apply button `self.sync_tab_can_apply()` par; Folder branch pehle se
  `folder_can_apply()` (= `can_apply_now`) + hint par tha — dono ek shape
- EVIDENCE: naya `App::sync_tab_can_apply` (`sync_apply_enabled` core reuse);
  Sync confirm row ab hint samet horizontal me; Folder preview section
  unchanged (radio grid paint me gaya, gate/hint same)

## G3 — cargo test full green

- CHECK: `cargo test --workspace`
- EXPECT: 43 suites ok, zero failed
- EVIDENCE: gate command output (round-1/tests.out me gui suite)

## G4 — clippy zero

- CHECK: `cargo clippy --workspace --all-targets`
- EXPECT: zero warning, zero error
- EVIDENCE: gate command output (empty)

## G5 — fmt sirf OWNS par, bare cargo fmt MANA

- CHECK: `cargo fmt --check -p autocompare-gui` (read-only, koi write nahi)
- EXPECT: exit 0
- EVIDENCE: gate command output (`FMT_EXIT:0`; bare `cargo fmt` chalaya hi nahi)

## G6 — banned grep zero new

- CHECK: mere diff ke `+` lines par
  `unwrap|expect(|panic!|todo!|unsafe|Color32::from_rgb|Visuals::`
- EXPECT: ZERO match
- EVIDENCE: gate command output (`BANNED_DONE`, koi match nahi);
  `#![forbid(unsafe_code)]` intact

## G7 — PNG magic + distinct

- CHECK: `file round-1/*.png` + magic + size + distinct
- EXPECT: `after.png` 9695 B distinct 255, `after-sync.png` 10732 B
  distinct 256, `after-folder.png` 10068 B distinct 256 — magic ok,
  >= 8192 ok, teeno alag sizes (koi fallback-copy nahi)
- EVIDENCE: `round-1/after.png`, `round-1/after-sync.png`, `round-1/after-folder.png`

## G8 — harness PASS

- CHECK: `bash tools/gui-polish/run-geo-typo.sh checks/gui-sync-bl/round-1`
- EXPECT: EXIT 0, `RESULT: PASS`
- EVIDENCE: `round-1/geo-typo-report.md` (Grid x10, monospace x55,
  strong x23, small x54, `.size` sirf `['14.0']`)

## G9 — Commit, no push (pathspec sirf OWNS)

- CHECK: `git log --oneline -1` + `git show --stat HEAD`
- EXPECT: `feat(gui): ...` conventional English message; fileset sirf
  `crates/gui/src/views.rs` + `crates/gui/src/app.rs` +
  `checks/gui-sync-bl/`; `git push` kabhi nahi chalaya
- EVIDENCE: log + show output

## DoD

- Folder grid me mode radio live (Update Both / Mirror), pick
  `set_folder_sync_mode` se judvaan (consent reset samet); Sync tab me
  confirm hint + shared gate; tests green, clippy/fmt/banned/PNG gates
  pass, REPORT Hinglish-Roman, commit local only, NO push.
