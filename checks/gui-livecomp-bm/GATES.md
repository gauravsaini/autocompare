# GATES — gui-livecomp-bm round-1

## Gate 1: live PNGs present + readable (file magic verify)
Method: `file round-1/*.png` — sab `PNG image data ... 8-bit/color RGBA, non-interlaced` aaye. Zero-byte/missing koi nahi.

| file | size px | magic |
|------|---------|-------|
| live-ours-home.png | 2880x1800 | PNG OK |
| live-ours-text.png | 2880x1800 | PNG OK |
| crop-ours-text.png | 1600x1264 | PNG OK |
| live-ours-text-zoom.png | 800x250 | PNG OK |
| live-ours-folder.png | 2880x1800 | PNG OK |
| crop-ours-folder.png | 1600x1264 | PNG OK |
| live-ours-table.png | 2880x1800 | PNG OK |
| crop-ours-table.png | 1600x1264 | PNG OK |
| live-bc-asfound.png | 2880x1800 | PNG OK |
| live-bc-text.png | 2880x1800 | PNG OK |
| crop-bc-text.png | 1440x1738 | PNG OK |
| live-bc-text-zoom.png | 640x200 | PNG OK |
| live-bc-folder.png | 2880x1800 | PNG OK |
| crop-bc-folder.png | 1440x1738 | PNG OK |
| live-bc-table.png | 2880x1800 | PNG OK |
| crop-bc-table.png | 1440x1738 | PNG OK |

Result: PASS (16/16 readable; 1 home + 3 ours views + 1 BC as-found + 3 BC views, har view ka fullscreen + window crop; text ke zoom crops extra).

## Gate 2: REPORT me gap table + specs
- REPORT.md me 17 gaps (G-T1..T7, G-F1..F5, G-B1..B5, G-C1..C2), har ek ke saath rank (P0/P1/P2) + exact follow-up spec (view/fn/lines, kya change, OWNS file). PASS.
- Rank counts: P0 = 6 (G-T1, G-F1, G-F2, G-B1, G-B2, G-B3), P1 = 7, P2 = 4.
- Colors par koi spec nahi (FROZEN note). PASS.
- Hinglish-Roman me likha hai, Devanagari nahi. PASS.

## Gate 3: constraints
- `crates/**` me ZERO edit (sirf read/grep). PASS.
- `checks/opencode-waves/STATUS.md` untouched. PASS.
- OWNS ke bahar koi file create/modify nahi (sirf `checks/gui-livecomp-bm/`). PASS.
- Koi window/tab close nahi kiya; BC me sirf 2 tabs khole (note OBSERVE.md me). PASS.

Overall: PASS (3/3).
