# Gates: leaf-1.1 VFS polish round-vfs3 (round-1)

OWNS: crates/vfs/**, checks/ac-loop/round-vfs3/**

Scope: LocalFS symlink safety (escape rejection, cycle handling), Zip/Tar.gz
truncated + hostile archive edges, .bcss snapshot round-trip plus diffs.

- [x] G1: autocompare-vfs test suite passes cleanly (isolated target dir)
  CHECK: CARGO_TARGET_DIR=/tmp/target-op-vfs3 CARGO_BUILD_JOBS=2 cargo test -p autocompare-vfs
  EXPECT: test result: ok

- [x] G2: Clippy is zero-warning on the vfs crate (all targets)
  CHECK: CARGO_TARGET_DIR=/tmp/target-op-vfs3 CARGO_BUILD_JOBS=2 cargo clippy -p autocompare-vfs --all-targets -- -D warnings
  EXPECT: Finished `dev` profile

- [x] G3: rustfmt is clean on OWNS rust sources
  CHECK: rustfmt --edition 2024 --check crates/vfs/src/lib.rs crates/vfs/src/trait_def.rs crates/vfs/src/error.rs crates/vfs/src/path.rs crates/vfs/src/local.rs crates/vfs/src/archive_zip.rs crates/vfs/src/archive_tar.rs crates/vfs/src/snapshot.rs crates/vfs/tests/vfs3_edges.rs && echo FMT_OK
  EXPECT: FMT_OK

- [x] G4: Zero banned macros in non-test OWNS code (indexing covered by G2 clippy)
  CHECK: rg -n 'unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!' crates/vfs/src && exit 1 || echo BANNED_ZERO
  EXPECT: BANNED_ZERO

- [x] G5: Edge fixture exists and round-1 report ends RESULT: PASS
  CHECK: test -f crates/vfs/tests/fixtures/edge-bcss-minimal.json && test -f checks/ac-loop/round-vfs3/round-1/REPORT.md && grep -q '^RESULT: PASS' checks/ac-loop/round-vfs3/round-1/REPORT.md && echo EDGE_AND_REPORT_OK
  EXPECT: EDGE_AND_REPORT_OK
