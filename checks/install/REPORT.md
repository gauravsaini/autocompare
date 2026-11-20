# Install check — early-feedback tarball (gui10)

RESULT: PASS

- Tarball: dist/autocompare-gui-0.5.0-macos-arm64.tar.gz
- Size: 5127179 bytes (4.9M, ls -lh)
- VERSION stamp: 0.5.0 f2c7d30
- Contents: autocompare-gui, VERSION
- Steps:
  - CARGO_TARGET_DIR=/tmp/target-install CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-gui → ok (4m37s)
  - bash scripts/install-macos.sh → ok (incl. --screenshot smoke, non-empty png)
  - ls -lh dist/autocompare-gui-*-macos-*.tar.gz > checks/install/artifact.txt → ok
- Gates: cargo fmt --check -p autocompare-gui clean; crates/gui/src/* untouched; no gui tests run.
