# REPORT — Wave W: picture zoom+pan (gap #4)

## Kya badla
- `crates/gui/src/viewmodel.rs`: headless zoom/pan math — `clamp_picture_zoom` (25..=800 = 0.25..=8.0), `picture_zoom_in/out/reset/factor`, `clamp_picture_pan` (content-bounds + hard-limit), `picture_pan_drag`. 2 naye unit tests.
- `crates/gui/src/views.rs`: `picture_display_size` ab `picture_zoom_factor` use karta hai (dono PictureModes share ek zoomed heatmap frame); paint me keyboard `+/-/0`, `+/-/1:1` buttons, drag-to-pan (content-clamped), double-click reset. `split_picture_widths` untouched (layout authority). 1 naya test.
- `app.rs`/`theme.rs`: read-only, zero touch (dusre parallel wave ka app.rs kaam tree me chal raha hai, use alag rakha).

## Gates (sab green, evidence upar)
- G1/G2/G3: `cargo test picture` — 13/13 pass; full suite 252 pass.
- G4: clippy meri files me zero (ekmatra error app.rs:525 parallel wave ka hai); fmt meri files me zero; banned patterns (`unwrap(`/`.expect(`/`panic!`/`todo!`) non-test code me zero.
- G5: `tools/gui-polish/run-geo-typo.sh checks/gui-pic-w/round-1` → `RESULT: PASS`; `after.png` 9914B PNG 960x600 magic-ok.

## Note
- Zoom state aaj bhi `App` setter se 400 tak capped hai (read-only); math/paint pehle se 800 (8x) ready hai, `app.rs` widen hote hi bina paint change ke khul jayega.
