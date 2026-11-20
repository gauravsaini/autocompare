# REPORT — Wave CN-OPENER (content-row live photo-proof unblock)

Wave: sess CN-OPENER. Scope (OWNS): `crates/gui/src/main.rs`,
`crates/gui/src/app.rs`, `checks/gui-opener-cn/` only. Baaki sab
(`views.rs`, `theme.rs`, `home.rs`, `screenshot.rs`, `STATUS.md`)
untouched — sirf read kiya.

## 1. Problem

Live captures me content rows (selective accent + filler hatch +
status 3-zone + cursor strip) photo-proof karne ke liye GUI me do
paths type karne padte hain. Is env me keyboard injection aur
drag-drop block hai, to paths andar pahunch hi nahi paate — round-7
verify wave atki hui thi.

## 2. Fix (sirf wiring, koi paint change nahi)

- Naya CLI flag: `--open <left> <right> [--tab <name>]`
  (non-interactive opener). Launch par dono path session me preload
  hote hain, manga tab khulta hai, phir auto-compare chalta hai taaki
  content rows bina typing ke banen.
- `--tab` default `text`; valid names: text/folder/hex/merge/table/
  sync/picture/foldermerge/audio. `home` reject hota hai (paths ke
  saath launcher kholna jhooth hota).
- `--help`/`-h` GUI khole bina usage print karke exit(0) karta hai
  (help-safe, test me verify hua).
- Missing/unknown values par honest error + exit(2); koi panic nahi.
- Reuse only: `set_tab` + `run_active_tab` (+ `require_paths` ka
  honest error panel). Colors FROZEN (koi naya Color32/Visuals nahi),
  geometry/typo numbers untouched, `#![forbid(unsafe_code)]` kayam,
  non-test me unwrap/expect/panic/todo zero.
- Existing flags/behavior untouched: `--screenshot` route same order
  me pehle exit karta hai; bina `--open` ke launch bilkul purana hai.

## 3. Proof

- `cargo test -p autocompare-gui`: 673 passed, 0 failed (2 naye
  opener tests: flag parse + preload, defaults/rejects/help).
- Binary proof (GUI khole bina): `--help` exit 0 + usage text;
  `--open only-one` exit 2 + honest error line.
- Fixture byte-stability: `before.png` aur `after.png` do alag
  `--screenshot` runs se, sha256 identical
  (`bde8ccf4...15dc`), `file` dono ko PNG 960x600 RGBA batata hai.
  Opener `--screenshot` path ko chhu hi nahi sakta (screenshot exit
  opener parse/apply se pehle hota hai) aur renderer files
  (`screenshot.rs`, `views*.rs`) diff me hain hi nahi.
- Live-proof khud nahi kiya (round-7 verify wave karegi). Suggested
  launch: `autocompare-gui --open <left> <right> --tab text`.

## 4. Files

- `crates/gui/src/app.rs`: `OpenRequest` struct + `tab_from_name` +
  `parse_open_args` + `apply_open_request` + `help_requested` +
  `help_text` + 2 tests (+264 lines).
- `crates/gui/src/main.rs`: `--help` early exit + opener parse +
  launch-time apply (+23/-1).
- `checks/gui-opener-cn/`: `REPORT.md`, `GATES.md`, `before.png`,
  `after.png`.
