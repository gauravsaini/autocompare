# Wave AB-MINI — REPORT (Gap #5 bacha hissa: text-gutter minimap click-to-jump)

Date: 2026-11-20 UTC. Repo: `/Users/ektasaini/autocompare`, branch `main`.
OWNS: `crates/gui/src/views.rs` + `crates/gui/src/gutter.rs` + `checks/gui-mini-ab/**` ONLY. NO push.

## Kya badla

- Text gutter (`paint_text_with_prefs`) ab `Sense::hover` ki jagah `Sense::click_and_drag` par hai.
  Click/drag par pointer y se `text_gutter_click_select` (thin contract over
  `gutter_jump_target`: diff par stay, equal par nearest-diff snap, bahar/empty par `None`)
  nearest-diff model row nikal kar `model.selected` set karta hai — wahi fraction→row→jump
  pattern jo hex minimap `hex_minimap_jump_to_row` (aur `App::hex_gutter_jump_to_row`, read-only) me hai.
- List gutter ke pehle paint hoti hai, isliye jump target ek frame ke liye egui temp store
  (`text_gutter_pending_id`, `Id::unique` — deprecated `Id::new` nahi) me rehta hai; agli frame list
  us row par scroll karti hai. Koi struct change nahi (KEVL `viewmodel.rs` untouched).
- Selection/viewport marker: selected row ki band par theme text tone me `ROW_PAD` height tick
  (GEO token reuse, koi naya color/size nahi), jump ke baad synced.
- `gutter.rs` me `gutter_y_for_row` (row→y inverse) + round-trip test; `views.rs` me
  `text_gutter_click_select` + pending-id stability test.
- Hex minimap untouched (wahan click/drag pehle se wired hai).

## Gates

- G1 (baseline green): pre-edit alag tail capture nahi hua (stash forbidden); HEAD state STATUS me
  289 green recorded tha; post-edit full suite 296 green (`round-1/cargo-test-tail.out`,
  harness `round-1/tests.out`). 296 me 2 naye tests samet (gutter marker + click contract).
- G2 (OWNS-only): `git diff --name-only` sirf `views.rs` + `gutter.rs`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD.
- G3 (mapping exact): `gutter_` filter 11 pass, `text_gutter_` filter 1 pass.
- G4 (paint wiring): text gutter rect `click_and_drag` par; `text_gutter_pending_scroll` key present;
  marker `gutter_y_for_row` + `ROW_PAD` par.
- G5 (hygiene): `cargo clippy -p autocompare-gui --all-targets` zero warnings
  (ek deprecated `Id::new` mila tha, `Id::unique` se fix); `cargo fmt --check` OWNS files clean;
  non-test banned (`unwrap`/`expect`/`panic`/`todo`) zero.
- G6 (harness): `tools/gui-polish/run-geo-typo.sh checks/gui-mini-ab/round-1` → `RESULT: PASS`,
  PNG 9624 bytes magic-ok distinct-ok.

## Evidence

- `round-1/after.png` (9624 B), `round-1/geo-typo-report.md` (RESULT: PASS),
  `round-1/tests.out` (296 passed), `round-1/screenshot.out` (empty = clean headless run),
  `round-1/cargo-test-tail.out` (post-edit tail).
