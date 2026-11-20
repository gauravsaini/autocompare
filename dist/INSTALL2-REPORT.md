# INSTALL2 report — early-feedback macos-arm64 tarball

- Tarball: `dist/autocompare-gui-0.5.0-macos-arm64.tar.gz`
- Bytes: 5126800
- SHA256: `ab08efa0fec0051128048cf218b4b90d31040a78f73917feefc6e2c85a098cea`
- Layout (repo convention, mirrors `scripts/install-macos.sh`): `autocompare-gui` + `VERSION`
  (leaf prompt said binary + README; repo scripts and README install section define
  binary + VERSION stamp, so VERSION was kept for install parity)
- `dist/VERSION`: `0.5.0 1a54372` (SHA refreshed to current HEAD; no version bump needed)
- Build: `CARGO_TARGET_DIR=/tmp/target-install2 CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-gui` → exit 0
- Verify:
  - `tar tzf` lists `autocompare-gui`, `VERSION` → PASS
  - extracted `autocompare-gui --help` exits 0 → PASS
- Result: PASS (round 1)
