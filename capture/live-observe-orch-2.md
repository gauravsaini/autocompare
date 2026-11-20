# Live observe orch-2 — 2026-11-20 22:22 AEDT (clean-room, outside-only)

## Sources
- own: /tmp/orch-geo-check.png 9765B magic-ok deterministic via cached gui
- BC: /tmp/bc-screen.png 888K fullscreen, 3 sessions open (t_left vs t_right, left vs right, tab csv)
- confidence: HIGH for observed layout, MEDIUM for behavior, UNVERIFIED for unopened panes

## Geometry gap (width/height/margins/padding/alignment)
- own top tabs: 9 static text tabs, no close, no multi-session tabs; BC: closable session tabs + Home/Sessions toolbar
- own toolbar: single checkbox row; BC: 2 rows (icon actions + path dropdowns + file meta date/bytes/rule)
- own panes: no center gutter, no copy arrows; BC: center divider with arrows
- own body: bottom 50% empty black; BC: full-height panes + status bar (1:1, Default text, Same/Insert)
- own split: not visibly draggable 20-80%; BC: draggable split

## Typography gap
- own: single weight, cramped line-height, gutter vs content same; BC: mono content, muted gutter, 1.4+ leading
- own intra-line highlight rects misaligned; BC: blue text on light purple, aligned

## Plan
- exq7 hex+gutter, exq8 textview (views.rs/viewmodel.rs), uxkey theme/keymap, clivcs2 CLI/VCS — all live, colors frozen
