# GATES — gui-harness-ax (offscreen fixtures live-contract sync)

Wave: AX-HARNESS. OWNS: `crates/gui/src/screenshot.rs` + `checks/gui-harness-ax/**` ONLY.
Branch `main`, fresh pull at start. `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).

## Gate results (2026-11-20 UTC)

| # | Gate | Command | Result |
|---|---|---|---|
| 1 | Unit tests (full crate) | `cargo test -p autocompare-gui` | PASS — 547 passed, 0 failed (screenshot 24/24, 8 new) |
| 2 | Harness round-1 | `zsh tools/gui-polish/run-geo-typo.sh checks/gui-harness-ax` | PASS — exit 0, `RESULT: PASS` |
| 3 | PNG valid (all 11) | `file` + magic + size + distinct | PASS — 960x600 RGBA, magic `89504e470d0a1a0a` ok, all ≥8192 B, 256 distinct bytes |
| 4 | Clippy, OWNS file | `cargo clippy -p autocompare-gui` and `--all-targets` | PASS — exit 0, zero `screenshot.rs` mentions in both profiles |
| 5 | fmt, OWNS file | `rustfmt --edition 2024 --check crates/gui/src/screenshot.rs` | PASS — clean |
| 6 | Banned patterns, non-test code | grep `unwrap(`/`expect(`/`panic!`/`todo!` on `screenshot.rs` minus `#[cfg(test)]`, `forbid(unsafe_code)` present | PASS — 0 hits |
| 7 | OWNS discipline | `git status --short` delta | PASS — only `crates/gui/src/screenshot.rs` modified + `checks/gui-harness-ax/**` added; `views.rs`/`views_table.rs`/`home.rs`/`gutter.rs`/`tree.rs`/`app.rs` untouched (read-only) |
| 8 | Colors frozen | `git diff` on `screenshot.rs` | PASS — zero new hues; only existing palette consts reused (`BG_*`, `FG_*`, `*_BG`, `ACCENT_TAB`) |

## Byte deltas vs AW2 (proof the sync landed)

| file | AW2 bytes | AX bytes | same? |
|---|---|---|---|
| home.png | 9011 | 14880 | NAHI (9-card grid, CTA 150x28, columns, no picker) |
| text.png | 9624 | 9627 | NAHI (ruler 16px, min-h 4px) |
| folder.png | 10068 | 10068 | haan (fixture unchanged by design) |
| table.png | 9973 | 9973 | haan (fixture unchanged by design) |
| hex.png | 10030 | 9999 | NAHI (shared ruler consts: gutter x936 w16) |
| picture.png | 9624 (= text fallback) | 8321 | NAHI (ab asli picture fixture) |
| wrap/table-find/folder-empty | — (no route) | 10455 / 8871 / 10081 | naye scenes |

## Known non-failures / deferrals (no jabardasti)

- Hex minimap parity (live `HEX_MINIMAP_W` 24px): fixture hex gutter ab ruler-width (16) par hai; poori minimap geometry paint-owner wave ka kaam — REPORT §8.
- Table grid key-col highlight: fixture key bar + badge + underline spans prove karta hai; Grid cell-level accent paint-owner wave ka — REPORT §7.
- Folder filtered-empty vs never-compared-empty ek hi idle hint share karte hain (live contract me teesra variant nahi) — REPORT §6.
- Pixel-font me bold/size/●○ nahi: title/desc hierarchy tone + rhythm se, markers `[x]`/`[ ]` ascii me, REPORT me document.
