# REPORT — wave chrome17 (app-chrome geometry, round-1)

Scope: toolbar height/padding/button rhythm, tabstrip widths + padding,
file-bar height + Browse alignment, metadata line rhythm — geometry FIRST,
typography SECOND (parity, no delta), colors FROZEN (33 touches, unchanged).

## Touched functions (all in `crates/gui/src/views.rs`, chrome region only)

CHANGED (paint/geometry):
- `action_bar_row_gap` — `TIGHT_GAP`(4) -> `SECTION_GAP`(8); two-row band is
  now 24+8+24 = 56 (roomier BC-style band); intra-row rhythm stays `TIGHT_GAP`.
- `paint_header` — row `item_spacing` `TIGHT_GAP` -> `SECTION_GAP` (title/note
  breathing); `set_min_size` still `header_bar_height()` (28, untouched).
- `paint_tabs` — `item_spacing` `TIGHT_GAP`(4) -> `SECTION_GAP`(8) (padded tabs);
  trailing `add_space(TIGHT_GAP)` -> `TAB_BOTTOM_GAP`(6) (strip rhythm parity
  with `paint_session_tab_strip`).

NEW (pure headless, narrow-720 edge):
- `chrome17_toolbar_side_air_720` — side air at 720 fallback = 260 symmetric.
- `chrome17_narrow720_no_overlap_gate` — aggregate no-overlap boolean.

TESTS (same file, `#[cfg(test)]` only):
- `chrome17_narrow720_no_overlap` (new, green) — mirrors
  `comparison/chrome-narrow-720.json`.
- `action_bar_geometry_reuses_toolbar_floor` (updated: gap 4 -> 8 + value
  assert; nothing else changed).

UNTOUCHED (owned by parallel wave / already at parity):
- `paint_text_body`, all gutter/marker code — never touched.
- File-bar paint (`paint_paths`/`paint_triple_paths` row `HEADER_H` 28,
  Browse 120x28 = row height so Browse is vertically aligned by construction;
  `paint_path_meta_row` thin `text_meta_line_height_px` 15.2) — already at BC
  rhythm, locked by asserts instead of edited (ponytail: no diff where parity
  holds). Typography ladder unchanged: tab Body 13 / section Heading 18 /
  status Small 11 via existing `TypoRole` tokens, no new sizes.

## Evidence

- Baseline: harness `RESULT: PASS`, PNG 10449 B magic-ok distinct-256,
  suite 1037 passed / 0 failed; `before.png` copied pre-edit.
- Post-change: harness `RESULT: PASS` (PNG 10449 B, magic ok, distinct 256;
  geometry signals: SPACE LG/MD/SM/XL/XS, margin/gap x603, Grid x16, row-h
  x315, width x85, gutter x1714; typo unchanged; colors 33).
- Suite: `cargo test -p autocompare-gui` -> 1038 passed / 0 failed
  (1037 + 1 new `chrome17_narrow720_no_overlap`).
- `cargo fmt -p autocompare-gui -- --check` -> clean (0).
- Banned-token grep on non-test delta -> empty (ZERO-BANNED).
- Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
- Edge fixture: `comparison/chrome-narrow-720.json` (720 fallback budgets:
  pair 200 / air 260 / strip-10 1572 scrolls / halves 180,180 / row 748 with
  caller scroll below 772 / layout 720 SideBySide vs 719 Stacked /
  meta 15.2 < header 28).

## Pixel gate (honest verdict)

```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL (expected diff)
pxdiff exit: 2
```

G6-pixel is BLOCKED by structure, not by quality: the fixture PNG is painted
manually in `screenshot.rs` from hardcoded chrome rects, so no allowed
chrome-region edit reaches its pixels (only `text_center_gutter_width` flows
through, and it is test-locked + in forbidden paint/gutter territory).
Precedent: exq-paint16 recorded the same decoupled-renderer finding. The egui
chrome delta is real (4 one-token rhythm edits above) but renders only in the
live app. Harness `RESULT: PASS` + full suite green both hold.
