# INSTALL3 report — early-feedback macos-arm64 tarball

- Tarball: `dist/autocompare-gui-0.5.0-macos-arm64.tar.gz`
- Bytes: 5127067
- SHA256: `50126ecd43f522ac0a64378c8c9b51cfc5d98593b45cbc13e065f1af35149b1f`
- Layout (repo convention, mirrors `scripts/install-macos.sh`): `autocompare-gui` + `VERSION`
  (leaf prompt said binary + README; repo scripts and README install section define
  binary + VERSION stamp, so VERSION was kept for install parity — same as INSTALL2)
- `dist/VERSION`: `0.5.0 46b8a06` (SHA refreshed to current HEAD; no version bump needed)
- Build: `CARGO_TARGET_DIR=/tmp/target-install3 CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-gui` → exit 0
- Verify:
  - `tar tzf` lists `autocompare-gui`, `VERSION` → PASS
  - extracted `autocompare-gui --help` exits 0 → PASS
- Result: PASS (round 1)
