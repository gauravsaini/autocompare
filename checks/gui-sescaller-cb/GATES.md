# GATES — gui-sescaller-cb (App session caller wiring)

Scope: `crates/gui/src/app.rs` only wiring. MODEL (`home.rs`) + PAINT
(`views.rs`) READ-ONLY. Live strip `paint_session_tab_strip` +
`SessionAction` match + T5/T6 bridge (switch par load, type ke baad save).

## Must-pass

1. `cargo test -p autocompare-gui` green. Naye tests:
   - switch action: `Switch(id)` par paths + cursor slots me load, status line set.
   - close action: `Close(id)` par sirf hit session band, neighbour active,
     unknown id / `None` par false, khaali par `tab` Home.
   - T5: setter bypass karke slot me type karne ke baad save par session
     paths update (trimmed).
   - T6: cursor save + switch wapas par restore.
2. `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero.
3. `cargo fmt --check -p autocompare-gui`: clean.
4. Banned strict non-test me zero: `unwrap(`/`expect(`/`panic!`/`todo!`/`unsafe`.
5. Colors: diff me `Color32::from_rgb` / `Visuals::` zero new.
6. Harness `tools/gui-polish/run-geo-typo.sh checks/gui-sescaller-cb/round-1/`:
   `RESULT: PASS` (max 6 rounds).
7. `checks/opencode-waves/STATUS.md` untouched. `git status --short` me sirf
   OWNS (`crates/gui/src/app.rs`, `checks/gui-sescaller-cb/`).

## Fixtures

- `round-1/after.png` fixture renderer se aata hai (`screenshot.rs`
  NO-TOUCH, live `App::update` se nahi), isliye BY wave se byte-identical
  hona EXPECTED hai — no-regression proof. Strip paint scenes khud BY wave
  me PNG-proven hain; is wave ka caller wiring headless tests se lock hai
  (switch/close/T5/T6, gate 1).
- Stable rehna chahiye: magic ok, >=8192B, distinct>64.
