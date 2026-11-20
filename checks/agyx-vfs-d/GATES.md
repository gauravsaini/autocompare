# AGYX-VFS-D GATES — VFS Parity & Archive/Snapshot Polish

Track: AGYX-D VFS swarm node
Scope: `crates/vfs/src/{archive_zip.rs, archive_tar.rs, snapshot.rs, trait_def.rs, path.rs}`, `crates/vfs/tests/{zip_test.rs, tar_test.rs, snapshot_test.rs}`, `checks/agyx-vfs-d/**` ONLY
Goal: VFS parity — Zip/Tar/Gzip ArchiveFs polish + SnapshotFs .bcss roundtrip + WalkOptions + SFTP/WebDAV trait extension stub (headless only, no network in tests, in-memory fake impl)

## GATES STATUS

- [x] G1: tests green — PASS: `cargo test -p autocompare-vfs` → 23 passed (local 3, snapshot 8, tar 6, zip 6), 0 failed.
  CHECK: cargo test -p autocompare-vfs
  EXPECT: test result: ok. 0 failed.
  ACTUAL: ok — 23 passed, 0 failed, 0 ignored.

- [x] G2: clippy zero warnings — PASS: `cargo clippy -p autocompare-vfs --all-targets -- -D warnings` exit 0, zero warnings.
  CHECK: cargo clippy -p autocompare-vfs --all-targets -- -D warnings
  EXPECT: exit 0 with zero warnings.
  ACTUAL: exit 0, zero warnings.

- [x] G3: rustfmt clean on owned files — PASS: `rustfmt --edition 2024 --check` on all owned files clean.
  CHECK: rustfmt --edition 2024 --check crates/vfs/src/path.rs crates/vfs/src/trait_def.rs crates/vfs/src/archive_zip.rs crates/vfs/src/archive_tar.rs crates/vfs/src/snapshot.rs crates/vfs/tests/zip_test.rs crates/vfs/tests/tar_test.rs crates/vfs/tests/snapshot_test.rs
  EXPECT: exit 0, formatting clean.
  ACTUAL: exit 0, clean.

- [x] G4: banned grep in non-test code — PASS: ZERO-BANNED (no unwrap, expect, panic, todo, unimplemented, unreachable, indexing_slicing in non-test src).
  CHECK: awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' "$f" | grep -v '^\s*//' | grep -v '#!\[forbid(unsafe_code)\]' | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable|unsafe"
  EXPECT: Zero banned pattern violations in crates/vfs/src.
  ACTUAL: Clean — 0 violations across all owned src files.

- [x] G5: OWNS proof — PASS: commit pathspec ONLY (`git add crates/vfs checks/agyx-vfs-d`), NEVER git add -A, NEVER push.
  CHECK: git diff --stat crates/vfs checks/agyx-vfs-d
  EXPECT: Only files matching OWNS pathspec.
  ACTUAL: All modified files strictly within OWNS.
