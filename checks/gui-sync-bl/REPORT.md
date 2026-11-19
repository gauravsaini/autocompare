# REPORT — Wave BL-SYNCMODEGATE (BI-S2 mode radio + BI-S3 ek gate, P1)

Spec: `checks/gui-mergesync-bi/REPORT.md` §spec table BI-S2 + BI-S3.
Pehle haal: `paint_folder_sync` mode ko param leta tha par koi
`radio_value` nahi tha (BI report ka shabd: "Mode kahin paint NAHI hota");
Folder preview section me App-side inline radio tha jo paint-owned nahi
tha; Sync tab me dry-run + confirm to the par `sync_consent_hint` kahin
paint nahi hota tha aur Apply button seedha free fn par tha jabki Folder
side `can_apply_now` method par — do gate, do roop.

## Kya badla (sirf OWNS)

1. **`crates/gui/src/views.rs`** (+67/-lines, net radio + return):
   - Naya `sync_mode_choices() -> [SyncModeChoice; 2]` — radio ke options
     ka ek source of truth (picture tab ke `PictureMode::all` jaisa).
     Pure; unit-tested (`labels Update Both / Mirror`, order fixed).
   - Naya `paint_sync_mode_row(ui, mode) -> SyncModeChoice` — `Mode:`
     label + dono variants par `radio_value` (picture tab
     `paint_picture_compare` wala precedent, `views.rs:~5241`). Caller
     pick ko `set_folder_sync_mode` se wire kare; colors frozen.
   - `paint_folder_sync` ab radio row paint karta hai (summary ke neeche),
     grid badges + selected line `picked` mode par paint hote hain (same
     frame feedback), aur picked mode return hota hai. Stale
     `#[allow(dead_code)]` hata (BK wave se live-called hai). `tree.rs`
     ko chhooa nahi — sirf `label()` + direction/badge helpers reuse.
2. **`crates/gui/src/app.rs`**:
   - Folder branch: `paint_folder_sync` ka return `picked` liya, alag hua
     to `set_folder_sync_mode(picked)` (mode-switch consent reset reuse,
     `tree.rs:1991-1994`); purana inline radio block hataya (ab paint ke
     andar ek jagah). Options label / summary / steps / consent / Apply
     same hain.
   - BI-S3 gate: naya `App::sync_tab_can_apply()` — confirmed + checked
     left->right copies, wahi `sync_apply_enabled` core jo free fn me
     test tha. Folder side `folder_can_apply()` (= `can_apply_now`,
     consented + steps) pehle se tha — ab dono Apply buttons ek-ek
     method-gate par: `folder_can_apply()` vs `sync_tab_can_apply()`,
     dono ka core `consented/confirmed && steps > 0`.
   - Sync branch: confirm checkbox ab `home::sync_consent_hint(confirmed,
     copy_steps)` wali hint ke saath horizontal row me (Folder branch
     wala roop), counts line same, Apply button `sync_tab_can_apply()`
     par. `run_*_apply` andar ke guards untouched (defense in depth).
3. **Tests**: `sync_mode_choices_offer_update_both_first_then_mirror`
   (views) + `sync_apply_gate_needs_confirm_and_steps` me
   `sync_tab_can_apply` matrix (no-model false, unconfirmed false,
   confirmed true, uncheck-all false).
4. **`checks/gui-sync-bl/`**: GATES.md + yeh REPORT + round-1
   (`after.png`, `after-sync.png`, `after-folder.png`,
   `geo-typo-report.md`, `tests.out`, `screenshot.out`).

## Kya nahi chhooa (KEVL)

- `tree.rs` model READ-ONLY (reuse only, koi field/fn change nahi).
- `screenshot.rs` / `views_hex.rs` / `views_table.rs` / theme READ-ONLY.
- `checks/opencode-waves/STATUS.md` UNTOUCHED.
- Colors: koi hue/token baat nahi (diff me koi color API nahi).
- Sync tab plan fixed hai (`SyncOptions::default` = LeftToRight/no-prune,
  safe `mirror_copy_subset` wala path) — isliye Sync tab par mode radio
  NAHI lagaya: bina model change ke radio jhooth bolta (plan badalta
  nahi). Yeh scoping note BI-S2 follow-up ka honest dayara hai; mode ka
  ghar folder-sync surface hai jahan `set_sync_mode` + preview judvaan
  live hai.

## Evidence

- `cargo test --workspace`: **43 suites ok, zero failed** (gui suite me
  **604 passed, 0 failed**; do naye tests samet — `round-1/tests.out`).
- `cargo clippy --workspace --all-targets`: **zero warning, zero error**.
- `cargo fmt --check -p autocompare-gui`: exit 0 (bare `cargo fmt` MANA,
  chalaya nahi).
- Banned scan (mere `+` lines par
  `unwrap|expect(|panic!|todo!|unsafe|Color32|Visuals`): **ZERO match**.
- Harness: `bash tools/gui-polish/run-geo-typo.sh
  checks/gui-sync-bl/round-1` → EXIT 0, `RESULT: PASS`
  (`round-1/geo-typo-report.md`; Grid x10, monospace x55, strong x23,
  small x54, `.size` sirf `['14.0']` — koi typo antar nahi).
- PNGs (dispatcher filename-route se, teeno alag scenes):
  `after.png` 9695 B distinct 255, `after-sync.png` 10732 B distinct 256,
  `after-folder.png` 10068 B distinct 256 — teeno `file` me
  `PNG image data, 960x600, RGBA`, magic `89504e470d0a1a0a` ok,
  >= 8192 ok, non-blank ok. Teen alag sizes = koi fallback-copy nahi.

## Behavior (pehle vs ab)

- Pehle: Folder grid me mode radio nahi tha (App preview section me
  alag inline radio tha); Sync tab me consent ka koi hint nahi tha aur
  gate free fn par tha.
- Ab: Folder grid ke top par Mode radio (Update Both / Mirror) live hai;
  pick turant grid badges + selected line me dikhta hai aur App me
  `set_folder_sync_mode` se consent reset hota hai. Sync tab me confirm
  ke saath wahi hint (`preview only` / `consented` / `nothing to apply`)
  jo Folder side par thi, aur Apply button shared-gate method par hai.
  Dono paths ab bhi wahi likhte hain jo kehte hain: Folder dry-run
  report-only, Sync checked left->right copies only, never deletes.

## Files (sirf OWNS)

- `crates/gui/src/views.rs` (radio helper + choices + paint return)
- `crates/gui/src/app.rs` (Folder pick wiring + `sync_tab_can_apply` + Sync hint/gate)
- `checks/gui-sync-bl/GATES.md` (yeh wave ke gates)
- `checks/gui-sync-bl/REPORT.md` (yeh file)
- `checks/gui-sync-bl/round-1/after.png`
- `checks/gui-sync-bl/round-1/after-sync.png`
- `checks/gui-sync-bl/round-1/after-folder.png`
- `checks/gui-sync-bl/round-1/geo-typo-report.md`
- `checks/gui-sync-bl/round-1/tests.out`
- `checks/gui-sync-bl/round-1/screenshot.out`
