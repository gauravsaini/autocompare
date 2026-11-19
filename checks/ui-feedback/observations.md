# Raw UI Observations (2026-11-20)

Observed via macOS System Events AX on Beyond Compare (PID 23316).

## Menus & Actions
- **Baithak (Session):** Tab/Window lifecycle, Session Persistence (Open/Save), Environment/Workspace settings, Side Swapping, Refresh/Re-evaluate.
- **Nazara (View):** Filter toggles (All/Changes/Unchanged/Context), Minor difference masking, Invisible character toggles, Line count display, Syntax coloring, Content wrapping, Pane layouts (Side-by-side, Vertical stack), Navigation aids (Thumbnail, Ruler, Info bars).
- **Sampadan (Edit):** Manipulation history (Undo/Redo), Clipboard actions, Transfer to opposite side (Section/Line), Manual alignment, Isolation.
- **Khoj (Search):** Change navigation (Forward/Back), Text location (Find/Replace), Jump to line, Marker system (Bookmarks).
- **Auzaar (Tools):** External utility access.

## Interface Elements
- Window title format: "left <--> right - Text Compare"
- Text Compare View Options: Side-by-side, Over-under.

## Observed Gaps in autocompare-gui
- P0: Panes are Read-only (no editing/undo/copy-to-side).
- P0: No difference navigation (Next/Prev).
- P1: No display filters (Show Diffs only, etc).
- P1: No Find/Replace/GoTo/Bookmarks.
- P2: Fixed Side-by-side layout (no Over-under/Word-wrap/Line-numbers toggles).
- P3: No session management (Save/Open/Swap Sides).
