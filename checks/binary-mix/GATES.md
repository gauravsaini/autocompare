# Gates: binary mix early feedback (no source edits)

OWNS: dist/** checks/binary-mix/**

Scope: Release binaries build via scripts/release-early-pack.sh with CARGO_TARGET_DIR=/tmp/target-agyx-binmix, smoke via --screenshot PNG magic-ok 960x600, sha256 tarballs, report in checks/binary-mix/REPORT.md ending RESULT: PASS/FAIL. Zero source edits to crates/**.

- [x] G1: release build cli+gui success via scripts/release-early-pack.sh
  CHECK: test -f /tmp/target-agyx-binmix/release/autocompare && test -f /tmp/target-agyx-binmix/release/autocompare-gui && echo "BUILD_OK"
  EXPECT: BUILD_OK
  EVIDENCE: Built with CARGO_TARGET_DIR=/tmp/target-agyx-binmix in 2m 40s. autocompare (1.72MB), autocompare-gui (12.01MB) generated cleanly, zero edits to crates/**.

- [x] G2: dist tarballs and sha256 checksums present
  CHECK: (cd dist && shasum -a 256 -c autocompare-0.5.0-macos-arm64.tar.gz.sha256) && shasum -a 256 -c checks/binary-mix/SHA256SUMS
  EXPECT: OK
  EVIDENCE: scripts/release-early-pack.sh staged cli (749KB) & gui (4.9MB) tarballs; combined tarball (5.6MB) packaged; shasum -a 256 verification passed with OK for all tarballs.

- [x] G3: CLI smoke check (--help + text diff/same)
  CHECK: sh checks/binary-mix/smoke-cli.sh
  EXPECT: CLI_SMOKE_OK
  EVIDENCE: smoke-cli.sh passed (diff_exit=1, same_exit=0, --help and --version exit 0). Output logged to checks/binary-mix/cli-*.out.

- [x] G4: GUI headless screenshot valid 960x600 PNG with magic bytes
  CHECK: test -f checks/binary-mix/gui-smoke.png && file checks/binary-mix/gui-smoke.png | grep -q "960 x 600" && head -c 4 checks/binary-mix/gui-smoke.png | od -An -tx1 | tr -s ' ' | grep -q "89 50 4e 47" && echo "GUI_SMOKE_OK"
  EXPECT: GUI_SMOKE_OK
  EVIDENCE: autocompare-gui --screenshot produced gui-smoke.png (9.5KB). file reports: "PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced", magic header 89 50 4e 47 0d 0a 1a 0a verified.

- [x] G5: Fresh extraction test from tarball in /tmp
  CHECK: sh checks/binary-mix/test-extract.sh
  EXPECT: EXTRACT_OK
  EVIDENCE: test-extract.sh extracted autocompare-0.5.0-macos-arm64.tar.gz in /tmp/test-extract-binmix-*, ran --help successfully, exit 0.

- [x] G6: checks/binary-mix/REPORT.md exists with RESULT: PASS
  CHECK: tail -n 1 checks/binary-mix/REPORT.md
  EXPECT: RESULT: PASS
  EVIDENCE: checks/binary-mix/REPORT.md generated with full metrics, Elon-style summary, and ends with RESULT: PASS.
