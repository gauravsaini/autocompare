# Study: what autocompare rebuilds

This is the clean-room study record behind autocompare, following the
study-then-rebuild methodology: observe what a user can see, never look
inside the original, write down evidence with honest confidence, and
rebuild from original work only.

## Method

The original application was observed **only from the outside**: its
installed bundle metadata and the user-visible behavior of the
file-comparison product class it belongs to.

Never done, and never to be done for this project: decompilation,
disassembly, reading the app's resources, strings or assets, copying its
names, icons, wording or code. Everything in autocompare is original
work.

Confidence levels: **HIGH** = directly observed this session;
**MEDIUM** = expected of the original from the product class, not
independently re-verified this session;
**UNVERIFIED** = planned verification.

## Directly observed (HIGH, 2026-11-18)

| Observation | Source |
|---|---|
| Original installed as a native macOS app, version 5.2.6.32774, bundle id `com.ScooterSoftware.BeyondCompare`, © 1996–2026 | bundle `Info.plist` |
| Registers itself as an **editor for all file types, folders and disks** (OSTypes `fold`, `disk`, `****`) | bundle `Info.plist`, `CFBundleDocumentTypes` |
| Own document types: settings packages (`.bcpkg`) and snapshots (`.bcss`) | bundle `Info.plist` |
| Localizations: de, en, fr, ja, zh_cn | bundle `Info.plist` |
| Ships a help book; executable is `BCompare`; runs with `LC_CTYPE=UTF-8` | bundle `Info.plist` |

## User-visible capability model (MEDIUM unless noted)

The original's session types, directly observed 2026-11-18 by
enumerating the running app's own accessibility UI (Session → New
Session menu): **Folder Compare, Folder Merge, Folder Sync, Text
Compare, Text Merge, Hex Compare, Media Compare, Picture Compare,
Table Compare** — observed, not assumed. (Registry and version
compares are not offered by this macOS build's menu.)

The following are expected behavior of the product class, still
MEDIUM pending a visual pass (see "Verification status"):

1. Text compare: two side-by-side panes; equal lines aligned; changed
   lines flagged with a background; the changed characters inside a
   changed line flagged more strongly; lines present on only one side
   shown as orphans with the other side blank; line numbers; next /
   previous difference navigation; content is editable.
2. Folder compare: recursive listing of two trees; per-node status
   (same, newer left, newer right, different, one-sided); size and date
   columns; name filters; copy/sync actions.
3. Hex compare: aligned byte view in hex and ASCII, differing bytes
   flagged.
4. Merge: base / left / right versions; non-conflicting edits applied;
   conflicts surfaced for manual resolution.

## What autocompare 0.1 rebuilds — and what it does not

| Original capability | autocompare 0.1 | Note |
|---|---|---|
| Text compare (side-by-side, inline highlights) | `autocompare text` | terminal rendering; `--ignore-case`, `--ignore-space-change`, `--width` |
| Unified patch view | `autocompare text --unified` | 3 lines of context |
| Folder compare (recursive, statuses, sizes, dates) | `autocompare folder` | `--content` byte compare, `--ignore-case` name matching, `--include`/`--exclude` globs, `--depth` cap |
| Hex compare | `autocompare hex` | 8-byte rows, differing bytes accented, `--max-hunks` |
| Three-way text merge | `autocompare merge` | git-style conflict markers; no merge UI |
| Table compare | `autocompare table` | cell-level `[old => new]` deltas, quoted fields |
| Folder sync | `autocompare sync` | dry-run default; `--apply` executes; prune; never overwrites a newer destination |
| GUI | not rebuilt | headless-first per the rebuild methodology; the CLI is the client |
| Folder merge, media and picture compares | not rebuilt | roadmap (all observed in the original's session menu) |
| Names, icons, wording | deliberately original | clean-room rule |

## Design decisions where the clone differs on purpose

- Exit codes follow the diff(1) convention — 0 same, 1 differences,
  2 error — instead of a GUI flow.
- Honest limits: an edit-budget fallback in the line diff, depth and
  hunk caps, `exact` / `truncated` flags surfaced to the user with a
  note. The original presents a polished GUI; a terminal clone states
  its limits.
- Colors: deleted red, inserted green, changed yellow — chosen for
  terminal legibility, not a copy of the original's palette.
- Folder comparison defaults to the size + modified-time model (the
  class default); `--content` switches to byte comparison.

## Verification status

- [x] Workspace compiles; `clippy -D warnings` green at scaffold
      (2026-11-18).
- [x] Line diff replay + minimality tests green (33 tests; five
      exhaustive release-mode sweeps over ~430k pairs verified replay,
      canonical form and minimal edit count).
- [x] Folder / binary / hex fixture tests green (39 tests).
- [x] Live sanity runs against real sample trees via the CLI
      (2026-11-18): all six folder statuses, `--content` flip from
      newer→different, identical-inputs exit 0, differences exit 1,
      missing path exit 2, hex hunk at the right offset, clean and
      conflicted merges with exact marker lines.
- [x] Session-type list re-verified against the running original
      (2026-11-18) via the macOS accessibility API (the app's own
      Session → New Session menu): Folder Compare, Folder Merge,
      Folder Sync, Text Compare, Text Merge, Hex Compare, Media
      Compare, Picture Compare, Table Compare.
- [ ] Visual pass (screenshots of pane layout, colors and navigation):
      attempted 2026-11-18 and **blocked by the macOS screen-recording
      permission** for this terminal context (`screencapture` fails
      with "could not create image"). Remediation: grant Screen
      Recording to the terminal app in System Settings → Privacy &
      Security, or capture manually. The MEDIUM items above stay
      MEDIUM until that pass happens.
