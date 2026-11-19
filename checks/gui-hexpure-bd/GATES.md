# Wave BD-HEXPURE — GATES (one observable outcome per gate)

OWNS: `crates/gui/src/views_hex.rs` + `checks/gui-hexpure-bd/**` ONLY.
`views.rs` / `app.rs` / `tree.rs` / `theme.rs` / `screenshot.rs` read-only, zero edit.
`checks/opencode-waves/STATUS.md` untouched (orchestrator-owned). Colors FROZEN.

1. Minimap contract tokens — 24px width + 220px height paint tokens se lock.
   CHECK: `cargo test -p autocompare-gui hex_minimap_alloc_tokens_lock_paint` EXPECT: pass
   (`alloc_height == views::HEX_MINIMAP_H == 220.0`, `alloc_size == (24, 220)`,
   `track_rect(origin)` min/size exact).

2. Minimap band rects — tuple bands ka track-anchored Rect judvaan, paint seg parity.
   CHECK: `cargo test -p autocompare-gui hex_minimap_band_rect` EXPECT: pass
   (row-1 band exact, wide hunk clamp, tall-file floor `y0 + GUTTER_MIN_SEGMENT_H`,
   bad input / zero-area track `None`; `band_rects` tuple layer se parity + offset order).

3. Hit-rect mapping — track-relative row hit, locked paint mapping se parity.
   CHECK: `cargo test -p autocompare-gui hex_minimap_row_at_pos` EXPECT: pass
   (edges exact, bahar/kharab `None`, `hex_minimap_y_for_row` + `views::hex_minimap_row_at_y`
   round-trip green).

4. Nibble accent rects — cell 16-split + high/low halves, sirf diffs par.
   CHECK: `cargo test -p autocompare-gui hex_nibble_accent_rects` EXPECT: pass
   (both/low-only/high-only/same matrix, halves cell ko poora cover, barabar par `(None, None)`).
   CHECK: `cargo test -p autocompare-gui hex_nibble_cell_rect` EXPECT: pass
   (16 columns exact, col >= 16 / zero-area `None`).
   CHECK: `cargo test -p autocompare-gui hex_row_nibble_accent_rects` EXPECT: pass
   (col order, barabar row par khali vec — poori-row band ka koi kaaran nahi).

5. Hygiene — MS guidelines + frozen colors + OWNS discipline.
   CHECK: `git diff HEAD -- crates/gui/src/views_hex.rs | grep -c '^-[^-]'` EXPECT: 0
   (sirf insertions, koi existing line rewrite nahi);
   banned `grep -nE 'unwrap\(|\.expect\(|panic!|todo!|unreachable!'` apne diff me EXPECT: zero;
   `Color32|Visuals` apne diff me EXPECT: zero;
   `cargo clippy -p autocompare-gui --all-targets` me views_hex hits EXPECT: zero;
   `rustfmt --edition 2024 --check crates/gui/src/views_hex.rs` EXPECT: clean;
   `git status --short` me apni files ke bahar EXPECT: sirf sibling waves ki pehle-se-maujood
   `M crates/gui/src/views.rs` + `M crates/gui/src/app.rs` (is wave ne unhe chhua nahi).

6. Harness proof — headless-first round passes with real PNG.
   CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-hexpure-bd/round-1` EXPECT: `RESULT: PASS`,
   PNG magic-ok (`89504e470d0a1a0a`), size >= 8192 B, distinct > 64.

7. Callsite spec — agle paint wave ke liye exact wiring REPORT me, is wave me koi paint edit nahi.
   CHECK: `git diff HEAD --stat -- crates/gui/src/views.rs crates/gui/src/app.rs` is wave ke commit me
   EXPECT: absent (REPORT §5 me fn + args + insert lines documented).
