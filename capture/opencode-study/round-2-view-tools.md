# Round 2 — View filters aur Tools (2026-11-20)

## Method (outside-only)
`View` aur `Tools` top menu ki AX item listing, shabdon ko paraphrase
karke likha (exact labels copy nahi kiye).

## Observation (apne words me)
- View menu me teen display filter hain: sab dikhao / sirf farq /
  sirf same, plus aas-paas context lines ka toggle.
- Ek "unimportant farq nazar-andaz karo" ka master switch hai, aur
  ek alag toggle jo nazar-andaz shuda lines ko dikhaye/chhupaye.
- Pane sajawat toggles: khaali-jagah nishan, line numbers, rang-sangat
  (syntax), line wrap, side-by-side vs upar-neeche layout, thumbnail,
  ruler, file info patti, toolbar show/hide.
- Tools menu me settings package export/import, factory reset, folder
  snapshot save, text file edit, aur patch-style preview milta hai.

## Confidence: HIGH (menu maujoodgi), MEDIUM (har toggle ka exact asar —
daba ke dekha nahi)

## Fixture idea
`display-filter-trip.txt`: 10-line pair jisme 2 lines whitespace-only
farq hon; probe: default run me 3 farq-hunks, "ignore-unimportant"
mode me 1 farq-hunk. `patch-preview.diff`: text compare ka unified
output golden file se milao.
