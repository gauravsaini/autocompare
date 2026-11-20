# Gates: BINARY-EARLY-2 installable binary (no source edits)

OWNS: dist/** checks/binary-early-2/**
FORBIDDEN: crates/** tools/** skills/** docs/** STATUS.md PLAN.md inbox/**

Scope: release build + strip + combined tarball + sha256 + fresh /tmp install test (CLI --help, text/folder smoke, GUI --screenshot PNG) + install snippet. Sirf build+package+verify, koi source edit nahi.

- [x] G1: Release build dono packages ka success.
  CHECK: CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-cli -p autocompare-gui
  EXPECT: Finished `release` profile
  EVIDENCE: Finished `release` profile [optimized] target(s) in 1m 20s (2026-11-20). autocompare 1683984 bytes, autocompare-gui 11909376 bytes (unstripped, Mach-O arm64).

- [x] G2: Strip + combined tarball + sha256 maujood.
  CHECK: ls -la dist/autocompare-0.5.0-macos-arm64.tar.gz dist/autocompare-0.5.0-macos-arm64.tar.gz.sha256
  EXPECT: dono files exist, sha256 match
  EVIDENCE: tarball 5504450 bytes; sha256 26719972f70ddfbb5a19e9ef3f3d4885489edb98f9fd8bfc4fafc3f4263547eb. Stripped sizes: CLI 1445432, GUI 10380384.

- [x] G3: Fresh /tmp install test — --help/--version + text/folder smoke.
  CHECK: tar -xzf dist tarball -C /tmp/ac-fresh; ./autocompare --help; ./autocompare text; ./autocompare folder
  EXPECT: --help exit 0; text diff exit 1, text same exit 0; folder diff exit 1
  EVIDENCE: --help exit 0, --version `autocompare 0.5.0`; TEXT_DIFF_EXIT:1, TEXT_SAME_EXIT:0, FOLDER_EXIT:1 (2026-11-20, /tmp/ac-fresh).

- [x] G4: GUI headless screenshot 1 valid PNG.
  CHECK: ./autocompare-gui --screenshot /tmp/ac-fresh/gui-smoke.png && file <png>
  EXPECT: exit 0 + PNG image data
  EVIDENCE: GUI_EXIT:0, `PNG image data, 960 x 600, 8-bit/color RGBA`, 9765 bytes. Copy: checks/binary-early-2/gui-smoke.png.

- [x] G5: Install snippet + Hinglish-Roman REPORT maujood.
  CHECK: ls checks/binary-early-2/INSTALL.md checks/binary-early-2/REPORT.md
  EXPECT: dono exist
  EVIDENCE: is commit me dono files (INSTALL.md install snippet, REPORT.md Hinglish-Roman).

- [x] G6: Commit pathspec sirf dist + checks/binary-early-2, no push.
  CHECK: git status --short; git log --oneline -1
  EXPECT: sirf dist/** + checks/binary-early-2/** staged/committed; working tree ke crates/** edits untouched; push nahi
  EVIDENCE: commit me dekhna (git show --stat HEAD).
