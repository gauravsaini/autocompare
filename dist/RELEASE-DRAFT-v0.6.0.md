# GitHub Release DRAFT — v0.6.0 (release pipeline verify, wave-1)

> Orchestrator ke liye: `gh release create v0.6.0 --draft --title "v0.6.0" --notes-file dist/RELEASE-DRAFT-v0.6.0.md dist/*.tar.gz dist/*.sha256`
> Attach: `dist/autocompare-cli-<version>-macos-<arch>.tar.gz` + `dist/autocompare-gui-<version>-macos-<arch>.tar.gz` + har tarball ka `.sha256` sidecar.
> Evidence: `checks/release-w1/REPORT.md` + `checks/release-w1/GATES.md`. Is wave me `dist/VERSION` change NAHI hua (read-only, abhi bhi `0.5.0 a6bb8e87ea9a92239c90cd3922baa096a2423fa0`).

## v0.6.0 — installable binaries (macOS)

Repo se install ki zaroorat nahi — neeche tarballs download karo, sha256 verify karo.

### Install (macOS, no cargo needed)

```sh
shasum -a 256 -c autocompare-cli-<version>-macos-<arch>.tar.gz.sha256
shasum -a 256 -c autocompare-gui-<version>-macos-<arch>.tar.gz.sha256
tar -xzf autocompare-gui-<version>-macos-<arch>.tar.gz
./autocompare-gui --screenshot /tmp/ac-smoke.png   # smoke: PNG banna chahiye (960x600, ~9KB)
tar -xzf autocompare-cli-<version>-macos-<arch>.tar.gz
./autocompare text <left> <right>                  # exit 0 = same, exit 1 = diff mila
```

### Is drop me kya test karein

1. CLI diff-conventions: text / folder / hex / merge / table / sync / picture / folder-merge / audio — exit codes `0` same, `1` diff.
2. GUI headless screenshot: `--screenshot out.png` valid PNG deta hai ya nahi.
3. Tarball layout: har tarball me binary + `VERSION` (`<version> <sha>` ek line) hona chahiye.
4. sha256 sidecar har tarball ke saath hona chahiye.

## Release pipeline (verified is wave me)

Profile (`Cargo.toml` `[profile.release]`): `lto = true`, `codegen-units = 1`.
Env: `CARGO_BUILD_JOBS=2`.

```sh
CARGO_BUILD_JOBS=2 cargo test -p xtask                      # 3 passed
CARGO_BUILD_JOBS=2 cargo run -p xtask -- layers             # ok
CARGO_BUILD_JOBS=2 cargo run -p xtask -- secrets            # ok
CARGO_BUILD_JOBS=2 cargo run -p xtask -- wasm               # ok (sirf evidence — gap #1 dekho)
CARGO_BUILD_JOBS=2 cargo run -p xtask -- check-parity       # RESULT: PASS (10/10 cases)
CARGO_BUILD_JOBS=2 cargo run -p xtask -- ci                 # ok
cargo build -p autocompare-cli -p autocompare-gui           # debug, logic verify — ok
CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-cli -p autocompare-gui  # ek baar, ~1m11s — ok
./target/release/autocompare --help                         # ok
./target/release/autocompare-gui --screenshot /tmp/x.png    # ok, PNG 960x600
```

Tarballs + sha256 (canonical process — `dist/` ko touch kiye bina `/tmp` me dry-run verified):

```sh
VERSION=$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)
SHA=$(git rev-parse --short HEAD); ARCH=$(uname -m)
printf '%s %s\n' "$VERSION" "$SHA" > VERSION.stamp
tar -czf "autocompare-cli-$VERSION-macos-$ARCH.tar.gz" -C target/release autocompare -C <stampdir> VERSION
tar -czf "autocompare-gui-$VERSION-macos-$ARCH.tar.gz" -C target/release autocompare-gui -C <stampdir> VERSION
shasum -a 256 autocompare-*.tar.gz > SHA256SUMS   # ya har tarball ke liye alag .sha256 sidecar
tar -tzvf <tarball>                               # binary + VERSION dono dikhne chahiye
```

Is wave ke dry-run sizes (HEAD `f071309` + dirty tree, arm64): cli tarball ~740K (binary 1.6M), gui tarball ~4.9M (binary 11M).

## Gaps (v0.6.0 se pehle fix karna hai)

1. `xtask wasm` sirf `autocompare-evidence` check karta hai; L0 me `autocompare-diff` bhi hai (`xtask/src/main.rs:275`). `cargo check --target wasm32-unknown-unknown -p autocompare-diff` is env me `std` crate error deta hai. Ya to diff ko wasm-clean karo, ya gate ka scope docs me saaf likho.
2. `dist/` artifacts inconsistent hain: `autocompare-cli/gui-0.5.0` tarballs me `VERSION` file NAHI hai (jabki `scripts/release-early-pack.sh:28-31` binary + VERSION pack karta hai); cli/gui ke liye koi `.sha256` sidecar nahi hai (sirf combined tarball ke liye hai). Agli release me har tarball = binary + VERSION, har tarball + `.sha256`.
3. Pack logic `xtask` me nahi hai — `scripts/release-early-pack.sh` aur `scripts/install-macos.sh` me duplicate hai, aur dono me sha256 generation nahi hai. Canonical fix: `xtask dist` (ya `xtask pack`) banao jo build + smoke + tar + sha256 ek jagah kare.
4. `xtask check-parity` path hardcoded hai (`.unlazy/v1/evidence/wave-0/round-0`), aur `xtask ci` parity ko call nahi karta (`main.rs:326`). Release gate me parity shamil karna hai ya nahi, ye decide karo.
5. `scripts/release-early-pack.sh` aur `install-macos.sh` `dist/VERSION` overwrite karte hain — dirty-tree vs pinned-commit ambiguity (v0.5.0 me pristine `/tmp` workcopy use karna pada tha). Release wave ko clean-tree ya `git archive` policy chahiye.
6. `scripts/verify-release-*.mjs` me absolute path (`/Users/ektasaini/autocompare`) hardcoded hai — portable nahi hai.
7. Sirf macOS arm64 verify hua hai; x86_64 tarball abhi pending hai.

## Known caveats

- Is draft ke saath koi naya tarball commit nahi hua — `dist/` me abhi bhi v0.5.0 artifacts hain. v0.6.0 tarballs agli release wave me fresh HEAD se banenge.
- Release build is wave me dirty tree par hua tha (media wave ke uncommitted edits `crates/diff`, `crates/compare` me the) — isliye upar sizes sirf reference hain, release-of-record nahi.
