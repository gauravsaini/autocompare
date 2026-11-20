# CM-LOCALRUN round-5 — cj round-4 se gap DELTA (pehli INTERACTIVE wave: tab-switch khula)

## 1. Summary

CJ round-4 (HEAD `5f6369b`) ke baad 2 code waves land hui: CK live-dark fix `01dbf01`
(per-frame theme re-assert, app.rs +16) aur CL toolbar wiring `48946aa` (paint_session_toolbar fn,
views.rs +71/-5). Is round me rebuild + recapture + LIVE CLICKS kiya — crates me ZERO content
edits, sirf `checks/gui-localrun-cm/` add hua. Natija:
- Item 1 (live-dark FAIL) FIXED: live app ab DARK khulta hai (live-home 286481 B, cj ka light
  295501 B gaya) — CK root-cause + fix live-photo-proof samet confirm.
- Item 3 (multi-session strip) FIXED: 3 live pills "Text 0 / Folder 1 / Table 2" (live-strip.png) —
  CG mode-T7 adopt ka live visual proof.
- Item 4 (toolbar icon-parity) SAME/partial: live-toolbar.png me text-only buttons, koi glyph nahi —
  CL fn ready par App call-site purana row (CL sec-5 prediction live-confirm).
- Item 2 (Text content: selective accent + filler hatch + status zones + cursor strip) PARTIAL:
  Text tab live khula (session "Text 0", path fields, pathmeta row, find bar, status line sab dikhe)
  par panes EMPTY rahe — path input ka har injection path env-block hua (detail sec 6).
  Content-row proof headless fixture-text.png se (byte-stable, changed band + accent spans +
  status counts + gutter segs render karta hai).
- Fixtures 4/4 cj se byte-identical (default-dark stability hold), tests 670 green, harness PASS.

## 2. Live run spec

- Branch main, HEAD `2032e4c` (CM launch commit; CK+CL waves andar).
- Build: `cargo build -p autocompare-gui` green (cached). Harness
  `tools/gui-polish/run-geo-typo.sh checks/gui-localrun-cm/round-5/` exit 0, `RESULT: PASS`.
- Headless fixtures: `--screenshot` filename-keyed scenes (home/text/folder/table), sab 960x600,
  exit 0, screenshot*.out khali.
- Live: do instances chalaye (PID 66217, phir 67028→67577; beech me ek silent exit — sec 6 dekho).
  Window frame dono me pos 320,91 size 800x632 — cj round-4 jaisa exact.
- Clicks: /tmp/clicker (CGSessionEventTap mouse; osascript click -25211 assistive-block par fail hua,
  session-tap pehla kaam karne wala rasta tha). Har action `round-5/actions.log` me timestamp samet.
- BC: observe-only — window-id 1394 se `screencapture -l` (obscured window ka direct capture,
  zero focus change, zero clicks). Title: `t_left.txt <--> t_right.txt - Text Compare`.
- Tests: `cargo test -p autocompare-gui` — 670 passed, 0 failed (cj me 669 the).
- Cleanup: apna instance kill kiya (gone confirm), Finder /tmp/cmdrop window close kiya,
  probe PNGs delete kiye; sibling processes ko touch nahi kiya.

## 3. Shot map (sab round-5/ me, `file` magic ok, Gate 1 me sizes)

| File | Size | Kya hai |
|---|---|---|
| live-desktop.png | 775487 | Full side-by-side: Table window front, Finder/browser peeche |
| live-home.png | 286481 | Home, DARK (CK fix proof — cj light 295501 se change) |
| live-text.png | 148930 | Text tab live: "Text 0" session, empty panes, path fields + pathmeta + find bar |
| live-folder.png | 147281 | Folder tab live: "Text 0 + Folder 1" strip, folder path fields, status line |
| live-table.png | 175205 | Table tab live: 3-pill strip, Keys/Tolerance/Find controls, status line |
| live-strip.png | 34103 | Strip close-up: Text 0 x / Folder 1 x / Table 2(active) + mode tabs |
| live-toolbar.png | 47623 | Toolbar close-up: text buttons, separator, toggles — koi glyph nahi |
| live-bc.png | 587537 | BC Text Compare (t_left vs t_right, changed row 2 blue, gutter arrows, status bar) |
| fixture-home/text/folder/table.png | 15007/9765/10149/9996 | Headless scenes, 4/4 cj se bytes MATCH |
| after.png | 9765 | Harness output (CE baseline + cj se identical) |
| actions.log | 2179 | Har click/capture/incident ka timestamp log |
| cargo-run.log | 0 | Direct binary launch log (GUI stdout silent) |
| tests.out | 849 | Harness quiet test log (full run: 670 passed) |
| geo-typo-report.md | 374 | Harness report, `RESULT: PASS` |

