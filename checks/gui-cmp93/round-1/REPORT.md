# REPORT: cmp93 round-1 folder compare orchestration (leaf-2.3)

Base: 7218042. OWNS: crates/compare/** + checks/gui-cmp93/**. No GUI touch.

## Kya bana

- `crates/compare/src/vfs_compare.rs` (naya): kisi bhi `Vfs` jodi par
  folder-tree compare — LocalFs, ZipFs, SnapshotFs. Wahi verdicts jo
  path walk deta hai (Same/Different/NewerLeft/NewerRight/LeftOnly/
  RightOnly), symlink kabhi follow nahi, depth cut honest truncated,
  cancellation har kadam par. Snapshot metadata-only hai isliye wahan
  size/mtime niyam lagte hain (byte compare nahi); Zip/LocalFs par
  content compare stream hota hai.
- `FolderCompare::run_over_vfs` / `run_over_vfs_cancellable`: engine
  ke options se VFS jodi par run.
- `JobRunner::spawn_vfs_folder_compare`: Arc<dyn Vfs> par cancellable
  background job, bounded progress.
- Sync Update Both / Mirror dry-run/apply, Folder Merge, JobRunner —
  pehle se maujood, naye `tests/vfs_compare.rs` me cover (8 test).
- Ponytail note: `glob_match` reuse (naya copy nahi), koi naya
  dependency nahi. Chhoda: VFS-backed sync plan builder — path planner
  hi canonical hai, jab GUI ko VFS plan chahiye tab add hoga.

## Gates (5/5 met)

- cmp93-G1: `cargo test -p autocompare-compare` — 12 suite, 165 passed, 0 failed.
- cmp93-G2: `--test vfs_compare` — 8 passed (LocalFs parity, Zip, Snapshot, Both, Mirror, Merge, JobRunner+cancel).
- cmp93-G3: `cargo fmt --check -p autocompare-compare` — FMT_OK.
- cmp93-G4: `cargo clippy -p autocompare-compare --all-targets -- -D warnings` — CLIPPY_OK; prod me banned zero (forbid unsafe compile-time; unwrap/expect/panic/todo sirf test files me file-level allow ke saath).
- cmp93-G5: `git status --short` — sirf crates/compare/** + checks/gui-cmp93/**; GUI/sessionpaint zero touch.

## Proof

- `checks/gui-cmp93/round-1/tests.out` — poora 165/0 log.
- Honest edge: SnapshotFs payload store nahi karta; content_compare
  uspar `UnsupportedFormat` deta hai — isliye snapshot test metadata
  niyam par hai (whole-second mtime fixture ke saath exact Same).
