# REPORT — early-feedback-5 round-1 local run (2026-11-20)

Scope: leaf local-run wave 5. OWNS only `checks/early-feedback-5/**`. Zero source edits (`crates/**`, `tools/**`, `skills/**` untouched). Isolated target `CARGO_TARGET_DIR=/tmp/target-op-localrun5`. Headless-first. Push: never.

## 1. Build (isolated target)

- Debug: `CARGO_TARGET_DIR=/tmp/target-op-localrun5 cargo build -p autocompare-cli -p autocompare-gui` — exit 0, 43.52s. Binaries: `debug/autocompare` (9.6MB), `debug/autocompare-gui`.
- Release: `CARGO_TARGET_DIR=/tmp/target-op-localrun5 cargo build --release -p autocompare-cli -p autocompare-gui` — exit 0, 3m53s. Binaries: `release/autocompare` (1.7MB), `release/autocompare-gui` (12MB). Release CLI smoke: `--version` → `autocompare 0.5.0`, `text --help` exit 0.

## 2. CLI — 10 cmds over tools/parity-harness/fixtures

Binary: `/tmp/target-op-localrun5/debug/autocompare`. Convention: exit 1 = different, 0 = same.

| # | Cmd | Exit | Want | Gate |
|---|---|---|---|---|
| 1 | `text t_left.txt t_right.txt` | 1 | 1 | PASS |
| 2 | `text t_same.txt t_same.txt` | 0 | 0 | PASS |
| 3 | `folder folder/left folder/right` | 1 | 1 | PASS |
| 4 | `hex h_left.bin h_right.bin` | 1 | 1 | PASS |
| 5 | `merge m_base.txt m_left.txt m_right.txt` | 1 | 1 | PASS |
| 6 | `table tab_left.csv tab_right.csv` | 1 | 1 | PASS |
| 7 | `sync sync/left sync/right` | 1 | 1 | PASS |
| 8 | `picture pic_left.png pic_right.png` | 1 | 1 | PASS |
| 9 | `folder-merge fmerge-base fmerge-left fmerge-right` | 1 | 1 | PASS |
| 10 | `audio a_left.wav a_right.wav` | 1 | 1 | PASS |

10/10 match convention — PASS.

## 3. GUI — 3 screenshots (default/dark/light) + checks

- `autocompare-gui --screenshot round-1/default.png` — exit 0.
- `autocompare-gui --screenshot round-1/dark.png --screenshot-mode dark` — exit 0.
- `autocompare-gui --screenshot round-1/light.png --screenshot-mode light` — exit 0.

| File | Bytes | Magic | Size | Sha256 (short) | Note |
|---|---|---|---|---|---|
| default.png | 9765 | `89504e47…` ok | 960x600 | `bde8ccf4…` | == dark, == old `gui.png` (BYTE-SAME) |
| dark.png | 9765 | ok | 960x600 | `bde8ccf4…` | default mode is dark |
| light.png | 9728 | ok | 960x600 | `7c7f8c8e…` | differs from dark (expected) |

Python struct check: width/height parsed from PNG IHDR for all three — PASS. Determinism: debug-rebuild render is byte-identical to committed `checks/early-feedback-5/gui.png` — zero drift, PASS.

## 4. Release tarball (record only, dist/ NOT committed)

- `dist/autocompare-0.5.0-macos-arm64.tar.gz` (5890587 bytes, contains `autocompare` + `autocompare-gui` + `VERSION`), sha256 `49785edbf5d3982b52425978f297e7724189dd492d73b95324747f0b56817dde`.
- Recorded here only. `git add` pathspec excludes `dist/` — working-tree tarball stays uncommitted.

## 5. TOP-10 geometry/typography gaps vs /tmp/bc-screen.png

Reference: `/tmp/bc-screen.png` (2880x1800 full-desktop capture; right window is the compare app showing a 3-line text pair with tab bar, action toolbar, path bars, metadata strip, twin panes, center gutter, status bar). Ours: `round-1/default.png` (960x600 fixture render). Outside-observe only: no window click, no AX dump, no string/asset copy. Dimensions below are layout/typo only — no color deltas.

