# REPORT — exq-folder2: dual-pane tree fill plus filter geometry

Wave: exq-folder2 (round-1). OWNS: `crates/gui/src/tree.rs`,
`crates/gui/src/home.rs`, `checks/gui-exq-folder2/**`. NO PUSH.

## Gap (BC Folder Compare dual-pane parity)

Apna `folder_tree_content_height` + home grid/tree content-height tha, par do
cheezen far thi: (1) dual-pane fill — pane widths/heights paint-only the
(`views::split_picture_widths`), headless me koi fill/waste/scroll math nahi;
(2) filter row — sirf height/pad/gap the, control widths/offsets/wrap-fit nahi.
BC me dono panes + gap hamesha row bharte hain (tang window scroll, crush nahi)
aur filter row (naam + Type/Status + chip + Clear) fit-check par wrap karti hai.

## Diff (OWNS only, headless, colors untouched)

`tree.rs` — naya section "Folder2 dual-pane fill + filter row geometry":
- Fill widths: `FOLDER_DUAL_MIN_HALF` 180 (views MIN_HALF parity), `FOLDER_DUAL_GAP`
  section rhythm, `FOLDER_DUAL_HANDLE_W` viewmodel single-source;
  `folder_dual_pane_widths` (split_picture_widths parity, clamp 0.20–0.80,
  non-finite guards), `folder_dual_width_waste` (computed widths par 0).
- Fill vertical: `FOLDER_DUAL_PAGE_BAR_H` 28; `folder_dual_chrome_height`
  (64, pending 92); `folder_dual_visible_rows`, `folder_dual_content_height`
  (khaali par chrome-only, 0 nahi), `folder_dual_needs_scroll`,
  `folder_dual_height_waste`.
- Filter row: `FOLDER_FILTER_NAME_W` 120 (views FIND_NEEDLE_W parity, test se
  pakka), kind 94 / status 108 / chip 56 / clear 115 (longest label x 7px
  advance + 24 frame pad); `folder_filter_row_content_width` (346 / 533),
  `folder_filter_row_needs_wrap`, `folder_filter_row_height` (24 / 56),
  `folder_filter_row_offsets` (8 / 136 / 238 / 354 / 418).
- Typo second: koi naya size nahi — bar body-regular mono, chip
  caption-strong mono reuse (test me pakka).
- 4 naye tests (`tree::folder2_tests`), edge fixture
  `round-1/comparison/dual-narrow-400.json` (400px window, active filters:
  row wraps 56, panes 184/184 waste 0, 200px me 5 rows waste 16).

`home.rs` — `folder_view_avail_height(viewport, folder_chrome)` (tab strip +
gap + tree chrome + status ghata kar; tang/non-finite par 0) + 1 test jo tree
chrome (wrapped filter 56 + dual 64 = 120) se 600px par 420 avail aur 14 dual
rows wire karta hai.

## Gates (evidence, 2026-11-20 UTC)

- `cargo test -p autocompare-gui`: 761 passed, 0 failed (baseline 755 + 5 naye;
  761st sibling text3 wave ka hai, OWNS se bahar).
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings/errors.
- `rustfmt --edition 2024 --check` tree.rs + home.rs: clean.
- Banned (non-test): `unwrap/`expect/`panic!`/`todo!` zero; `forbid(unsafe_code)` kept.
- Colors: diff me `Color32`/`Visuals` zero (harness count 33 pre-existing hai).
- Harness `tools/exquisite/run.sh checks/gui-exq-folder2/round-1`: `RESULT: PASS`
  (round 1/6), PNG 9765 B, magic ok, non-blank.

## Follow-up (dusri waves, is wave me NO-TOUCH)

- Paint adoption (`views.rs` owner): panes `folder_dual_pane_widths` se, rows
  `folder_dual_visible_rows` se, filter bar wrap `folder_filter_row_needs_wrap`
  + `folder_filter_row_height` + `folder_filter_row_offsets` se.
- Background scan progress: engine-owned, is wave ke bahar.
