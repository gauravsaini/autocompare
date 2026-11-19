# BS-BIN binary rebuild — REPORT

Wave: BS-BIN (early feedback macOS binary). Sirf verify + rebuild, koi Rust change nahi.

## 1. Kya kiya

- `scripts/install-macos.sh` verify kiya (`sh -n` OK) — script me koi change nahi chahiye tha, isliye file untouched hai.
- Fresh rebuild chalaya: `bash scripts/install-macos.sh` (cargo build --release + in-script smoke + tarball).
- VERSION bump nahi kiya — Cargo version abhi bhi `0.5.0` hai, sirf `dist/VERSION` stamp purane SHA (`980e71d`) se HEAD (`7086d4f`) par refresh hua.

## 2. Binary size

- `target/release/autocompare-gui`: 11,831,632 bytes (~11.3 MiB), arm64.
- `dist/autocompare-gui-0.5.0-macos-arm64.tar.gz`: 5,040,664 bytes (~4.8 MiB), andar binary + VERSION.
- `dist/` gitignored hai, isliye tarball commit me nahi hai — ye local fresh artifact hai.

## 3. Install steps (user ke liye)

```sh
scripts/install-macos.sh
# tarball: dist/autocompare-gui-<version>-macos-<arch>.tar.gz (binary + VERSION)
tar -xzf dist/autocompare-gui-*.tar.gz -C /tmp/ac && /tmp/ac/autocompare-gui
scripts/install-macos.sh --install   # PREFIX me copy (default $HOME/.local/bin)
```

README ka install section (lines 22-29) pehle se sahi hai — script path aur tarball pattern match karte hain, isliye README ko touch nahi kiya.

## 4. Smoke proof

- Script ka inbuilt smoke (`--screenshot` offscreen PNG, non-empty check) rebuild ke andar pass hua.
- Independent smoke: `./target/release/autocompare-gui --screenshot /tmp/bs-smoke.png` → `/tmp/bs-smoke.png: PNG image data, 960 x 600, 8-bit/color RGBA` (9695 bytes). Valid PNG, exit 0, koi window nahi khuli.
- Note: `--help` is binary par kaam nahi karta (hang hota hai, SIGTERM 143) — smoke ke liye `--screenshot` hi supported rasta hai, jo script aur README dono me documented hai.

## 5. Untouched proof (gates)

- `cargo test -p autocompare-gui`: 611 passed, 0 failed — crates me koi change nahi.
- `git status`: sirf doosri waves ki untracked dirs dikhi (`checks/gui-foldergrid-br/`, `checks/gui-typo-bs/`), unko touch nahi kiya.
- `checks/opencode-waves/STATUS.md`: untouched. `skills/`, `crates/**`: untouched.
