# Live GUI feedback 17 (round-lr2) — early-feedback runner

Date (UTC): 2026-11-20. Branch: main (no switch). Read-only on source; only harness + checks written.

## What was run
1. `sh tools/local-run/run.sh` — exit 0 (RESULT: PASS per REPORT.md).
   - CLI parity 10/10, GUI shots PASS (shot-default/dark/light.png, PNG magic ok), dist tarball + sha256 ok, edge-empty non-empty out, GAPS-TOP10.md present, FMT ok (owned scope clean).
2. Live-window check:
   - Launched `./target/debug/autocompare-gui > /tmp/gui17-stdout.log 2>&1 &` (pid 25448), `sleep 6`, `screencapture -x -o /tmp/live-gui17.png` (exit 0, 749967 B), then `kill <pid>` — `pgrep -x autocompare-gui` empty (GUI_KILLED_OK).
   - `/tmp/gui17-stdout.log` empty (no crash output).
3. Viewed:
   - `/tmp/live-gui17.png` (whole screen, foreground autocompare window over BC + Codex behind)
   - `checks/ac-loop/round-lr2/gui/shot-default.png` (10449 B headless fixture shot, Text tab, fixtures/left.rs vs right.rs)
   - `/tmp/bc-capture.png` (904993 B) + `/tmp/bc-text-compare-evidence.md`

## Live screenshot verdict
- Launch: OK — window "Home - autocompare - autocompare" appears centered, dark theme, no blank/crash. Home grid (9 cards), tab strip Home/Text/Folder/Hex/Merge/Table/Sync/Picture/FolderMerge/Audio, Swap Sides / Reload-Recompare / Prev / Next, Word Wrap / Line Numbers / Syntax Highlighting, Light Mode toggle all render legibly.
- Mismatch vs fixture: live window shows Home launcher; fixture shot shows Text Compare diff view. Live run did not auto-open a diff session, so fixture shot is NOT what a cold launch shows. Headless shots are also tiny (~10 KB, low-res offscreen) vs live HiDPI window — fixture does not represent live geometry/typography.
- Verdict on capture itself: PASS (window launches and paints); content parity vs BC: NOT yet.

## Top 8 visual gaps vs Beyond Compare (ordered by user-visible pain; geometry/typography first, colors last)
1. Cold launch lands on Home launcher grid, not side-by-side panes — BC opens straight into the diff; ours needs an extra "Open Text Compare" click. Lives in: Home view `Comparison Sessions` cards + top tab strip.
2. No per-pane file bar with path + timestamp/size/encoding metadata — BC has path field + "Yesterday, 9:31:05pm 17 bytes Everything Else" per side; ours has one thin `fixtures/left.rs` label row. Lives in: Text view header above each pane.
3. Center change-gutter too narrow, markers weak — BC ~20px gutter with yellow `=>` arrows only on changed rows; ours is a single `=>` char column. Lives in: center column between left/right Text panes.
4. Row height/density too tight with no cell padding — BC rows ~22px with line-number gutter + text inset; fixture rows look cramped and truncated. Lives in: diff-row layout in Text Compare panes.
5. Toolbar lacks BC's icon+label session controls and second nav-arrow row — ours is text-only tabs plus small Swap/Reload/Prev/Next buttons. Lives in: toolbar strip directly under the window title.
6. No per-pane (`1:1 | Default text`) or bottom (`Unimportant differences | Same | Insert | Load…`) status strips — ours collapses to one `Text: exact | 7 rows… | edit: clean` line. Lives in: bottom status bar of Text view.
7. No intra-line emphasis typography — BC changed row keeps mono text with blue changed-token text; ours is full-row flat wash (yellow/red/green) with no token-level cue. Lives in: changed-row text renderer in Text panes.
8. Palette/filler finish (colors last) — BC light panes + hatched empty filler; ours dark-only chrome with harsher flat red/green/yellow fills and solid-black empty area. Lives in: Text-pane theme + empty/filler area style.

## Verdict: shippable early build? FAIL
- Shell launches stably and Home grid is legible, but cold launch shows no diff, file bars/status strips/center gutter geometry differ, and headless fixture shots under-represent live rendering. Not yet shippable as a BC text-compare replacement; shippable only as a launcher shell. Next step: cold-launch (or one-click) into Text Compare with file bars + gutter + status strips matching evidence-md geometry targets.

## Paths
- Live: /tmp/live-gui17.png; Fixture: checks/ac-loop/round-lr2/gui/shot-default.png; BC: /tmp/bc-capture.png + /tmp/bc-text-compare-evidence.md; GUI log: /tmp/gui17-stdout.log (empty).
