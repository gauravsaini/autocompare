# REPORT — binary early-2 (installable combined tarball)

Date: 2026-11-20 · Branch: `main` · Scope: `dist/** + checks/binary-early-2/**` only (crates/** me zero edit, sirf build+package+verify)

## 1. Kya kiya (summary)

Binary early install wala goal poora hua: release build dono packages ka,
strip karke ek combined tarball `dist/autocompare-0.5.0-macos-arm64.tar.gz`
+ sha256 banaya, fresh `/tmp/ac-fresh` dir me install test kiya (CLI
--help/version + text/folder smoke + GUI --screenshot PNG), aur install
snippet `checks/binary-early-2/INSTALL.md` me rakha.

## 2. Build evidence

```text
$ CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-cli -p autocompare-gui
   Compiling autocompare-cli v0.5.0 ...
   Compiling autocompare-gui v0.5.0 ...
    Finished `release` profile [optimized] target(s) in 1m 20s
```

Env: `rustc 1.99.1` (Homebrew), macOS 27.2.1, arm64. Unstripped sizes:
CLI 1683984 bytes, GUI 11909376 bytes (dono Mach-O arm64).

## 3. Package evidence

Strip ke baad: CLI 1445432 bytes, GUI 10380384 bytes. Tarball
`dist/autocompare-0.5.0-macos-arm64.tar.gz` = 5504450 bytes, andar dono
binaries (`autocompare`, `autocompare-gui`).

```text
26719972f70ddfbb5a19e9ef3f3d4885489edb98f9fd8bfc4fafc3f4263547eb  dist/autocompare-0.5.0-macos-arm64.tar.gz
```

Verify: `shasum -a 256 -c dist/autocompare-0.5.0-macos-arm64.tar.gz.sha256`
(sha256 file isi commit me hai).

Note: `dist/` me pehle se maujood alag-alag `autocompare-cli-*` /
`autocompare-gui-*` tarballs ko haath nahi lagaya — ye combined tarball
naya addition hai.

## 4. Fresh /tmp install test evidence

```text
$ tar -xzf dist/autocompare-0.5.0-macos-arm64.tar.gz -C /tmp/ac-fresh
$ /tmp/ac-fresh/autocompare --help     -> exit 0 (subcommands list hua)
$ /tmp/ac-fresh/autocompare --version  -> autocompare 0.5.0, exit 0
$ /tmp/ac-fresh/autocompare text left.txt right.txt (diff wali files)   -> exit 1
$ /tmp/ac-fresh/autocompare text same1.txt same2.txt (same files)        -> exit 0
$ /tmp/ac-fresh/autocompare folder fa fb (alag content)                  -> exit 1
$ /tmp/ac-fresh/autocompare-gui --screenshot /tmp/ac-fresh/gui-smoke.png -> exit 0
$ file gui-smoke.png -> PNG image data, 960 x 600, 8-bit/color RGBA (9765 bytes)
```

PNG copy: `checks/binary-early-2/gui-smoke.png`. GUI me `--screenshot`
flag ab kaam karta hai (purani binary-au wave me `--help` block hota tha —
ye fix kisi aur wave ne kiya, is wave ne sirf verify kiya).

## 5. Caveats (distribute karne se pehle baaki kaam)

1. Dono binaries abhi ad-hoc/linker-signed hain — quarantine laga download
   Gatekeeper reject karega. Chahiye: Developer-ID signing + notarization
   (detail purani `checks/binary-au/REPORT.md` me hai).
2. Sirf macOS arm64 tarball hai; Intel (x86_64) universal build nahi.
3. GUI bare Mach-O hai, `.app` bundle nahi — double-click Finder me
   Terminal kholega.
4. Repo working tree me dusri waves ke uncommitted `crates/**` edits hain
   (git status me dikhte hain) — is wave ne unko touch nahi kiya, aur ye
   build unhi edits ke saath hua hai. Pristine-HEAD guarantee nahi de rahe.

## 6. Gates result

`GATES.md` me G1–G6 sab `[x]` — build PASS, tarball+sha256 PASS, CLI smoke
PASS, GUI PNG PASS, snippet+report PASS, commit pathspec sirf
`dist + checks/binary-early-2` (push nahi kiya).

RESULT: PASS
