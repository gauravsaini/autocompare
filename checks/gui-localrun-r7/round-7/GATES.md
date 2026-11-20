# GATES — R7-LOCALRUN round-7 (CN-opener live content proof + CO/CP/CQ live check)

Scope: READ-ONLY verify. `crates/**`, `scripts/**`, `tools/**` me ZERO change — sirf rebuild +
`--open` launches + captures + 3 session-tap clicks. OWNS: sirf `checks/gui-localrun-r7/`.
Baaki checks + STATUS files untouched.

## Gate 1 — PNG magic + size (round-7/, 19 PNGs)

`file` magic sab ok (PNG RGBA, non-interlaced), sab >= 8192 bytes:

| File | Dims | Bytes | Magic | Size gate |
|---|---|---|---|---|
| after.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-home.png | 960x600 | 15007 | PNG ok | PASS |
| fixture-text.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-folder.png | 960x600 | 10149 | PNG ok | PASS |
| fixture-table.png | 960x600 | 9996 | PNG ok | PASS |
| fixture-accent.png | 960x600 | 8734 | PNG ok | PASS |
| fixture-light.png | 960x600 | 9728 | PNG ok | PASS |
| fixture-dark.png | 960x600 | 9765 | PNG ok | PASS |
| live-desktop.png | 2880x1800 | 745192 | PNG ok | PASS |
| live-open.png | 1600x1264 | 251114 | PNG ok | PASS |
| live-content.png | 1600x440 | 46472 | PNG ok | PASS |
| live-focus.png | 1600x440 | 47079 | PNG ok | PASS |
| live-focus2.png | 1600x440 | 53066 | PNG ok | PASS |
| live-navfocus.png | 1600x640 | 91895 | PNG ok | PASS |
| live-hatch.png | 1600x1264 | 220246 | PNG ok | PASS |
| live-hatch-content.png | 1600x440 | 46895 | PNG ok | PASS |
| live-strip.png | 1600x160 | 37916 | PNG ok | PASS |
| live-toolbar.png | 1600x260 | 82313 | PNG ok | PASS |
| live-bc.png | 1532x1830 (-l1394) | 587537 | PNG ok | PASS |

19/19 PASS. (Ek probe capture beech me aaya tha — use round me nahi rakha, delete kiya;
detail REPORT sec-9 me hai.)

## Gate 2 — harness RESULT:PASS

`tools/gui-polish/run-geo-typo.sh checks/gui-localrun-r7/round-7/` exit 0.
`geo-typo-report.md`: PNG 9765 bytes ok, magic ok, distinct 256 ok, `RESULT: PASS`.
Signals: Grid x14, monospace x50, strong x26, small x52, `.size(14.0)`, width calls 28 —
round-6 jaisa exact (CN/CO/lightflag/CP-GEO/CQ-TYPO ke baad bhi zero fixture-drift).

## Gate 3 — tests green

`cargo test -p autocompare-gui`: 674 passed, 0 failed (round-6 me 671 the;
+2 CN-OPENER +1 LIGHTFLAG = 674 exact — is run ka source touch zero).
Full log `tests-full.out` me, harness quiet log `tests.out` me.

## Gate 4 — byte-stability (round-6 -> round-7, sha256-12)

| File | round-6 sha12 | round-7 sha12 | Verdict |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH |
| after.png | 9765 B | 9765 B | MATCH |
| live-bc.png | `bfab…` 587537 B | `bfab…ba88` 587537 B | sha-IDENTICAL |

4/4 + after MATCH; BC sha-identical — default (dark) path guarantee hold karta hai.

## Gate 5 — live content proof (GOAL check-list, sab PNG-backed)

- [x] `--open` preload: title `t_left.txt <--> t_right.txt - Text - autocompare` (live-open)
- [x] Accent spans live: beta/BETA gold spans + row-2 select band (live-content/navfocus)
- [x] Filler hatch live: hatch ticks + red band, unequal pair (live-hatch-content)
- [x] Cursor strip live: "2:1 | 2 Changed" + preview lines (live-navfocus), "1:1 | 1 Same" (live-focus2)
- [x] Status left+mid live: counts + verdict (content/focus2/navfocus); right zone finding darj (REPORT sec-6)
- [x] Toolbar glyphs live: `<->` + `▶` prefix (live-toolbar) — CO adopt pixel-verify
- [x] `--help` exit 0, `--open` bad-args exit 2, bad `--screenshot-mode` exit 2 (CLI proof)
- [x] Light headless: fixture-light 9728 B vs dark 9765 B (CP-LIGHTFLAG proof)

## Gate 6 — hygiene + commit rule

- `git status --short` me launch se commit tak sirf `?? checks/gui-localrun-r7/` — source zero
  change (`git diff HEAD -- crates/ scripts/ tools/` empty; niche verify).
- Commit sirf `git add checks/gui-localrun-r7` (pathspec-only). `git add -A` MANA.
- STATUS files (root + `checks/opencode-waves/STATUS.md`) untouched — koi M entry nahi.
- PUSH MANA — commit local rahega.
- BC ko observe-only rakha (window-id 1394 capture, zero clicks/focus change; swift winlister
  /tmp me, repo-touch zero). Apne dono instance kill kiye (gone confirm), probe + /tmp scratch
  delete kiye, sibling processes/files untouched.
- Window frame dono launches me 320,91 800x632 — round-4/5/6 jaisa exact.

## Result

ALL GATES PASS (gates 1-6). CR sec-8 re-check + right-zone overflow finding REPORT.md me hai.
