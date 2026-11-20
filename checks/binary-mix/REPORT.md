# REPORT — Binary Mix Early-Feedback Release Pack

Subject: Binary Mix Early Release Pack – 2026-11-20

## 1. Summary
Release binaries build, smoke test, packaging, aur checksum verification complete ho gayi hai without touching `crates/**` (clean-room). Isolated `CARGO_TARGET_DIR=/tmp/target-agyx-binmix` use karke binaries build hui, `scripts/release-early-pack.sh` reuse hua, aur 960x600 headless GUI screenshot pass hua.

## 2. Key Metrics
- Progress: 100% (6/6 gates met)
- Build profile: `release` [optimized] via cargo in 2m 40s
- Binary sizes:
  - `autocompare` CLI: 1,720,016 bytes (~1.72 MB)
  - `autocompare-gui` GUI: 12,014,752 bytes (~12.01 MB)
- Package tarballs:
  - `autocompare-cli-0.5.0-macos-arm64.tar.gz`: 766,834 bytes (~749 KB)
  - `autocompare-gui-0.5.0-macos-arm64.tar.gz`: 5,124,402 bytes (~4.9 MB)
  - `autocompare-0.5.0-macos-arm64.tar.gz`: 5,885,027 bytes (~5.6 MB)
- GUI Screenshot: 960 x 600 PNG, 9,765 bytes, magic `89 50 4e 47 0d 0a 1a 0a` verified

## 3. Highlights
- ✅ **Build**: Clean-room release build with `CARGO_TARGET_DIR=/tmp/target-agyx-binmix`. Zero source edits in `crates/**`.
- ✅ **Packaging**: `scripts/release-early-pack.sh` reuse karke `dist/` tarballs generate kiye aur shasum sha256 checksums verify kiye.
- ✅ **Smoke**: CLI smoke (`--help`, `--version`, text diff=1, text same=0) aur GUI headless screenshot (`--screenshot`) dono 100% pass.
- ✅ **Extraction**: Fresh `/tmp` directory me tarball extract karke execution verify kiya.

## 4. Blockers
- None. Build aur smoketests bina kisi hurdle ke pass ho gaye.

## 5. Risks / Decisions Needed
- Signing: Binaries ad-hoc linker-signed hain (Gatekeeper standard warning dega bina developer cert ke). Distribution ke liye Apple Developer ID notarization recommend hota hai.
- Arch: macos-arm64 only (Apple Silicon native).

## 6. Check Ledger (6/6 Gates Met)
- G1: Release build CLI+GUI — exit 0, binary present
- G2: Dist tarballs & sha256 — exit 0, checksums matched
- G3: CLI smoke parity — exit 0, diff=1, same=0
- G4: GUI headless screenshot — exit 0, 960x600 PNG magic verified
- G5: Clean extract test — exit 0, bin runs in /tmp
- G6: Verification report — complete

RESULT: PASS
