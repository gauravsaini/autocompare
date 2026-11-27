<p align="center">
  <img src="logo.png" alt="AutoCompare logo" width="48">
</p>

<h1 align="center">AutoCompare</h1>

<p align="center">
  A fast, native <strong>Beyond Compare alternative</strong> built in safe Rust + Tauri v2.<br/>
  Side-by-side text, folder, table, hex, 3-way merge, folder sync, and media comparison — on desktop and CLI.
</p>

---

## Highlights

- **Native Desktop App (Tauri v2 + React 19 + Tailwind v4)** — Lightweight macOS/cross-platform UI powered directly by the Rust comparison engine, with 60fps virtualized scrolling, light/dark themes, tabbed sessions, and command palette (`Cmd+K`).
- **Pure, Safe Rust Engine** — `#![forbid(unsafe_code)]`, clippy-enforced panic-free production code (`unwrap`/`expect`/indexing panics denied), paged IPC windows so huge files never stall the UI, and symlink-safe directory walks.
- **Scriptable Headless CLI** — Every comparison mode is also available from the terminal with standard `diff(1)` exit codes (`0` equal, `1` differences/conflicts, `2` error) and HTML/Text/XML/Patch report export.

## Desktop Comparison Modes

| Mode | Key Capabilities |
| :--- | :--- |
| **Text Compare** | Continuous notepad-style inline editing (`Enter` split, `Backspace`/`Delete` join, multi-line paste, live re-diff), per-line & `Shift+Click` range merge, backend-synced `Cmd+Z` Undo/Redo, `Cmd+F` / `Cmd+H` Find & Replace, syntax highlighting (`Source Code`, `JSON`, `YAML`, `XML/HTML`, `Markdown`, `CSV`), blue unimportant-diff highlighting (`Minor` toggle), manual line alignment (`F7` / `Align With…`), external file-change reload prompt, and a 5-tab Beyond Compare–style Rules dialog (`Specs`, `Format` with per-side encoding overrides, `Importance` with regex grammar presets, `Alignment` with Unaligned/Standard/Myers/Patience/`Never align differences`, and `Replacements`). |
| **Folder Compare** | Synchronized dual-pane virtualized directory tree, Quick (size + timestamp tolerance) vs Binary byte-for-byte comparison, semicolon-separated `Name Filters` (`*.rs; -target/*; -node_modules/*`), and double-click to open file pairs in Text Compare. |
| **Table Compare** | Virtualized CSV/TSV grid with key-column alignment, custom delimiters (`,`, `\t`, `;`, `|`), case/whitespace ignore, and numeric tolerance (`±0.01`), configured via the tabbed Table Rules dialog (`Columns`, `Format`, `Importance`). |
| **Hex Compare** | Paged binary viewer + ASCII pane with virtualized scrolling, hex byte search, inline byte-level editing, and disk persistence (`hex_apply_edit` + `hex_save_side`). |
| **3-Way Text Merge** | Synchronized 3-pane conflict cards (`Left \| Base \| Right`), keyboard resolution (`Alt+1/2/3/4`, `Alt+N/P`), per-conflict inline custom editor, bulk `Take All Left/Right`, and editable 4th output pane. |
| **3-Way Folder Merge** | Tree-level 3-way merge plan with per-row conflict overrides and double-click into 3-Way Text Merge. |
| **Folder Sync** | Preview-first one-way and mirror sync plans (`dry_run`) with apply execution and failure reporting. |
| **Media Compare** | Side-by-side PNG pixel comparison with difference heatmaps and WAV/MP3 audio window comparison. |

## Quick Start (Development)

### Prerequisites
- **Rust** (stable toolchain) + `cargo-tauri` (`cargo install tauri-cli`)
- **Node.js** `20+` (verified on Node `26.x`)

### Run the Desktop App (Dev)
```sh
cd apps/desktop
npm install
npm run dev
```

### Run Tests & Build
```sh
# Frontend unit tests (zero-dep Node test runner) & production bundle
cd apps/desktop
npm test
npm run build

# Rust workspace tests (400+ engine & IPC tests)
cargo test --workspace

# Playwright E2E specs (v2 living suite)
cd apps/desktop
npm run test:e2e
```

## Build macOS `.dmg` Installer

Create a clean, compressed `AutoCompare-<version>-macos-<arch>.dmg` with `/Applications` drag-to-install symlink, ad-hoc (or Developer ID) signing, and SHA-256 checksum:

```sh
# Native architecture (Apple Silicon on M-series Macs)
scripts/build-macos-dmg.sh

# Or via npm script inside apps/desktop
cd apps/desktop && npm run package:dmg

# Optional flags:
scripts/build-macos-dmg.sh --arch universal --open
scripts/build-macos-dmg.sh --arch aarch64 --skip-tests
```

Output lands in `dist/AutoCompare-<version>-macos-<arch>.dmg` alongside `dist/AutoCompare-<version>-macos-<arch>.dmg.sha256`.
To sign with an Apple Developer ID certificate, export `APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"` before running the script.

## CLI Usage

Build the standalone CLI binary (`target/release/autocompare`):

```sh
cargo build --release -p autocompare-cli
```

Common commands:

```sh
# Side-by-side or unified text diff
autocompare text old.txt new.txt [--unified] [--ignore-case] [--ignore-space-change]

# Recursive folder comparison
autocompare folder left/ right/ [--content] [--include "*.rs"] [--exclude "target/*"]

# Byte-level hex comparison
autocompare hex bin_a.bin bin_b.bin

# CSV / delimited table comparison
autocompare table old.csv new.csv [--delimiter ";"]

# 3-way text merge & 3-way folder merge
autocompare merge base.txt left.txt right.txt --output merged.txt
autocompare folder-merge base/ left/ right/ --apply --output merged/

# Folder sync (dry-run by default; pass --apply to execute)
autocompare sync left/ right/ --direction l2r [--apply]

# Picture (PNG) and Audio (WAV/MP3) comparison
autocompare picture left.png right.png --heatmap diff.png
autocompare audio a.wav b.wav

# Export HTML, Text, XML, or Patch reports from any subcommand
autocompare text left.txt right.txt --report html:report.html
```

### Exit Codes
`0` no differences · `1` differences found (or merge conflicts) · `2` error.

## Architecture

- **`crates/diff`** — Pure diff & merge algorithms (Myers, Patience, inline word/char spans, 3-way merge).
- **`crates/compare`** — Filesystem, text, table, binary/hex, folder sync, 3-way folder merge, PNG picture, and WAV/MP3 audio engines.
- **`crates/cli`** — Thin, scriptable terminal renderer and report generator.
- **`apps/desktop/src-tauri`** — Tauri v2 IPC command layer with paged row/tree/hex/table session caches.
- **`apps/desktop/src/v2`** — Clean-slate React 19 + Tailwind v4 desktop interface.

## License

MIT — see [LICENSE](LICENSE).
