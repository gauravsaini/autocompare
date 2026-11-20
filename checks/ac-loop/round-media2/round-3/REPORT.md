# round-3 REPORT — media totals (FLAC/MP3/MP4-AAC tags, PCM kept)

## Assumptions
- No `lofty`: `crates/diff/Cargo.toml` outside OWNS, no new deps allowed.
  Extended the hand-rolled total parsers instead; PCM decode paths untouched.
- `MediaTags` gains `disc_total`: all literals use `..Default`, safe.
  `AudioTolerance`/`TableCompareOptions` still untouched (bare literals exist).

## What changed (`crates/diff/src/media_tags.rs` only)
- `MediaTags::disc_total: Option<u32>` + `diff_media_tags` "Disc Total" row.
- Vorbis: `TRACKNUMBER n/t`, `DISCNUMBER d/t` slash forms; `DISCTOTAL`.
- ID3: `TPOS d/t` now sets disc_total (TRCK already set track_total).
- MP4: `trkn` now also sets track_total; new `disk` atom (disc/disc_total,
  zero = unknown = None). Restored `extract_mp4_track_number` after an
  edit clobbered it (caught by compiler, smallest fix).
- 4 unit tests, no unwrap/expect/panic, no indexing.

## Fixtures (`comparison/`)
- `tags_flac_disc.flac`, `tags_tpos.mp3`, `tags_disk.m4a` (generated,
  layouts identical to unit tests) + `media_totals_expected.txt`

## Gates
- lib media_tags tests: 10 passed; clippy 0; fmt clean (this round)

## RESULT: PASS
