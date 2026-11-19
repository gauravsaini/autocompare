# Wave W — GATES: picture zoom+pan (gap #4)

Scope: `crates/gui/src/views.rs` + `crates/gui/src/viewmodel.rs` + `checks/gui-pic-w/**` only (`app.rs`/`theme.rs` read-only).

## G1 — zoom helpers clamp 0.25..=8.0 (headless)
- CHECK: `cargo test -p autocompare-gui picture_zoom`
- EXPECT: green; `clamp 25/800`, `step +25/-25 saturating`, `reset=100`, `factor 25->0.25, 800->8.0`.

## G2 — pan helpers clamp to content bounds (headless)
- CHECK: `cargo test -p autocompare-gui picture_pan`
- EXPECT: green; `drag adds delta then clamps to +/-content (capped by PICTURE_PAN_LIMIT)`, non-finite/zero content -> `(0,0)`-safe.

## G3 — zoom applies to both PictureModes, split stays authority
- CHECK: `cargo test -p autocompare-gui picture_display`
- EXPECT: green; `picture_display_size` uses zoom-factor helper (800->8x), `split_picture_widths` unchanged + still sums to row.

## G4 — paint wires keyboard/buttons/drag/double-click (compile + headless math)
- CHECK: `cargo clippy -p autocompare-gui -- -D warnings && cargo test -p autocompare-gui`
- EXPECT: 0 warnings, all tests green (249 + new), no banned tokens (`grep -rn 'unwrap(\|\.expect(\|panic!\|todo!'` in edited non-test code -> empty).

## G5 — harness proof under checks/gui-pic-w/
- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-pic-w/round-1 && ls checks/gui-pic-w/round-1/after.png`
- EXPECT: `geo-typo-report.md` contains `RESULT: PASS`, PNG magic-ok, `REPORT.md` filed, commit NO push.
