# INSTALL — autocompare 0.5.0 (macOS arm64, binary early-2)

> Cargo/repo ki zaroorat nahi — neeche tarball se seedha install karo.
> Signing/notarization caveats ke liye `REPORT.md` dekho.

## 1. Download + verify

```sh
tar -xzf autocompare-0.5.0-macos-arm64.tar.gz
shasum -a 256 -c autocompare-0.5.0-macos-arm64.tar.gz.sha256
# expect: 26719972f70ddfbb5a19e9ef3f3d4885489edb98f9fd8bfc4fafc3f4263547eb
```

Tarballs me 2 binaries hain: `autocompare` (CLI) + `autocompare-gui` (desktop).

## 2. Smoke test (fresh dir me)

```sh
./autocompare --version   # -> autocompare 0.5.0
./autocompare --help      # subcommands: text folder hex merge table sync picture folder-merge audio
./autocompare text left.txt right.txt   # exit 0 = same, exit 1 = diff mila
./autocompare folder dirA dirB           # exit 0 = same, exit 1 = diff mila
./autocompare-gui --screenshot /tmp/ac-smoke.png  # PNG banna chahiye (960x600)
file /tmp/ac-smoke.png    # -> PNG image data
```

## 3. PATH me daalo (optional)

```sh
mkdir -p "$HOME/.local/bin"
cp autocompare autocompare-gui "$HOME/.local/bin/"
export PATH="$HOME/.local/bin:$PATH"
```

## 4. Quarantine note (download kiya hua binary)

Browser se download karne par macOS quarantine flag laga sakta hai (Gatekeeper
reject — ad-hoc signed binaries, verify: `spctl -a ./autocompare`).
Local testing ke liye hi:

```sh
xattr -dr com.apple.quarantine autocompare autocompare-gui
```

Real distribution me Developer-ID signing + notarization chahiye — detail
`REPORT.md` § caveats me hai.
