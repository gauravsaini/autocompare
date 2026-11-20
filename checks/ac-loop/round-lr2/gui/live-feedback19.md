# Live GUI feedback 19 (round-lr2) — early-feedback runner

Date (UTC): 2026-11-21. Branch: main (no switch). Read-only on source; only harness + checks written.

HEAD: 43857f92c5d2848f3179b0e93eebf316902421a9
Harness: `sh tools/local-run/run.sh` — exit 0 (CLI_10_OK, PNG_3_OK, DIST_OK, RESULT: PASS).

## What was run
1. `git pull --ff-only origin main` — already up to date, HEAD 43857f92c5d2848f3179b0e93eebf316902421a9.
2. `sh tools/local-run/run.sh` — exit 0.
3. Live-window check (Text Compare via CLI flag):
   - Flags: `./target/debug/autocompare-gui --help` shows `--open <left> <right> [--tab <name>]` + `--screenshot <png>` — headless way to open Text Compare EXISTS.
   - Launched `./target/debug/autocompare-gui --open tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --tab text > /tmp/gui19-stdout.log 2>&1 &` (pid 35755), `sleep 5`, `screencapture -x -o /tmp/live-gui19.png` (exit 0, 722867 B), then `kill 35755` — `pgrep -x autocompare-gui` empty (GUI_KILLED_OK).
   - `/tmp/gui19-stdout.log` empty (0 B, no crash output).
4. Viewed:
   - `/tmp/live-gui19.png` (live window, Text Compare open via --open flag)
   - `checks/ac-loop/round-lr2/gui/shot-default.png` (10708 B headless fixture shot, Text tab)
   - `/tmp/bc-capture.png` (904993 B) + `/tmp/bc-text-compare-evidence.md`

## Do chrome17+shot18+typo17 show live?
- YES. Live window opened straight into Text Compare (title `t_left.txt <--> t_right.txt - Text - autocompare`), not Home launcher — `--open` flag works.
- chrome17 (app chrome geometry): visible — tabstrip Home/Text/Folder/Hex/Merge/Table/Sync/Picture/FolderMerge/Audio has airy 8px rhythm with Text pill selected; toolbar two-row band (Swap Sides / Reload-Recompare row, Prev/Next row) with breathing room; header `autocompare drag & drop | pure engines` + Light Mode toggle spaced, not cramped.
- shot18 (fixture chrome pixels): fixture `shot-default.png` is 10708 B (post-shot18 size, was 10449 B); live HiDPI window shows the same chrome structure (file bars + Browse... + Compare files, Find + Show filters, Section/Line/All nav) at real window scale.
- typo17 (text typography): visible — diff rows `alpha / beta=>BETA / gamma` render mono with token-level highlight (yellow token wash on `beta`/`BETA` distinct from row), line numbers, `All: 3 rows` status in small type.

## Verdict on "exquisite deltas visible live": PASS
- The three waves' deltas (chrome spacing, fixture chrome, token typography) all render in the LIVE window at real DPI, not just headless PNGs.

## Top 5 remaining gaps vs Beyond Compare (geometry/typography first, colors last)
1. Center change-gutter still a single `=>` char column, not BC's ~20px gutter with yellow `=>` only on changed rows — lives in center column between left/right Text panes.
2. No per-pane file-bar metadata line (BC `Yesterday, 9:31:05pm 17 bytes Everything Else v U U`) — ours has one thin path label + Browse buttons only — lives in Text view header above each pane.
3. Toolbar is text-only tabs + small Swap/Reload/Prev/Next buttons, missing BC's icon+label session controls + second nav-arrow row — lives in toolbar strip directly under the window title.
4. No per-pane (`1:1 | Default text`) or bottom (`Unimportant differences | Same | Insert | Load...`) status strips — ours collapses to one `modified @... | Text: exact | wrap:off` line — lives in bottom status bar of Text view.
5. Palette/filler finish (colors last) — BC light panes + hatched empty filler + blue token text; ours dark-only chrome with flat yellow token wash + solid-black empty area — lives in Text-pane theme + empty/filler area style.

## Paths
- Live: /tmp/live-gui19.png; Fixture: checks/ac-loop/round-lr2/gui/shot-default.png; BC: /tmp/bc-capture.png + /tmp/bc-text-compare-evidence.md; GUI log: /tmp/gui19-stdout.log (empty, 0 B).
