# INSTALL — autocompare binaries (draft, early feedback)

> Scope: building + running from source on macOS. For distribution
> readiness (signing, notarization, blockers) see `REPORT.md` in this folder.

## Binaries produced by `cargo build --release`

| Binary | Path | Size | Type |
|---|---|---|---|
| `autocompare` (CLI) | `target/release/autocompare` | 1,592,768 bytes (~1.5M) | Mach-O 64-bit arm64 |
| `autocompare-gui` (desktop) | `target/release/autocompare-gui` | 11,438,080 bytes (~11M) | Mach-O 64-bit arm64 |

Measured 2026-11-20 on Apple Silicon (arm64), macOS 27.2.1,
`rustc 1.99.1` (Homebrew). Release profile uses `lto = true`,
`codegen-units = 1` (see workspace `Cargo.toml`).

## Prerequisites

- Rust stable toolchain (rustup recommended; Homebrew `rust` also works —
  this build used Homebrew rustc 1.99.1).
- macOS: Xcode Command Line Tools (`xcode-select --install`) for the
  system linker + AppKit/Metal frameworks the GUI links against.
- Linux (for reference, from `README.md` — GUI only): the CI set
  `libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libx11-dev
  libxrandr-dev libxi-dev libgl1-mesa-dev`. The CLI needs nothing extra.
- Disk: ~a few GB for `target/` during the first release build
  (dependency codegen + LTO artifacts).

## Build steps

```sh
git clone <repo-url> autocompare && cd autocompare
cargo build --release
```

- Result: `target/release/autocompare` and `target/release/autocompare-gui`.
- Reference build took **55.80s** on this machine (warm registry, cold
  `target/` LTO link dominates).
- To build only the CLI (faster, no GUI deps):
  `cargo build --release -p autocompare-cli`.

## Run steps

```sh
./target/release/autocompare --version   # -> autocompare 0.5.0
./target/release/autocompare --help      # subcommands: text folder hex merge table sync picture folder-merge audio ...
./target/release/autocompare text left.txt right.txt
```

GUI:

```sh
./target/release/autocompare-gui
```

Note: the GUI is a windowing (eframe) app — it does **not** implement
`--help`/CLI flags. Running it with `--help` blocks on the event loop
instead of exiting (observed: still running after 120s, killed by timeout).
Launch it bare from a graphical session.

Optional: add to `PATH` for the shell session:

```sh
export PATH="$PWD/target/release:$PATH"
```

## macOS notes

- Both binaries link **system frameworks/dylibs only** (`otool -L`
  verified: CLI → `libiconv` + `libSystem`; GUI → AppKit, Metal,
  QuartzCore, Foundation, …). Nothing to bundle via `dylibbundler`.
- Fresh `cargo`-built binaries are **ad-hoc (linker) signed only** —
  on *your own* build machine they run fine, but the same file
  downloaded from the internet gets a quarantine flag and Gatekeeper
  rejects it (`spctl -a` → `rejected`, verified). First-run workaround
  for local testing only:
  `xattr -dr com.apple.quarantine <binary>`.
  Real distribution needs Developer-ID signing + notarization —
  see `REPORT.md`.
- Current binaries are **arm64-only**. Intel Macs would need a
  universal (`lipo`) build or Rosetta 2 — not set up yet (see `REPORT.md`).
- The GUI ships as a bare Mach-O executable, not a `.app` bundle:
  double-clicking in Finder opens Terminal. A proper `.app`
  (`Info.plist`, icon, `Contents/MacOS/`) does not exist yet.
