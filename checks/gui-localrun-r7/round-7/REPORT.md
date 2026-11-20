# R7-LOCALRUN round-7 — CN-opener live content-row photo-proof + CO/CP/CQ live check

## 1. Summary

Round-6 (CR) ke baad tree me chaar waves land hue: CN-OPENER (`--open` flag),
CO-TBARCALL (toolbar call-site adopt), CP-LIGHTFLAG (`--screenshot-mode`),
aur pehle se maujood CP-GEO + CQ-TYPO. Is run me `crates/**` me ZERO edits —
sirf rebuild + `--open` launches + live captures + 3 session-tap clicks.

- Fixtures 4/4 round-6 se byte-identical (sha256-12 MATCH, sizes same) — default-dark
  path CN/CO/lightflag ke baad bhi stable.
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-localrun-r7/round-7/` exit 0,
  `RESULT: PASS` (Grid x14, mono x50, strong x26, small x52 — round-6 jaisa exact).
- CN-OPENER live-proof: `--open t_left.txt t_right.txt --tab text` se Text tab preload +
  auto-compare; live-open.png me 3 content rows (1 alpha/alpha, 2 beta/BETA gold accent
  + center-gutter take arrows, 3 gamma/gamma). Title bar me dono paths + "Text" confirm.
- Hatch live-proof: unequal pair (4 vs 2 lines) par live-hatch-content.png me filler side
  par hatch ticks (`||||||||`) + red insert band + take arrows — `paint_filler_hatch` live.
- Cursor strip live-proof: selection par focused preview line ("2:1 | 2 Changed" +
  beta/BETA mono lines; row-1 par "1:1 | 1 Same") — `paint_text_cursor_strip` live.
- Status 3-zone: left counts ("All: 3 rows (3 shown)") + mid verdict ("no selection" /
  "Same" / "Changed") live dikhe. Right nav-badge zone is state me khali (finding sec-6).
- Toolbar pixel-verify (CO): live-toolbar.png me `Swap Sides` par `<->` prefix aur
  `Reload / Recompare` par `▶` prefix live dikhe — glyph parity PASS.
- BC observe-only: live-bc.png round-6 se byte-IDENTICAL (sha `bfab...ba88`, 587537 B).
- Tests 674 passed, 0 failed (round-6 me 671 the; +2 CN opener +1 lightflag = 674 exact,
  is run me touch zero).
- Window frame dono launches me 320,91 800x632 — round-4/5/6 jaisa exact.

## 2. Live run spec

- Launch HEAD `e34f4f` (STATUS-only docs commit; code = CN + CO + lightflag + CP-GEO + CQ-TYPO).
- Build: `cargo build -p autocompare-gui` 14:19:10 green (6.47s; 14:00 wala binary stale tha,
  lightflag 14:01 ke baad ka — isliye fresh rebuild zaroori tha).
- Harness 14:19:21 exit 0, `RESULT: PASS`.
- Headless fixtures 14:19 (7 scenes): home/text/folder/table + accent + light + dark, sab
  960x600, exit 0, screenshot-*.out khali. Light 9728 B (dark se alag), dark 9765 B (stable).
- Live-1 (PID 95098): `--open <t_left> <t_right> --tab text`, 14:20:16 se kill tak alive.
  Frame 320,91 800x632. Title: `t_left.txt <--> t_right.txt - Text - autocompare` confirm.
- Clicks (sirf apne window par, /tmp/clicker session-tap): {387,486} text-label no-op,
  {460,458} row-1 gutter select, {435,412} Next diff-nav. osascript clicks is env me BLOCKED
  (-25211) rehte hain, isliye sirf read-only frame/title + clicker use hua.
- BC: observe-only — swift winlister (/tmp me, repo-touch zero) se id 1394 confirm, phir
  `screencapture -x -l1394` (zero focus change, zero click). Title
  `t_left.txt <--> t_right.txt - Text Compare` confirm.
- Live-2 hatch (PID 98700): `--open /tmp/r7_hatch_left.txt /tmp/r7_hatch_right.txt --tab text`
  (4 vs 2 lines), frame phir 320,91 800x632 exact. Capture ke baad kill (gone confirm).
- Tests: `cargo test -p autocompare-gui` — 674 passed, 0 failed (`tests-full.out` me full log).
- Cleanup: dono apne PID kill kiye, probe PNG + /tmp scratch delete kiye, sibling processes/files
  untouched.

## 3. Shot map (sab round-7/ me, `file` magic ok, Gate 1 me dims)

| File | Dims | Bytes | Kya hai |
|---|---|---|---|
| live-desktop.png | 2880x1800 | 745192 | Full desktop: opener window front (320,91 800x632), Codex peeche |
| live-open.png | 1600x1264 (@2x) | 251114 | Text tab LIVE content rows: accent spans + take arrows + status + cursor strip zone |
| live-content.png | 1600x440 | 46472 | Rows close-up, no-selection state (accent + arrows + left/mid status) |
| live-focus.png | 1600x440 | 47079 | click-1 (text-label) ke baad: no-op proof, "no selection" barkarar |
| live-focus2.png | 1600x440 | 53066 | Row-1 select: full-row band + cursor strip "1:1 \| 1 Same" + status mid "Same" |
| live-navfocus.png | 1600x640 | 91895 | Next-nav ke baad: row-2 band + cursor strip "2:1 \| 2 Changed" + status mid "Changed" |
| live-hatch.png | 1600x1264 | 220246 | Hatch launch full window (4-vs-2 pair preload) |
| live-hatch-content.png | 1600x440 | 46895 | Filler hatch ticks + red insert band + take arrows, status "All: 4 rows (4 shown)" |
| live-strip.png | 1600x160 | 37916 | Session strip: single `t_left` pill + mode tabs row |
| live-toolbar.png | 1600x260 | 82313 | Toolbar: `<->` / `▶` glyph prefix live + toggles + path rows |
| live-bc.png | 1532x1830 (-l1394) | 587537 | BC Text Compare, round-6 se sha-IDENTICAL |
| fixture-home/text/folder/table.png | 960x600 | 15007/9765/10149/9996 | Headless scenes, 4/4 round-6 se MATCH |
| fixture-accent.png | 960x600 | 8734 | Accent scene (changed-band headless proof) |
| fixture-light.png | 960x600 | 9728 | `--screenshot-mode light` render (dark se alag) |
| fixture-dark.png | 960x600 | 9765 | `--screenshot-mode dark` (default-stable proof) |
| after.png | 960x600 | 9765 | Harness output (CE baseline jaisa) |
| actions.log | — | — | Har launch/click/capture/cleanup ka timestamp log |
| cargo-run.log / cargo-run-hatch.log | — | 0/0 | Dono launches silent (GUI stdout clean) |
| tests.out | — | 853 | Harness quiet log; tests-full.out (46269 B) me 674-green full log |
| geo-typo-report.md | — | 374 | Harness report, `RESULT: PASS` |

## 4. Fixture byte DELTA (round-6 vs round-7, sha256-12)

| File | round-6 | round-7 | Match |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH |
| after.png | 9765 B | 9765 B | MATCH |
| live-bc.png | 587537 B (`bfab…`) | 587537 B (`bfab…ba88`) | sha-IDENTICAL |

Live sizes round-6 -> round-7 (same size-class): strip 33796 -> 37916 (single-pill Text
context vs 3-pill Table context); toolbar 50154 -> 82313 (lambe fixture paths + glyph
prefixes); open 251114 vs text 149736 (content rows vs empty chrome — expected jump).

## 5. Gap DELTA table — round-6 vs round-7 (CR sec-8 input + 4 waves ka live check)

### Item 1 — CN-OPENER content rows (GOAL, pehle live-pending)

| Check | round-6 | round-7 | Verdict |
|---|---|---|---|
| Content rows live | Panes EMPTY (input-block) | 3 rows live: alpha/alpha, beta/BETA accent + take arrows, gamma/gamma | PASS/proved — input-gate khul gaya |
| Accent spans | Sirf fixture-accent se | Live: beta/BETA gold spans + row-2 select band | PASS |
| Filler hatch | Live-pending | Live: unequal pair par hatch ticks + red band | PASS/proved |
| Cursor strip | Live-pending | Live: "2:1 \| 2 Changed" + beta/BETA preview | PASS/proved |
| Status 3-zone | Live-pending | left + mid live; right nav-badge is state me khali (sec-6 finding) | PARTIAL — code-contract ok, paint-state finding |

### Item 2 — toolbar glyph parity (CO-TBARCALL)

| Check | round-6 | round-7 | Verdict |
|---|---|---|---|
| Toolbar live | Plain text, koi glyph nahi | `<->` Swap + `▶` Reload prefix live dikhe | PASS/proved — CO adopt live |

### Item 3 — CP-GEO + CQ-TYPO live

| Check | round-7 | Verdict |
|---|---|---|
| Harness signals | Grid x14, mono x50, strong x26, small x52, `.size(14.0)`, width 28 — round-6 jaisa exact | PASS/hold — paint-neutral waves ka koi fixture-drift nahi |
| Live geometry | Tight 4px rhythms, mono line-number gutters, HEADER_H rows — shots me stable | PASS/hold |
| Right-edge overflow (CR gap-3) | Abhi bhi hai: right path field edge par kat raha; nav-badge/status-right isise clip lagte hain | SAME/open — CP-geo scope se bahar tha (paging/picture rows), Text overflow agla input |

### Item 4 — CP-LIGHTFLAG

| Check | round-7 | Verdict |
|---|---|---|
| Headless light | fixture-light.png 9728 B valid, dark se alag; bad-value exit 2 | PASS |
| Live toggle | "Light Mode" pill top-right live dikha (dark session me) | PASS/chrome-proof (light-live render headless route se covered) |

### Item 5 — misc

| Check | Verdict |
|---|---|
| BC tulna | PASS (observe): live-bc.png sha-IDENTICAL; changed row 2 blue band, take arrows, hatch, status zones sab barkarar |
| Tests | PASS — 674 green, zero fail (671 +2 CN +1 lightflag = 674 exact) |
| Session strip | Single `t_left` pill (opener preload se naam aaya) + mode tabs — CG adopt stable |
| Click-note | Text labels clickable nahi (sirf blank gutter/take buttons) — live-focus.png no-op iska photo-proof hai |

## 6. Incidents / findings (follow-up ke liye)

1. click-1 {387,486} beta text par NO-OP gaya — `ui.label` rows me `Sense::click` nahi hai;
   select sirf blank center-gutter se hota hai (same rows) ya take buttons se (diff rows,
   jo copy karte hain). Koi state change nahi hua; live-focus.png iska proof hai.
2. Status right zone (nav badge + F3 hints) live me is state me dikha nahi — nav row aur
   status row dono ka right-end khali hai jabki code right-dock paint karta hai. Wajah
   overflow lagti hai (right path field edge-par-kata = CR gap-3): right-dock content-width
   par tikta hai jo viewport se bahar hai. CP-GEO follow-up input (Text body overflow).
3. osascript click is env me BLOCKED (-25211) barkarar hai — sirf read-only frame/title
   use hua, clicks /tmp/clicker se. BC me koi click/focus nahi.
4. `screencapture -l` is machine par numeric window id ke sath KAAM karta hai (round-6 note
   confirm; swift winlister /tmp me chalaya, repo-touch zero).

## 7. Geometry/typography signals (read-only, harness-first)

- Grid x14, monospace x50, strong x26, small x52, `.size(14.0)`, width calls 28,
  text_style_height x4, Color32/Visuals 29 (info-only) — round-6 jaisa exact.
- Matlab: CN (wiring-only) + CO (call-site-only) + lightflag (mode-route) + CP-GEO
  (paging/picture rows) + CQ-TYPO (theme-only) me se kisi ne Text fixture pixels nahi badle.

## 8. CR sec-8 exquisite gaps — re-check (kya badla)

- Gap 1 (4 navigation rows stack): SAME — opener se pills row single hai par stack waisa hi.
- Gap 2 (oversized toolbar): IMPROVED-hissa — glyph prefixes aa gaye (`<->`/`▶`), par
  button widths abhi bhi wide text pills hain (floor 120x28 same).
- Gap 3 (right-edge overflow): SAME + nayi evidence — nav-badge/status-right clip isi se.
- Gap 4 (double status lines): SAME — pathmeta + bottom bar dono hain.
- Gap 5 (Table rhythm): NOT-COVERED is run me (Text tab scope tha).
- Gap 6 (khali void): IMPROVED-hissa — content aate hi void gaya; cursor strip + status
  zones ne khaali jagah li. Khali-state ka void abhi bhi waisa hoga.
- Gap 7 (strip pills): SAME — single pill state me gaps judge nahi ho sakta.
- Typo gaps 1-6: SAME/hold — hierarchy/labels/status-contrast/line-height/keys tabular/
  Light-Mode pill sab live-open me round-6 jaise dikhe (paint-neutral waves the).

## 9. Notes

- Is run me `crates/**`, `scripts/**`, `tools/**`, baaki checks ko touch nahi kiya — sirf
  `checks/gui-localrun-r7/` (naya untracked dir). STATUS files untouched (koi M entry nahi).
- BC window me koi click/edit/focus-change nahi — sirf window-id capture + title confirm.
- Probe PNG (`probe-nav.png`) round se delete kiya (uska content live-navfocus me hai);
  actions.log me uska zikr audit trail ke liye rehne diya.
- Sibling processes/files ko haath nahi lagaya. PUSH MANA — commit local rahega.
