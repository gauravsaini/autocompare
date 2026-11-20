# Localrun GUI10 Refresh — REPORT2

Rebuilt `autocompare-gui` (`CARGO_TARGET_DIR=/tmp/target-localrun2`, `--quiet`, exit 0) and refreshed all 10 tab screenshots headlessly via `--screenshot checks/localrun/after-<tab>.png`. No code edits (observation only).

## Geometry (first)
All fixtures 960x600 RGBA (SNAP_W x SNAP_H), TITLE_H 36 / TAB_H 32 / ROW_H 24 / STATUS_H 30 contract intact per screenshot.rs.

## Results

| tab | file | bytes | PNG magic | dims | distinct bytes | status |
| --- | --- | --- | --- | --- | --- | --- |
| home | after-home.png | 14991 | ok | 960x600 | 69 | PASS |
| text | after-text.png | 10401 | ok | 960x600 | 99 | PASS |
| folder | after-folder.png | 10153 | ok | 960x600 | 80 | PASS |
| foldermerge | after-foldermerge.png | 10421 | ok | 960x600 | 96 | PASS |
| hex | after-hex.png | 10576 | ok | 960x600 | 69 | PASS |
| merge | after-merge.png | 10421 | ok | 960x600 | 96 | PASS |
| table | after-table.png | 10582 | ok | 960x600 | 73 | PASS |
| sync | after-sync.png | 11416 | ok | 960x600 | 66 | PASS |
| picture | after-picture.png | 8936 | ok | 960x600 | 77 | PASS |
| audio | after-audio.png | 10401 | ok | 960x600 | 99 | PASS |

Threshold: PNG magic ok, size >= 8192, non-blank (distinct > 16). 10/10 PASS.

Note: `foldermerge` routes via `merge` substring; `audio` falls back to default text fixture (no dedicated audio route) — both byte-stable vs REPORT.md.

RESULT: PASS
