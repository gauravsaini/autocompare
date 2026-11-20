# REPORT — early-feedback-7 round-1 local run (2026-11-20)

Scope: leaf local-run wave 7. OWNS only `checks/early-feedback-7/**`. Zero source edits (`crates/**`, `tools/**`, `dist/**` untouched; source read-only). Branch: `main`. HEAD: `b534c3d` (docs launch; code HEAD covers toolbar `f48710e` + home3 `502e404` + tree2 `abcf36c` + pathmeta `ce74d5c`). Push: never.

## 1. Build

- `cargo build -p autocompare-gui` — exit 0 (`Finished dev profile`, cached rebuild). Log: `build.out`.

## 2. GUI — 5 headless screenshots (text/folder/table/hex/home)

- `autocompare-gui --screenshot checks/early-feedback-7/round-1/<name>.png` — all 5 exit 0.
- Filename routes the view (`screenshot.rs:326-363`: `hex`/`table`/`folder`/`home` substring match, else text).

| File | Bytes | Magic | Size | Sha256 (short) |
|---|---|---|---|---|
| text.png | 10363 | `89504e470d0a1a0a` ok | 960x600 | `47102bdc…` |
| folder.png | 10153 | `89504e470d0a1a0a` ok | 960x600 | `c5b54d09…` |
| table.png | 10582 | `89504e470d0a1a0a` ok | 960x600 | `ddbcd2aa…` |
| hex.png | 10576 | `89504e470d0a1a0a` ok | 960x600 | `ef6915e3…` |
| home.png | 14991 | `89504e470d0a1a0a` ok | 960x600 | `36ebb7a8…` |

Python struct check: width/height parsed from PNG IHDR for all five — PASS. `file(1)`: all five `PNG image data, 960 x 600, 8-bit/color RGBA` — PASS.
Determinism notes (measured, not assumed):
- `text.png` sha `47102bdc…` is byte-identical to `/tmp/orch-exq-final.png` (10363B) — `cmp ... IDENTICAL`. Text fixture has ZERO drift vs the orch baseline.
- `folder.png` (`c5b54d09…`) and `home.png` (`36ebb7a8…`) are sha-prefix-identical to round-6's committed shots — no drift on those two fixtures.
- `table.png` (`e1ddba38…` → `ddbcd2aa…`, 9996→10582B) and `hex.png` (`81be8452…` → `ef6915e3…`, 10004→10576B) moved vs round-6 — consistent with the toolbar/pathmeta/segmented-status waves landing in live paint.

## 3. Tests

- `cargo test -p autocompare-gui` — 778 passed, 0 failed (lib target; main + doctests 0 tests) — PASS. Log: `tests.out`.
- Delta vs round-6 (748 → 778, +30) matches the landed waves (toolbar session tabs/action bar/jump + home3 + tree2 + pathmeta).

## 4. CLI — 10 cmds over tools/parity-harness/fixtures

Convention: exit 1 = inputs differ, 0 = same. All 10 ran to a convention exit (no crash/exit-2) — 10/10 PASS. Log: `cli-exits.txt` + `cli-*.out`.

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

## 5. TOP-10 remaining geometry/typography gaps

References: `/tmp/bc-screen.png` (2880x1800 desktop shot; right window is the compare app on a 3-line text pair: session tab row, two-row labeled icon toolbar, per-pane path bars + metadata strip, twin panes + arrow-marked center gutter, hatched blank-fill, per-pane cursor strips + segmented bottom bar) vs ours (the five shots above, 960x600 fixture renders). Second baseline: `/tmp/orch-exq-final.png` (10363B) — our `text.png` is byte-identical to it, so vs-orch text gap is ZERO by measurement; remaining vs-orch gaps are fixture-route coverage (session-strip/jump/tree/guide scenes prove in unit tests, not in these 5 PNGs) plus the table/hex deltas noted above. Outside-observe only: no window click, no AX dump, no string/asset copy. Colors out of scope — layout/type only.

