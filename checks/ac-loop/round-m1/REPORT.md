# Round-m1 REPORT — media-depth wave (Binary/Table/Picture/Media parity)

## Kya kiya (1 round me)

1. **Table workbook bug fix** (`crates/diff/src/table.rs`): `diff_workbooks` me RightOnly
   sheet kabhi report nahi hoti thi — dusri loop ki condition
   `!seen && !b_names.contains` hamesha false thi (b_names me saare
   right sheets hain). Ab sirf `!seen_b` check hai. BC parity: missing
   sheet ab honestly RightOnly milti hai.
2. **Media tags totality** (`crates/diff/src/media_tags.rs`): `diff_media_tags`
   ab `format` (Flac/Mp3/Mp4/AAC/Unknown) aur saare `extra_tags` ko
   key-wise compare karta hai (match/differ/left-only/right-only,
   duplicate keys positional). Pehle ye dono fields ignore hote the.
3. **Picture Tolerance/Range/Blend + PPM** (`crates/compare/src/picture.rs`):
   naya `PictureTolerance { tolerance, channel_range, composite_on_white }`
   aur `compare_picture_files_with_tolerance` — decode PNG + BMP + P6 PPM,
   heatmap bhi same tolerance se. `lib.rs` forbidden tha isliye naya
   struct picture.rs me rakha (existing `PictureOptions` ko haath nahi lagaya).
4. **Audio tolerance + totality** (`crates/compare/src/media.rs`): naya
   `AudioTolerance`, `compare_audio_files_with_tolerance`,
   `compare_audio_files_total` (+ `AudioTotality`: peak_delta, rms_delta,
   left/right peak+rms, duration_diff). Config mismatch ab bhi honest
   `config_mismatch` diff hai, guess nahi.
5. **Chhote helpers**: `HexDiff::is_identical`, `PictureDiff::differ_ratio`
   (pure, total, no panic).
6. **Hostile inputs**: truncated FLAC (`fLaC` + jhoothi length), khaali/
   aadhe PPM, aadhi ID3 — sab `None` ya honest diff, koi crash nahi.
   Saara non-test code `forbid(unsafe)` + deny
   (unwrap/expect/panic/todo/unreachable/index-slicing) me hai.

## Fixtures (checks/ac-loop/round-m1/comparison/)

- `table_key_left.csv` / `table_key_right.csv` / `table_key_expected.txt`
  (key=[0], tol=0.1: matching=3, modified=0, left_only=0, right_only=1)
- `hex_shift_expected.txt` (4-byte insert: None/Fast/Complete first_diff=4,
  Complete me RightOnly{len:4}, exact=true)
- `workbook_rightonly_expected.txt` (Extra sheet RightOnly, identical=false)
- `media_tags_extra_expected.txt` (Format + ENCODER differing, truncated FLAC Some/no-panic)
- `picture_tol_expected.txt` (strict 1 vs relaxed 0, range tight/loose)
- `audio_tol_expected.txt` (strict false vs relaxed true, peak_delta 0.1)
- Naya integration test: `crates/diff/tests/media_depth_round_m1.rs`
  (4 tests: workbook right/left-only, tags extra+format, tags one-sided)
- Compare-layer unit tests `picture.rs` / `media.rs` ke andar
  `#[cfg(test)]` me (compare/tests dir forbidden tha).

## Verify

- `CARGO_BUILD_JOBS=2 cargo test -p autocompare-diff -p autocompare-compare`:
  sab suites green (diff 71+33+44+20+4 naye, compare existing + 6 naye unit).
- `cargo clippy -p autocompare-diff -p autocompare-compare --all-targets`:
  0 warning, 0 error.
- `rustfmt --edition 2024 --check` (sirf owned files): clean.
- `cargo fmt --check` poore workspace pe ab bhi `cli` me 2 import-order
  diff dikhata hai — wo forbidden area hai, haath nahi lagaya.

## Note (lib.rs forbidden)

- `crates/compare/src/lib.rs` aur `crates/diff/src/lib.rs` ko haath nahi
  lagaya, isliye naye diff/image/audio fns `lib.rs` se re-export nahi hain.
  Compare-layer parity isliye picture.rs/media.rs me local implement ki
  (public `RgbaImage`/`PcmAudio` fields par). Agle wave me lib.rs owner
  inhe re-export kare to wiring aur patli hogi.

## RESULT: PASS
