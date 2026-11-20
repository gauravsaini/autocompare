# GATES — CR-LOCALRUN round-6 (cm round-5 -> cr DELTA baseline + early feedback)

Scope: READ-ONLY baseline + early feedback. `crates/gui/src/*` me ZERO change — sirf rebuild +
recapture + clicks. OWNS: sirf `checks/gui-localrun-cr/`. Baaki checks + STATUS files
untouched (`git status` me sirf `?? checks/gui-localrun-cr/`).

## Gate 1 — PNG magic + size (round-6/, 13 PNGs)

`file` magic sab ok (PNG RGBA, non-interlaced), sab >= 8192 bytes:

| File | Dims | Bytes | Magic | Size gate |
|---|---|---|---|---|
| after.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-home.png | 960x600 | 15007 | PNG ok | PASS |
| fixture-text.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-folder.png | 960x600 | 10149 | PNG ok | PASS |
| fixture-table.png | 960x600 | 9996 | PNG ok | PASS |
| live-home.png | 1600x1264 (@2x = 800x632) | 286482 | PNG ok | PASS |
| live-text.png | 1600x1264 | 149736 | PNG ok | PASS |
| live-folder.png | 1600x1264 | 148293 | PNG ok | PASS |
| live-table.png | 1600x1264 | 176112 | PNG ok | PASS |
| live-strip.png | 1600x160 | 33796 | PNG ok | PASS |
| live-toolbar.png | 1600x260 | 50154 | PNG ok | PASS |
| live-bc.png | 1532x1830 (window-id 1394) | 587537 | PNG ok | PASS |
| live-desktop.png | 2880x1800 | 726552 | PNG ok | PASS |

13/13 PASS. (Ek invalid browser-occluder capture beech me aaya tha — use round me nahi rakha;
detail REPORT sec-6 me hai.)

## Gate 2 — harness RESULT:PASS

`tools/gui-polish/run-geo-typo.sh checks/gui-localrun-cr/round-6/` exit 0.
`geo-typo-report.md`: PNG 9765 bytes ok, magic ok, distinct 256 ok, `RESULT: PASS`.
Signals: Grid x14, monospace x50, strong x26, small x52, `.size(14.0)`, width calls 28,
Color32/Visuals 29 — cm round-5 jaisi exact (harness 13:39 ka = pre-CP/CQ signals).

## Gate 3 — tests green

`cargo test -p autocompare-gui`: 671 passed, 0 failed (cm me 670 the; +1 beech me landed
sibling waves ka hai — is run ka source touch zero). Full log `tests-full.out` me,
harness quiet log `tests.out` me.

## Gate 4 — default-dark byte-stability (cm round-5 -> cr round-6, sha256-12)

| File | cm round-5 sha12 | cr round-6 sha12 | Verdict |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH |
| after.png | 9765 B | 9765 B | MATCH (cm/CE baseline jaisa) |

4/4 MATCH + after MATCH — default (dark) path byte-stable, guarantee hold karta hai.

## Gate 5 — hygiene + commit rule

- `git status --short` me launch se commit tak sirf `?? checks/gui-localrun-cr/` — source zero
  change (`git diff HEAD -- crates/ scripts/ tools/` empty).
- Commit sirf `git add checks/gui-localrun-cr` (pathspec-only). `git add -A` MANA.
- STATUS files (root + `checks/opencode-waves/STATUS.md`) untouched — koi M entry nahi.
- PUSH MANA — commit local rahega.
- BC ko observe-only rakha (window-id 1394 capture, zero clicks/focus change; title
  `t_left.txt <--> t_right.txt - Text Compare` confirm). Apna instance kill kiya (gone
  confirm), probe PNGs delete kiye, sibling processes/files ko haath nahi lagaya.
- Window frame 320,91 800x632 — cm round-4/5 jaisa exact. Teenon mode-tab clicks title-bar
  se verify hue (Text/Folder/Table).

## Result

ALL GATES PASS (gates 1-5). Sec-8 exquisite gaps REPORT.md me — ye CP/CQ agle round ka input hai.
