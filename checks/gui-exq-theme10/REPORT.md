# Wave THEME10 — REPORT (round-1)

Scope sirf OWNS: `crates/gui/src/theme.rs` + `checks/gui-exq-theme10/**`.
BC vs ours gap honest: BC PNG ~888K, ours `after.png` ~10K — paint wave ka
kaam baaki hai; is wave me pane hierarchy (headers vs body vs meta) focus.
Paint-neutral headless hai, isliye PNG byte same raha; hierarchy tokens +
test se lock hai.

## Kya badla (theme.rs only)

- Geometry FIRST: `THEME10_PANE_HEADER_GAP` (12.0 = `GEO_OUTER_MARGIN`),
  `THEME10_PANE_BODY_GAP` (8.0 = `GEO_SECTION_GAP`), `THEME10_PANE_META_GAP`
  (4.0 = `GEO_TIGHT_GAP`), `THEME10_PANE_HEADER_H` (28.0 =
  `CHROME_HEADER_BAR_H`), `THEME10_PANE_PAD_X` (12.0 = `GEO_OUTER_MARGIN`).
  Koi naya literal nahi, sab SPACE/GEO/CHROME scale reuse.
- Typography SECOND: `TypoRole::pane_header()` (Heading 18 strong),
  `pane_body()` (Mono 12 regular-mono, `content_textcenter` alias),
  `pane_meta()` (Small 11 weak, `caption` alias). Size/family/weight/
  line-height/letter-spacing reuse only, koi naya point size nahi.
- Gate: `theme10_pane_hierarchy_ok()` (gap order 4 < 8 < 12 + size order
  18 > 12 > 11 + line-height px order header > body > meta).
- Colors FROZEN: zero naya `Color32`/`Visuals`, count 33 se 33 hi raha.

## Baseline -> After (round-1, same dir)

- `cargo test -p autocompare-gui`: 868 passed, 0 failed (+1: theme10 test).
- Harness `tools/exquisite/run.sh checks/gui-exq-theme10/round-1`: RESULT PASS.
- PNG: 10401 bytes, magic ok, distinct 256 (BC ~888K se farak paint wave ka kaam hai).
- Signals: margin/gap x309 -> x346, row-h x125 -> x129, gutter x960 -> x1036,
  TypoRole x748 -> x807 (naye pane aliases), monospace/strong/size/weights same.
- `rustfmt --check theme.rs`: clean. Banned (non-test): 0.
- Sibling note: `gutter.rs`/`views_hex.rs` me sibling ka dirty diff hai, usko
  touch nahi kiya; package clippy usi wajah se red hai — owned file clean hai.

## Rounds: 1 of 6. Green.
