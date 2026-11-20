# REPORT — wave shot18 (fixture chrome pixels, round-1)

Scope: fixture-renderer mirror of chrome17 app geometry — taller toolbar band
with button rhythm, padded tabstrip tabs, file-bar + metadata rows kept;
center gutter + yellow markers intact. Geometry FIRST, typography SECOND
(no delta), colors FROZEN (33 touches, unchanged).

## Touched functions (all in `crates/gui/src/screenshot.rs`, fixture chrome only)

CHANGED (non-test paint):
- `paint_topchrome_toolbar` — taller band: single 24px row -> two stacked
  24px rows + 8px inter-row gap = 56 band (live `views::action_bar_row_gap`
  SECTION_GAP rhythm via `action_bar_row_height`/`action_bar_row_gap`
  helpers, no new consts). Row 1 keeps `home::topchrome_*` actions +
  `>>` overflow; row 2 adds `[Prev]`/`[Next]` 96x24 pair with 8px cluster
  air (`views::action_nav_arrow_gap`) + right-docked `3:1` cursor readout.
  Hues reuse `[32,32,32]`/`[45,45,45]`/`text`/`dim` only.
- `render_text_fixture_png` — padded tabstrip (boxes 60x24 -> 60x22 at
  +4px-per-tab shifted x: Text 64->68, Folder 138->148, Hex 200->222,
  Merge 244->278, Table 300->346, Sync 356->410, Picture 404->474,
  FolderMerge 474->544, Audio 574->644; 8px inter-tab rhythm + 6px
  trailing air) + `pane_top` 142 -> 174 (114 + 56 band + 4 air) so the
  taller band holds without pane overlap. File-bar call unchanged
  (`paint_pathmeta_strip` with existing `fixtures/left.rs|right.rs` labels
  + own-words second metadata lines `2026-01-02 03:04|03:05 | 128|132 B |
  Text: exact | wrap:off`). Center/gutter paint untouched.
- `paint_chrome` — same padded tabstrip numbers as above (shared chrome
  consistency for wrap/accent/hex/table/merge/home routes; narrow branch
  untouched).

UNTOUCHED (proves parity by staying):
- `paint_pathmeta_strip` — field rows + second metadata line already at
  parity (own-words per-side info line, gap #2); reused as-is (ponytail:
  no diff where parity holds).
- Center gutter + `TEXT_CENTER_MARK` (`=>`, `CHANGED_FG` reuse) + row paint
  — shot15 contract holds; `text_fixture_center_layout` still
  `(12,448,464,20,488,448)`.
- `views.rs` — never touched (parallel wave owns it).

TESTS (same file, `#[cfg(test)]` only):
- `shot18_chrome720_no_overlap` (new, green) — mirrors
  `comparison/shot-chrome-720.json`.

## Evidence

- Baseline (pre-edit): harness `RESULT: PASS`, PNG 10449 B magic-ok
  distinct-256, suite 1038 passed / 0 failed; `before.png` copied pre-edit.
- Post-change: harness `RESULT: PASS` (PNG 10708 B, magic ok, distinct 256;
  geometry: SPACE LG/MD/SM/XL/XS, margin/gap x608, Grid x16, row-h x318,
  width x85, gutter x1715; typo unchanged; colors 33).
- Suite: `cargo test -p autocompare-gui` -> 1039 passed / 0 failed
  (1038 + 1 new `shot18_chrome720_no_overlap`).
- `cargo fmt -p autocompare-gui -- --check` -> clean (0).
- Banned-token grep on non-test delta -> empty (ZERO-BANNED).
- Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
- Edge fixture: `comparison/shot-chrome-720.json` (720 fallback: toolbar
  pair 200 + side air 260, strip-10 1572 scrolls, file-bar halves 180/180,
  narrow panes 340/340 + gutter 696 + margin 8 = 720, layout 720/719
  SideBySide/Stacked, tab boxes 22h/6 trailing, band 56/pane_top 174,
  meta 15.2 < header 28).

## Pixel gate

```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 138850 of 576000
diff_bbox: (0,40)-(959,448)
VERDICT: DIFFERENT
pxdiff exit: 0
```

BBox x-range 0..959 spans the live 20px center (464..484) and y-range
40..448 spans chrome (36..174) + hunk-head rows, so the shot15 gutter +
`=>` markers are inside the diff region and not regressed (markers paint
at the same relative rows, shifted with `pane_top` 142->174; panes
174..560 height 386 hold all 7 rows + floor).
