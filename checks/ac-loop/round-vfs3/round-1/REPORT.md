# ac-loop round report (round-vfs3/round-1) — VFS polish: symlink safety, hostile archives, snapshot round-trip

## Scope (PLAN C3 follow-up, leaf-1.1 OWNS only)
- `crates/vfs/**`: LocalFS symlink safety, Zip/Tar.gz reader edges, `.bcss` snapshot round-trip + diffs.
- `checks/ac-loop/round-vfs3/**`: this report, gates ledger, one edge fixture.

## What changed
1. **LocalFS symlink safety (no behavior change, covered by new tests)**: self-loop
   `SymlinkCycle` refusal, dangling-link tolerance, `refuse_cycles=false`
   leniency, symlink `delete` keeps outside target, in-root symlink reads work.
2. **Zip/Tar.gz hostile-reader hardening (2 one-line diffs)**: `read_stream` /
   entry parsing no longer pre-allocate from untrusted central-directory and
   header sizes (`Vec::new()` instead of `Vec::with_capacity(untrusted)`).
   Truncated tails rejected as `CorruptedArchive`; absolute-path entries are
   contained inside the archive root via `VfsPath` normalization.
3. **Snapshot round-trip + diffs**: new `edge-bcss-minimal.json` fixture
   (JSON + gzip round-trip, `SnapshotFs` browse, read-only enforcement,
   `UnsupportedFormat`/`NotAFile` paths), corrupted-input rejection
   (garbage, empty, truncated gzip), dir-to-file type change counts as
   `modified` with `detailed_diff`/`summary` agreement.

## Edge fixture
- `crates/vfs/tests/fixtures/edge-bcss-minimal.json`: minimal 3-entry snapshot
  (`docs/`, `docs/note.txt` crc32 222957957, `empty.bin`).

## Verify (isolated target dir `/tmp/target-op-vfs3`, jobs=2)
- `cargo test -p autocompare-vfs`: 48 passed, 0 failed
  (was 34: 4 local + 5 path + 10 snapshot + 8 tar + 7 zip; now +14 `vfs3_edges`).
- `cargo clippy -p autocompare-vfs --all-targets -- -D warnings`: 0 warnings.
- `rustfmt --edition 2024 --check` on all OWNS rust sources + new test: clean.
- Banned-scan `unwrap(|expect(|panic!|todo!|unimplemented!|unreachable!` in
  `crates/vfs/src`: zero matches (indexing covered by clippy `indexing_slicing`).
- Skipped: zip-symlink entry creation is impossible with zip 2.6.1
  (`unix_permissions` masks to `0o777`, discarding file-type bits); symlink
  archive semantics are covered via the tar symlink test instead.

## Files
- `crates/vfs/src/archive_zip.rs`, `crates/vfs/src/archive_tar.rs`: 1 line each.
- `crates/vfs/tests/vfs3_edges.rs`: 14 new edge tests.
- `crates/vfs/tests/fixtures/edge-bcss-minimal.json`: 1 new fixture.

RESULT: PASS
