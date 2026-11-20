# REPORT: GUTTER20 live egui 20px center gutter (BC parity)

Round: checks/gui-exq-gutter20/round-1/ -> RESULT: PASS
Harness: PNG 10703 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: SPACE tokens LG/MD/SM/XL/XS, margin/gap consts x608, Grid x16,
row-h x327, width calls x85, gutter x1726; TypoRole x1063, monospace x26,
strong x45, .size ['14.0'] (koi naya size nahi), colors 33 (frozen, zero growth)

## Finding: live paint already compliant — test-only delta ($ponytail)

Gap #1 (live-feedback19) ke khilaf live paint paths audit kiye: wrap-on
head (~views.rs:3637), wrap-on continuation (~3734), wrap-off head (~3922),
wrap-off continuation (~3973), wrap-off trailing-blank (~3840) — sab
pehle se `ui.set_width(text_center_gutter_width())` = 20.0 par hain,
`text_center_marker_fg()` (CHANGED_FG) se `=>` sirf
`text_center_hunk_head` rows par, `text_center_gutter_band_fill` se
selection bands, balanced trailing blanks se floor fill. Isliye koi paint
delta nahi — laziest minimal diff = ek headless tall-scroll proof test +
edge fixture. screenshot.rs untouched (parallel wave owns it).

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Live center gutter width | 20px (`TEXT_CENTER_GUTTER_W` = SPACE_LG + SPACE_XS), wrap-on/off sab paths — unchanged | unchanged (test locks it: `text_center_gutter_width() == 20.0`, column_x 590 @1200, cx 600, hit band 588..612) |
| Hunk-head arrows | `=>` via `text_center_marker_fg` sirf diff-hunk heads par, Equal blank — unchanged | unchanged (test locks: labels Changed/Deleted/Inserted `=>`, Equal None; 1200-row heads [2,500,1000]; scroll windows [0,60)->[2], [470,530)->[500], [980,1040)->[1000], [60,470)->[]) |
| Selection bands + floor | band selection par hi, floor balanced blanks — unchanged | unchanged (test locks: band_fill(true)/!false; balanced(1200.0,17.6,9)==60 even) |
| Tall-scroll contract | koi views.rs-side tall proof nahi tha (gutter.rs-side GUTTER11 alag owner) | naya: fixture `comparison/gutter-tall-1200.json` + test `gutter20_tall_gutter_stable` |

Typography second: koi naya role/size nahi (report .size ['14.0'] only).
Colors FROZEN: views.rs delta me 0 Color32/Visuals touch (repo-wide 33).

## Functions touched (views.rs ONLY — paint fns modified: none)

- ADDED (test): `gutter20_tall_gutter_stable` — 1200-row model banata hai
  (Changed pair @2, Deleted @500, Inserted pair @1000, baaki Equal), full-model
  + scroll-window heads, gutter geometry, marker labels/ink, band fill,
  tall floor assert karta hai; fixture `gutter-tall-1200.json` mirror.
- REUSED, NOT MODIFIED (helpers + paint paths): `text_center_gutter_width`,
  `text_center_hunk_head`, `text_center_hunk_marker`,
  `text_center_marker_label`, `text_center_marker_fg`,
  `text_center_copy_direction`, `text_center_gutter_band_fill`,
  `text_center_marker_inset`, `text_center_marker_content_w`,
  `text_center_hit_slop_px`, `text_center_column_x_for_avail`,
  `text_center_arrow_cx_for_avail`, `text_center_hit_band_for_avail`,
  `text_pane_pair_widths`, `text_trailing_blank_rows_balanced`,
  `text_row_top_rhythm_px`; paint call-sites wrap-on head/continuation,
  wrap-off head/continuation/trailing-blank (already 20px-compliant).

## Gates

cargo test -p autocompare-gui 1042 passed / 0 failed (incl.
`views::tests::gutter20_tall_gutter_stable` ok); harness RESULT: PASS;
rustfmt --edition 2024 --check clean; non-test banned-token delta
(unwrap/expect/panic/todo/unimplemented/unreachable/indexing) 0; colors 33.
pxdiff: `before: 960x600 ch=4 / after: 960x600 ch=4 / diff_pixels: 0 of
576000 / diff_bbox: none / VERDICT: IDENTICAL (expected diff)` exit 2 —
gate ABANDONED (fixture path live egui pixels nahi dekhta + delta
headless-test-only; procedural proof = tall geometry test + full suite).
