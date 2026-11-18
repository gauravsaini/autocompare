# TIMELINE

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
