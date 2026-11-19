# Wave AV-PICTURE Report — picture/compare zoom-fit parity

Date: 2026-11-19 UTC. Repo: /Users/ektasaini/autocompare (main).
OWNS: `crates/gui/src/views.rs` + `checks/gui-picture-av/**` only.
Parallel-wave files untouched: `views_table.rs`/`tree.rs`/`app.rs`/
`home.rs`/`viewmodel.rs` (working tree me `home.rs` ka diff dusri
wave ka hai — is commit se bahar).

## 1. Kya bana (geometry first, BC fit-to-window parity)

- `PICTURE_ZOOM_STEPS = [25,50,100,200,400]` (`views.rs`): step-row ka
  contract. Sab App-clamped paint range (25..=400) ke andar, to har
  step `set_zoom_pct` se reachable; 8x headless math untouched.
- `picture_fit_zoom_pct(natural, avail) -> u32`: largest 25pp ladder
  step jis par frame viewport me fit ho (narrow axis wins, floor to
  `PICTURE_ZOOM_STEP_PCT`); guards (zero/NaN/Inf) -> 100 reset;
  output `clamp_zoom_pct` se, to paint hamesha honor kare.
- `picture_center_offset(content, avail) -> f32`: content chhota ho to
  aadha leftover, else 0 (scroll takes over); NaN/Inf -> 0.
- Paint (`paint_picture_compare`): zoom slider ke neeche step row —
  `Fit` button (heatmap natural + remaining viewport se fit pct,
  bina heatmap disabled) + 5 preset `selectable_value` (exact-hit
  highlight). Heatmap frame: content viewport se chhota ho to
  `vertical_centered` + top offset se centered (equal margins);
  bada content purana top-left scroll path rakhta hai (no regression).
- Typography second: koi naya `.size` nahi (harness: sirf pre-existing
  `14.0`); step row default button scale, `steps:` prefix small+weak
  (existing pattern); dimension labels same mono FontId.
- Colors: zero change (harness Color32/Visuals count pre-existing 22).

## 2. File:line refs (`crates/gui/src/views.rs`)

- `PICTURE_ZOOM_STEPS`, `picture_fit_zoom_pct`,
  `picture_center_offset`: picture section, `clamp_zoom_pct` se pehle.
- Step row: `paint_picture_compare` me zoom-slider row ke baad.
- Centered frame: `paint_picture_compare` me ScrollArea `show`
  closure (viewport capture + centered/plain branch, `output.inner`
  ab bhi `Response` — drag-pan/double-click reset unchanged).
- Tests: `views::tests::picture_fit_zoom_snaps_down_to_steps`,
  `picture_center_offset_halves_leftover`,
  `picture_zoom_steps_live_in_paint_range`.

## 3. Gates (sab green)

- `cargo test -p autocompare-gui`: 522 passed, 0 failed (+3 new).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: 0.
- `cargo fmt -p autocompare-gui -- --check`: clean (sirf apna diff
  rustfmt kiya; dusri wave ki files untouched).
- Banned scan (non-test `views.rs`): `unwrap(`/`.expect(`/`panic!`/
  `todo!` -> empty.
- Harness `checks/gui-picture-av/round-1`: `RESULT: PASS`
  (PNG 9624B, magic ok, distinct 256; geometry Grid x7,
  text_style_height x4; typo monospace x67, strong x19, small x80).

## 4. Commit

- `feat(gui): picture zoom-fit parity with centered frame` — NO PUSH
  (orchestrator verifies then pushes). Sirf OWNS paths staged.
