# UI Feedback Brief (2026-11-20) — user: "UI horrible, comparable product se bahut far"

Source: live outside-observation of BCompare (PID 23316) via System Events AX, HIGH confidence.
Our GUI reference: checks/release-smoke/gui-smoke.png (release binary render).

## BC Text Compare — what exists (all HIGH, just observed)
- View menu: Show All / Differences / Same / Context filters, Ignore Unimportant,
  Whitespace + Line Numbers + Syntax Highlighting + Word Wrap toggles,
  Side-by-Side AND Over-Under layouts, Thumbnail, Text Details, Ruler, File Info.
- Edit menu: live editing (Undo/Redo, Cut/Copy/Paste), Copy to Right / Copy Line
  to Right, Align With, Isolate, indent controls.
- Search menu: Next/Prev Difference AND Difference Section, Find/Replace,
  Go To, Bookmarks (toggle/goto/clear).
- Session menu: New Tab/Window, Open/Save Session, Workspaces, Locked, Swap
  Sides, Reload/Recompare.
- Window title carries session identity ("left <--> right - Text Compare").

## Our GUI gaps (ordered by user-visible impact)
1. Read-only panes — no editing, no Copy to Right/Left, no Undo. (Biggest feel gap.)
2. No diff navigation — no Next/Prev Difference, no section stepper.
3. No display filters — no Show Differences-Only / Context views.
4. No Find/Replace, no Go To, no bookmarks.
5. Layout single fixed side-by-side — no Over-Under, no thumbnail toggle,
   no word-wrap/line-number toggles wired to UI.
6. No session lifecycle — no Save/Open Session, no workspace, no Swap Sides.
7. AX tree near-empty (egui custom draw) — same as BC's custom views, so
   parity is acceptable here; skip AX work (YAGNI).

## Proposed fix order (for Wave-C / next GUI leaf)
- P0: Copy Section Right/Left + Next/Prev Difference nav (makes it feel like a tool, not a picture).
- P1: Display-filter segmented control (All / Diffs / Same / Context) + Find box.
- P2: Over-Under layout toggle + word-wrap + line-numbers toggles.
- P3: Session Save/Open + Swap Sides + Reload.
