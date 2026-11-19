# BO-LOCALRUN GATES — round-1

## Gate results

| Gate | Expect | Actual | Status |
|---|---|---|---|
| Live PNGs present | live-desktop, live-home, live-bc round-1/ me | 3 files present (2880x1800, 1600x1264, 1440x1738) | PASS |
| File magic ok | sab `PNG image data` | `file` se verified, sab PNG RGBA non-interlaced | PASS |
| Headless screenshots | fixture home/text/folder/table 960x600 | 4 files, sab 960x600 PNG (text visually verified) | PASS |
| REPORT me gap table | P0/P1/P2 + exact file OWNS | P0x2, P1x4, P2x2, har row me exact file paths | PASS |
| REPORT me specs | run spec + evidence table + craft mapping + loop | section 2, 3, 6, 7 present | PASS |
| ZERO code edits | is run me koi file edit nahi; sirf checks/localrun-bo/ add | PASS (note: tree me doosre waves ke pehle-se uncommitted changes the — crates/gui/src/views.rs, views_table.rs, checks/gui-geo-bn/ — unhe touch/commit nahi kiya) |
| Commit, no push | docs(checks) commit, PUSH MANA | local commit, push nahi kiya | PASS |

## Specs (evidence)

- live-desktop.png: 2880x1800 PNG, full side-by-side (Home front + BC peeche)
- live-home.png: 1600x1264 PNG, `-R320,91,800,632` region, full Home window
- live-bc.png: 1440x1738 PNG, `-R720,31,720,869` region, BC Text Compare (left half occluded, as-is)
- fixture-home/text/folder/table.png: 960x600 PNG each, `--screenshot` renders
- cargo-run.log: dev profile Finished + Running proof
- Binary: target/debug/autocompare-gui (Mach-O arm64), PID 6133 live tha capture time
- BC: /Applications/Beyond Compare.app v5 (Text Compare session, 3 session tabs observed)

## Open follow-ups (next round ke liye, blocker nahi)

- P1-4 theme default anomaly (live Light Mode vs `dark_mode=true` default) — root cause round-2 me.
- text/folder/table live captures — tab-switch interaction wale wave ke saath ya manual round me.
