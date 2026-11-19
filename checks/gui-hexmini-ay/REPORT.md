# AY-HEXMINI — Hex minimap + nibble parity (headless geometry)

Tareekh: 2026-11-20 UTC
Workspace: `/Users/ektasaini/autocompare` (branch `main`)
Scope: `crates/gui/src/views_hex.rs` + `checks/gui-hexmini-ay/**` ONLY — `views.rs`,
`views_table.rs`, `screenshot.rs`, `gutter.rs` read-only (parallel wave lock), zero edit.
Harness: `tools/gui-polish/run-geo-typo.sh checks/gui-hexmini-ay/round-1` → `RESULT: PASS`.

Goal (AX2 REPORT §10.3/10.4): 24px minimap hit-rect geometry (`HEX_MINIMAP_W`) +
offset-proportional gutter + kaun sa nibble badla uska cell-level geometry.
Geometry first, typography second, colors FROZEN (koi Color32/Visuals/hue change nahi).

## §1. Kya badla (views_hex.rs, +327 lines, sab pure headless)

**Minimap hit-rect + offset-proportional bands (§10.4, geometry):**
- `hex_minimap_strip_width()` — gutter bar ka double (12→24px). Single source
  gutter contract (`gutter_bar_width(true)`); test paint ke `HEX_MINIMAP_W` se
  equality lock karta hai, taaki allocate rect aur hit-rect kabhi alag na hon.
- `hex_minimap_band_for_hunk(hunk, total_rows, height)` — offset-proportional band:
  `first = offset/16`, `last = (offset+run-1)/16`, y fraction se; sub-row floor
  `y0 + GUTTER_MIN_SEGMENT_H` (wahi 2px theme token jo paint ke `hex_minimap_seg_end`
  = `y0 + ROW_PAD` me hai). Khali run / row-se-bahar / kharab input par `None`.
- `hex_minimap_bands(total_len, height, hunks)` — poori band list offset order me
  (`hex_row_count` se rows). Paint ka inline hunk loop isi list par aata hai.
- `hex_minimap_y_for_row(row, total_rows, height)` — gutter `gutter_y_for_row`
  (usize) ka u64 grid judvaan; selection-marker paint isi par lagta hai. Wahi
  `None` contract; test locked `views::hex_minimap_row_at_y` ke saath round-trip
  karta hai (`y+0.5` wapas usi row).

**Nibble parity cell geometry (§10.3, poori-row paint se aage):**
- `hex_nibbles_differ(left, right) -> (high, low)` — kaun sa aadha-byte alag hai.
- `HexNibbleCell { offset, col, high_diff, low_diff }` + `hex_diff_nibble_cells
  (row_start, left, right)` — 16-byte row ke SIRF badle bytes, offset order me;
  barabar bytes aur missing slots skip (saturating, kabhi panic nahi).
- `hex_nibble_text_role()` — typography second: wahi shared `TypoRole::Mono`
  (grid cells monospace paint hote hain), koi naya token nahi.

Koi naya token/color nahi: width gutter se, floor `GUTTER_MIN_SEGMENT_H` se,
typo `Mono` se, merge-gap precedent `hex_gutter_geometry` jaisa. Non-test code me
`unwrap/expect/panic/todo` zero; `Color32/Visuals` code me zero.

## §2. Paint adoption wiring (views.rs owner ke liye, is wave me LOCKED — no edit)

`views.rs` parallel wave ke paas hai, isliye paint adoption exact mapping spec hai:

1. Minimap strip (`paint_hex`, `bar_w = HEX_MINIMAP_W`): `HEX_MINIMAP_W` ki jagah
   `views_hex::hex_minimap_strip_width()` — `debug_assert_eq!(rect.width(), ...)`
   green rehta hai (test equality lock hai).
2. Minimap bands (inline `first/last/y0/y1` loop + `hex_minimap_seg_end`): us loop
   ki jagah `views_hex::hex_minimap_bands(view.left_size.max(view.right_size),
   rect.height(), &view.hunks)` — fraction math + floor dono andar hain, track rect
   + `CHANGED_BG` frozen rehte hain. `hex_minimap_row_at_y` / `hex_minimap_jump_to_row`
   bodies untouched (hit→jump path same).
3. Nibble cells (byte grid `format_hex_grid_cells` `??` whole-row marks): caller-side
   byte slices par `views_hex::hex_diff_nibble_cells(row_start, left, right)` se sirf
   badle cells accent; khali vec = row me koi antar nahi. Note: model me byte values
   nahi (sirf offset/len) — slices App/engine side se aayengi (OPEN wiring, merge-apply
   precedent jaisa; jabardasti naya model field nahi banaya).

## §3. Gates green proof

- `cargo test -p autocompare-gui`: **553/553** (baseline 547 + 6 naye: strip/bands/
  y-for-row/nibbles/cells/role).
- `cargo clippy -p autocompare-gui --all-targets`: **zero** (ek `indexing_slicing`
  test line pakdi, `.get`-free inline struct se fix).
- `cargo fmt -p autocompare-gui -- --check`: **clean**.
- Banned non-test: `unwrap\(|\.expect\(|panic!|todo!|unreachable!` **zero**;
  `Color32|Visuals` code me **zero** (2 hits sirf "nahi use karna" wale comments).
- Harness round-1: `RESULT: PASS` (PNG 9627 B, magic ok, distinct 256).
- `git status --short`: sirf `M crates/gui/src/views_hex.rs` + `?? checks/gui-hexmini-ay/`.

## §4. Rule follow proof

- OWNS se bahar zero edit; `STATUS.md` untouched; colors frozen.
- Commit conventional `feat(gui): ...`, NO PUSH (orchestrator verify karke push karega).
