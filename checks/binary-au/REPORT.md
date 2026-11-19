# REPORT — binary distribution readiness (wave AU-BINARY, early feedback)

Date: 2026-11-20 · Branch: `main` · Repo: autocompare · Scope: `checks/binary-au/**` only (source read-only, no code touched)

## 1. Release-build result: SUCCESS (no error to report)

```text
$ cargo build --release
   Compiling autocompare-cli v0.5.0 (/Users/ektasaini/autocompare/crates/cli)
   Compiling autocompare-gui v0.5.0 (/Users/ektasaini/autocompare/crates/gui)
    Finished `release` profile [optimized] target(s) in 55.80s
```

Zero errors, zero warnings surfaced by `tail -30`. Environment:
`rustc 1.99.1` (Homebrew), macOS 27.2.1, arm64. Artifacts:

- `target/release/autocompare` — 1,592,768 bytes (~1.5M), Mach-O arm64. Smoke OK: `--version` → `autocompare 0.5.0`, `--help` lists subcommands.
- `target/release/autocompare-gui` — 11,438,080 bytes (~11M), Mach-O arm64. **Not launch-tested** (windowed app; launching would pop a window on this machine — left for a manual/GUI wave).

## 2. What binary distribution needs (checklist for dev waves)

1. **Developer ID signing.** Today both binaries are ad-hoc/linker-signed only:
   `Signature=adhoc`, `TeamIdentifier=not set` (`codesign -dv` verified).
   Needed: `codesign --sign "Developer ID Application: <Team>" --options runtime --timestamp <binary>` (hardened runtime for notarization).
2. **Notarization + stapling.** `spctl -a target/release/autocompare` → **`rejected`**. Any quarantined download will be Gatekeeper-blocked ("cannot be opened"). Needed: `ditto -c -k` zip → `xcrun notarytool submit` → `xcrun stapler staple`. Requires paid Apple Developer membership — confirm who owns the Team/cert.
3. **`.app` bundle for the GUI.** Currently a bare Mach-O; Finder double-click opens Terminal. Needed: `autocompare-gui.app/Contents/{Info.plist,MacOS/,Resources/}` + icon (`.icns`). Decide bundle id (e.g. `com.<org>.autocompare-gui`) — affects signing/notarization.
4. **Universal binary (Intel).** Current output is arm64-only. If Intel Macs are in scope: `rustup target add x86_64-apple-darwin`, build both, `lipo -create`. GUI ~11M/arm64 will roughly double.
5. **CLI distribution format.** CLI is self-contained (only `libiconv` + `libSystem`, `otool -L` verified) — a signed zip/tarball or `cargo install`-style script suffices; consider `cargo-dist`/Homebrew formula later.
6. **Linux GUI deps note.** GUI on Linux needs X11/Wayland dev libs at build time (list in `INSTALL.md`); dynamic-link check (`ldd`) on a Linux release build still to be done — not covered in this mac-only wave.

## 3. Known blockers (no source fix attempted — dev waves own fixes)

| # | Blocker | Evidence |
|---|---|---|
| 1 | Gatekeeper rejects unsigned binaries (`spctl -a` → `rejected`) | verified this wave; no cert exists in this environment |
| 2 | No `.app` wrapper / `Info.plist` / icon for the GUI | repo has no bundling config (`cargo-bundle`/`tauri-bundler` absent) |
| 3 | No universal (x86_64) build setup | single-target `cargo build --release` only |
| 4 | GUI has no CLI surface: `--help` blocks on the event loop (120s timeout, had to kill) — automation/smoke scripts must launch it bare, not probe flags | observed this wave |
| 5 | GUI runtime launch untested headless (windowed eframe app) | deliberately not launched; needs manual open test |

## 4. Early-feedback verdict

Binary story is **healthy at the build layer** (clean release build, small self-contained CLI, reasonable 11M GUI) but **not distributable yet**: signing identity + notarization + `.app` bundling are all missing, and Gatekeeper rejection is confirmed, not theoretical. Recommend the dev waves pick up items 1–3 of §2 in that order.
