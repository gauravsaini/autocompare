# REPORT — wave status20 (fixture status strips, round-1)

Scope: fixture-renderer status strips parity against Beyond Compare reference.
BC displays per-pane position/format strips above each pane bottom and a bottom legend strip with result words.
Our fixture bottom was previously a single truncated segmented status line; status20 implements per-pane one-line strips
(cursor position, format word, result word — single-line honest form from fixture viewmodel) plus a bottom legend strip with 4 segment words in own clean-room phrasing ("Differences", "Same", "Insert", "Overview").
Shot15 center gutter + yellow hunk markers, shot18 chrome, and shot19 typo rows remain intact.
Geometry FIRST, typography SECOND (no delta), colors FROZEN (repo count exactly 33).

## Touched functions (all in `crates/gui/src/screenshot.rs`)

NEW (non-test paint helpers, pure):
- `PANE_STATUS_H` (const = 18) — per-pane status strip height sitting above pane bottom (560 - 18 = 542).
- `format_pane_status_line` — formats per-pane status line from cursor, format word, and result word (`"cursor | format | result"`).
- `paint_pane_status_strip` — renders one per-pane status strip: dark panel fill `[32, 32, 32, 255]`, 1px top border `[45, 45, 45, 255]`, single-line text clipped to pane width.
- `fixture_legend_segments` — static 4 segment words in clean-room phrasing: `["Differences", "Same", "Insert", "Overview"]`.
- `paint_bottom_legend_strip` — renders 30px bottom legend strip at canvas bottom (y=570..600) with 4 equal zones, dividers, and clipped segment text.

CHANGED (non-test paint):
- `render_text_fixture_png` — rewires bottom status: calls `paint_pane_status_strip` for left pane (`"3:1 | Text: exact | 4 equal, 1 changed, 1 deleted"` at `(12, 542, 448, 18)`) and right pane (`"3:1 | Text: exact | 4 equal, 1 changed, 1 inserted"` at `(488, 542, 448, 18)`), followed by `paint_bottom_legend_strip` at y=570..600.
- `paint_segmented_status` — marked `#[allow(dead_code)]`.

UNTOUCHED:
- `paint_topchrome_toolbar`, `paint_chrome`, `paint_pathmeta_strip`, `pane_top 174` — shot18 chrome intact.
- `text_fixture_center_layout` — still `(12, 448, 464, 20, 488, 448)`; `TEXT_CENTER_MARK` (`=>`, `CHANGED_FG`) — shot15 intact.
- `fixture_text_row_h` (18), line number column math, diff row loops — shot19 typo rows intact.
- `render_narrow_text_fixture_png`, `render_text_accent_fixture_png`, and all other routes — untouched.
- `views.rs` — NEVER touched (parallel wave owns it).

TESTS (same file, `#[cfg(test)]` only):
- `status20_narrow720_no_overlap` (new, green) — mirrors `comparison/status-narrow-720.json`.

## Evidence

- Baseline (pre-edit): harness `RESULT: PASS`, PNG 10703 B magic-ok distinct-256, suite 1041 passed / 0 failed; `before.png` copied pre-edit; pxdiff self-compare `VERDICT: IDENTICAL`.
- Post-change: harness `RESULT: PASS` (PNG 11085 B, magic ok, distinct 256; geometry: SPACE LG/MD/SM/XL/XS, margin/gap x608, Grid x16, row-h x327, width x85, gutter x1720; typo unchanged; colors 33).
- Suite: `cargo test -p autocompare-gui` -> 1042 passed / 0 failed (1041 baseline + 1 new `status20_narrow720_no_overlap`).
- `cargo fmt -p autocompare-gui -- --check` -> clean (0).
- Banned-token grep on all added non-test lines (unwrap/expect/panic/todo/unreachable/indexing) -> empty (`ZERO-BANNED`).
- Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
- Edge fixture: `comparison/status-narrow-720.json` (720px narrow: per-pane strips fit within 340px panes without overlap across 4px central gap, bottom legend 4 zones scale/clip cleanly across 180px zones; shot15 center gutter + shot18 chrome intact).
- Pixel gate (verbatim):
```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 17873 of 576000
diff_bbox: (0,542)-(959,584)
VERDICT: DIFFERENT
pxdiff exit: 0
```
  Bbox honesty: y-range 542..584 covers the new per-pane status strips (y=542..560) and the bottom legend strip (y=570..600); x-range 0..959 spans across canvas. Chrome at y~40, center gutter at x~464..484, and hunk rows at y~174..341 are pixel-identical (zero diff there = intact by the strongest proof).
