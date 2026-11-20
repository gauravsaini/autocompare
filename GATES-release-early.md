# GATES: BINARY wave — installable release binaries early feedback

OWNS: dist/** scripts/release-early* checks/bin-early/** GATES-release-early.md
FORBIDDEN: crates/** STATUS.md PLAN.md

Scope: cargo build --release CLI+GUI, fresh check, smoke (CLI parity + GUI screenshot), dist packaging, GitHub release draft notes (attach orchestrator karega).

- [x] G1: Release binaries fresh (mtime > HEAD) + executable.
  CHECK: node scripts/verify-release-fresh.mjs
  EXPECT: RELEASE_FRESH_OK
  EVIDENCE: exit=0; output=RELEASE_FRESH_OK; staged from pristine a6bb8e8 build (/tmp workcopy via git archive — dirty tree crates/diff uncompilable, wave-forbidden to touch)

- [x] G2: CLI release parity smoke PASS (10 runs, diff-convention exits).
  CHECK: sh checks/bin-early/run-smoke.sh
  EXPECT: RESULT: PASS
  EVIDENCE: exit=0; REPORT.md RESULT: PASS, 10/10 exits match (text=1, text-same=0, baaki 1)

- [x] G3: GUI headless screenshot smoke PASS (valid PNG, size > 0).
  CHECK: sh checks/bin-early/run-smoke.sh
  EXPECT: GUI_SMOKE_OK
  EVIDENCE: exit=0; smoke-gui.png 9765 bytes, magic 89 50 4E 47; harness od-spacing bug fixed (tr -s)

- [x] G4: dist/ me fresh tarballs + VERSION stamp (version + HEAD sha).
  CHECK: sh scripts/release-early-pack.sh --check-only
  EXPECT: DIST_OK
  EVIDENCE: exit=0; DIST_OK; cli tarball 724333 B, gui tarball 5081073 B, VERSION=0.5.0 a6bb8e8; draft notes dist/RELEASE-DRAFT-v0.5.0.md
