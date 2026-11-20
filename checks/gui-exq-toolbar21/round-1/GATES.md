# GATES — exq-toolbar21 round-1 (fixture toolbar icon+label row, visual-spec-first)

Base: dd94ce9. Worktree: /tmp/wt-toolbar21. Scope: `crates/gui/src/screenshot.rs` ONLY (+ `checks/gui-exq-toolbar21/**`).
Clean-room: original geometric glyphs only (plain `fill_rect` rects/triangles/lines), no copied strings/icons/assets.

## Baseline (pre-edit, recorded BEFORE implementing)
- Harness: `CARGO_TARGET_DIR=/tmp/target-exq-toolbar21 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-toolbar21/round-1 crates/gui/src` → `RESULT: PASS`.
- Report: PNG 11085 bytes (min 8192 ok), magic ok, distinct 256 (ok); COLORS 33.
- Self-compare (`cp after.png before.png`, then `pxdiff.py before.png after.png`):
  - `before: 960x600 ch=4 / after: 960x600 ch=4 / diff_pixels: 0 of 576000 / diff_bbox: none / VERDICT: IDENTICAL` (exit 0).

## Gates (CHECK → EXPECT; all must hold post-edit)

- CHECK: harness re-run green → EXPECT: `tools/exquisite/run.sh` exits 0, `exquisite-report.md` ends `RESULT: PASS` (PNG ≥8192B, magic ok, non-blank).
- CHECK: pixel gate differs → EXPECT: `python3 tools/exquisite/pxdiff.py before.png after.png --expect-diff` exits 0 with `VERDICT: DIFFERENT`, `diff_pixels > 0`; `diff_bbox` still spans chrome top (y~40: toolbar icon band) through center gutter (x~464..484) and hunk rows down to bottom strips (y~542..584) — gutter/markers/chrome/typo rows/status strips intact, only toolbar band + nav chevrons add paint.
- CHECK: toolbar icons → EXPECT: each visible row-1 button keeps its existing rect (same `topchrome_toolbar_button_width` widths + `TOPCHROME_BUTTON_GAP` spacing, same `TOPCHROME_TOOLBAR_ACTIONS` labels clipped not removed) with a 12px original geometric mark (`fill_rect` only, existing palette hues) left of its label; labels + spacing preserved.
- CHECK: nav-arrow row → EXPECT: thin second row under the toolbar keeps existing Prev/Next 24px rhythm (`action_bar_row_height`/`action_nav_arrow_gap`) and adds two small chevron triangles (`<` / `>`, 3 stacked `fill_rect` bars each, original design, existing palette hues) — one per nav button.
- CHECK: geometry frozen elsewhere → EXPECT: `PATH_TOOLBAR_Y`=114, band 56 (`row_h + row_gap + row_h`), pane_top 174 (`114+56+4`); `text_fixture_center_layout()` still `(12,448,464,20,488,448)`; `PANE_STATUS_H`=18, strip_y 542; legend/status strips y570..600 intact.
- CHECK: edge fixture → EXPECT: `checks/gui-exq-toolbar21/round-1/comparison/toolbar-narrow-720.json` exists (720px: icons+labels compress without overlap) + headless test `toolbar21_narrow720_no_overlap` passes.
- CHECK: full suite → EXPECT: `cargo test -p autocompare-gui` green (0 failures).
- CHECK: fmt → EXPECT: `cargo fmt --check` clean (only touched file `crates/gui/src/screenshot.rs` formatted).
- CHECK: banned tokens on non-test delta → EXPECT: added non-test lines contain no `unwrap(`/`expect(`/`panic!`/`todo!`/`unreachable!` and no indexing (`git diff` added lines grepped; empty result).
- CHECK: colors frozen → EXPECT: `Color32::from_rgb` + `Visuals::` count across `crates/gui/src/*.rs` stays exactly 33 (glyphs reuse existing palette only: `fixture_palette().text`/`.dim`, existing `[45,45,45,255]`/`[60,60,60,255]` band tones — zero new hues).

## Post-edit results (implementation done, pasted verbatim)
- Harness re-run: `RESULT: PASS` — PNG 11254 bytes (min 8192 ok), magic ok, distinct 256 (ok); COLORS 33.
- Pixel gate (`python3 tools/exquisite/pxdiff.py before.png after.png --expect-diff`, exit 0):
  - `before: 960x600 ch=4 / after: 960x600 ch=4 / diff_pixels: 1995 of 576000 / diff_bbox: (16,120)-(866,162) / VERDICT: DIFFERENT`
  - Diff confined to toolbar band (y120..162 inside band y114..170); everything outside the bbox is byte-identical, so gutter/markers/chrome/typo rows/status strips are intact.
- Full suite: `cargo test -p autocompare-gui` → 1044 passed, 0 failed (incl. `screenshot::tests::toolbar21_narrow720_no_overlap`).
- `cargo fmt --check` → clean (exit 0).
- Banned-token grep on added non-test lines (`unwrap\(|expect\(|panic!|todo!|unreachable!` + indexing) → empty.
- Colors: `Color32::from_rgb` + `Visuals::` = 33 (unchanged).
