# GATES — gui-darkfull-ci (option C full: A + persist + fixture mode-param)

Scope: `main.rs` (storage load), `app.rs` (storage save + headless parse helpers),
`screenshot.rs` (FixtureMode + default-dark dispatch). NO-TOUCH: views.rs, home.rs,
viewmodel.rs, theme.rs, baaki crates/gui/src/*, baaki checks, STATUS.md.

Root-cause (gui-darkroot-cc sec-5): A wave (9e7b25b) ne startup apply joda par
`dark_mode` per-run-only hai — restart par wapas default, aur fixture renderer ka
koi mode switch nahi (frozen-dark).

## Gate 1 — persist tests (headless, eframe Storage reuse, naya framework zero)

- `dark_mode_from_storage`: trim precedent (`SessionSnapshot::new` trim) —
  `" 0 "`/`"light"` → false, `"1"`/`"true"`/`"dark"` → true,
  unknown/khali/None → declared intent true (jhootha light kabhi nahi,
  `age_secs` unknown→None precedent jaisa graceful).
- `dark_mode_to_storage` roundtrip: true→"1"→true, false→"0"→false.
- `App::save` fake-Storage test: dark=false save karke `get_string(KEY)=="0"`.
- `main.rs` create closure: `cc.storage` se load, phir `set_visuals` (A path reuse —
  `app.rs:4819` first-frame guard TABHI jab persist ke liye zaroori ho, warna nahi).

## Gate 2 — fixture default-dark byte-stability

- `render_fixture_png(path)` == `render_fixture_png_with_mode(path, Dark)` byte-identical
  (wrapper, koi caller/test touch nahi).
- Dark palette consts (`BG_DARK`/`BG_PANEL`/`FG_TEXT`/`FG_DIM`) unchanged — existing
  fixtures ke bytes same rahen.

## Gate 3 — fixture mode-param proof

- `FixtureMode::{Dark, Light}`, `Default = Dark`.
- Dark vs Light bytes differ (mode observable), dono valid PNG (magic + >=8192B + non-blank).
- Light hues sirf `eframe::egui::Visuals::light()` se (existing Visuals reuse, naya palette zero).

## Gate 4 — rust/MS gates

- `cargo test -p autocompare-gui` green. `cargo clippy` zero. `cargo fmt --check` clean.
- Banned zero (non-test): `unwrap`/`expect`/`panic!`/`todo!`/`unsafe` naya zero.
- Colors zero new: live paint me naya `Color32`/`Visuals` hue zero (sirf existing API reuse).

## Gate 5 — harness + hygiene

- `tools/gui-polish/run-geo-typo.sh checks/gui-darkfull-ci/round-1/` → `RESULT: PASS` (max 6 rounds).
- Har test/gate se PEHLE `git status --short`: sirf OWNS dirty
  (`crates/gui/src/{main,app,screenshot}.rs`, `checks/gui-darkfull-ci/`).
- Commit pathspec-only, `git add -A` MANA. `STATUS.md` MANA. PUSH MANA.