## 4. Fixture byte delta (cj round-4 vs cm round-5, sha256-12)

| File | cj round-4 | cm round-5 | Match |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH |
| live-home.png | 295501 (LIGHT) | 286481 (DARK) | DIFFER — CK fix live-visible |

## 5. Gap DELTA table — cj round-4 vs cm round-5

### Item 1 — live dark (CK fix ke baad)

| Check | cj round-4 observed | cm round-5 observed | Verdict + exact evidence |
|---|---|---|---|
| Live dark | LIGHT khulta tha (white bg + Light badge), CK fix code-only tha | DARK khulta hai: dark title bar + dark bg + dark cards; badge "Light Mode" flag-true + ctx-dark ke sath (sahi toggle affordance) | PASS — CK `01dbf01` live-confirm. live-home 286481 B ~ CK round-1 ke 286482 B (1 B desktop-state farak) |

### Item 2 — Text tab live (P1-2/P1-3/P1-1/P0-2)

| Check | cj round-4 observed | cm round-5 observed | Verdict + exact evidence |
|---|---|---|---|
| Text tab chrome | Tab khola hi nahi tha (observe-only rule) | Tab LIVE khula: title "(empty) <--> (empty) - Text", session pill "Text 0" + x, path fields (left/right + Browse), pathmeta history dropdown, Find bar (needle + Ignore case + Prev/Next), status "no text results yet..." | PASS (chrome live-visible) |
| Selective accent + filler hatch (P0-2) | Code PASS, live-visual pending | Panes EMPTY ("nothing to display: both sides are empty") — input injection block (sec 6), isliye accent/filler rows live nahi dikhe | SAME/live-pending. Content proof: fixture-text.png (byte-stable; changed band + inverted accent spans + filler rows render) |
| Status zones (P1-2) + cursor strip (P1-3) | Code PASS (CD), live-visual pending | Status line live dikhi par empty-state wali ("no text results yet"); 3-zone counts + cursor strip ko content chahiye | SAME/live-pending (wajah input-block, paint nahi) |
| Pathmeta row (P1-1) | Code PASS, live-visual pending | Path-meta history dropdown live Text tab me VISIBLE (screenshot me "Text: exact..." combo) | PASS (live-visible, empty-state me bhi) |

### Item 3 — multi-session strip

| Check | cj round-4 observed | cm round-5 observed | Verdict + exact evidence |
|---|---|---|---|
| Session strip live | Empty-state line thi, strip khali (koi session khola nahi) | 3 LIVE pills: "Text 0" x, "Folder 1" x, "Table 2" (active blue) — live-strip.png close-up + live-table.png full | PASS — har mode-tab click ne session adopt kiya (CG `799c949` live-confirm) |

### Item 4 — toolbar icon-parity (P2-1)

| Check | cj round-4 observed | cm round-5 observed | Verdict + exact evidence |
|---|---|---|---|
| Toolbar live | Header row dikhi, pixel-verify nahi hua | live-toolbar.png: "Swap Sides" + "Reload / Recompare" PLAIN TEXT buttons, separator, 3 checkboxes — koi glyph/icon nahi | SAME/partial — CL fn (`paint_session_toolbar`) ready par App call-site abhi purana text row paint karta hai (`app.rs:5118-5139`); CL REPORT sec-5 follow-up live-confirm hua, abhi khula hai |

### Item 5 — light fixture artifact + misc

