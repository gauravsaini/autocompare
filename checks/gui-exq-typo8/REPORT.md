# Wave TYPO8 — REPORT (round-1)

Scope sirf OWNS: `crates/gui/src/theme.rs` + `checks/gui-exq-typo8/**`.
BC vs ours gap honest: BC PNG ~888K, ours `after.png` ~10K — geometry
wale din ka kaam pehle, is wave me typography focus. Paint-neutral headless
hai, isliye PNG byte same raha; hierarchy tokens + test se lock hai.

## Kya badla (theme.rs only, 83 lines)

- Geometry FIRST: `TYPO_PARA_GAP_SMALL` (4.0 = `GEO_TIGHT_GAP`), `TYPO_PARA_GAP_BODY`
  (8.0 = `GEO_SECTION_GAP`), `TYPO_SECTION_GAP_CHROME` (12.0 = `GEO_OUTER_MARGIN`),
  `TYPO_ROW_GAP_MONO` (4.0). Koi naya literal nahi, sab SPACE scale reuse.
- Typography SECOND: `TypoRole::para_gap()` — Small/HomeDesc hug (XS),
  Body/Mono/HomeButton breathe (SM), Heading/HomeTitle section separate (MD).
  Size/family/weight/line-height/letter-spacing untouched, koi naya point size nahi.
- Gate: `typo_rhythm_ok()` (gap order 4 < 8 < 12 + Small < Mono < Body line-height px)
  aur `typo_roles_plus_rhythm_ok()` (home-scale + textcenter + home4 + rhythm).
- Colors FROZEN: zero naya `Color32`/`Visuals`, count 33 se 33 hi raha.

## Baseline -> After (round-1 re-run, same dir)

- `cargo test -p autocompare-gui`: 848 passed, 0 failed (+1: typo8 rhythm test).
- Harness `tools/exquisite/run.sh checks/gui-exq-typo8/round-1`: RESULT PASS.
- PNG: 10401 bytes, magic ok, distinct 256 (BC ~888K se farak paint wave ka kaam hai).
- Signals: margin/gap x284 -> x309, row-h x121 -> x125, gutter x909 -> x960,
  TypoRole x734 -> x748 (naye rhythm tokens), monospace/strong/size/weights same.
- `rustfmt --check theme.rs`: clean. Banned (non-test): 0 (unsafe hit sirf forbid line).
- Sibling note: `gutter.rs` me sibling ka 280-line dirty diff hai, usko touch nahi
  kiya; workspace clippy/fmt usi wajah se red hai — owned file clean hai.

## Rounds: 1 of 6. Green.