1. Session tab row missing (width/alignment) — ref has a multi-tab row with per-tab close boxes above the toolbar; ours has one app-tab strip and no per-session tabs. Anchor: `crates/gui/src/views.rs:898` (`HEADER_H` tab bar sizing).
2. Action toolbar too small (width/height/padding) — ref has a two-row labeled action cluster (session actions + jump/merge arrows); ours is a single row with two actions only. Anchor: `crates/gui/src/views.rs:429` (`session_toolbar_button_size`, 96x24 floor) + `views.rs:456-462` (swap/reload labels).
3. No prev/next difference jump cluster (spacing/alignment) — ref shows dedicated jump arrows under the toolbar; ours has no jump control or position label. Anchor: `crates/gui/src/views.rs:467-469` (toolbar wiring; only swap/reload call-sites live).
4. Path bars lack dropdown + metadata strip (height/margin) — ref has path field with dropdown affordance plus a second metadata line per pane (timestamp/byte-count/format); ours is path field + browse button, single row. Anchor: `crates/gui/src/views.rs:423-429` (toolbar button/path sizing).
5. Center gutter has no per-hunk action (width/alignment) — ref center gap carries small directional markers per hunk; ours is a plain splitter with fixed hit width. Anchor: `crates/gui/src/views.rs:95-106` (`SPLIT_HANDLE_W`, `SPLIT_HIT_W`) + `views.rs:122` (`TEXT_CENTER_GUTTER_W`).
6. Blank-fill alignment missing (height/spacing) — ref pads the shorter pane with a hatched blank region so rows stay aligned; ours leaves empty space below the last row. Anchor: `crates/gui/src/views.rs:146-148` (`row_height_for_line = line_h + ROW_PAD`).
7. Status bar is one line, not segmented (width/alignment) — ref bottom bar is segmented per pane (cursor position, format, result, edit mode); ours is a single summary line. Anchor: `crates/gui/src/views.rs:104` (`HEADER_H = 28.0`, shared chrome height; status reuses same beat).
8. Type scale flat, tracking zero (family/weight/line-height/spacing) — ref chrome uses proportional family with visible size/weight steps between title, labels, and editor text; ours documents heading-18/body-13/small-11/mono-12 with `TYPO_LETTER_SPACING 0.0` no-op and tightened multipliers. Anchor: `crates/gui/src/theme.rs:15-20` + `theme.rs:76` + `theme.rs:99-105` (chrome 1.2 / body 1.40 / mono 1.30).
9. Session cards text-only, grid breathes less (margin/padding) — ref home shows icon tiles with generous gaps; ours is 3 text columns with tight gaps. Anchor: `crates/gui/src/home.rs:163` (`HOME_GRID_COLUMNS 3`) + `home.rs:179` (`HOME_GRID_GAP`) + `home.rs:205` (`HOME_CARD_CTA_W`).
10. Tree indent + status marks light (spacing/alignment) — ref folder tree has deeper indent guides and clearly separated status marks; ours uses flat rows with thin segment minimums. Anchor: `crates/gui/src/gutter.rs:40` (`GUTTER_SEGMENT_MIN_H = SPACE_XS`) + `gutter.rs:46` (`GUTTER_HIT_SLOP`) + `views.rs:148` (row height).

Closed this round: ZERO (default render byte-same as prior `gui.png` — no visual gap closed by evidence). Newly opened: ZERO.

## 6. Verify + hygiene

```
git status --short -- checks/early-feedback-5/
ls -l checks/early-feedback-5/round-1/
git add checks/early-feedback-5/round-1/REPORT.md checks/early-feedback-5/round-1/default.png checks/early-feedback-5/round-1/dark.png checks/early-feedback-5/round-1/light.png
git commit -m "checks(early-feedback-5): round-1 local run evidence" -- checks/early-feedback-5/
git status  # dist/ tarball + other nodes' crates/** stay uncommitted; push: never
```

RESULT: PASS