| Check | Verdict |
|---|---|
| Light-mode PNG artifact (`--screenshot-mode light` flag) | SAME/open — CLI abhi sirf dark render karta hai; test-level proof (fixture_mode_param_switches_hues) green hai |
| BC tulna | PASS (observe): live-bc.png me changed row 2 dono taraf blue band, gutter arrows, status bar "Same/Insert" zones — hamara fixture-text.png wahi contract render karta hai (changed band + accent + status counts + gutter segs), par DARK palette me |
| Tests | PASS — 669 -> 670 green, zero fail |
| Harness | PASS — `RESULT: PASS`, geometry signals cj jaisi exact |

## 6. Incidents + input-block postmortem (follow-up ke liye)

1. osascript click BLOCKED: `System Events click at` ne -25211 diya (assistive access nahi).
   Fix: /tmp/clicker (CGSessionEventTap mouse) — in-app clicks 5/5 kaam kiye (Text/Folder/Table tabs).
2. Keyboard injection unreliable: session-tap keys TextEdit tak pahunche hi nahi; hid-tap se ek
   "ABC" marker ek baar dropdown me dikha par repeat nahi hua (atomic hid-click+type bhi fail).
   Focus ring sahi field par thi, frontmost sahi tha — phir bhi text land nahi hua.
   Subah ka lesson: eligibility gate (TCC/key-event path) mouse se alag hai; key events yahan
   drop hote hain. Koi keystroke BC/app me galat jagah nahi gaya (har round ke baad probe se verify).
3. Drag-drop fail: Finder synthesized click par select-highlight nahi dikhaya — Finder icon-drag
   start hi nahi hua, isliye drop populate nahi hua (`apply_dropped_paths` code sahi hai, test green).
4. Silent exit (PID 67028, ~13:30): process pgrep se gayab, frontmost ChatGPT par tha, cargo-run.log
   khali, DiagnosticReports me is waqt ki koi crash report nahi (13:02-13:05 ki reports purane
   test-binary stack-overflow ki hain — `cargo test` ke dauran ki, is run ke harness TESTS green
   hone ke baad bhi note karne layak). Restart (PID 67577) par sab kuch dobara clean chala.
   Root-cause unknown — report kiya, retry success.
5. BC region capture pehli baar me occluders (browser/Finder) dikhaya — window-id 1394 se `-l`
   capture ne sahi BC pixels diye (zero focus change). Aage se obscured windows ke liye yahi rasta.

## 7. Geometry/typography signals (read-only, harness-first)

- Grid x14, monospace x50, strong x26, small x52, `.size(14.0)` — cj round-4 jaisa exact.
- Width calls 28 (cj jaisa). Colors info-only 29 (cj 27; +2 CK/CL waves ka, is run me zero touch).

## 8. Bachi gaps (follow-up waves ke liye)

1. Text content-rows ka live photo-proof (accent + filler hatch + status 3-zone + cursor strip):
   is env me keyboard/drag input block hai — agle wave me ya to file-dialog path (Browse click +
   native dialog me path paste — wahi key-gate lagega) ya CLI flag (`--open left right --tab text`
   jaisa non-interactive opener) chahiye. Tab-switch rule khulne ka fayda liya; input-gate ab bottleneck hai.
2. P2-1 toolbar App call-site adopt (`app.rs` toolbar row -> `paint_session_toolbar`): CL sec-5 ka
   kaam, live me verify karna (phir icon-parity PASS hoga).
3. Light fixture PNG artifact (`--screenshot-mode light` flag) — abhi bhi khula hai.
4. Silent-exit root-cause (13:30 wala) — dobara dikhe to crash/CPU-profile samet pakadna.

## 9. Notes

- Is run me `crates/**`, `scripts/**`, `tools/**`, baaki checks ko touch nahi kiya — sirf
  `checks/gui-localrun-cm/` (untracked naya dir). STATUS files untouched.
- BC window me koi click/edit/focus-change nahi — sirf window-id capture + title confirm.
- Probe PNGs (probe-*) round se delete kiye; actions.log me unka zikr rehne diya (audit trail).
- PUSH MANA tha — commit local rahega.
