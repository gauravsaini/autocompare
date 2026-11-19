# Wave R zoom (font zoom model + keybinding patch)

## Study (Gap #9 OPEN)

- `crates/gui/src/theme.rs:51,55,57`: `TYPO_BODY_SIZE 13.0`, `TYPO_MONO_SIZE 12.0`,
  `TYPO_MONO_SIZE_MAX 13.0` fixed; koi `zoom_level`/keybinding nahi.
- `TypoRole::size()` (`theme.rs:129`) single source hai; `gutter.rs:100-107`
  (`gutter_text_role`/`gutter_text_size` Mono passthrough) aur `home.rs:138-150`
  (HomeTitle/HomeDesc/HomeButton role passthrough) read-only rakhe — koi edit nahi.
- Gap wave-p-gaps REPORT me OPEN tha (EXQ9/L-06 font zoom Cmd+/-).

## Build (sirf theme.rs + patch, koi geometry/color edit nahi)

`crates/gui/src/theme.rs:71-134` zoom model (163 nayi lines, purani lines untouched):

- Steps: `ZOOM_MIN_LEVEL -2` (:77), `ZOOM_MAX_LEVEL 4` (:79),
  `ZOOM_DEFAULT_LEVEL 0` (:81), `ZOOM_STEP_PX 1.0` (:83).
- Clamp: `ZOOM_MIN_SIZE 8.0` (:85), `ZOOM_MAX_SIZE 24.0` (:87),
  `ZOOM_LEVELS [-2..4]` (:89).
- Resolver: `clamp_zoom_level` (:92), `zoom_offset_px` (:103),
  free `zoomed_size(role, level)` (:111) = base + level*STEP, phir MIN..MAX clamp.
- Method: `TypoRole::zoomed_point_size` (:118) + `rich_zoomed` (:125, wahi
  family/weight/style, sirf size zoomed; line-height shared).
- Level 0 par har role ka size base ke barabar (passthrough invariant).

Step table (Body 13 + lv, Mono 12 + lv, sab MIN..MAX ke andar):

| level | Body | Mono |
| --- | --- | --- |
| -2 | 11.0 | 10.0 |
| -1 | 12.0 | 11.0 |
| +0 | 13.0 | 12.0 |
| +1 | 14.0 | 13.0 |
| +2 | 15.0 | 14.0 |
| +3 | 16.0 | 15.0 |
| +4 | 17.0 | 16.0 |

`checks/gui-zoom-r/INTEGRATION.patch` (app.rs, 49 insertions, apply --check PASS):

- `zoom_level: i8` field + `ZOOM_DEFAULT_LEVEL` default (view pref, per-run only).
- `zoom_level()` / `set_zoom_level()` (clamped) / `zoom_in()` / `zoom_out()` /
  `zoom_reset()`; status me `font zoom: N` note hota hai.
- `eframe::App::ui` start me Cmd/Ctrl + Plus/Equals (=in), Minus (=out),
  Num0 (=reset); `i.modifiers.command` Mac/Win/Linux sab cover karta hai.
- app.rs me koi direct edit nahi; orchestrator patch apply karega.

## Check (round 2, sab green)

- `cargo test -p autocompare-gui`: 229 passed, 0 failed (5 naye zoom tests).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: exit 0.
- `cargo fmt -- --check`: clean.
- Harness `checks/gui-zoom-r/run-zoom.sh` -> `round-R-1/zoom-report.md`: RESULT PASS
  (tests 229/0, patch apply ok, PNG 9914B magic ok distinct ok).
- Paint-neutral: geometry tokens untouched, `Color32::from_rgb` count 18
  (frozen wave se koi naya hue nahi).

## Files

- `crates/gui/src/theme.rs` (+163, zoom model + 5 tests).
- `checks/gui-zoom-r/INTEGRATION.patch`, `run-zoom.sh`, `round-R-1/`, `REPORT.md`.
- Touch nahi kiya: views/app/gutter/home/hex/table/tree/viewmodel/main/
  screenshot, STATUS.md. Koi push nahi.
