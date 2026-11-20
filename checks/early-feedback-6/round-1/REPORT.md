# REPORT — early-feedback-6 round-1 local run (2026-11-20)

Scope: leaf local-run wave 6. OWNS only `checks/early-feedback-6/**`. Zero source edits (`crates/**`, `tools/**` untouched; source read-only). Branch: `main`. Push: never.

## 1. Build

- `cargo build -p autocompare-gui` — exit 0 (`Finished dev profile`, cached rebuild in 0.20s).

## 2. GUI — 5 headless screenshots (text/folder/table/hex/home)

- `autocompare-gui --screenshot checks/early-feedback-6/round-1/<name>.png` — all 5 exit 0.
- Filename routes the view (`screenshot.rs:211-261`: `folder`/`hex`/`table`/`home` substring match, else text).

| File | Bytes | Magic | Size | Sha256 (short) |
|---|---|---|---|---|
| text.png | 9765 | `89504e470d0a1a0a` ok | 960x600 | `bde8ccf4…` |
| folder.png | 10153 | `89504e470d0a1a0a` ok | 960x600 | `c5b54d09…` |
| table.png | 9996 | `89504e470d0a1a0a` ok | 960x600 | `e1ddba38…` |
| hex.png | 10004 | `89504e470d0a1a0a` ok | 960x600 | `81be8452…` |
| home.png | 14991 | `89504e470d0a1a0a` ok | 960x600 | `36ebb7a8…` |

Python struct check: width/height parsed from PNG IHDR for all five — PASS. `file(1)`: all five `PNG image data, 960 x 600, 8-bit/color RGBA` — PASS.
Determinism note: `text.png` sha `bde8ccf4…` is byte-identical to the committed round-5 `gui.png`/`default.png` — text render has ZERO drift since round-3. That is stability, not a closed gap.

## 3. Tests

- `cargo test -p autocompare-gui` — 748 passed, 0 failed (lib target; main + doctests 0 tests) — PASS.

## 4. CLI — 10 cmds over tools/parity-harness/fixtures

Convention: exit 1 = inputs differ, 0 = same. All 10 ran to a convention exit (no crash/exit-2) — 10/10 PASS.

| # | Cmd | Exit | Reads as |
|---|---|---|---|
| 1 | `text t_left.txt t_right.txt` | 1 | differ — expected |
| 2 | `text t_same.txt t_same.txt` | 0 | same — expected |
| 3 | `folder folder/left folder/right` | 1 | differ — expected |
| 4 | `hex h_left.bin h_right.bin` | 1 | differ — expected |
| 5 | `merge m_base.txt m_left.txt m_right.txt` | 1 | conflict/diverge — expected |
| 6 | `table tab_left.csv tab_right.csv` | 1 | differ — expected |
| 7 | `sync sync/left sync/right` | 1 | plan non-empty — expected |
| 8 | `picture pic_left.png pic_right.png` | 1 | differ — expected |
| 9 | `folder-merge fmerge-base fmerge-left fmerge-right` | 1 | diverge — expected |
| 10 | `audio a_left.wav a_right.wav` | 1 | differ — expected |

## 5. TOP-10 geometry/typography gaps vs /tmp/bc-screen.png

Reference: `/tmp/bc-screen.png` (2880x1800 desktop shot; right window is the compare app on a 3-line text pair: session tab row, two-row labeled action toolbar, per-pane path bars with metadata strip, twin panes + arrow-marked center gutter, hatched blank-fill, segmented status bars). Ours: the five shots above (960x600 fixture renders). Outside-observe only: no window click, no AX dump, no string/asset copy. Colors out of scope — layout/type only.

