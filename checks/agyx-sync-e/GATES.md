# GATES — AGYX-E FOLDER SYNC MERGE Headless Parity

Wave: AGYX-E FOLDER SYNC MERGE swarm node.
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)
- `crates/compare/src/folder.rs`
- `crates/compare/src/sync.rs`
- `crates/compare/src/folder_merge.rs`
- `crates/compare/src/jobs.rs`
- `crates/compare/tests/folder.rs`
- `crates/compare/tests/sync.rs`
- `crates/compare/tests/folder_merge.rs`
- `crates/compare/tests/jobs.rs`
- `checks/agyx-sync-e/**`

## FORBIDDEN
`crates/compare/src/lib.rs`, `crates/compare/Cargo.toml`, `crates/compare/src/table.rs`, `picture.rs`, `media.rs`, `binary.rs`, `text.rs`, `crates/vfs`, `crates/diff`, `crates/sessions`, `crates/reports`, `crates/cli`, `crates/gui`, `STATUS.md`, `PLAN.md`, `checks/gui-*` dirs.

## Acceptance Gates Status

### G1 — Folder Tree Expand/Collapse Planning Support (`folder.rs`)
- `FolderComparison` exposes `tree_nodes` yielding `FolderTreeNode` with accurate depth, expansion, and children status.
- Expand all, collapse all, toggle expansion, ancestor resolution, direct children, and visibility queries work correctly and deterministically.
- `cargo test -p autocompare-compare --test folder` -> 20 passed; 0 failed.
- Status: PASS

### G2 — SyncPlanner All Modes + Conflict Navigation + Reports (`sync.rs`)
- `SyncPlanner` supports Update Right, Update Left, Update Both, and Mirror modes.
- `SyncPlan` provides conflict counting and navigation (`conflicts`, `conflict_paths`, `first_conflict`, `last_conflict`, `next_conflict`, `prev_conflict`).
- `SyncReport` provides summary statistics and success validation.
- Cancellable sync planning and apply support.
- `cargo test -p autocompare-compare --test sync` -> 17 passed; 0 failed.
- Status: PASS

### G3 — FolderMerge Plan/Apply + Conflict Navigation + Reports (`folder_merge.rs`)
- `FolderMerge` provides high-level `plan`, `apply`, `dry_run`, `run`, and conflict stepper helpers.
- `TreeMergePlan` provides conflict navigation (`has_conflicts`, `next_conflict`, `prev_conflict`) and step counters.
- `TreeMergeReport` provides summary statistics and success validation.
- Cancellable merge planning and apply support.
- `cargo test -p autocompare-compare --test folder_merge` -> 16 passed; 0 failed.
- Status: PASS

### G4 — JobRunner Cancellation + Bounded Progress (`jobs.rs`)
- `JobProgress` provides percentage/fraction calculations and completeness checks.
- `JobHandle` provides bounded progress buffer draining and polling without blocking.
- Background sync plan/apply and folder merge plan/apply honor cancellation tokens and report step progress.
- `cargo test -p autocompare-compare --test jobs` -> 8 passed; 0 failed.
- Status: PASS

### G5 — Clippy Zero Warnings & Formatting Clean
- Clippy passes with `--all-targets --no-deps -- -D warnings` on autocompare-compare: ZERO warnings.
- Formatting is clean on all owned files with `rustfmt --edition 2024 --check`: exit 0.
- Status: PASS

### G6 — MS Rust Banned Patterns Zero in Non-Test Code
- Non-test code contains zero `unwrap()`, zero `.expect(`, zero `panic!`, zero `todo!`, zero `unimplemented!`, zero `unreachable!`, zero unsafe blocks, and zero indexing/slicing (`[...]`).
- `rg -n "(\.unwrap\(|\.expect\(|panic!|todo!|unimplemented!|unreachable!)" crates/compare/src/folder.rs crates/compare/src/sync.rs crates/compare/src/folder_merge.rs crates/compare/src/jobs.rs` -> 0 matches.
- `rg -n "(unsafe\s+\{|unsafe\s+fn)" crates/compare/src/folder.rs crates/compare/src/sync.rs crates/compare/src/folder_merge.rs crates/compare/src/jobs.rs` -> 0 matches.
- `cargo clippy -p autocompare-compare --lib --no-deps -- -D clippy::indexing_slicing` -> 0 warnings.
- Status: PASS

### G7 — OWNS Boundary Proof
- Only owned files will be staged and committed via explicit pathspec.
- Other waves' uncommitted files in workspace are strictly untouched.
- Status: PASS
