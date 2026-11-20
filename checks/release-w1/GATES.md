# release-w1 GATES

| Gate | Result | Evidence |
|------|--------|----------|
| xtask test (`cargo test -p xtask`) | PASS | 3 passed (layer-mapping, package-name, dependencies) |
| xtask layers | PASS | `layers: ok` |
| xtask secrets | PASS | `secrets: ok` |
| xtask wasm (evidence only) | PASS (scope-gap) | `wasm: ok`; note: diff L0 check nahi hota — `cargo check --target wasm32-unknown-unknown -p autocompare-diff` me `std` error (gap, is wave me fix nahi) |
| xtask check-parity | PASS | `RESULT: PASS`, 10/10 (text/folder/hex/merge/table/sync/picture/folder-merge/audio/text-same) |
| xtask ci | PASS | `ci: ok` (layers+secrets+wasm; parity shamil nahi — gap) |
| debug build cli+gui | PASS | `cargo build -p autocompare-cli -p autocompare-gui` ok |
| release build cli+gui (ek baar) | PASS | `CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-cli -p autocompare-gui`, ~1m11s, binaries 1.6M + 11M |
| cli smoke `--help` | PASS | usage + 10 commands dikhe, exit 0 |
| gui smoke `--screenshot` | PASS | exit 0, PNG 960x600 ~9.5K |
| tarball+sha256 dry-run | PASS (process) | `/tmp/ac-release-w1-pack` me binary+VERSION pack, `tar -tzvf` ok, `shasum -a 256` ok; `dist/` ko touch nahi kiya |
| dist artifacts consistency | FAIL (gap-doc) | cli/gui tarballs me VERSION nahi, `.sha256` sidecar nahi; scripts me sha256 generation nahi |
| dist/VERSION unchanged | PASS | read-only rakha, abhi bhi `0.5.0 a6bb8e87ea9a92239c90cd3922baa096a2423fa0` |
| scope discipline | PASS | sirf `xtask/**` (read), `dist/RELEASE-DRAFT-v0.6.0.md` (naya), `checks/release-w1/**` (naya); `dist/VERSION` sirf read; version bump nahi |

Verdict: release pipeline verify ho gaya, gates pass; tarball/sha256 consistency gaps v0.6.0 se pehle fix karne hain (draft me 7 gaps listed). `xtask` me is wave me koi code change nahi.
