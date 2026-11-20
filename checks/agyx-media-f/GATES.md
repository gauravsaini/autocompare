# AGYX-MEDIA-F GATES — Table + Picture + Media + Hex Headless Parity

Track: AGYX-F TABLE PICTURE MEDIA HEX swarm node
Repo: /Users/ektasaini/autocompare
OWNS (ONLY):
- crates/compare/src/table.rs
- crates/compare/src/picture.rs
- crates/compare/src/media.rs
- crates/compare/src/binary.rs
- crates/compare/tests/table.rs
- crates/compare/tests/picture.rs
- crates/compare/tests/media.rs
- crates/compare/tests/binary.rs
- crates/diff/src/table.rs
- crates/diff/src/image.rs
- crates/diff/src/audio.rs
- crates/diff/src/media_tags.rs
- crates/diff/src/hex.rs
- crates/diff/tests/table.rs
- crates/diff/tests/image.rs
- crates/diff/tests/audio.rs
- checks/agyx-media-f/**

## GATES LIST

- [x] G1: tests green
  CHECK: `cargo test -p autocompare-compare --lib --test table --test picture --test media --test binary && cargo test -p autocompare-diff --lib --test table --test image --test audio`
  RESULT: PASS — 187 tests passed across both crates (autocompare-compare: binary 14, media 5, picture 7, table 11; autocompare-diff: lib 68, audio 44, image 20, table 38).

- [x] G2: clippy zero warnings
  CHECK: `cargo clippy -p autocompare-compare -p autocompare-diff --lib --tests 2>&1 | grep -E "crates/(diff|compare)/(src|tests)/(table|image|audio|media_tags|hex|picture|media|binary)\.rs"`
  RESULT: PASS — 0 warnings on all owned files.

- [x] G3: rustfmt check
  CHECK: `cargo fmt --check -p autocompare-diff -p autocompare-compare`
  RESULT: PASS — clean formatting with zero diffs.

- [x] G4: banned grep in non-test code
  CHECK: Python non-test scanner for unwrap/expect/panic/todo/unimplemented/unreachable/unsafe in non-test code of owned files.
  RESULT: PASS — Total banned pattern errors: 0.

- [x] G5: OWNS proof
  CHECK: Two separate pathspec-only commits touching strictly owned files in autocompare-diff and autocompare-compare crates.
  RESULT: PASS — Untouched other swarms' changes, no git add -A, zero push.
