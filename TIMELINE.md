# TIMELINE

## UPDATED ON : 2026-11-18

### Feature (2026-11-18) — v0.3.0: picture compare and three-way folder merge

1. **Scope**: `autocompare picture` (PNG pixel diff, bounds, heatmap; first third-party dep: pure-Rust `png` crate) and `autocompare folder-merge` (plan/apply three-way tree merge over the existing content-merge engine); 7 of the original's 9 observed session types now rebuilt.
2. **Tests** (before → after): 126 → 167 workspace tests, `clippy -D warnings` clean; live runs verified picture stats against hand-computed ground truth (14/64 symmetric-difference pixels, bbox) and merge output file by file (clean merges, markers, deletions, empty dirs, output-precondition refusal).
3. **Files changed**: `crates/diff/src/image.rs`, `crates/compare/src/{picture,folder_merge}.rs`, `crates/cli/src/{render_picture,render_foldermerge}.rs`, README, docs/study.md.

## UPDATED ON : 2026-11-18

### Feature (2026-11-18) — v0.2.0: table compare and folder sync

1. **Scope**: `autocompare table` (RFC-4180-lite delimited parsing, cell-level `[old => new]` deltas) and `autocompare sync` (plan/apply folder sync, dry-run default, prune, symlink-safe, per-step failure tolerance); both features taken from the original's session menu (verified via accessibility API).
2. **Tests** (before → after): 72 → 126 workspace tests (+39 table, +15 sync), `clippy -D warnings` clean; live runs verified table deltas/alignment and sync dry-run, apply (copies + prune delete on disk) and re-plan convergence.
3. **Files changed**: `crates/diff/src/table.rs`, `crates/compare/src/{table,sync}.rs`, `crates/cli/src/{render_table,render_sync}.rs`, README, docs/study.md.

## UPDATED ON : 2026-11-18

### Task (2026-11-18) — v0.1.0 released; original's session types verified

1. **Release**: tagged v0.1.0 (text, folder, hex, merge comparison) and published the GitHub release; CI green on all pushes.
2. **Verification**: enumerated the running original's Session → New Session menu via the macOS accessibility API (Folder Compare, Folder Merge, Folder Sync, Text Compare, Text Merge, Hex Compare, Media Compare, Picture Compare, Table Compare) and corrected docs/study.md; the visual screenshot pass is blocked by the screen-recording permission (remediation recorded).
3. **Files changed**: docs/study.md, TIMELINE.md.

## UPDATED ON : 2026-11-18

### Feature (2026-11-18) — Clean-room comparison tool rebuilt from a study of the original

1. **Scope**: text (side-by-side + unified), recursive folder, hex and three-way merge comparison as a pure-Rust workspace (`crates/diff` → `crates/compare` → `crates/cli`), built clean-room from a user-facing study of the original app (`docs/study.md`); no code, assets or wording copied.
2. **Tests** (before → after): none → 72/72 workspace tests, `clippy -D warnings` clean; 5 exhaustive release-mode sweeps (~430k pairs) verify diff replay, canonical form and minimality; live CLI sanity verified all six folder statuses, `--content` reclassification, exit codes 0/1/2 and conflict markers.
3. **Files changed**: `crates/*`, `docs/study.md`, `README.md`, `.github/workflows/ci.yml`, `LICENSE`.
