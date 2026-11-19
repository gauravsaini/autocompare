# REPORT — Wave AX-PAINTWIRE (checks/gui-paintwire-ax)

Wave: AX-PAINTWIRE. AW2 localrun feedback (`checks/localrun-aw2/REPORT.md` §9:
gutter teen jagah teen widths, Home live-contract-bana-par-paint-pending) ka live
paint adoption — sirf OWNS (`crates/gui/src/views.rs`): text overview-ruler gutter
contract par, `paint_home` home contract par. Geometry first, typography second
(same sizes/weights, koi naya size nahi), colors frozen.

## Kya bana (sirf OWNS: `crates/gui/src/views.rs`)

- Ruler width: `text_minimap_ruler_width` ab `gutter_ruler_width_for_font`
  (16.0 numbers ke saath, 8.0 bina) — legacy 12/6 `text_gutter_paint_width`
  untouched (GUTTER_W chrome lock + parity tests barkarar). Paint rect + click
  target apne aap 16/8 par (same `bar_w` plumbing, koi naya token nahi).
- `SEGMENT_MIN_H` 4.0: paint geometry `viewmodel::gutter_geometry` (gutter
  contract re-export) se aati hai — tall-file 1-row hunk 4px band par rehta hai.
- Slop hit-test: naya `text_ruler_hit_slop` (= 2.0, `GUTTER_HIT_SLOP`) +
  `text_ruler_segment_hit` (`gutter_segment_at_y_with_slop`); paint click path
  me slop gate wired (khali track par jump nahi, 4px bands finger se lagte hain).
- Dono directions (offset<->row): marker ab band-center par
  (`text_minimap_row_center_y` = `gutter_row_center_y`, contract intent ke
  mutabiq) — `text_minimap_row_at_y(center) == row` round-trip tested; band-top
  `gutter_y_for_row` mapping bhi round-trip tested.
- Home: `paint_home` me card min-h `home_paint_card_height` = 99.6 (legacy 75
  floor se `max`), CTA `add_sized` 120x28 (`home_paint_cta_size`, hit 3360 vs
  purana 2640), grid 3 cols + `HOME_GRID_GAP` 8 rhythm, title strong-14 vs desc
  small-weak-11 (same sizes/weights), `debug_assert!(!HOME_SHOWS_PATH_PICKER)`
  lock (bachta chrome 32). Naya `paint_home_recent` (title flex + kind 96 + age
  72, do-line rows, honest empty hint) + pure `home_recent_row_title`; App
  wiring (history + now_secs) caller wave ka hai — `#[allow(dead_code)]`
  precedent (`merge_apply_*`) par, tests live rakhte hain.
- Colors: zero `Color32`/`Visuals`/palette touch (frozen).

Tests (2 naye, `views.rs` tests module, koi banned API nahi):

- `text_ruler_adopts_gutter_contract`: widths 16/8 + legacy se chauda, slop 2.0,
  tall-file segment 4.0, slop hit (exact miss / slop hit), dono-disha round-trip
  (top + center), bad-input `None` contract.
- `home_paint_adopts_home_contract`: card 99.6 > 75, CTA (120, 28) + area 3360,
  grid 3 + gap 8, hierarchy ok + sizes 14/13, picker hidden + chrome 32,
  columns tile (460 = title+96+72+pad+gap), row-title honesty (khali/2-path/3-way).

## Gates proof (2026-11-20 UTC, final)

- `cargo test -p autocompare-gui` — 547 passed, 0 failed (views:: 153/153, 2 naye
  samet). Beech me `screenshot::tests::dispatcher_routes_new_scenes` (parallel
  wave ka naya vis-ax-dispatch test, unke OWNS `screenshot.rs` ka transient WIP)
  1 fail tha — unke land par retry me full green; ruler/Home scenes
  (`text/home valid_png`, `dispatcher_routes_by_file_name/all_tabs`) shuru se green.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — exit 0, zero
  warnings (transient parallel-edit error ke baad retry me clean; zero `views.rs`
  mentions).
- `rustfmt --edition 2024 --check crates/gui/src/views.rs` — clean (sirf OWNS file).
- Banned grep OWNS non-test me: `.unwrap()`/`.expect(`/`panic!`/`todo!`/
  `unimplemented!`/`unreachable!` — zero hits; `unsafe` sirf `#![forbid(unsafe_code)]`.
- `tools/gui-polish/run-geo-typo.sh checks/gui-paintwire-ax/round-1` — exit 0,
  `RESULT: PASS` (after.png 9627 B, magic `89504e470d0a1a0a` ok, 256 distinct bytes;
  `.size` values `['14.0']` — koi naya size nahi; colors info-only, untouched).
- OWNS discipline: `git status` me sirf `crates/gui/src/views.rs` (M) +
  `checks/gui-paintwire-ax/**` (nikaya); `screenshot.rs`/`checks/gui-harness-ax/`
  parallel waves ke hain — stage nahi kiye, chhooe nahi.

## Files

- `crates/gui/src/views.rs` — ruler + Home paint adoption + 2 tests (+296/-29).
- `checks/gui-paintwire-ax/GATES.md`, `REPORT.md`, `round-1/` — is wave ke artifacts.
