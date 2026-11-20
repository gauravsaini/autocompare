# CI-DARKFULL — option C full: persist + fixture mode-param (round-1 PASS)

## 1. Flow (root-cause se fix tak)

- Root-cause (gui-darkroot-cc sec-5): A wave (9e7b25b) ne startup apply joda
  (`main.rs:52` me `set_visuals(dark)`), par `dark_mode` per-run-only tha —
  restart par wapas default, aur fixture renderer ka koi mode switch nahi tha.
- Unlazy order follow kiya: pehle `GATES.md` likha (persist tests, default-dark
  byte-stability, mode-param proof), phir implement kiya.
- Beech me ek self-bug pakda: mechanical const-swap `FixturePalette::of` ke andar
  bhi lag gaya tha (`bg: fixture_palette().bg` — infinite recursion, stack overflow).
  HEAD-stash se confirm kiya baseline green tha, phir Dark arm wapas consts par kiya.
  Tests green hone ke baad aage badha.

## 2. Fix (3 owned files, laziest diff)

- `crates/gui/src/main.rs`: create closure ab `cc.storage` se `dark_mode` load karta hai
  (`App::dark_mode_from_storage`, trim precedent) aur usi hisab se `set_visuals`
  (dark/light, dono existing `Visuals` API) lagata hai. A path reuse — `app.rs:4819`
  first-frame guard zaroori nahi tha, isliye joda nahi.
- `crates/gui/src/app.rs`: `DARK_MODE_STORAGE_KEY` + `dark_mode_from_storage` /
  `dark_mode_to_storage` (trim precedent, unknown/khali par declared intent dark)
  + `impl eframe::App for App` me `fn save` (storage me "1"/"0"). Naya framework zero —
  sirf built-in eframe Storage.
- `crates/gui/src/screenshot.rs`: `FixtureMode::{Dark, Light}` (`Default = Dark`),
  `render_fixture_png` ab `render_fixture_png_with_mode(_, Dark)` wrapper hai
  (callers/tests untouched), chaar base hues `fixture_palette()` se aate hain
  (thread-local mode guard, nested-safe). Light hues sirf
  `eframe::egui::Visuals::light()` fields se — naya palette zero.

## 3. Gates (round-1)

- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-darkfull-ci/round-1/` →
  `RESULT: PASS` (round 1/6). PNG 9765 bytes, magic ok, non-blank.
- Byte-stability: `cmp` CE baseline (`gui-darkfix-ce/round-1/after.png`) se
  byte-identical — default path purani fixtures se same bytes deta hai.
- `cargo test -p autocompare-gui`: 669 passed, 0 failed (5 naye: storage parse,
  save roundtrip, default-dark, wrapper==Dark, Dark-vs-Light proof + guard-restore).
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings
  (ek `matches!` suggestion aaya tha, fix karke re-verify).
- `cargo fmt --check`: clean (sirf apni lines format hui).
- Banned zero (non-test me `unwrap(`/`expect(`/`panic!`/`todo!`/`unsafe` naya zero;
  `unwrap_or` wale pre-existing pattern untouched).
- Colors zero new: live paint me naya `Color32` hue zero; light sirf existing
  `Visuals::light()` reuse.
- Shared-workdir: har test/gate se pehle `git status --short` — sirf OWNS dirty.
  `STATUS.md` untouched. PUSH nahi kiya (orchestrator verifies + pushes).

## 4. Follow-up (orchestrator ke liye)

- Live re-capture photo-proof (Light-Mode OS par dark khulna + toggle ke baad
  restart par persist) is wave ke bahar hai — headless `save`/parse tests + harness
  PASS is wave ka proof hai.
- Light fixture abhi sirf chaar base hues badalta hai (diff bands frozen-dark);
  poori light-scene chahiye to alag wave me bands ka `Visuals::light()` mapping.
