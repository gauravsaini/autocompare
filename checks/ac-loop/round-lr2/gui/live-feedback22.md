# Live GUI feedback 22 (round-lr2) — gutter live verify

Date (UTC): 2026-11-21. Branch: main (no switch). Read-only on source; only harness + checks written.

HEAD: 1f69383776e4d23049f1ba804320cb8a2cff683a
Harness: `sh tools/local-run/run.sh` — exit 0 (CLI_10_OK, PNG_3_OK, DIST_OK, RESULT: PASS).
Screenshot: /tmp/live-gui22.png (722170 B, screencapture exit 0). GUI log: /tmp/gui22-stdout.log (empty, 0 B).

## What was run
1. `git pull --ff-only origin main` — already up to date, HEAD 1f69383776e4d23049f1ba804320cb8a2cff683a.
2. `sh tools/local-run/run.sh` — exit 0.
3. Live-window check (Text Compare via CLI flag, per live-feedback19 method):
   - Launched `./target/debug/autocompare-gui --open tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --tab text > /tmp/gui22-stdout.log 2>&1 &` (pid 47549), `sleep 5`, `screencapture -x -o /tmp/live-gui22.png` (exit 0, 722170 B), then `kill 47549` — `pgrep -x autocompare-gui` empty (GUI_KILLED_OK).
   - `/tmp/gui22-stdout.log` empty (0 B, no crash output).
4. Viewed:
   - `/tmp/live-gui22.png` (live window, Text Compare open via --open flag, title `.../t_left.txt <--> .../t_right.txt - Text - autocompare`)
   - `checks/ac-loop/round-lr2/gui/shot-default.png` (11254 B headless fixture shot)
   - `/tmp/bc-capture.png` (904993 B) BC Text Compare reference.

## Gutter verdict (gutter-live-verify): FAIL
- The LIVE window center gutter is still a single `=>` pill badge rendered on the changed row 2 only (`beta` / `BETA`), inline between the left/right panes.
- It is NOT the gutter20 target: no full-height continuous ~20px gutter strip, no yellow hunk-head arrows isolated to changed rows across the pane height. Unchanged rows (1 `alpha`, 3 `gamma`) show blank center with no gutter column at all.
- Fixture `shot-default.png` shows the same single-`=>` treatment (one `=>` on its changed row), confirming gutter20 egui work is not visible live either.

## Toolbar mirror check (toolbar21): NOT YET live (as expected — fixture-only)
- Reported honestly: the live toolbar does NOT show the toolbar21 fixture glyph-mark + chevron Prev/Next treatment. Live toolbar remains the older two-row plain-button set: `Swap Sides` / `Reload / Recompare` row, `Prev` / `Next` row, Find + `Show: All/Differences/Same/Context` filter row, and `Section -> / <- Section / Line -> / <- Line / All -> / <- All` text nav row (simple arrow chars only, no icon+label session strip).
- No code change made; this matches the expectation that toolbar21 was fixture-only headless work.

## Top 5 remaining gaps vs Beyond Compare (geometry/typography first, colors last)
1. Center change-gutter still a single `=>` badge on the changed row only, not BC's full-height ~20px gutter with yellow arrows on changed rows — lives in center column between left/right Text panes.
2. No per-pane file-bar metadata line (BC `Yesterday, 9:31:05pm 17 bytes Everything Else v U U`) — ours has one thin path label + Browse buttons only — lives in Text view header above each pane.
3. Toolbar is plain text-button rows, missing BC's icon+label session strip (Home/Sessions/All/Diffs/Same/Context/Minor/Rules/Format/Copy) + second nav-arrow row — lives in toolbar strip directly under the window title.
4. No per-pane (`1:1 | Default text`) or bottom (`Unimportant differences | Same | Insert | Load...`) status strips — ours collapses to one `modified @... | Text: exact | wrap:off` line — lives in bottom status bar of Text view.
5. Palette/filler finish (colors last) — BC light panes + hatched empty filler + blue token text; ours dark-only chrome with flat yellow token wash + no hatch under the 3 rows — lives in Text-pane theme + empty/filler area style.

## Paths
- Live: /tmp/live-gui22.png; GUI log: /tmp/gui22-stdout.log (empty, 0 B); Fixture: checks/ac-loop/round-lr2/gui/shot-default.png; BC: /tmp/bc-capture.png.
