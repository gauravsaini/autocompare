# binary-orch-3 (early install, clean HEAD build)

Binary `main@c4d9292` se, bina kisi source edit ke. Working tree me uncommitted
crates/** WIP tha jo compile nahi ho raha tha (vfs E0382), isliye clean HEAD ka
alag worktree `/tmp/orch3-src` me build kiya gaya. crates/** me zero edit.

## Install (fresh, /tmp test jaisa)

```sh
tar -xzf dist/autocompare-orch-3-macos-arm64.tar.gz -C /tmp/orch3-test
/tmp/orch3-test/autocompare --help
/tmp/orch3-test/autocompare text left.txt right.txt --color never
/tmp/orch3-test/autocompare folder dirA dirB --color never
/tmp/orch3-test/autocompare-gui --screenshot out.png
```

Verify:

```sh
shasum -a 256 dist/autocompare-orch-3-macos-arm64.tar.gz
# ceaec4c54aee78d2d06b1e93b022b2b13a4b25e22e8e8f48d44cda4f260eade4
```

## Contents

- `autocompare` (cli, stripped ~1.4MB, 10 commands: text/folder/hex/merge/table/sync/picture/folder-merge/audio/session)
- `autocompare-gui` (egui, stripped ~10MB, `--screenshot` supported)
- `gui.png` (960x600 reference screenshot, is round me liya gaya)
- `smoke.sh + smoke/` (G4 re-runnable gate: text + folder smoke)

Ad-hoc local binaries, no signature. Exit convention diff-style hai:
0 = identical, 1 = differences mile, >=2 = error.