1. Per-session tab row missing (height/alignment) — ref has a multi-tab session row with per-tab close boxes above the toolbar; ours has one app-tab strip (`Text` highlighted) and no session tabs. Anchor: `crates/gui/src/views.rs:104` (`HEADER_H = 28.0`, shared chrome height reused for the strip).
2. Action toolbar is one text row, not a two-row labeled icon cluster (height/padding) — ref shows Home/Sessions/All/Diffs/Same/Context/Minor/Rules/Format/Copy icon+label buttons plus a second nav row; ours is `[Swap Sides] [Reload/Recompare] [x] Word Wrap …` text. Anchor: `crates/gui/src/views.rs:458` (`session_toolbar_button_size`, 120x28 floor) + `views.rs:485-486` (swap/reload labels).
3. No prev/next difference jump cluster or position label (spacing/alignment) — ref has dedicated jump arrows under the toolbar and a `1:1` cursor readout per pane; ours has no jump control and only a summary status line. Anchor: `crates/gui/src/views.rs:527-534` (toolbar wiring; only swap/reload call-sites).
4. Path bars lack dropdown affordance + per-pane metadata strip (height/margin) — ref path row has dropdown + folder icons and a second metadata line per pane (timestamp/byte-count/format/rules); ours is path field + `Browse…`, single row. Anchor: `crates/gui/src/screenshot.rs:307-364` route (single chrome/text rows painted; no second strip).
5. Center gutter has no per-hunk action markers (width/alignment) — ref center gap carries small directional arrows per hunk; ours is a plain 16px overview ruler with band segments. Anchor: `crates/gui/src/views.rs:122` (`TEXT_CENTER_GUTTER_W`) + `screenshot.rs:137-142` (`GUTTER_X/GUTTER_W`, `GUTTER_RULER_WIDE` 16).
6. Blank-fill alignment missing (height/spacing) — ref pads the shorter pane with a hatched blank region so rows stay aligned; ours leaves empty space below the last row (visible in text/folder/table/hex shots). Anchor: `crates/gui/src/views.rs:175-177` (`row_height_for_line = line_h + ROW_PAD`).
7. Status bar is one line, not segmented (width/alignment) — ref bottom bar is segmented per pane (cursor position, format, result, edit mode); ours is a single summary line (`7 rows: …`). Anchor: `crates/gui/src/views.rs:104` (`HEADER_H` shared chrome beat; status reuses it).
8. Type scale flat, tracking zero (family/weight/line-height/spacing) — ref chrome uses proportional family with visible size/weight steps between title, labels, and editor text; ours documents a fixed ramp with an explicit no-op tracker. Anchor: `crates/gui/src/theme.rs:76` (`TYPO_LETTER_SPACING = 0.0`) + `theme.rs:20-21,221`.
9. Home cards text-only, grid breathes less (margin/padding) — ref home shows icon tiles with generous gaps; ours is 3 text columns (`HOME_GRID_COLUMNS 3`) with tight gaps and full-width CTA buttons; recent list is flat rows with thin kind/age columns. Anchor: `crates/gui/src/home.rs:163` (`HOME_GRID_COLUMNS`) + `home.rs:179` (`HOME_GRID_GAP`) + `home.rs:205` (`HOME_CARD_CTA_W`).
10. Tree indent + status marks light (spacing/alignment) — ref folder tree has deeper indent guides and clearly separated status marks; ours uses ~14px/depth indent with single-glyph marks (`=`, `>`, `R`) in a flat row. Anchor: `crates/gui/src/gutter.rs` (`GUTTER_SEGMENT_MIN_H` 4.0 floor) + `screenshot.rs:1475-1476` (indent 2 units ≈ 14px/depth).

Closed this round: ZERO (text render byte-same as prior `gui.png` — no visual gap closed by evidence). Newly opened: ZERO.

## 6. Verify + hygiene

```
git status --short -- checks/early-feedback-6/
ls -l checks/early-feedback-6/round-1/ && file checks/early-feedback-6/round-1/*.png
git add checks/early-feedback-6/
git commit -m "docs(feedback): local6 round-1 run evidence" -- checks/early-feedback-6/
git status  # crates/** untouched; push: never
```

RESULT: PASS (build 0, 5/5 screenshots magic+size ok, 748 tests pass, 10/10 CLI convention exits, TOP-10 filed above)
