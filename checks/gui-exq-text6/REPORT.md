# TEXT6 viewport report (round-1, baseline PASS)

BC gap note: BC 888K vs ours ~10K. BC viewport gora fill, unchi rows, neeli inline
accent, poori-row selection. Ours dark theme me kala void + chhoti rows dikhi.

Is wave me koi paint edit nahi — baseline lock kiya taaki agla wave nape-tule
geometry par lage. Dekha gaya:

- Line numbers (1-6) dono panes me sahi, gutter right-aligned fixed width.
- Selected-row fill per-pane padded labels par poori pane chaudi (CHANGED_BG reuse).
- Viewport fill structure me hai (trailing hatched blanks `panes_h` tak), par dark
  theme + TRANSPARENT bg par kala void lagta hai; hatch ticks `dim` me bahut halke.
- Row rhythm: measured mono + ROW_PAD (2.0); BC se patla padta hai.
- Typo pehle se TypoRole par hai (Mono 12 regular, Small/Body chrome) — naya size nahi.

Agla wave (geometry first): ROW_PAD saans, trailing fill me gutter continuity,
center-gutter selection band. Colors frozen, koi naya Color32 nahi.
