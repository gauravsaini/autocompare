# Wave AV-PICTURE — GATES: picture/compare zoom-fit parity

Scope: `crates/gui/src/views.rs` + `checks/gui-picture-av/**` ONLY
(`views_table.rs`/`tree.rs`/`app.rs`/`home.rs`/`viewmodel.rs` read-only,
`checks/opencode-waves/STATUS.md` orchestrator-owned).

Colors FROZEN: koi Color32/Visuals/hue change nahi.
Typography: labels same mono FontId, koi naya `.size` nahi.

## G1 — fit-to-window math (headless)
- CHECK: `cargo test -p autocompare-gui picture_fit_zoom`
- EXPECT: green; exact-fit 800x600@400x300=50, 62.5 floors to 50,
  narrow-axis wins, huge viewport caps 400 (paint range), tiny floors 25,
  zero/NaN/Inf guard -> 100 (reset, no fake zoom).

## G2 — centered geometry math (headless)
- CHECK: `cargo test -p autocompare-gui picture_center_offset`
- EXPECT: green; 100-in-400 -> 150.0, equal/oversize -> 0.0,
  NaN/Inf -> 0.0 (paint never feeds NaN into layout).

## G3 — zoom-step row mapping (headless)
- CHECK: `cargo test -p autocompare-gui picture_zoom_steps`
- EXPECT: green; `PICTURE_ZOOM_STEPS = [25,50,100,200,400]`,
  har step `clamp_zoom_pct` me identity (sab `set_zoom_pct` reachable).

## G4 — full gates on OWNS
- CHECK: `cargo test -p autocompare-gui` (522 green) &&
  `cargo clippy -p autocompare-gui --all-targets -- -D warnings` (0) &&
  `cargo fmt -p autocompare-gui -- --check` (clean) &&
  non-test `views.rs` me banned tokens zero
  (`unwrap(`/`.expect(`/`panic!`/`todo!` -> empty).
- EXPECT: sab green; naye paint/helpers me koi banned token nahi.

## G5 — harness proof under checks/gui-picture-av/
- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-picture-av/round-1`
- EXPECT: `geo-typo-report.md` me `RESULT: PASS`, PNG magic-ok + >=8192B,
  `REPORT.md` filed, conventional commit, NO PUSH.
