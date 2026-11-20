# REPORT — CP-LIGHTFLAG: `--screenshot-mode light` CLI flag (round-1 PASS)

## 1. Summary

- Fixture PNG ab CLI se light mode me nikalta hai:
  `--screenshot <png> --screenshot-mode light`. Flag ke bina default dark rehta hai.
- `before.png` (default, flag-less) purane frozen CE baseline se byte-identical hai.
  `after.png` (`--screenshot-mode light`) valid light PNG hai, dark se bytes alag.
- Palette ko chhuti nahi: koi naya `Color32` nahi, light hues sirf existing
  `eframe::egui::Visuals::light()` reuse (fixture-mode param wave ka precedent).
- Geometry/typo numbers untouched. `app.rs`, `views.rs`, `theme.rs`, `STATUS.md` untouched.

## 2. Change (OWNS only, 2 rs files + ye checks dir)

- `crates/gui/src/screenshot.rs`:
  - Naya pure helper `screenshot_mode_from_args(&[String]) -> Result<FixtureMode, String>`.
    Default `Dark`; `dark`/`light` accept; unknown ya missing value par `Err`
    (opener precedent: caller exit-code 2). Koi `unwrap`/`expect`/`panic` nahi.
  - Naya test `screenshot_mode_flag_defaults_dark_and_accepts_light`: default dark,
    flag-less dark, explicit dark/light, unknown (`sepia`) Err, missing-value Err,
    plus flag-parse se render karke dark-vs-light bytes-differ proof.
- `crates/gui/src/main.rs`:
  - `--screenshot` route ab `screenshot_mode_from_args` se mode nikalta hai aur
    `render_fixture_png_with_mode(path, mode)` bulata hai (pehle fixed
    `render_fixture_png`). Parse fail par `autocompare-gui: <err>` + exit 2
    (`parse_open_args` precedent jaisa). Help text `app.rs` me hai jo OFF-LIMITS
    hai, isliye wahan koi change nahi.

## 3. Proof (CLI se nikale gaye artifacts, isi dir me)

- `before.png`: flag ke bina render — PNG 960x600 RGBA, 9765 bytes,
  `checks/gui-darkfix-ce/round-1/after.png` (frozen CE baseline) se `cmp` identical.
- `after.png`: `--screenshot-mode light` render — PNG 960x600 RGBA, 9728 bytes,
  `before.png` se bytes alag (`cmp` differs), magic ok.
- Exit codes: bad value (`sepia`) -> 2, missing value -> 2, ok renders -> 0.
- Sibling wave (CO-TBARCALL) same workdir me parallel chal rahi thi; uske `app.rs`
  change se mera diff alag rakha — commit pathspec-only, uski files untouched.

## 4. Gates (detail `GATES.md` me)

- `cargo test -p autocompare-gui`: 674 passed, 0 failed (1 naya lightflag test samet).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero warnings.
- `rustfmt --edition 2024 --check` (dono OWNS rs files): clean.
- Banned-grep (non-test): naya zero (sirf pre-existing `forbid(unsafe_code)` line).
- Colors-grep (diff me `Color32`/`Visuals`): zero naye.
- `file` dono PNGs par: PNG image data, 960x600, 8-bit/color RGBA.
- `STATUS.md` untouched. NO PUSH (local commit only).
