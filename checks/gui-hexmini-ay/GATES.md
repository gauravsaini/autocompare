# Wave AY-HEXMINI — GATES (one observable outcome per gate)

OWNS: `crates/gui/src/views_hex.rs` + `checks/gui-hexmini-ay/**` ONLY.
`views.rs` / `views_table.rs` / `screenshot.rs` / `gutter.rs` untouched (read-only).
`checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).

1. Minimap hit-rect parity — strip 24px, gutter single-source, paint token se lock.
   CHECK: `cargo test -p autocompare-gui hex_minimap_strip_width` EXPECT: pass
   (`== gutter_bar_width(true) * 2.0 == 24.0 == views::HEX_MINIMAP_W`).

2. Offset-proportional bands — hunk offset/len se minimap y-bands, sub-row floor paint jaisa.
   CHECK: `cargo test -p autocompare-gui hex_minimap_bands_are_offset_proportional` EXPECT: pass
   (row bands exact, tall-file floor `y0 + GUTTER_MIN_SEGMENT_H`, bad input empty).

3. Marker inverse — row->y locked paint row-mapping ke saath round-trip.
   CHECK: `cargo test -p autocompare-gui hex_minimap_y_for_row` EXPECT: pass
   (`hex_minimap_y_for_row(r)+0.5` wapas `views::hex_minimap_row_at_y == r`).

4. Nibble parity — kaun sa nibble badla, cell-level (poori-row se aage).
   CHECK: `cargo test -p autocompare-gui nibble` EXPECT: pass
   (`hex_nibbles_differ` high/low flags, `hex_diff_nibble_cells` sirf badle bytes, missing skip).

5. Typo second — nibble cells wahi mono role, koi naya token nahi.
   CHECK: `cargo test -p autocompare-gui hex_nibble_text_role` EXPECT: pass (`Mono`, size 12.0).

6. Hygiene — MS guidelines + frozen colors + OWNS discipline.
   CHECK: non-test `grep -nE 'unwrap\(|\.expect\(|panic!|todo!|unreachable!' crates/gui/src/views_hex.rs`
   EXPECT: zero; `Color32|Visuals` code me zero (sirf "nahi" wale comments);
   `cargo clippy -p autocompare-gui --all-targets` EXPECT: zero;
   `cargo fmt -p autocompare-gui -- --check` EXPECT: clean;
   `git status --short` EXPECT: sirf `views_hex.rs` + `checks/gui-hexmini-ay/**`.

7. Harness proof — headless-first round passes with real PNG.
   CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-hexmini-ay/round-1` EXPECT: `RESULT: PASS`,
   PNG magic-ok, size >= 8192.
