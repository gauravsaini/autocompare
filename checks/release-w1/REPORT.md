# release-w1 REPORT (Hinglish-Roman)

Wave: release-verify, branch `main`. Push/pull kuch nahi kiya. Sirf apne paths likhe: `xtask/**` (padha, change nahi), `dist/RELEASE-DRAFT-v0.6.0.md` (naya), `dist/VERSION` (sirf read), `checks/release-w1/**` (naya). `xtask` me koi code change nahi — gate fix ki zaroorat nahi padi, saare gates pass the, gaps docs me likh diye.

## Kya padha

- `xtask/src/main.rs`: subcommands `layers` (L0-L3 layering + egui/eframe/rfd sirf gui me), `secrets` (private-key/`ghp_`/`github_pat_`/`AKIA` scan, `.png/.wav/.bin/.bak/target` skip, khud ko skip), `wasm` (sirf `autocompare-evidence`, diff missing — gap), `check-parity` (hardcoded `tools/parity-harness/run-round.sh` + round-0 path), `ci` (layers+secrets+wasm, parity nahi).
- `Cargo.toml` release profile: `lto = true`, `codegen-units = 1`. Version `0.5.0`, `dist/VERSION` = `0.5.0 a6bb8e87ea9a92239c90cd3922baa096a2423fa0` (change nahi kiya).
- `scripts/release-early-pack.sh` + `scripts/install-macos.sh`: dono `cargo build --release` + smoke + `tar -czf binary + VERSION` karte hain, par sha256 generate nahi karte. Actual `dist/` me cli/gui tarballs me VERSION bhi nahi hai — script vs artifact mismatch (gap #2).

## Kya chalaya (CARGO_BUILD_JOBS=2, release sirf ek baar)

1. `cargo test -p xtask` — 3 passed.
2. `cargo run -p xtask -- layers` — `layers: ok`.
3. `cargo run -p xtask -- secrets` — `secrets: ok`.
4. `cargo run -p xtask -- wasm` — `wasm: ok` (sirf evidence; diff ka alag check fail hota hai — gap #1).
5. `cargo run -p xtask -- check-parity` — `RESULT: PASS`, 10/10 cases (text/folder/hex/merge/table/sync/picture/folder-merge/audio/text-same).
6. `cargo run -p xtask -- ci` — `ci: ok`.
7. Debug `cargo build -p autocompare-cli -p autocompare-gui` — ok (logic verify).
8. Release `cargo build --release -p autocompare-cli -p autocompare-gui` — ek baar, ~1m11s, ok. Binary: `autocompare` 1.6M, `autocompare-gui` 11M.
9. Smoke: `autocompare --help` ok; `autocompare-gui --screenshot /tmp/x.png` ok, PNG 960x600 ~9.5K, `file` = PNG image data.
10. Tarball dry-run `/tmp/ac-release-w1-pack` me (dist ko touch nahi): binary + VERSION stamp pack kiya, `tar -tzvf` me dono dikhe, `shasum -a 256` se SHA256SUMS banaya. Cli tarball ~740K, gui ~4.9M.

## Gaps (detail `dist/RELEASE-DRAFT-v0.6.0.md` me)

1. wasm gate adhura (diff missing, diff wasm me std error).
2. dist inconsistent (cli/gui me VERSION nahi, sha256 sidecar nahi).
3. pack logic xtask me nahi, 2 scripts me duplicate, sha256 kahin nahi.
4. parity hardcoded path, ci me shamil nahi.
5. VERSION overwrite policy unclear (dirty vs pinned build).
6. verify-*.mjs me absolute path hardcoded.
7. Sirf arm64 verified, x86_64 pending.

## Files

- `dist/RELEASE-DRAFT-v0.6.0.md` — naya, v0.5.0 wali ko haath nahi lagaya.
- `checks/release-w1/REPORT.md` (ye file) + `checks/release-w1/GATES.md`.
- Commit pathspec-only hoga, `git add -A` nahi, push nahi.
