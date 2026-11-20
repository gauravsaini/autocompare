# Live GUI feedback 24 (round-lr2) — gutter23 verify

Date (UTC): 2026-11-21. Branch: main (no switch). Read-only on source; only checks written.

HEAD: 888db3c7e52758a61d4f2e5d298e5e2fcf7871ad
Harness: `sh tools/local-run/run.sh` — exit 0 (CLI_10_OK, PNG_3_OK, DIST_OK, RESULT: PASS).
Screenshot: /tmp/live-gui24.png (729490 B, screencapture exit 0). GUI log: /tmp/gui24-stdout.log (empty, 0 B).

## What was run
1. `git pull --ff-only origin main` — already up to date, HEAD 888db3c7e52758a61d4f2e5d298e5e2fcf7871ad.
2. `sh tools/local-run/run.sh` — exit 0.
3. Live-window check (Text Compare via CLI flag, per live-feedback23 method):
   - Launched `./target/debug/autocompare-gui --open tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --tab text > /tmp/gui24-stdout.log 2>&1 &` (pid 57475), `sleep 5`, `screencapture -x -o /tmp/live-gui24.png` (exit 0, 729490 B), then `kill 57475` — `pgrep -x autocompare-gui` empty (GUI_KILLED_OK).
   - `/tmp/gui24-stdout.log` empty (0 B, no crash output).
4. Viewed:
   - `/tmp/live-gui24.png` (2880x1800 live window, Text Compare open via --open flag) + zoomed crops `/tmp/gutter-crop.png` (rows) and `/tmp/live24-bottom.png` (window bottom).
   - `/tmp/bc-capture.png` (2880x1800) BC Text Compare reference.
   - Pixel probes via pillow: gutter-cell bg exactly (48,48,48); row-2 glyph pixels (252,236,169); rows 1/3 gutter cells max exactly (48,48,48).

## gutter23 verify (prev verdict live-feedback23: gutter FAIL on mirror22 build)
Gutter23 claim: full-height center track + yellow frameless arrows + mirror22 strips intact.

- (a) full-height center track at 48,48,48 visible on ALL rows: PASS
  - 48,48,48 cells pixel-confirmed on row 1 (alpha), row 2 (beta/BETA), row 3 (gamma), and filler rows below. Rows 1/3 no longer blank (the live23 failure mode is fixed).
  - Note: cells are discrete per-row rects with 1-row bg (27,27,27) gaps between them, not one seamless strip — recorded as gap 4 below, but the claim as stated (track visible on all rows, 48.0 gate) passes.
- (b) yellow => arrows in CHANGED_FG on changed rows only via painter.text, no Button-pill frame: PASS
  - Row-2 center shows `=>` with glyph pixels (252,236,169) warm yellow; rows 1/3 gutter cells are pure (48,48,48) with no glyph. Crop shows plain text, no rounded-pill background; horizontal pixel scan shows clean 27->48 cell edges with no border/frame pixels.
- (c) mirror22 bottom status strips still intact: FAIL
  - Live window bottom (crop `/tmp/live24-bottom.png`) shows filler rows + gutter cells running to the window edge — no per-pane `1:1 | Default text` strips and no `Unimportant differences | Same | Insert | Load...` legend inside our window. The strips visible at the screenshot bottom belong to the BC reference window behind. Same absence as live23; gutter23 did not regress them (they were never live-visible), but the claim is not evidenced in the screenshot.

## Top 5 remaining gaps vs Beyond Compare (geometry/typography first, colors last)
1. No per-pane file-bar metadata line (BC `Yesterday, 9:31:05pm 17 bytes Everything Else v U U`) — ours has one thin path label + Browse buttons only — lives in Text view header above each pane.
2. Toolbar is plain text-button rows, missing BC's icon+label session strip (Home/Sessions/All/Diffs/Same/Context/Minor/Rules/Format/Copy) + second nav-arrow row — lives in toolbar strip directly under the window title.
3. No per-pane (`1:1 | Default text`) or bottom (`Unimportant differences | Same | Insert | Load...`) status strips — ours collapses to one `modified @... | Text: exact | wrap:off` line — lives in bottom status bar of Text view.
4. Center gutter is discrete per-row 48,48,48 cells with bg gaps, not BC's full-height continuous ~20px gutter — and arrows are `=>` text glyphs vs BC's small chevron markers — lives in center column between left/right Text panes.
5. Palette/filler finish (colors last) — BC light panes + hatched empty filler + blue token text; ours dark-only chrome with flat yellow token wash + no hatch under the 3 rows — lives in Text-pane theme + empty/filler area style.

## Paths
- Live: /tmp/live-gui24.png; crops: /tmp/gutter-crop.png, /tmp/live24-bottom.png; GUI log: /tmp/gui24-stdout.log (empty, 0 B); BC: /tmp/bc-capture.png.
