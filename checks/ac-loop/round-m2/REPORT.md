# Round-M2 — Media Wiring (dead-code khatam)

Branch: main. Push nahi, pull nahi. Sirf OWNS paths chhuye; STATUS/PLAN/inbox/baaki crates untouched.

## Kya kiya (order me)

1. Saare `#[allow(dead_code)]` OWNS me se hataye — src me 57 + `compare/tests/common` ka 1 module-allow.
2. Har dead item ya to real flow me wire kiya, ya delete kiya. Masking wapas nahi lagayi.
3. `grammar.rs` / `merge.rs` / `text_diff.rs` ke 3 module-allow chhode — ye OWNS ke bahar hain (forbidden).

## Wired (live flow me joda)

- `decode_bmp` / `decode_ppm` / `decode_image` (diff::image) → `compare::picture::decode_side` ab sirf
  `decode_image` call karta (PNG/BMP/P6 sniff). Duplicate `decode_bmp_fallback` + `decode_ppm_fallback` +
  `decode_side_any` private dups delete. Re-export: `autocompare_diff::decode_image`.
- `diff_audio_with_tolerance` (diff::audio) → `compare_audio_files_with_tolerance` ab isi ko call karta;
  private duplicate `diff_tolerant` delete. Re-export: `autocompare_diff::diff_audio_with_tolerance`.
- `AudioTolerance` + `compare_audio_files_with_tolerance` (compare::media) → `compare_audio_files` ab
  inko delegate karta (tolerance 0 = exact; behavior same, single code path). Re-export dono ka compare lib me.
- Redundant allows hataye, pehle se live the: `PcmAudio::peak/rms`, `read_media_tags`,
  `decode_xlsx` (via `Workbook::from_xlsx`), `ByteEditBuffer` (public `hex` mod).

## Deleted (koi real flow nahi — duplicate ya demo)

- diff::image: `encode_bmp`, `picture_blend`, `picture_difference_blend`, `diff_pictures_with_offset`,
  `picture_heatmap_with_offset`, `nudge_image`, `PictureBlendMode` (Exact/CompositeBackdrop/Difference),
  `PictureOptions::blend_mode`, `composite_over`. Saath me 3 unit tests.
- diff::audio: `sanitize`, `sanitized`, `to_mono`, `resample`, `PcmTotalityMetrics`, `TotalAudioDiff`,
  `diff_audio_total`. Saath me 4 unit tests. (Compare-side duplicate totality bhi gayi — neeche.)
- diff::table: `parse_tsv`, `diff_table_by_key_sorted`. Saath me 2 unit tests.
- compare::media: `AudioTotality`, `compare_audio_files_total`, `totality_of`, `frame_count_of`,
  `duration_of`, `peak_of`, `rms_of`, `load_media_tags`, `MediaTagsComparison`,
  `compare_media_tags_files`, `FullMediaComparison`, `compare_media_files_full` + 3 unit tests.
  (Totality dono layer me duplicate thi; pure-side wali bhi gayi kyunki koi caller nahi.)
- compare::picture: `PictureTolerance`, `compare_picture_files_with_tolerance`, `pixels_match_tol`,
  `composite_white`, `pixel_at`, `diff_with_tolerance`, `heatmap_with_tolerance`,
  `compare_picture_files_with_offset` + 3 unit tests. (Tolerant/offset compare ka koi CLI flow nahi;
  exact path diff ke tolerance/range engine par pehle se hai.)
- compare::table: `detect_table_delimiter`, `load_table_tsv`, `compare_table_files_tsv`,
  `compare_table_files_by_key_sorted`, `load_workbook`, `WorkbookComparison`, `compare_workbook_files`.
  (xlsx decode diff-side `Workbook::from_xlsx` me live hai.)
- compare::binary: `read_bytes_at_offset`, `load_byte_edit_buffer`, `apply_byte_buffer_to_file`.
  (Byte-edit backend ka koi caller nahi; `ByteEditBuffer` type public API me raha.)

## Naye tests (wired flow ka proof)

- `compare/tests/picture.rs`: BMP fixture compare (sniff path), PPM-vs-PNG cross-format identical.
- `compare/tests/media.rs`: 1-LSB delta strict me differ, tolerance 0.001 me identical.
- `compare/tests/common`: smoke test — saare shared helpers use hote hain, isliye har binary me
  dead-warning khatam bina allow ke.

## Behavior note (honest)

- `compare_picture_files` ab PPM bhi accept karta (pehle PNG/BMP); error text ab
  "not a decodable PNG, BMP or PPM" hai. Purana assert ("not a decodable PNG" substring) pass hai.
- diff ka `PictureOptions` internal type tha (kabhi export nahi hua) — `blend_mode` hatane se public API nahi badli.

## Verify

- `cargo test -p autocompare-diff -p autocompare-compare`: 335 passed, 0 failed (baseline 333).
- `cargo clippy -p autocompare-diff -p autocompare-compare --all-targets`: zero warnings/errors.
- `cargo check --workspace --all-targets`: zero warnings/errors (CLI/GUI toote nahi).
- `rustfmt --check` touched files par clean (`cargo fmt --all` ne CLI files chhuye the — revert kiye).
- `grep allow(dead_code)` OWNS paths me zero hits (bahar sirf 3 forbidden module-allows bache).
