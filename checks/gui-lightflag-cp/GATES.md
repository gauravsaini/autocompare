# GATES — CP-LIGHTFLAG: `--screenshot-mode light` CLI flag

Scope (OWNS): `crates/gui/src/main.rs`, `crates/gui/src/screenshot.rs`,
`checks/gui-lightflag-cp/` only. OFF-LIMITS: `app.rs`, `views.rs`, `theme.rs`,
`STATUS.md` (sab untouched), NO PUSH.

## Gate 1 — tests (green + naya lightflag test)

- Command: `cargo test -p autocompare-gui`
- Result: 674 passed, 0 failed, 0 ignored.
- Naya test: `screenshot::tests::screenshot_mode_flag_defaults_dark_and_accepts_light`
  (default dark, flag-less dark, explicit dark/light, unknown Err, missing-value Err,
  flag-parse-to-render bytes-differ proof). Pehle se green
  `fixture_mode_param_switches_hues` bhi pass.

## Gate 2 — clippy

- Command: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
- Result: zero warnings.

## Gate 3 — fmt

- Command: `rustfmt --edition 2024 --check crates/gui/src/main.rs crates/gui/src/screenshot.rs`
- Result: clean (ek round me diff aaya tha, `rustfmt` lagakar re-verify clean).

## Gate 4 — banned-grep (non-test code)

- Method: dono OWNS rs files me `#[cfg(test)]` ke upar wala hissa scan,
  pattern `unwrap(` / `expect(` / `panic!` / `todo!` / `unsafe`.
- Result: naya zero. Sirf pre-existing `#![forbid(unsafe_code)]` lines hit
  (task ke mutabiq kayam rakhni thi).
- Rust rule: non-test me `unwrap`/`expect`/`panic`/`todo` nahi joda;
  naya code `match` + `map_err`-style (`ok_or_else` precedent) par hai.

## Gate 5 — colors-grep (palette freeze)

- Method: `git diff` (OWNS rs) me `^+` lines par `Color32` / `Visuals` scan.
- Result: zero naye. Light hues purane `FixturePalette::of(Light)` reuse se aate hain
  (`eframe::egui::Visuals::light()` fields). Geometry/typo consts untouched.

## Gate 6 — default-dark byte-stability

- `render_fixture_png` ab `render_fixture_png_with_mode(_, Dark)` wrapper hai (caller same).
- CLI proof: flag-less `before.png` == `--screenshot-mode dark` render (`cmp` identical).
- Cross-wave proof: `before.png` == `checks/gui-darkfix-ce/round-1/after.png`
  (frozen CE baseline, `cmp` identical).

## Gate 7 — PNG magic (`file`)

- `before.png`: PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced (9765 bytes).
- `after.png`: PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced (9728 bytes).
- `cmp before.png after.png` -> differ (mode observable), dono magic ok.

## Gate 8 — OWNS (`git show --stat`, pathspec-only commit)

- Commit message: `feat(gui): --screenshot-mode light CLI flag (CP-LIGHTFLAG)`.
- Staged pathspec ONLY: `git add crates/gui/src/main.rs crates/gui/src/screenshot.rs checks/gui-lightflag-cp`.
- `git add -A` MANA (sibling wave ki files stage nahi honi chahiye).
- Expected stat: 2 rs files + `checks/gui-lightflag-cp/{REPORT.md,GATES.md,before.png,after.png}`.

## Gate 9 — STATUS-untouched + no live process

- `git status --short` / `git diff --name-only` me `STATUS.md` absent.
- Koi dev-server/background process launch nahi kiya; sirf headless CLI renders
  (exit 0/2) jo turant exit hote hain.
