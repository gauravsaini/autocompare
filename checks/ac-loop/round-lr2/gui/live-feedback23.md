# Live GUI feedback 23 (round-lr2) — mirror22 re-verify

Date (UTC): 2026-11-21. Branch: main (no switch). Read-only on source; only harness + checks written.

HEAD: 173a485dbb437b1b39ef86824d8b49ce8f128d43
Harness: `sh tools/local-run/run.sh` — exit 0 (CLI_10_OK, PNG_3_OK, DIST_OK, RESULT: PASS).
Screenshot: /tmp/live-gui23.png (724696 B, screencapture exit 0). GUI log: /tmp/gui23-stdout.log (empty, 0 B).

## What was run
1. `git pull --ff-only origin main` — already up to date, HEAD 173a485dbb437b1b39ef86824d8b49ce8f128d43.
2. `sh tools/local-run/run.sh` — exit 0.
3. Live-window check (Text Compare via CLI flag, per live-feedback22 method):
   - Launched `./target/debug/autocompare-gui --open tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --tab text > /tmp/gui23-stdout.log 2>&1 &` (pid 51028), `sleep 5`, `screencapture -x -o /tmp/live-gui23.png` (exit 0, 724696 B), then `kill 51028` — `pgrep -x autocompare-gui` empty (GUI_KILLED_OK).
   - `/tmp/gui23-stdout.log` empty (0 B, no crash output).
4. Viewed:
   - `/tmp/live-gui23.png` (live window, Text Compare open via --open flag, title `tools/parity-harness/fixtures/t_left.txt <--> tools/parity-harness/fixtures/t_right.txt - Text - autocompare`)
   - `/tmp/bc-capture.png` (2880x1800) BC Text Compare reference.

## mirror22 re-verify (prev verdict live-feedback22: gutter FAIL — single => pill, transparent unselected rows)
Mirror22 claim: frameless gap-rect arrows + continuous track fill on every live center cell + live bottom status strips.

- (a) full-height ~20px center track visible on ALL rows (not only changed): FAIL
  - Live center shows a single `=>` on changed row 2 only (`beta` / `BETA`). Rows 1 (`alpha`) and 3 (`gamma`) show blank center with no track column at all. No full-height continuous strip.
- (b) yellow => arrows on changed rows only, no pill-button frame: FAIL
  - The row-2 `=>` is light gray/white, not yellow, and is the same single-token treatment as live22 — no frameless gap-rect yellow arrows. No evidence the mirror22 arrow treatment is visible live.
- (c) bottom bar shows per-pane strips + legend segments: FAIL
  - Live Text view has no bottom `1:1 | Default text` per-pane strips and no `Unimportant differences | Same | Insert | Load...` legend. Status collapses to the single top `modified @... | Text: exact | wrap:off` line. BC reference behind the live window shows both strip rows; ours does not.

## Top 5 remaining gaps vs Beyond Compare (geometry/typography first, colors last)
1. Center change-gutter still a single `=>` token on the changed row only, not BC's full-height ~20px gutter with yellow arrows on changed rows — lives in center column between left/right Text panes.
2. No per-pane file-bar metadata line (BC `Yesterday, 9:31:05pm 17 bytes Everything Else v U U`) — ours has one thin path label + Browse buttons only — lives in Text view header above each pane.
3. Toolbar is plain text-button rows, missing BC's icon+label session strip (Home/Sessions/All/Diffs/Same/Context/Minor/Rules/Format/Copy) + second nav-arrow row — lives in toolbar strip directly under the window title.
4. No per-pane (`1:1 | Default text`) or bottom (`Unimportant differences | Same | Insert | Load...`) status strips — ours collapses to one `modified @... | Text: exact | wrap:off` line — lives in bottom status bar of Text view.
5. Palette/filler finish (colors last) — BC light panes + hatched empty filler + blue token text; ours dark-only chrome with flat yellow token wash + no hatch under the 3 rows — lives in Text-pane theme + empty/filler area style.

## Paths
- Live: /tmp/live-gui23.png; GUI log: /tmp/gui23-stdout.log (empty, 0 B); BC: /tmp/bc-capture.png.
