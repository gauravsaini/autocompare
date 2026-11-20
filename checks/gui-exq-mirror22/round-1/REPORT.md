# REPORT: mirror22 live gutter + status mirror (BC parity)

Round: checks/gui-exq-mirror22/round-1/ -> RESULT: PASS
Harness: PNG 11254 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: SPACE LG/MD/SM/XL/XS, margin/gap consts x608, Grid x16,
row-h x327, width calls x85, gutter x1746; TypoRole x1063, monospace x26,
strong x45, .size ['14.0'] (koi naya size nahi), colors 33 (frozen, zero growth)

## Diagnosis (live-feedback22 FAIL root cause)

Live Text Compare via `--open` (wrap default OFF: `App::default().word_wrap=false`
-> `App::text_find_prefs` (app.rs) -> `views::paint_text_with_find`
-> `paint_text_body` (views.rs:3216) wrap-OFF branch
(`ScrollArea::both().id_salt("text_diff_rows").show_rows`, head ~views.rs:3948).
Width was NOT the bug: all 6 live center call-sites already
`ui.set_width(text_center_gutter_width())` = 20.0 (`TEXT_CENTER_GUTTER_W`
= SPACE_LG + SPACE_XS, views.rs:129):
wrap-on head (~3637), wrap-on continuation (~3751), wrap-on trailing-blank
(~3817), wrap-off trailing-blank (~3877), wrap-off head (~3948),
wrap-off continuation (~4025). Helpers reused:
`text_center_gutter_width` (129), `text_center_hunk_head` (144),
`text_center_marker_label` (4966), `text_center_marker_fg` (4977, CHANGED_FG),
`text_center_gutter_band_fill` (4930), `text_center_copy_direction` (4984).

Exact reasons (two visual bugs, no feature gate off, no shadowed const):
1. Pill-badge call-site: hunk-head marker used `ui.small_button(...)`
(egui `Button::small` WITH frame) at wrap-on head + wrap-off head, so the
yellow `=>` painted as a pill button on the changed row only — not the
plain gap-rect yellow arrow. Equal rows painted only
`allocate_exact_size(..., Sense::click)` with NO fill (band gate
`text_center_gutter_band_fill(selected)==selected` false when unselected),
so the 20px column was transparent on rows 1/3 — visually "no gutter".
2. Missing continuous track: no unselected-row fill anywhere (head top band
rect, blank cells, continuation, both trailing floors). Fix = complement
fill: selected -> CHANGED_BG band (existing), unselected -> existing
`track_for(dark)` track (no new color/const).

Status bottom bar IS painted live in views.rs (no viewmodel change needed,
honest fallback, no fake data): `paint_text_status_bar` (3196, 3-zone
`StatusZones`), `paint_text_cursor_strip` + `paint_text_pane_status` (5606,
per-pane `text_pane_cursor_labels` L-/R- fallback + `text_pane_detail_label`
label-only when preview empty), `paint_text_legend_bar` (5727, 4 static
segments via `text_legend_segments`/`format_text_legend_line`). Fixture
status20 words differ by clean-room phrasing only; structure (per-pane +
4-seg legend) already mirrors live.

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Live center gutter width | 20px all 6 sites (already) | unchanged (locked by `mirror22_live_gutter_path`: 20.0, col 590@1200, cx 600, hit 588..612) |
| Hunk-head arrows | `small_button` pill frame, changed row only | frameless `Button::small().frame(false)`, yellow `text_center_marker_fg` text only, still `text_center_hunk_head` gate + 2px `text_center_marker_inset` (heads [2,500,1000]) |
| Continuous strip | transparent on unselected (band only when selected) | every cell fills: band CHANGED_BG when selected, `track_for(dark)` when not (`text_center_gutter_track_fill` = complement); trailing wrap-on/off floors also track-filled |
| Bottom bar | per-pane L-/R- + 4-seg legend already live | unchanged (verified honest fallback; no fake numbers) |
| Tall scroll | gutter20 proof only | new `comparison/mirror-tall-1200.json` + `mirror22_live_gutter_path` locks live wrap-off math across windows [0,60)->[2], [470,530)->[500], [980,1040)->[1000] |

Typography second: koi naya role/size nahi (.size ['14.0'] only).
Colors FROZEN: views.rs delta me 0 Color32/Visuals touch (repo-wide 33).

## Functions touched (views.rs ONLY; screenshot.rs + crates/sessions/* untouched)

- ADDED (pure helper): `text_center_gutter_track_fill` — `!band_fill`, continuous-strip complement, existing band reuse.
- CHANGED (paint, same helpers, no new consts): wrap-on head vertical (band/track + frameless marker), wrap-on blank `_`/Equal cells (band/track), wrap-on continuation (band/track), wrap-on trailing floor (track), wrap-off trailing floor (track), wrap-off head vertical (band/track + frameless marker), wrap-off blank `_`/Equal cells (band/track).
- ADDED (test): `mirror22_live_gutter_path` — 1200-row model (Changed pair @2, Deleted @500, Inserted pair @1000), full + window heads, 20px geometry, label/ink gates, band/track complement, balanced floor, per-pane L-/R- + legend 4-seg honest fallback; mirrors `comparison/mirror-tall-1200.json`.
- REUSED, NOT MODIFIED: `text_center_gutter_width`, `text_center_hunk_head`, `text_center_hunk_marker`, `text_center_marker_label`, `text_center_marker_fg`, `text_center_copy_direction`, `text_center_gutter_band_fill`, `text_center_marker_inset`, `text_center_marker_content_w`, `text_center_hit_slop_px`, `text_center_column_x_for_avail`, `text_center_arrow_cx_for_avail`, `text_center_hit_band_for_avail`, `text_pane_pair_widths`, `text_trailing_blank_rows_balanced`, `text_row_top_rhythm_px`, `text_pane_cursor_labels`, `text_pane_detail_label`, `text_legend_segments`, `format_text_legend_line`, `track_for`, `CHANGED_BG`, `CHANGED_FG`.
- Live bottom-bar paint (verified, untouched logic): `paint_text_status_bar`, `paint_text_pane_status`, `paint_text_legend_bar`.

## Gates

cargo test -p autocompare-gui 1045 passed / 0 failed (incl.
`views::tests::mirror22_live_gutter_path` ok); harness RESULT: PASS;
rustfmt --check clean; non-test banned-token delta
(unwrap/expect/panic/todo/unimplemented/unreachable/indexing) 0; colors 33.
pxdiff (verbatim, WITH --expect-diff):
```
before: 960x600 ch=4
after: 960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL (expected diff)
exit: 2
```
gate ABANDONED with reason: fixture path (`screenshot.rs` fixture renderer)
cannot see live egui pixels; delta is views.rs live-only (headless-test
procedural proof: tall gutter math + band/track complement + suite green).
No fake pixels. Colors exactly 33.
