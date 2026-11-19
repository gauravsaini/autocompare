# Wave V MINI — GATES (one observable outcome per gate)

1. Mapping exact — `hex_minimap_row_at_y` / `hex_minimap_jump_to_row` behavior unchanged.
   CHECK: `cargo test -p autocompare-gui hex_minimap` EXPECT: all pass, fn bodies untouched.

2. Strip wider — hex minimap allocate width exceeds old 12px gutter bar.
   CHECK: `grep -n HEX_MINIMAP_W crates/gui/src/views.rs` + screenshot EXPECT: `HEX_MINIMAP_W = 24.0 = 2*GUTTER_W`, paint uses it for `allocate_exact_size`.

3. Segment hits bigger — sub-2px hunk bands grow to a visible floor.
   CHECK: `cargo test -p autocompare-gui minimap_seg` EXPECT: pass (`hex_minimap_seg_end` floors at `y0 + ROW_PAD`).

4. Gutter uniformity green — text gutter + token identities untouched.
   CHECK: `cargo test -p autocompare-gui gutter_widths_stay_uniform` + `exquisite_geometry_tokens` EXPECT: pass.

5. Hygiene — MS guidelines + frozen colors.
   CHECK: `grep -rn 'unwrap\|expect\|panic\|todo' crates/gui/src/views.rs` (non-test) EXPECT: zero hits; `cargo clippy -p autocompare-gui -- -D warnings` EXPECT: zero; `cargo fmt --check -p autocompare-gui` EXPECT: clean; `git diff --stat` EXPECT: only `views.rs` + `checks/gui-mini-v/**`; no `Color32::from_rgb`/hue edits (diff grep).

6. Harness proof — headless-first round passes with real PNG.
   CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-mini-v/round-V-1` EXPECT: `RESULT: PASS`, PNG magic-ok, size ≥ 8192.
