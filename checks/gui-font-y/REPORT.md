# Wave Y-FONT — REPORT (round-1, Gap #9 font zoom Cmd+/-)

Scope: `crates/gui/src/theme.rs` + `crates/gui/src/app.rs` + `checks/gui-font-y/**` ONLY.
`views.rs` / `viewmodel.rs` / `screenshot.rs` / `tree.rs` untouched (sibling Y-SPLIT ka OWNS).
Geometry first, typography second, colors FROZEN. No new deps. Clean-room: bahar se observe only.

## Baseline -> After

- `cargo test -p autocompare-gui`: 271 passed -> 275 passed, 0 failed
  (+2: `zoom_scale_stays_in_factor_band_and_keeps_hierarchy`,
  `font_zoom_keys_resolve_and_apply`; +2 count me do pehle-landed zoom tests bhi shamil
  jo is wave me dobara green prove hue).
- Harness `tools/gui-polish/run-geo-typo.sh -> round-1/geo-typo-report.md`: RESULT PASS
  (PNG 9624B magic-ok distinct-ok, geometry/typo shape same, colors info-only).
- Harness `checks/gui-font-y/run-font-y.sh -> round-1/font-y-report.md`: RESULT PASS
  (8/8 adds ok, paint-neutral x2, frozen removals 0, banned 0, 275/0 tests).
- PNG: 9624 bytes, magic `89504e470d0a1a0a`, distinct 256.

## Gap #9 close hone ka tark (study -> build)

- Study: Wave R me zoom model (`ZOOM_LEVELS -2..=4`, `zoomed_size`, `rich_zoomed`) +
  App `zoom_level` state + Cmd/Ctrl + `=`/`-`/`0` keybinding land ho chuke the, par do
  kami thi: (1) task ka `0.8..=2.0`-style factor contract kahin prove nahi tha;
  (2) keybinding ka `ui.input` chord khud headless-untested tha (sirf setters tested the).
- Build (sirf OWNS, paint-neutral): theme me `zoom_scale_factor(role, level)` free fn +
  `TypoRole::zoomed_scale_factor` method (level 0 = 1.0; har role x har step band me);
  app me `ZoomKey` enum + `App::resolve_zoom_key` (reset > in > out priority) +
  `App::apply_zoom_key` (bool wapas: level badla ya no-op); `eframe::App::ui` ab wahi
  resolver feed karta hai (9-line if/else -> 2 lines, chord behavior same; sirf
  degenerate in+out-same-frame chord ab In-priority leta hai, pehle net-zero double-apply tha).
- Typo hierarchy barkarar: uniform 1px steps se har level par
  Small < Mono < Body < HomeTitle < Heading, HomeDesc == Small, HomeButton == Body.

## Diff (theme +56, app +92/-9, sirf OWNS)

- `crates/gui/src/theme.rs`: `zoom_scale_factor` + `zoomed_scale_factor` + 1 test.
  `rich()` / `rich_zoomed()` paint path unchanged (`extra_letter_spacing` 0.0 x2).
- `crates/gui/src/app.rs`: `ZoomKey` + resolve/apply + `ui()` rewire + 1 test.
- `checks/gui-font-y/`: `GATES.md`, `run-font-y.sh`, `round-1/`, `REPORT.md`.
- Zero diff: `views.rs` / `viewmodel.rs` / `screenshot.rs` / `tree.rs` (sibling OWNS),
  geometry consts, color tokens.

## Frozen proof

- `git diff -- crates/gui/src/theme.rs crates/gui/src/app.rs` me zero `-` lines matching
  `SPACE_|GEO_|GUTTER_|Color32::from_rgb|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_`.
- No rectangle/position/size edits, no color-token value changes.

## Honest scope note (orchestrator ke liye)

- Mere OWNS files: `cargo fmt` clean, banned grep ZERO-BANNED (non-test),
  `#![forbid(unsafe_code)]` dono me, unit tests 275/0.
- Full-tree `cargo clippy --all-targets -- -D warnings` aur `cargo fmt -- --check` filhal
  RED hai — wajah sirf sibling Y-SPLIT ka in-flight uncommitted kaam
  (`views.rs:62` unused `SPLIT_*` imports + `views.rs:2729,2765` fmt spots;
  `git diff --stat` me `views.rs`/`viewmodel.rs` uncommitted dikhenge, mere commit me nahi hain).
  Maine un files ko chhua tak nahi (disjoint OWNS rule). Sibling ke land hote hi
  full-tree gates dobara green prove honge.
- Koi push nahi (orchestrator verifies+pushes).
