# round-2 REPORT — picture Blend + zoom/pan/offset-nudge (methods on RgbaImage)

## Assumptions
- `crates/diff/src/lib.rs` untouched (outside OWNS): new API is
  `impl RgbaImage` methods only — visible via the already-exported type.
- No new deps, no new fields, no signature changes.

## What changed (`crates/diff/src/image.rs` only)
- `RgbaImage::nudge(dx, dy)` — pan/offset-nudge, vacated = transparent.
- `RgbaImage::viewport(x, y, w, h)` — pan window crop, OOB/empty = None.
- `RgbaImage::scaled(factor)` — integer nearest-neighbour zoom.
- `RgbaImage::difference_blend(other)` — per-channel |a-b| over overlap.
- `RgbaImage::diff_with_offset(other, dx, dy, tolerance)` — tolerance
  compare over in-frame pairs only; `total` = compared pixels.
- 5 unit tests (no unwrap/expect/panic, no indexing).
- Fixed own bug during build: u32 overflow on extreme offsets
  (range guard before `as u32`), out-of-frame pairs skipped not counted.

## Fixtures (`comparison/`)
- `pic_base_2x1.png`, `pic_nudged_2x1.png` (generated, zlib-verified)
- `picture_offset_expected.txt`

## Gates
- lib image tests: 11 passed; clippy 0; fmt clean (this round)

## RESULT: PASS
