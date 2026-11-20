# Round 3 — Layout geometry aur Search/nav (2026-11-20)

## Method (outside-only)
Front text-compare window ki AX position/size + top-level AX children,
aur `Search` menu listing. Screenshot nahi (screen-recording permission
blocked, pehle se noted). Sab andaze screen pixels me.

## Observation (apne words me)
- Window title pattern: `left <--> right - Text Compare` (dobara HIGH).
- Front window: origin lagbhag (720,31), size lagbhag 720 chaudai x
  869 unchai. Matlab aadhi-screen tolerable compare window.
- Window ke top-level AX bachche: 4 button + 1 static text — ek patli
  toolbar patti (button unchai ~30px) jisme agla/pichhla farq jaise
  navigation control hain, uske neeche do barabar pane.
- Andaza geometry: title-bar ~28px, toolbar patti ~30px, status patti
  neeche ~22px, beech me do pane har ek ~340px chaudai, darmiyan me
  patla splitter (~6px) aur line-number gutter (~40px). Ye andaze hain,
  nape nahi (UNVERIFIED).
- Search menu: agla/pichhla farq, farq-section jump, edit jump,
  find/replace, go-to-line, bookmark lagao/jump/saaf — matlab farq-dar-
  farq navigation aur bookmark model app me maujood hai.

## Confidence: window size/title HIGH; pane/gutter/statusbar andaze UNVERIFIED.

## Fixture idea
`nav-bookmarks.seq`: 5-hunk text pair par probe jo hunk-dar-hunk
aage/pichhe jump kare aur hunk index report kare (next/prev section
ka console analogue). `geometry-smoke.txt`: GUI --screenshot run karke
sirf PNG size/aspect check (pane barabar-bate hue hain ya nahi).
