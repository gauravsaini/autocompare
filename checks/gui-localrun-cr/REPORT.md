# CR-LOCALRUN round-6 — cm round-5 se cr DELTA baseline + early feedback

## 1. Summary

CM round-5 (HEAD `9f3b576`, docs commit; code = CK `01dbf01` + CL `48946aa` samet) ke
baad is run me `crates/**`, `scripts/**`, `tools/**` me ZERO edits — sirf rebuild +
recapture + LIVE clicks. Natija:

- Fixtures 4/4 cm round-5 se byte-identical (sha256-12 MATCH, sizes same) — default-dark
  path stable hold karta hai.
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-localrun-cr/round-6/` exit 0,
  `RESULT: PASS` (signals cm jaisi exact: Grid x14, mono x50, strong x26, small x52).
- Live app DARK khulti hai (CK fix hold), frame pos 320,91 size 800x632 exact.
  Teenon mode tabs LIVE khule: Text / Folder / Table (titles samet verify).
- Session strip 3 pills tak pahunchi: "Text 0 / Folder 1 / Table 2(active)" (CG adopt hold).
- Toolbar abhi bhi text-only buttons hai (CL fn ready, App call-site purana row) — SAME/open.
- Tests 671 passed, 0 failed (cm me 670 the; +1 beech ki landed waves ka, is run me zero touch).
- Timeline caveat: CP-GEO (`353130a` views.rs, 13:44:44) aur CQ-TYPO (`183d00c` theme.rs,
  13:42:05) is run ke BEECH me land hue. Mera binary exec 13:42:00 ka hai — CP code usme
  ho hi nahi sakta (commit exec ke baad). CQ paint-neutral hai (koi naya size/color/geometry
  nahi, koi call-site adopt nahi), isliye live pixels par uska asar zero. Shots pure
  cm-code baseline hain. Dono waves ka verify round-7 scope me rahega.
- Sec-8 me user feedback ("UI horrible hai, comparable product se far hai") par EXQUISITE
  GAPS list kiye — ye CP/CQ agle round ka input hai.

## 2. Live run spec

- Launch HEAD `5593c02` (STATUS-only commit; code `9f3b576` = cm round-5 wala). Run ke dauran
  HEAD `1bd6213` tak badha (sibling landings) — mera tree clean raha, sirf naya dir.
- Build: `cargo build -p autocompare-gui` 13:39:09 green (cached, 0.26s). Harness 13:39:18
  exit 0, `RESULT: PASS`.
- Headless fixtures 13:41: `--screenshot` filename-keyed scenes (home/text/folder/table),
  sab 960x600, exit 0, screenshot-*.out khali.
- Live: ek instance (PID 71657), launch 13:42:00. Window frame 320,91 800x632 — cm jaisa exact.
- Clicks: /tmp/clicker (CGSessionEventTap). Text {412,212}, Folder {458,212}, Table {615,212} —
  teeno kaam kiye (title bar se confirm). osascript clicks is env me BLOCKED (-25211) rehte hain.
- BC: observe-only — window-id 1394 se `screencapture -l` (sahi BC pixels, zero focus change,
  zero click). Title: `t_left.txt <--> t_right.txt - Text Compare` confirm. Note: `-l` is
  machine par KAAM karta hai (numeric window id ke sath) — CQ note ke ulat.
- Tests: `cargo test -p autocompare-gui` — 671 passed, 0 failed (`tests-full.out` me full log).
- Cleanup: apna PID 71657 kill kiya (gone confirm), probe PNGs delete kiye (`/tmp/cr_probe_*`,
  `/tmp/cm_*` ko haath nahi lagaya — the hi nahi; sibling processes untouched).

## 3. Shot map (sab round-6/ me, `file` magic ok, Gate 1 me sizes/dims)

| File | Dims | Bytes | Kya hai |
|---|---|---|---|
| live-desktop.png | 2880x1800 | 726552 | Full desktop: Table window front (320,91 800x632), user browser peeche |
| live-home.png | 1600x1264 (@2x) | 286482 | Home, DARK (cm 286481 se +1 B — desktop-state noise) |
| live-text.png | 1600x1264 | 149736 | Text tab LIVE: "Text 0" pill, path fields + Browse, pathmeta row, Find bar, empty-state line |
| live-folder.png | 1600x1264 | 148293 | Folder tab LIVE: 2 pills, folder path fields, status line |
| live-table.png | 1600x1264 | 176112 | Table tab LIVE: 3 pills, Keys/Tolerance/Find rows, status lines |
| live-strip.png | 1600x160 | 33796 | Strip close-up: Text 0 x / Folder 1 x / Table 2(active) + mode tabs |
| live-toolbar.png | 1600x260 | 50154 | Toolbar close-up: text buttons, separator, toggles — koi glyph nahi |
| live-bc.png | 1532x1830 (-l1394) | 587537 | BC Text Compare, cm se byte-size EXACT same |
| fixture-home/text/folder/table.png | 960x600 | 15007/9765/10149/9996 | Headless scenes, 4/4 cm se MATCH |
| after.png | 960x600 | 9765 | Harness output (cm/CE baseline jaisa) |
| actions.log | — | — | Har click/capture/incident ka timestamp log |
| cargo-run.log | — | 0 | Direct launch log (GUI stdout silent) |
| tests.out | — | 849 | Harness quiet log; tests-full.out (46058 B) me full 671-green log |
| geo-typo-report.md | — | 374 | Harness report, `RESULT: PASS` |

## 4. Fixture byte DELTA (cm round-5 vs cr round-6, sha256-12)

| File | cm round-5 | cr round-6 | Match |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH |
| after.png | 9765 B | 9765 B | MATCH |

Live sizes cm -> cr (same size-class, sub-1% render noise): home 286481 -> 286482 (+1);
text 148930 -> 149736 (+806); folder 147281 -> 148293 (+1012); table 175205 -> 176112 (+907);
strip 34103 -> 33796 (-307); toolbar 47623 -> 50154 (+2531, Table-tab context me zyada controls);
bc 587537 -> 587537 (EXACT); desktop 775487 -> 726552 (desktop state alag, expected).

## 5. Gap DELTA table — cm round-5 vs cr round-6

### Item 1 — live dark

| Check | cm round-5 | cr round-6 | Verdict |
|---|---|---|---|
| Live dark | DARK khulta tha (CK fix) | DARK khulta hai, same palette, "Light Mode" badge samet | PASS/hold — CK fix stable |

### Item 2 — Text tab live

| Check | cm round-5 | cr round-6 | Verdict |
|---|---|---|---|
| Text tab chrome | Session "Text 0", path fields, pathmeta, Find bar, status | Same chrome, title `(empty) <--> (empty) - Text` verify hua | PASS/hold |
| Content rows (accent + filler + status zones + cursor strip) | Panes EMPTY (input-block) | Panes EMPTY (same env block; keyboard/drag unreliable) | SAME/live-pending. Content proof fixture-text.png se (byte-stable) |
| Pathmeta row | Live visible | Live visible ("Text: exact \| wrap:off lines:on" + dropdown) | PASS/hold |

### Item 3 — multi-session strip

| Check | cm round-5 | cr round-6 | Verdict |
|---|---|---|---|
| Session strip | 3 pills (Text 0 / Folder 1 / Table 2 active) | Wahi 3 pills, live-strip.png close-up samet | PASS/hold — CG adopt stable |

### Item 4 — toolbar icon-parity (P2-1)

| Check | cm round-5 | cr round-6 | Verdict |
|---|---|---|---|
| Toolbar live | Text-only buttons, koi glyph nahi | Wahi: "Swap Sides" + "Reload / Recompare" plain text, separator, 3 checkboxes | SAME/open — CL follow-up abhi bhi khula hai |

### Item 5 — misc

| Check | Verdict |
|---|---|
| BC tulna | PASS (observe): live-bc.png cm se byte-size exact; changed row 2 blue band, gutter arrows, filler hatch, status zones sab dikhe |
| Light fixture artifact (`--screenshot-mode light`) | SAME/open — CLI abhi bhi sirf dark render karta hai |
| Tests | PASS — 671 green, zero fail (cm 670; +1 sibling waves ka, tree me unka code hai, mera touch zero) |
| Harness | PASS — `RESULT: PASS`, signals cm jaisi exact |

## 6. Incidents (follow-up ke liye)

1. Browser-occluder (13:43): click-1 {412,212} browser window par gaya (koi foreground browser
   region cover kar raha tha) — uske baad wala capture invalid nikla (browser address-bar
   dropdown, 497955 B). Fix: app ko System Events se frontmost kiya, click-2 retry se Text tab
   khula (title confirm), capture redo kiya (149736 B, sahi pixels). Invalid shot round me nahi
   rakha; actions.log me incident darj hai.
2. Keyboard/drag input abhi bhi env-block hai (CM sec-6 jaisa) — isliye content-row proof
   fixture se, live me chrome-only. Koi keystroke BC/app me galat jagah nahi gaya.
3. Silent-exit is round me NAHI dikha — PID 71657 launch (13:42:00) se kill tak alive raha.
   CM wala ~13:30 case dobara nahi hua.
4. `-l` note: `screencapture -x -l1394` sahi BC window pixels deta hai (587537 B, visually
   verified) — round-7 me obscured-window captures ke liye yahi rasta use ho sakta hai.

## 7. Geometry/typography signals (read-only, harness-first)

- Grid x14, monospace x50, strong x26, small x52, `.size(14.0)` — cm round-5 jaisa exact
  (harness 13:39 ka hai = pre-CP/CQ signals; CP/CQ verify round-7 scope).
- Width calls 28 (cm jaisa). Colors info-only 29 (cm jaisa exact — is run me zero touch).

## 8. EXQUISITE GAPS — user feedback input (CP/CQ agle round ke liye)

User ne kaha UI horrible hai aur comparable product (BC) se far hai. Apne live shots
(live-home/text/folder/table + strip/toolbar close-ups) ko live-bc.png se tulna karke
ye structural gaps mile (sab visual-pixel evidence par, koi code change nahi kiya):

### Geometry (CP-GEO input)

1. Chrome rows ka stack bahut lamba hai: pills row + mode-tabs row + session-label row
   (`(empty) <--> (empty) - Table`) + toolbar row — content se pehle 4 navigation rows.
   BC me ek tab bar + ek toolbar hai. Hamari pills row aur tabs row lagbhag same kaam karti
   dikhti hain (dono se session/tab badalta hai) — redundant vertical kharcha.
2. Toolbar buttons oversized hain: "Swap Sides" / "Reload / Recompare" wide text pills +
   teen checkboxes ek hi row me; BC ka toolbar compact icon groups me bata hai
   (Home/Sessions/All/Diffs/Same/Context/Minor/Rules/Format/Copy + nav-row). Hamare toolbar
   ke neeche khali jagah + path row me uneven widths dikhte hain.
3. Right-edge overflow: Text/Folder/Table teenon live shots me right path field window edge
   par kat raha hai (dayen kinare scrollbar sliver jhalak rahi hai) — content viewport se
   wide hai. BC me panes window me exactly fit hain.
4. Do status lines same baat kehti hain: inline hint ("no table results yet: type both...") +
   bottom bar ("table: no results yet | ready: ...") — ek hi message do jagah. BC ka single
   bottom status bar segmented zones me hai (Same | Insert | ...).
5. Table controls ki rhythm uneven hai: Keys row (0-7 + Clear keys) tight hai, Tolerance row
   (exact/0.01/0.10/1.00 + custom + Apply + 2 checkboxes) bheed wali hai, Find row usse halki —
   teenon rows ki height/density ek jaisi nahi lagti. BC ke control rows uniform height ke hain.
6. Khali-pan void: status ke neeche ~55% window khali kaala maidan hai (koi filler, minimap,
   placeholder structure nahi). BC khali jagah me bhi gutter + rulers + grid lines dikhata hai.
7. Strip pills left-aligned tairte hain, pill gaps bade hain, `x` buttons chhote hain — pill row
   aur tabs row ke beech alignment rishta kamzor lagta hai (dono rows ke left edges ek line
   par nahi aate).

### Typography (CQ-TYPO input)

1. Hierarchy flat hai: brand ("autocompare"), section label, field placeholder, status line —
   sab lagbhag ek hi size/weight me render hote hain. BC me titles bold-large, meta gray,
   content black/blue — teen level saaf dikhte hain. Hamare header me koi "bada" element nahi.
2. Label vs input confuse hota hai: "Find:"/"Keys:"/"Tolerance:" labels aur "needle"/"e.g. 0..."
   placeholders same rang-size me hain; BC me labels muted-gray aur values strong hain.
3. Status/hint text bahut dim hai: "(nothing to display: both sides are empty)" aur bottom bar
   ka contrast low hai (halka gray on near-black), door se padhna mushkil. BC ka status bar
   crisp segmented text me hai.
4. Line-height rhythm tuta hai: control rows ghani packed hain jabki sections ke beech bade
   gaps hain (path row -> pathmeta -> find -> void). BC uniform row heights + ruler separators
   se steady rehta hai.
5. Keys 0-7 proportional font me hain — ankde visually uneven chalte hain; BC ke line numbers
   right-aligned mono gutter me hain. Hamare numeric controls mono/tabular me zyada stable lagenge.
6. "Light Mode" pill (sun emoji + text) top-right me baseline se utra lagta hai aur title-bar
   state duplicate karta hai — noise jodta hai, hierarchy me madad nahi karta.

### BC tulna (live-bc.png se)

- BC: do baraabar panes + center gutter (yellow take-arrows), changed row dono taraf blue band,
  hatched filler, line-number gutters, upar icon toolbar (2 rows), neeche ruler + segmented
  status bar. Har zone ka kaam ek nazar me samajh aata hai.
- Hamara fixture-text.png wahi contract paint karta hai (changed band + accent spans + status
  counts + gutter segs) par sirf headless canvas me — live app me content pahunchte hi ye
  polish dikhegi. Chrome (toolbar/strip/status/type-rhythm) hi woh farak hai jo user ko
  "horrible" laga — content paint nahi.

## 9. Notes

- Is run me `crates/**`, `scripts/**`, `tools/**`, baaki checks ko touch nahi kiya — sirf
  `checks/gui-localrun-cr/` (naya untracked dir). STATUS files untouched (koi M entry nahi).
- BC window me koi click/edit/focus-change nahi — sirf window-id capture + title confirm.
- Probe PNGs (`/tmp/cr_probe_*`) round se delete kiye; actions.log me unka zikr rehne diya
  (audit trail). Sibling CP/CQ processes/files ko haath nahi lagaya.
- PUSH MANA tha — commit local rahega.
