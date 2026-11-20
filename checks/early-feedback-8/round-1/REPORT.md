# LOCAL8 round-1 early feedback (checks only, no src edits)

- Branch: main (dirty, so NO pull; CONTINUE per wave)
- BC observe: pid 23316 `/Applications/Beyond Compare.app/.../BCompare` running, outside-observe only (no control)
- Refs: `/tmp/bc-screen.png` 909131B 2880x1800, `/tmp/land-verify.png` 10363B 960x600 (md5 == our gui-text PNG: deterministic harness)
- Isolated target: `CARGO_TARGET_DIR=/tmp/target-op-local8` CLI binary OK (9.3M); GUI isolated build hit "No space left" once, fallback `target/debug/autocompare-gui` 73M used (documented, no src change)
- CLI: 10/10 OK (exit 1 = differences-found per `crates/cli/src/outcome.rs:13` diff semantics, not failure)
- PNGs: 3x 960x600 magic OK >=8192B (text 10363B, folder 10153B, home 14991B), distinct 256 (>64 non-blank)
- Tests: `cargo test -p autocompare-cli` PASS (exit 0); `cargo test -p autocompare-gui` FAIL compile (pre-existing, NOT fixed per owns-rule) -> `crates/gui/src/views_hex.rs:3586` `TABLE_CELL_PAD_X` not in scope (suggests `HEX_CELL_PAD_X`), see tests-gui.out

## Gates
- 10/10 CLI ok: PASS
- 3+ PNGs magic/size/non-blank: PASS
- REPORT TOP-10 gaps: PASS (below)

## RESULT: PASS

## TOP-10 geometry+typo gaps vs real BC (outside-observe)
1. Toolbar: BC icon strip (Home/Sessions/All/Diffs/Same/Context/Minor/Rules/Format/Copy ~48px, grouped) vs ours single text line y114 `[Swap Sides] [Reload...]` — no icon hit-targets, no filter grouping.
2. Path rows: BC full path + dropdown + folder picker + per-pane `timestamp | bytes | rules` aligned; ours 380x24 boxes + `Browse...`, meta y100 12px dim, no dropdown affordance width parity.
3. Gutter: BC per-pane line-number gutter + center diff connectors; ours 16px right-edge ruler x936 (`GUTTER_X`) + 4px tall segments, too coarse, wrong side for text compare.
4. Row height/type: ours `ROW_H` 24 fixed headless pixel-font, no antialias/baseline; BC system font ~13px, proper leading, line numbers right-aligned dim.
5. Intra-line: BC blue text + light select + red BETA char; ours full-row bands (changed yellow/deleted red/inserted green) + inverted word blocks — harsh, no underline style.
6. Tabs: BC session tabs with file icons + close X (`t_left.txt`, `left<->right`, `tab_left.csv`); ours 32px text strip, active 60x24 block, no close/icons.
7. Status: BC `cursor | format | Same/Insert | Load` with encoding; ours 30px strip y570 dividers 240/480/720 arbitrary, dim 1px font, no encoding flag.
8. Folder columns: BC tree + size/time aligned columns; ours single column rows, counts line wraps, no size/time grid.
9. Home: ours 306x100 cards x9 + 224x28 CTA (3-col); BC session manager list + toolbar paradigm — different IA, not just skin.
10. Chrome/scale: BC native retina 2880x1800 titlebar; ours fixed 960x600 headless, traffic-light rects, dark-frozen palette only, no HiDPI.

## Open defect (no fix per owns-rule)
- `crates/gui/src/views_hex.rs:3586`: `TABLE_CELL_PAD_X` unresolved; existing `HEX_CELL_PAD_X` at line 1915. Needs owner fix + `cargo test -p autocompare-gui`.

## Files
- cli-01..10, gui-*-960x600.png/.out, tests-cli.out, tests-gui.out
