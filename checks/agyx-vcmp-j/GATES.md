# GATES — VCMP-J VFS and Compare Headless Parity

Wave: SWARM-VCMP-J vfs+compare folder/sync node.
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)
- `crates/vfs/**`
- `crates/compare/src/folder.rs`
- `crates/compare/src/sync.rs`
- `crates/compare/src/merge.rs`
- `crates/compare/src/jobs.rs`
- `crates/compare/tests/**`
- `checks/agyx-vcmp-j/**`

## FORBIDDEN
`crates/compare/src/lib.rs`, `crates/compare/Cargo.toml`, `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`, `crates/cli/**`, `crates/gui/**`, `crates/diff/**`, `crates/sessions/**`, `crates/reports/**`.

## Acceptance Gates Status

### G1 — LocalFS Symlink-Cycle Refusal & Safe Walk (`local.rs`)
- `LocalFs::walk_symlink_safe` and `Vfs::walk` detect directory symlink cycles and refuse them with `VfsError::SymlinkCycle` when `refuse_cycles: true`.
- Symlinks are never followed when `follow_symlinks: false`.
- `cargo test -p autocompare-vfs --test local_test` -> PASS.

### G2 — Zip/Tar Round-Trip & Path Safety (`archive_zip.rs`, `archive_tar.rs`)
- Pure-Rust `ZipFs` and `TarGzFs` serialize and deserialize archives faithfully.
- Zip slip path traversal attacks (`../` paths) are detected and rejected.
- Corrupted archives return `VfsError::CorruptedArchive` without panic or crash.
- `cargo test -p autocompare-vfs --test zip_test --test tar_test` -> PASS.

### G3 — Beyond Compare Session Snapshot (.bcss) Round-Trip (`snapshot.rs`)
- `.bcss` snapshot serialization (`serialize_bcss_compressed`, `serialize_bcss_json`) and deserialization (`deserialize_bcss`) round-trip with schema versioning (`BCSS_SCHEMA_VERSION = 1`).
- `SnapshotFs` exposes an immutable virtual filesystem view over captured directory metadata.
- Diff summary (`SnapshotDocument::diff_summary`) detects added, removed, modified, and identical entries.
- `cargo test -p autocompare-vfs --test snapshot_test` -> PASS.

### G4 — Folder Compare over LocalFS + Zip + .bcss (`folder.rs`)
- `FolderCompare::compare_vfs` and `FolderCompare::compare_vfs_cancellable` compare any two `dyn Vfs` instances (LocalFS vs Zip, LocalFS vs .bcss, Zip vs .bcss).
- Fast-path CRC32 comparison when metadata provides checksums; streaming byte equality check up to 64MB cap.
- Honest depth limiting, case-insensitive matching, and glob filtering over virtual filesystems.
- `cargo test -p autocompare-compare --test folder_vfs` -> PASS.

### G5 — SyncPlanner All Modes + Mirror + Dry-Run + Apply (`sync.rs`)
- `SyncPlanner` supports all 4 modes: `UpdateRight`, `UpdateLeft`, `Both` (Update Both), and `Mirror` (with prune).
- Dry-run planning without mutations and apply phase with error isolation (`SyncReport`).
- Conflict detection and bidirectional navigation (`next_conflict`, `prev_conflict`).
- `cargo test -p autocompare-compare --test sync` -> PASS.

### G6 — Three-Way Folder Merge Plan & Apply (`folder_merge.rs`)
- `FolderMerge` plans three-way directory reconciliation over base, left, and right trees.
- Detects non-conflicting text edits, binary conflicts, and deletion/addition rules.
- Conflict navigation and summary statistics.
- `cargo test -p autocompare-compare --test folder_merge` -> PASS.

### G7 — Cancellable Background JobRunner (`jobs.rs`)
- `JobRunner` executes folder compare, sync planning/apply, and folder merge on dedicated worker threads with cancellation flag.
- `JobRunner::spawn_folder_compare_vfs` runs VFS folder comparisons in the background.
- Bounded progress history queue (max 64 updates) with non-blocking poll and drain.
- `cargo test -p autocompare-compare --test jobs` -> PASS.

### G8 — Clippy Zero Warnings & Rustfmt Clean
- `cargo clippy -p autocompare-vfs -p autocompare-compare --all-targets -- -D warnings`: ZERO warnings.
- `rustfmt --check` on all modified files in OWNS: Clean (exit 0).

### G9 — MS Rust Banned Patterns Zero in Non-Test Code
- Non-test code contains zero `unwrap()`, zero `.expect(`, zero `panic!`, zero `todo!`, zero `unimplemented!`, zero `unreachable!`, zero unsafe blocks (`#![forbid(unsafe_code)]`).
- `cargo clippy -p autocompare-vfs -p autocompare-compare --lib --no-deps -- -D clippy::indexing_slicing`: ZERO warnings.

### G10 — ac-loop Harness Round 1 RESULT: PASS
- `tools/ac-loop/run.sh checks/agyx-vcmp-j/round-1` exits 0 with `RESULT: PASS`.
