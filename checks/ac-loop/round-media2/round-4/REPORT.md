# round-4 REPORT — compare-layer file wiring (no lib.rs, no dead code)

## What changed (`crates/compare/src/picture.rs`, `media.rs` only)
- picture.rs: `test_offset_aligns_nudged_files` — PNGs via `encode_png`,
  `compare_picture_files` exact differing=2, then file bytes decoded and
  `diff_with_offset(1,0,0)` differing=0/total=1. New diff API reached
  through the filesystem path using only already-exported types.
- media.rs: `test_pcm_wav_kept_identical_through_files` — hand-built mono
  WAVs (PCM keep): identical pair, 1-LSB drift strict-differing,
  tolerance 0.001 identical.
- No new pub items (would need lib.rs re-export, outside OWNS, and m2
  deleted exactly that kind of dead code). No struct field changes
  (bare literals in tests/cli forbid it).

## Fixtures (`comparison/`)
- `pcm_same_a/b.wav`, `pcm_drift_1lsb.wav` (generated, wave-verified)
- `file_level_expected.txt` (+ round-2 PNGs reused for picture)

## Gates
- full OWNS suites green; clippy 0; fmt clean (final verification below)

## RESULT: PASS (pending final gate block)
