# Local-Run Early-Feedback Report (Wave H agent A)

Date: 2026-11-19 UTC. Repo: /Users/ektasaini/autocompare. Scope: sirf checks/local-run-exq/** me likha, crates/** me zero edit. Clean-room: bahar se observation only, koi string/asset copy nahi.

## 1. Metrics

- Release build: `cargo build --release -p autocompare-gui` exit 0, cargo time 52.25s, wall 52s.
- Binary: target/release/autocompare-gui, 10851744 bytes (~10.4MB).
- Harness: tools/gui-polish/run-geo-typo.sh round-dir checks/local-run-exq/harness, HARNESS_EXIT 0, RESULT: PASS.
- Tests: cargo test -p autocompare-gui, 123 passed, 0 failed, 0.31s.
- PNG: checks/local-run-exq/after.png, 9914 bytes, magic 89504e470d0a1a0a OK, 960x600 RGBA 8-bit, distinct bytes 256.
- Harness stats: Grid::1, text_style_height x4, monospace x17, strong x5, small x10, .size 14.0 x1, width calls 6.

## 2. Build log tail

```
Compiling autocompare-gui v0.5.0
Finished release profile [optimized] target(s) in 52.25s
exit=0 elapsed=52s
-rwxr-xr-x 10851744 target/release/autocompare-gui
```

## 3. Test tail

```
test result: ok. 123 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
```

## 4. PNG provenance

Harness ne checks/local-run-exq/harness/after.png banaya (`--screenshot` via screenshot.rs:32 render_fixture_png), usko checks/local-run-exq/after.png me copy kiya. `file` bola PNG 960x600 RGBA, magic OK, 9914 bytes (>=8192 gate pass).

## 5. Gap list vs reference comparator (10-15 bullets, file:line refs)

Geometry:
1. Panes ke beech draggable splitter nahi hai; text panes available width se bante hain (crates/gui/src/app.rs:1217, views.rs:453 paint_text_with_prefs). Fixture me split x=12/476/940 par hard hai (screenshot.rs:118-135).
2. Hex tab me byte grid nahi hai; model sirf summary Vec<String> hai jo paint_lines se paint hota hai (app.rs:1420-1429, viewmodel.rs:840 build_hex_summary). Offset ruler, per-byte cursor, nibble nav missing hai.
3. Table tab me column grid nahi hai; har row ek hi monospace string "label | text" paint hoti hai (views.rs:401). Codebase me Grid::1 hi ek use hai (harness stat). Header row, column align, per-cell nav missing hai.
4. Picture tab me side-by-side/overlay slider, zoom, pan nahi hai; sirf summary lines + optional heatmap image (views.rs:948-961).
5. Thumbnail gutter geometry maujood hai (gutter.rs:34 gutter_geometry, min segment 1.5px, merge gap 1.0) par width 12px/6px hai (gutter.rs:83-85) aur gutter par click-to-jump handler nahi dikha.
6. Folder tab views::paint_folder se rewire pending hai (views.rs:32-35), paint inline app.rs me hai; tree indent fixed 14px (views.rs:43) aur size/time ke liye side-by-side columns nahi hain.
7. Merge tab single merged-output list paint karta hai (views.rs:319 paint_merge); base/left/right 3-pane layout nahi hai jabki TriplePaths alag stored hain (app.rs:38-44).
8. Sync/FolderMerge/Audio tabs plain summary lines paint karte hain (app.rs:1450-1493); per-file checklist/action UI nahi hai. Apply preview-only stub hai (app.rs:1411-1417).

Typography:
9. Type scale frozen hai: heading 18 / body 13 / small 11 / mono 12 (theme.rs:35-46). User font-size control nahi hai; letter-spacing documented no-op hai (theme.rs:53).
10. Headless PNG 5x7 bitmap glyphs se banta hai (screenshot.rs:599-631), real egui font stack se nahi; PNG layout rects prove karta hai, real kerning/hinting nahi.
11. Table changed cells inline marker wali single string hain (views.rs:374-376); per-cell bold/color span nahi, poori row ek CHANGED_FG me paint hoti hai (views.rs:401-404).

Usability blockers:
12. Text me find box nahi hai; sirf F3 / Ctrl+N/P nav hai (views.rs:489-498). Match highlight aur match count missing hai.
13. Merge resolve sirf conflict index 0 par whole-file Take Left/Right/Base/Both karta hai (views.rs:338-343); per-hunk/per-line pick nahi hai.
14. MAX_ROWS truncation par "showing first N" note ke baad paging ya load-more nahi hai (views.rs:468-476; merge/table me same pattern).
15. Compare async channel par chalta hai (app.rs:16) par progress/cancel UI nahi dikha; bade inputs par sirf "coarse comparison" note milta hai (views.rs:462-467).

## 6. Next steps

1. Hex byte-grid viewmodel + offset ruler (paint_lines se Grid layout tak).
2. Table per-column Grid + header row (Grid::1 se aage).
3. Gutter click-to-jump + merge per-hunk resolve.
4. Find box (match count + highlight) text view me.
5. Truncation paging ya virtualized rows.

Gates: no source changes, report-only. Commit locally, NO push.
