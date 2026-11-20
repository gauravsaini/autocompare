# GATES — wave filebar24 (fixture file-bar, round-1)

OWNS: `crates/gui/src/screenshot.rs` (fixture file-bar ONLY) + `checks/gui-exq-filebar24/**` ONLY. NEVER touch `crates/sessions/*`, `crates/gui/src/views.rs`, `crates/gui/src/theme.rs`, never `git add -A`.
Colors FROZEN (no new hues; repo count must stay exactly 33).
Clean-room: describe reference behavior in own words, never copy strings/assets.

Scope: fixture-renderer mirror of the per-pane file-bar row.
Reference shows one file-bar row above each pane: a path field on the left
(truncated when long) plus a metadata segment (timestamp + byte size + format
word, honest fallback when stat fails — mirror the status20 fallback honesty).
Extend the status20 per-pane strips precedent upward to the file-bar; do not
invent a new layout language.
Keep toolbar21 glyphs/chevrons, shot15 gutter/markers, shot18 chrome, shot19
typo rows, status20 strips intact.
GEOMETRY FIRST (file-bar height, per-pane column widths, path-field vs metadata
alignment, padding/margins vs toolbar above and pane below), typography SECOND
(metadata font size/weight/line-height/letter-spacing — no new sizes), colors LAST (frozen).

## G1 — Baseline green (pre-edit)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-filebar24 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-filebar24/round-1 crates/gui/src` + `cp round-1/after.png round-1/before.png` + `python3 tools/exquisite/pxdiff.py before.png after.png`
- EXPECT: harness `RESULT: PASS`; `after.png` magic ok, >= 8192 B, distinct > 64; pxdiff `VERDICT: IDENTICAL` (self-compare proves gate wiring, not a pass).
- EVIDENCE (verbatim):
```
# Exquisite report (round-1)
- PNG: 11254 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x327, width calls x85, gutter x1753
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
before: 960x600 ch=4
after: 960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
pxdiff exit: 0
```

## G2 — OWNS-only diff

- CHECK: `git status --short` + `git diff --name-only`
- EXPECT: only `checks/gui-exq-filebar24/**` + `crates/gui/src/screenshot.rs`; no `views.rs`/`theme.rs`/`sessions/*`. REPORT.md lists every touched fn.
- EVIDENCE: `git status --short` shows only `M crates/gui/src/screenshot.rs` + `?? checks/gui-exq-filebar24/` as owned changes (other dirty paths — `checks/gui-sessionpaint-by/**.png` — pre-existed, never staged); `views.rs`/`theme.rs` untouched (0 matches). REPORT.md lists every touched fn.

## G3 — Geometry FIRST (file-bar row above each pane)

- CHECK: `git diff -- crates/gui/src/screenshot.rs`
- EXPECT: minimal delta inside fixture file-bar paint:
  + new consts: `FILE_BAR_Y` (=76, reuses `PATH_FIELD_Y`), `FILE_BAR_H` (=36 = field 24 + meta 12), `FILE_BAR_BROWSE_W` (=75, existing Browse width);
  + new pure helper `format_file_bar_meta(timestamp, bytes, format_word)` reusing `viewmodel::format_path_pane_meta` with `"-"` honest fallback when all parts empty (mirrors status20 `format_pane_status_line`);
  + `paint_pathmeta_strip` rewired: per-pane bar backgrounds spanning true pane columns (`text_fixture_center_layout` at 960, `dual_pane_widths`/`dual_pane_x_positions` below 960), path field + Browse refit inside each bar, metadata segment clipped to bar width with `"-"` fallback when empty;
  + `render_text_fixture_png`: meta args become 3-part (timestamp | bytes | format word) via `format_file_bar_meta`, e.g. left `"2026-01-02 03:04 | 128 B | Text: exact"`;
  + bar bottom (76+36=112) <= toolbar (114); toolbar band 114..170, pane_top 174 intact; shot15 center layout `(12, 448, 464, 20, 488, 448)` + `=>` markers, shot19 row rhythm, status20 strips/legend intact;
  + no new deps, colors frozen.
- EVIDENCE: `git diff` adds `FILE_BAR_Y/H/BROWSE_W` consts + `format_file_bar_meta` + `paint_pathmeta_strip` bar-column rewire + 3-part meta call in `render_text_fixture_png` + `filebar24_narrow720_no_overlap` test only; toolbar/chrome/center-layout/row-rhythm/status20 helpers untouched.

## G4 — Typography SECOND, minimal

- CHECK: `git diff -- crates/gui/src/screenshot.rs | grep -E "^\+" | grep -E "\.size\(|RichText|font" || echo TYPO-CLEAN`
- EXPECT: no new sizes/weights/families; fixture keeps 5px pen + existing `draw_string` and `clip_to_width` (ellipsis language already in codebase).
- EVIDENCE: `TYPO-CLEAN` (no added `.size(`/`RichText`/`font`; grep empty).

## G5 — Edge fixture + headless test (narrow 720 file-bar no overlap)

- CHECK: `ls checks/gui-exq-filebar24/round-1/comparison/filebar-narrow-720.json` + `cargo test -p autocompare-gui filebar24_narrow720_no_overlap`
- EXPECT: fixture exists (720px: bars (12,76,340,36)/(356,76,340,36), 4px gap, path truncates with ellipsis, metadata clipped to bar and never overlapping pane content, bar bottom 112 above toolbar 114); test green.
- EVIDENCE: `test screenshot::tests::filebar24_narrow720_no_overlap ... ok` (mirrors `comparison/filebar-narrow-720.json`: 340px bars, 4px gap, no overlap, 257px fields, 324px meta budget, `"-"` fallback).

## G6 — Harness PASS + PNG valid + pixel gate

- CHECK: re-run harness + `python3 tools/exquisite/pxdiff.py before.png after.png --expect-diff`
- EXPECT: harness `RESULT: PASS`; PNG magic `89504e470d0a1a0a`, >= 8192 B, distinct > 64; pxdiff exit 0 (`DIFFERENT`) with full output pasted below and into REPORT.md; diff bbox sits in the file-bar band above the panes (y~76..112); toolbar/gutter-overview/text rows/status strips intact (zero diff there).
- EVIDENCE (verbatim post-run):
```
# Exquisite report (round-1)
- PNG: 11213 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x327, width calls x85, gutter x1753
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
before: 960x600 ch=4
after: 960x600 ch=4
diff_pixels: 12958 of 576000
diff_bbox: (12,76)-(952,111)
VERDICT: DIFFERENT
pxdiff exit: 0
```
  Bbox honesty: y76..111 = file-bar band only (bars y76..112, fields y76..100, meta y100..112); toolbar 114..170, panes 174+, status 542+ pixel-identical = intact proof.

## G7 — Suite green + fmt + banned-token + colors

- CHECK: `cargo test -p autocompare-gui 2>&1 | grep "test result"` + `cargo fmt -p autocompare-gui -- --check` + banned-token grep on non-test delta + colors count check.
- EXPECT: all tests pass; fmt clean (0); zero banned tokens in non-test delta; repo colors count exactly 33.
- EVIDENCE (verbatim):
  + Suite: `test result: ok. 1047 passed; 0 failed` (baseline 1046 + 1 new `filebar24_narrow720_no_overlap`)
  + `cargo fmt -p autocompare-gui -- --check`: clean (exit 0)
  + Banned-token grep on non-test delta: `ZERO-BANNED` (word tokens + subscript)
  + Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
