# REPORT — binary-4 early-feedback (release CLI+GUI tarball, macos-arm64)

Date: 2026-11-20 · HEAD: `1b0daa7` · version `0.5.0` · Scope: `checks/binary-4/**` only (crates/** me zero edit, sirf build+package+verify)

## 1. Build

```text
$ CARGO_TARGET_DIR=target/agent-binary4 cargo build --release -p autocompare-cli -p autocompare-gui
Finished `release` profile [optimized] target(s) in 4m 04s
```

Env: `rustc 1.99.1` (Homebrew), darwin arm64. Isolated target `target/agent-binary4` (default `target/release` untouched). No source edits. No strip step.

Artifacts (Mach-O 64-bit arm64, dono):

- `target/agent-binary4/release/autocompare` — 1719568 bytes
- `target/agent-binary4/release/autocompare-gui` — 12014992 bytes

## 2. Package

Tarball (staged copy se, absolute path nahi):

```text
$ tar -czf checks/binary-4/autocompare-0.5.0-macos-arm64.tar.gz -C /tmp/ac-b4/stage autocompare autocompare-gui
```

- `checks/binary-4/autocompare-0.5.0-macos-arm64.tar.gz` — 5881325 bytes
- sha256: `6f2c346be9dad326027853e689a44f34d0feafe9477b8e06422566eccfcb9040`
- sha file: `checks/binary-4/autocompare-0.5.0-macos-arm64.tar.gz.sha256` (`shasum -a 256 -c` OK)
- andar: `autocompare`, `autocompare-gui` (tar -tzf verified)

## 3. Verify 6/6 — PASS

| # | Check | Evidence |
|---|---|---|
| 1 | CLI --help exit 0 | `HELP_EXIT=0`, output `checks/binary-4/cli-help.out` (3230 B, subcommands list) |
| 2 | CLI text smoke | diff files -> exit 1, same files -> exit 0 (`TEXT_DIFF_EXIT=1`, `TEXT_SAME_EXIT=0`); outputs `cli-text-diff.out` / `cli-text-same.out` |
| 3 | GUI PNG 960x600 valid | `GUI_EXIT=0`, `checks/binary-4/gui-smoke.png` 9765 bytes, magic `89504e470d0a1a0a`, IHDR 960x600, `file` = PNG image data 960x600 8-bit RGBA |
| 4 | Tarball extracts | `EXTRACT_EXIT=0`, `/tmp/ac-b4/extract` me dono binaries, extracted `autocompare --help` exit 0 |
| 5 | sha matches | `shasum -a 256 -c ...sha256` -> `OK`, `SHA_MATCH=1` |
| 6 | Version string present | `autocompare --version` -> `autocompare 0.5.0` (exit 0); `strings` me `0.5.0` maujood |

## 4. Caveats

1. Ad-hoc sign: dono binaries `adhoc,linker-signed` hain (`codesign -dv` verified, TeamIdentifier not set). Quarantine lagi download Gatekeeper reject karega — chahiye Developer-ID signing + notarization. Is wave me sign nahi kiya.
2. Sirf macOS arm64 hai; x86_64/universal nahi. GUI bare Mach-O hai, `.app` bundle nahi.
3. Working tree me dusri waves ke uncommitted `crates/**` edits hain — is wave ne unko touch nahi kiya; ye build unhi edits ke saath hua, pristine-HEAD guarantee nahi.
4. Tarball `checks/binary-4/` ke andar hai (owns-only constraint); `dist/` ko haath nahi lagaya.

## 5. Files (is dir me)

- `REPORT.md` (ye file), `gui-smoke.png`, `cli-help.out`, `cli-text-diff.out`, `cli-text-same.out`, `gui.out`
- `autocompare-0.5.0-macos-arm64.tar.gz` + `.sha256`

RESULT: PASS
