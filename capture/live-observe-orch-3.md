# Live observe orch-3 — bottom waste root cause (clean-room, outside-only)

## Symptom
- own 9765B PNG me panes ke neeche ~50% black waste, BC me full-height panes + status bar
- confidence HIGH (screenshot), MEDIUM (code pointers), UNVERIFIED (live resize)

## Pointers (no edit, for OWNS owners)
- app.rs:5505ff Text tab paints paths + meta + find bar, then show_split_content + views::paint_text_with_find
- views.rs:2261 avail_h = available_height().max(100.0), views.rs:3409 same, views.rs:5767 ScrollArea::both, views.rs:6217 available_height
- hypothesis: panes row-count * row-height se height lete hain, remaining available_height fill nahi karte, isliye waste
- fix direction (exq8 OWNS views.rs/viewmodel.rs): panes ko remaining height fill karna, split 20-80% draggable, center gutter arrows, 960x600 + narrow-720 wrap, zero clipped

## Status
- 4 waves live, exq8 owns fix, home1 owns home/tree, uxkey theme, clivcs2 cli, exq7 WIP preserved