1. Session tab row missing in these fixtures (height/alignment) — ref has a per-session tab row (`t_left.txt <--> t_right.txt`, `left <--> right`, `tab_left.csv <--> tab_right.csv`) with close boxes above the toolbar; ours has one app-tab strip (`Home Text Folder Hex …`, `Text` highlighted) and no session tabs in the 5 PNGs. Code has session-strip scenes (tests prove PNGs) but the default fixture route doesn't paint them. Anchor: `crates/gui/src/views.rs:104` (`HEADER_H = 28.0`, shared chrome beat reused for the strip).
2. Action toolbar is one text row, not a two-row labeled icon cluster (height/padding) — ref shows Home/Sessions/All/Diffs/Same/Context/Minor/Rules/Format/Copy icon+label buttons plus a second nav row; ours is `[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax` single text row (glyph-led labels exist in code but fixture still reads as one text beat). Anchor: `crates/gui/src/views.rs:479` (`session_toolbar_button_size`, 96x24-class floor) + `views.rs:507-513` (swap/reload labels).
3. No prev/next difference jump cluster or N/M badge in fixtures (spacing/alignment) — ref has dedicated jump arrows under the toolbar plus a per-pane `1:1` cursor readout; ours has no jump control in the 5 PNGs, only a summary line and the bottom `3:1` segment. The jump/action-bar painter exists in code but is not on this route. Anchor: `crates/gui/src/views.rs:744` (`paint_session_action_bar`) + `views.rs:1688` (`diff_nav_position`) + `views.rs:579-587` (1:1 readout contract).
4. Path bars lack dropdown affordance depth + history combo (height/margin) — ref path row has dropdown + folder icons per pane; ours is `[ ] path … v Browse…` single row per pane. Partially closed: the second metadata line per pane now paints (`2026-01-02 … | 128 B | Text: exact | wrap off`, visible in text/table/hex shots). Remaining: single-row height, no history combo box in the fixture. Anchor: `crates/gui/src/views.rs:764-805` (`path_meta_role`, `format_path_meta`, `paint_path_meta_row`) + `screenshot.rs:161-216` (path strip paint).
5. Center gutter has no per-hunk action markers in fixtures (width/alignment) — ref center gap carries small directional arrows per hunk; ours is a plain gap with band segments only on the right-edge overview ruler. Tree2/gutter-arrow code exists but not visible on the text route. Anchor: `crates/gui/src/views.rs:122` (`TEXT_CENTER_GUTTER_W`) + `views.rs:103` (`GUTTER_W`) + `theme.rs:272-276` (`GUTTER_BAR_WIDE`, `GUTTER_MIN_SEGMENT_H`).
6. Blank-fill alignment missing (height/spacing) — ref pads the shorter pane with a hatched blank region so rows stay aligned; ours leaves empty space below the last row (visible in text/folder/table/hex shots). Anchor: `crates/gui/src/views.rs:196-198` (`row_height_for_line = line_h + ROW_PAD`) + `views.rs:105` (`ROW_PAD`).
7. Status is segmented but not per-pane (width/alignment) — ref has per-pane cursor strips (`1:1 | Default text` per pane) above a segmented bottom bar (`Unimportant differences | Same | Insert | Load…`); ours has one bottom line (`3:1 | Text: exact | 7 rows: … | edit: clean`) with `|` dividers — the segmentation wave landed, the per-pane upper strip did not (in these fixtures). Anchor: `crates/gui/src/views.rs:4464-4484` (`text_pane_cursor_labels`) + `views.rs:104` (`HEADER_H` status beat).
8. Type scale flat, tracking zero (family/weight/line-height/spacing) — ref chrome uses proportional family with visible size/weight steps between title, labels, and editor text; ours documents a fixed ramp with an explicit no-op tracker. Anchor: `crates/gui/src/theme.rs:76` (`TYPO_LETTER_SPACING = 0.0`) + `theme.rs:20-21,111-120` (no-op doc + guard).
9. Home cards text-only, grid breathes less (margin/padding) — ref home shows icon tiles with generous gaps; ours is 3 text columns (`HOME_GRID_COLUMNS 3`) with tight gaps and full-width CTA buttons; recent list is flat rows with thin kind/age columns. Code has an EXQ gap/CTA path but the fixture still paints the old beat. Anchor: `crates/gui/src/home.rs:163` (`HOME_GRID_COLUMNS`) + `home.rs:179-185` (`HOME_GRID_GAP` vs `HOME_GRID_GAP_EXQ`) + `home.rs:211-249` (`HOME_CARD_CTA_W`).
10. Tree indent + status marks light, table panes not split (spacing/alignment) — ref folder tree has deeper indent guides and clearly separated status marks; ours uses a shallow indent step with single-glyph marks (`v`, `>`, `=`, `R`) in a flat row (visible in folder.png). Table fixture renders full-width rows (`2 | bob | 20 | 22 | ! !`) with empty right bands instead of twin panes. Anchor: `crates/gui/src/views.rs:61,337-341` (`TREE_INDENT = 12.0`) + `views.rs:861-879` (CTA/grid beat) + `theme.rs:276` (`GUTTER_MIN_SEGMENT_H` floor).

Closed this round (measured): ZERO visual gaps closed in these 5 fixtures vs round-6 on text/folder/home (sha-identical); table/hex bytes moved (toolbar/pathmeta/segmented-status paint). Newly opened: ZERO.

## 6. Verify + hygiene

```
git status --short -- checks/early-feedback-7/
ls -l checks/early-feedback-7/round-1/ && file checks/early-feedback-7/round-1/*.png
git add checks/early-feedback-7/
git commit -m "docs(feedback): local7 round-1 run evidence" -- checks/early-feedback-7/
git status  # crates/**, tools/**, dist/** untouched; push: never
```

RESULT: PASS (build 0, 5/5 screenshots magic+size ok, 778 tests pass, 10/10 CLI convention exits, TOP-10 filed above)
