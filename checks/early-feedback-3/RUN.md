# RUN — early-feedback-3 local run (2026-11-20)

Scope: read-only smoke in `/Users/ektasaini/autocompare`. No `crates/**` edits.
`CARGO_BUILD_JOBS=2` for all cargo invocations. No push.

## 1. Commands + outputs

### C1 — CLI top help (10 subcommands)
Command:
```
CARGO_BUILD_JOBS=2 cargo run -p autocompare-cli -- --help
```
Result: exit 0. `Finished dev profile`, `Running target/debug/autocompare --help`.
Usage: `autocompare [OPTIONS] [COMMAND]`. 10/10 commands present:

1. `text` — Compare two text files line by line, side by side
2. `folder` — Compare two folder trees recursively by size and modified time
3. `hex` — Compare two files byte by byte
4. `merge` — Merge two files that share a common base version
5. `table` — Compare two delimited (CSV-style) tables cell by cell
6. `sync` — Plan (and optionally apply) a folder sync
7. `picture` — Compare two PNG images pixel by pixel
8. `folder-merge` — Plan (and optionally apply) a three-way folder merge
9. `audio` — Compare two audio files (WAV, MP3) sample by sample
10. `session` — Load and run a saved comparison session

Top flags present: `--color`, `--report`, `--ro/--ro1/--ro2`, `--automerge`,
`--mergeoutput`, `--vcs1..4`, `--title1..4`, `--session`, `--save-session`,
`--exit-code-mode (diff/bc)`, `--bc-exit-code`, `--report-title`, `--solo`,
`--nobackups`, `--reviewconflicts`, `-h/--help`, `-V/--version`.

### C2 — GUI screenshot (960x600 valid PNG)
Command:
```
CARGO_BUILD_JOBS=2 cargo run -p autocompare-gui -- --screenshot checks/early-feedback-3/gui.png
```
Result: exit 0. `Finished dev profile`, `Running target/debug/autocompare-gui --screenshot ...`.
File: `checks/early-feedback-3/gui.png` — 9765 bytes, `file` = `PNG image data, 960 x 600, 8-bit/color RGBA`,
magic `89504e470d0a1a0a` ok, Python struct width x height = `960x600`.
sha256: `bde8ccf481c6445b8e36dd7a3308d1c647b2bd0bae8dee578fc86472f61815dc`.
`cmp -s gui.png fixture-text.png` → BYTE-IDENTICAL (default route = text fixture).

### C3 — workspace tests summary
Command:
```
CARGO_BUILD_JOBS=2 cargo test --workspace
```
Result: all suites ok, 0 failures.
Aggregate (grep `^test result:`): **46 suites, 1174 passed, 0 failed, 5 ignored**.
Largest suite: `autocompare-gui` unittests — 703 passed.
Notable others: `autocompare-diff` 65, `audio` 44, `table` 38, `api` 33 (5 ignored).
Full per-suite lines retained in shell history; no failures to triage.

## 2. TOP-10 gaps vs Beyond Compare 5 (geometry/typography only)

NOTE: colors are FROZEN — not evaluated, no color deltas listed by design.
Anchors are own-code locations (file+line at run time via grep); BC side is
paraphrased from `docs/study.md` UI Observation & Parity (HIGH, outside-only).

1. Over-under layout missing — `crates/gui/src/views.rs:75` (`SPLIT_HANDLE_W 8.0`, `SPLIT_VISUAL_W 1.0`).
   Fixed side-by-side only. Expected: side-by-side / over-under switch reflows panes + splitter hit rect stays centered.
2. Thumbnail/minimap click-jump missing — `crates/gui/src/views.rs:91-95` (`HEX_MINIMAP_W = 2*GUTTER_W`, `HEX_MINIMAP_H 220.0`), `crates/gui/src/gutter.rs:41` (`GUTTER_SEGMENT_MIN_H = SPACE_XS 4.0`).
   Right edge is a thin ruler, not a full-height clickable map. Expected: full-height overview, click jumps, 4px bands stay separable.
3. Diff-to-diff nav + count absent — `crates/gui/src/views.rs:404-433` (toolbar has `Swap Sides` U+2194 + `Reload` glyph only).
   Expected: prev/next-difference buttons + `3/12` count label in session toolbar.
4. Display filter row absent — `crates/gui/src/app.rs:2626` area (folder filtered-counts only), `views.rs:411` toolbar.
   Expected: All / Differences / Same / Context segmented control per session toolbar, reflowing the row.
5. Find/replace + bookmarks strip absent — `crates/gui/src/app.rs:640` area (text split/filter, no search strip).
   Expected: Ctrl+F bar geometry (field + prev/next + replace + bookmark gutter markers).
6. Copy-to-side affordance absent in center gutter — `crates/gui/src/views.rs:97-107` (`TEXT_CENTER_GUTTER_W = HEADER_H + GUTTER_W + 2*TIGHT_GAP`).
   Gutter paints but holds no per-row arrow hit targets. Expected: per-diff arrow buttons + Undo/Redo toolbar state.
7. Wrap / line-numbers / whitespace toggles not wired to paint — `crates/gui/src/theme.rs:72-74` (`TYPO_WRAP_PANE_CHARS 80`, `TYPO_FULL_PANE_CHARS 200`), `views.rs:561-565` (ruler locked to `gutter_bar_width_for_font(TYPO_MONO_SIZE 12.0, …)`).
   Expected: toggles change wrap width, ruler width (16 wide / 8 narrow), and row breaks live.
8. Flat type scale, zero tracking — `crates/gui/src/theme.rs:58-77` (heading 18 strong / body 13 / small 11 weak / mono 12 regular / home-title 14; `TYPO_LETTER_SPACING 0.0`; weights 400/500/600).
   Hierarchy rides on size alone; chrome/tab/status share body/small. Expected: distinct heading/body/gutter steps + tabular numerals in ruler/gutter.
9. Home grid breathes poorly — `crates/gui/src/views.rs:31-32,372-399` (`HOME_GRID_COLUMNS`, `HOME_GRID_GAP`, `HOME_SECTION_GAP`; card `~306.67px`, CTA `224x28`, min-h `99.6`; `SESSION_TAB_MAX_W 120.0`, `TAB_BOTTOM_GAP 6.0`).
   Text-only cards, cramped tabs. Expected: icon-led cards, airier grid/section gaps, wider tab max.
10. Folder-tree indent + recent-row rhythm weak — `crates/gui/src/gutter.rs:26-47` (ruler 16/8, bar 12/6, `GUTTER_HIT_SLOP = GUTTER_MERGE_GAP 2.0`), `views.rs:700` (`RECENT_COL_GAP/RECENT_ROW_GAP`), `views.rs:126-128` (`row_height = line_h + ROW_PAD 2.0`).
    Deep nesting + `>`/`=` markers scan poorly. Expected: deeper indent steps, branch rules, two-line recent rows with clearer 2px-pad rhythm.

## 3. Verify (no push)
```
git status --short -- checks/early-feedback-3/
ls -l checks/early-feedback-3/gui.png && file checks/early-feedback-3/gui.png
git add checks/early-feedback-3/RUN.md checks/early-feedback-3/gui.png
git commit -m "checks(early-feedback-3): local run evidence" -- checks/early-feedback-3/
git status; git log --oneline -2   # push: never
```
