# Gates: binary early install orch-3 (no source edits)

OWNS: dist/** checks/binary-orch-3/**

Scope: Release build cli+gui, strip + tarball + sha256, fresh /tmp install test (--help + text/folder smoke + gui --screenshot PNG), install README snippet. crates/** me koi edit nahi.

- [x] G1: release build cli+gui success
  CHECK: CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-cli -p autocompare-gui
  EXPECT: Finished `release` profile
  EVIDENCE: 2026-11-20, CWD=/tmp/orch3-src (clean HEAD c4d9292 worktree, zero source edits), exit 0, "Finished `release` profile [optimized] target(s) in 3m 25s"; re-verify cached "in 0.52s". Note: repo working tree ka dirty crates/** WIP compile nahi hota (vfs E0382), isliye HEAD worktree use kiya.

- [x] G2: strip + tarball + sha256 present
  CHECK: test -f dist/autocompare-orch-3-macos-arm64.tar.gz && shasum -a 256 dist/autocompare-orch-3-macos-arm64.tar.gz
  EXPECT: autocompare-orch-3-macos-arm64.tar.gz
  EVIDENCE: 2026-11-20, exit 0, ceaec4c54aee78d2d06b1e93b022b2b13a4b25e22e8e8f48d44cda4f260eade4 (5.5MB; stripped: cli 1.46MB, gui 10.46MB).

- [x] G3: fresh /tmp install --help works
  CHECK: /tmp/orch3-test/autocompare --help
  EXPECT: Usage
  EVIDENCE: 2026-11-20, exit 0, "Usage: autocompare [OPTIONS] [COMMAND]" + 10 commands (text/folder/hex/merge/table/sync/picture/folder-merge/audio/session).

- [x] G4: text + folder smoke pass
  CHECK: sh checks/binary-orch-3/smoke.sh
  EXPECT: SMOKE-OK
  EVIDENCE: 2026-11-20, exit 0, "SMOKE-OK". Exit convention diff-style hai: 0 = identical, 1 = differences, >=2 = error (pehle pipe me head ka exit dekh ke confuse hua tha, phir direct exit code se confirm kiya: diff pe 1, same pe 0).

- [x] G5: gui --screenshot produces 1 PNG
  CHECK: test -f checks/binary-orch-3/gui.png && file checks/binary-orch-3/gui.png
  EXPECT: PNG image data
  EVIDENCE: 2026-11-20, exit 0, "PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced".

- [x] G6: install README snippet present
  CHECK: test -f checks/binary-orch-3/README.md && grep -q "tar -xzf" checks/binary-orch-3/README.md && echo SNIPPET-OK
  EXPECT: SNIPPET-OK
  EVIDENCE: 2026-11-20, exit 0, "SNIPPET-OK".
