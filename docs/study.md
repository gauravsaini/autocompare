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

## User-visible capability model (MEDIUM)

A user of this product class sees the following in the original
(expected behavior of the class, to be re-verified against the running
app in a later pass — see "Verification status"):

1. Session types: text compare, three-way text merge, folder compare,
   folder sync, table compare, hex compare, media compare, registry
   compare, version compare.
2. Text compare: two side-by-side panes; equal lines aligned; changed
   lines flagged with a background; the changed characters inside a
   changed line flagged more strongly; lines present on only one side
   shown as orphans with the other side blank; line numbers; next /
   previous difference navigation; content is editable.
3. Folder compare: recursive listing of two trees; per-node status
   (same, newer left, newer right, different, one-sided); size and date
   columns; name filters; copy/sync actions.
4. Hex compare: aligned byte view in hex and ASCII, differing bytes
   flagged.
5. Merge: base / left / right versions; non-conflicting edits applied;
   conflicts surfaced for manual resolution.

## What autocompare 0.1 rebuilds — and what it does not

| Original capability | autocompare 0.1 | Note |
|---|---|---|
| Text compare (side-by-side, inline highlights) | `autocompare text` | terminal rendering; `--ignore-case`, `--ignore-space-change`, `--width` |
| Unified patch view | `autocompare text --unified` | 3 lines of context |
| Folder compare (recursive, statuses, sizes, dates) | `autocompare folder` | `--content` byte compare, `--ignore-case` name matching, `--include`/`--exclude` globs, `--depth` cap |
| Hex compare | `autocompare hex` | 8-byte rows, differing bytes accented, `--max-hunks` |
| Three-way text merge | `autocompare merge` | git-style conflict markers; no merge UI |
| GUI | not rebuilt | headless-first per the rebuild methodology; the CLI is the client |
| Table / registry / media / version compares, sync actions, snapshots, scripting | not rebuilt | roadmap |
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
- [ ] Re-verify the MEDIUM items above against the running original
      (screenshot pass). Not done this session; until then those items
      stay MEDIUM.
