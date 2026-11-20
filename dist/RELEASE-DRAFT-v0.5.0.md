# GitHub Release DRAFT — v0.5.0-early (installable binary, early feedback)

> Orchestrator ke liye: `gh release create v0.5.0-early --draft --title ... --notes-file dist/RELEASE-DRAFT-v0.5.0.md dist/*.tar.gz`
> Attach: `dist/autocompare-cli-0.5.0-macos-<arch>.tar.gz` + `dist/autocompare-gui-0.5.0-macos-<arch>.tar.gz`
> Pinned commit: a6bb8e8 (proof: checks/bin-early/pin.env). Pristine-HEAD build (/tmp workcopy),
> kyunki repo working tree me dusri waves ke uncommitted edits hain (crates/diff breakage) — BINARY wave crates/** touch nahi karta.

## v0.5.0 early feedback — macOS binaries

Repo se install ki zaroorat nahi — neeche tarballs download karo.

### Install (macOS, no cargo needed)

```sh
tar -xzf autocompare-gui-0.5.0-macos-arm64.tar.gz
./autocompare-gui --screenshot /tmp/ac-smoke.png   # smoke: PNG banna chahiye
# optional: PREFIX=$HOME/.local/bin sh scripts/install-macos.sh --install
tar -xzf autocompare-cli-0.5.0-macos-arm64.tar.gz
./autocompare text <left> <right>                  # exit 1 = diff mila
```

### Is drop me kya test karein (early feedback)

1. CLI diff-conventions: text / folder / hex / merge / table / sync / picture / folder-merge / audio — exit codes `0` same, `1` diff.
2. GUI headless screenshot: `--screenshot out.png` valid PNG deta hai ya nahi.
3. Install path: `scripts/install-macos.sh` bina cargo knowledge ke chalta hai ya nahi.

### Known caveats

- Sirf macOS arm64 tarball is drop me (x86_64 baad me).
- Working tree ke uncommitted wave-edits is build me NAHI hain — pinned commit a6bb8e8 tak ka code hai.
- Feedback `checks/inbox/` ya issue me dein: kya chala, kya toota, exact command + exit code ke saath.
