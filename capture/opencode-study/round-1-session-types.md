# Round 1 — Session types (2026-11-20)

## Method (outside-only)
Running app (PID 23316) ki accessibility menu tree `Session > New Session`
submenu se ginwayi gayi list. Koi binary/resource read nahi, sirf AX names.

## Observation (apne words me)
Nayi compare shuru karne ke liye app 9 kism ke session deti hai,
teen folder-family (compare / merge / sync), do text-family
(compare / merge), aur chaar specialist (hex, media, picture, table).

## Confidence: HIGH
Seedhi AX listing se, 2026-11-18 wali list se match karti hai.

## Fixture idea (parity-harness ke liye)
`session-types-9.txt`: har session kism ke liye chhota fixture pair
(text pair, hex pair, table csv pair, picture png pair) aur ek
`--list-sessions` jaisa console probe jo 9 kism gin ke exit 0 de.
Koi original wording copy nahi — sirf generic kism-naam.
