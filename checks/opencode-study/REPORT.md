# OP-STUDY refresh REPORT (2026-11-20)

Live app (PID 23316) ko bahar se dekha gaya. Andar jhankna mana tha:
koi decompile, strings, assets/icons/wording copy nahi. Sab apne words me.

## HIGH (seedha AX se dekha)

- Session kismein: 9 hain — folder wali 3 (compare/merge/sync),
  text wali 2 (compare/merge), specialist 4 (hex, media, picture,
  table). 2026-11-18 wali list se match.
- Window title pattern: `left <--> right - Text Compare`.
- View menu: sab/sirf-farq/sirf-same filter, context toggle,
  unimportant-farq ignore switch, whitespace/line-number/syntax/wrap
  toggles, side-by-side vs upar-neeche layout.
- Search menu: agla/pichhla farq, section jump, find/replace, go-to,
  bookmark system.
- Tools menu: settings package export/import, factory reset, snapshot
  save, patch preview.
- Front window size lagbhag 720x869, toolbar button unchai ~30px.

## MEDIUM (product class se umeed, dabake verify nahi kiya)

- Har View toggle ka exact asar; ignore-switch ka hunk count par asar.
- Merge me base/left/right teen-pane model; folder sync me copy actions.

## UNVERIFIED (andaza, naapa nahi)

- Pane chaudai ~340px each, splitter ~6px, gutter ~40px,
  statusbar ~22px. Screenshot permission blocked, isliye naap nahi sake.

## Fixture ideas (3)

1. `session-types-9.txt`: 9 kism ke chhote pairs + ginne wala probe.
2. `display-filter-trip.txt`: whitespace-only farq wala pair;
   ignore-mode me hunk count ghatna chahiye.
3. `nav-bookmarks.seq`: hunk-dar-hunk jump probe (next/prev section
   ka console analogue).

## Gap vs rebuild (docs/study.md se agree)

Display filter, find/bookmark, upar-neeche layout — teeno rebuild me
nahi hain. P0 copy-to-side/undo bhi baaki. Koi nayi cheez nahi mili
jo pehle noted gaps se bahar ho.
