# Raw UI Observations (2026-11-20)

Observed via macOS System Events AX on the running original app (PID 23316).
Menu labels below are short functional facts (HIGH confidence); no prose,
strings, or assets copied.

## Menus (Text Compare session active)
- Session: New Tab / New Window, Open Session, Save Session / Save As,
  Session Settings, Locked, Workspaces (Load/Save), Swap Sides, Reload, Recompare.
- View: Show All, Show Differences, Show Same, Show Context,
  Ignore Unimportant Differences; toggles for Whitespace, Line Numbers,
  Syntax Highlighting, Word Wrap; Side-by-Side and Over-Under layouts;
  Thumbnail, Text Details, Hex Details, Alignment Details, Ruler, File Info.
- Edit: Undo, Redo, Align With, Isolate, Copy to Right, Copy Line to Right,
  indent controls, Cut/Copy/Paste, Select Section.
- Search: Next/Previous Difference, Next/Previous Difference Section,
  Next/Previous Edit, Find, Replace, Go To, Toggle/Go-to/Clear Bookmark.
- Tools: Export/Import Settings, Save Snapshot, Edit Text File, View Patch.

## Interface Elements
- Window title follows the `left <--> right - Text Compare` pattern.
- Status/footer and toolbar present; custom-drawn panes expose minimal AX
  (same constraint as our egui renderer — AX parity out of scope).

## Observed Gaps in autocompare-gui
- P0: Panes are read-only (no editing, undo, or copy-to-side).
- P0: No difference navigation (next/prev).
- P1: No display filters (all/diffs/same/context).
- P1: No find/replace, go-to, or bookmarks.
- P2: Fixed side-by-side layout (no over-under, wrap, or line-number toggles).
- P3: No session save/open, workspaces, or swap-sides.
