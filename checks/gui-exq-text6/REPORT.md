# TEXT6 viewport report (round-1, spans-plus-filler PASS)

BC gap note: BC 888K vs ours ~10K. BC viewport gora fill, unchi rows, neeli inline
accent, poori-row selection. Ours dark theme me kala void + chhoti rows dikhi.

Is wave me text-side pure helpers (koi paint rect/color edit nahi):

- Intra-line span pads symmetric: total SPACE_XS (4.0), half per side (2.0).
- Span underline weight via TYPO tokens: regular 1px, semibold 2px.
- Filler row height Mono line-height par lock (15.6, bina ROW_PAD).
- Left/right row alignment 960 par: 448 + 448 + 48 + 2 * 8 = 960.
- Spans char boundaries par clamp, refined word span (18, 27) poora.

Row rhythm: measured mono + ROW_PAD (2.0); BC se patla padta hai.
Typo pehle se TypoRole par hai (Mono 12 regular, Small/Body chrome) — naya size nahi.
Agla wave (geometry first): ROW_PAD saans, trailing fill me gutter continuity,
center-gutter selection band. Colors frozen, koi naya Color32 nahi.
