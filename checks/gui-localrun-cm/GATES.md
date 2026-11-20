# GATES — CM-LOCALRUN round-5 (cj round-4 -> cm round-5 interactive re-verify)

Scope: READ-ONLY re-verify + pehli interactive wave (tab-switch allowed, orchestrator approval).
`crates/gui/src/*` me ZERO change — sirf rebuild + recapture + clicks. OWNS: sirf `checks/gui-localrun-cm/`.
Baaki checks + STATUS files untouched (`git status` me sirf `?? checks/gui-localrun-cm/`).

## Gate 1 — PNG magic + size (round-5/, 13 PNGs)

`file` magic sab ok (PNG RGBA, non-interlaced), sab >= 8192 bytes:

| File | Dims | Bytes | Magic | Size gate |
|---|---|---|---|---|
| after.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-home.png | 960x600 | 15007 | PNG ok | PASS |
| fixture-text.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-folder.png | 960x600 | 10149 | PNG ok | PASS |
| fixture-table.png | 960x600 | 9996 | PNG ok | PASS |
| live-home.png | 1600x1264 (@2x = 800x632) | 286481 | PNG ok | PASS |
| live-text.png | 1600x1264 | 148930 | PNG ok | PASS |
| live-folder.png | 1600x1264 | 147281 | PNG ok | PASS |
| live-table.png | 1600x1264 | 175205 | PNG ok | PASS |
| live-strip.png | 1600x160 | 34103 | PNG ok | PASS |
| live-toolbar.png | 1600x260 | 47623 | PNG ok | PASS |
| live-bc.png | 1532x1830 (window-id 1394) | 587537 | PNG ok | PASS |
| live-desktop.png | 2880x1800 | 775487 | PNG ok | PASS |

13/13 PASS.

## Gate 2 — harness RESULT:PASS

`tools/gui-polish/run-geo-typo.sh checks/gui-localrun-cm/round-5/` exit 0.
`geo-typo-report.md`: PNG 9765 bytes ok, magic ok, distinct 256 ok, `RESULT: PASS`.
Signals: Grid x14, monospace x50, strong x26, small x52, `.size(14.0)`, width calls 28 —
cj round-4 jaisa exact. `Color32/Visuals` touches 29 (cj me 27 tha) — info-only; farak beech ki
landed waves (CK darkbug per-frame re-assert + CL toolbar fn) ka hai, is run me source touch zero.

## Gate 3 — tests green

`cargo test -p autocompare-gui`: 670 passed, 0 failed (cj me 669 the; +1 beech ki waves ka).
Full log harness ke `tests.out` (quiet dots) + final full run me same count.

## Gate 4 — default-dark byte-stability (cj round-4 -> cm round-5, sha256-12)

| File | cj round-4 sha12 | cm round-5 sha12 | Verdict |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH |
| after.png | 9765 B | 9765 B | MATCH cj + CE baseline |

4/4 MATCH + after MATCH — default (dark) path byte-stable, CI guarantee hold karta hai.

## Gate 5 — hygiene + commit rule

- `git status --short` me launch se commit tak sirf `?? checks/gui-localrun-cm/` — source zero change
  (`git diff --stat HEAD -- crates/` empty).
- Commit sirf `git add checks/gui-localrun-cm` (pathspec-only). `git add -A` MANA.
- STATUS files (root + `checks/opencode-waves/STATUS.md`) untouched — status me koi M entry nahi.
- PUSH MANA — commit local rahega, orchestrator verify + push karega.
- BC ko observe-only rakha (window-id capture, zero clicks/focus change); apna instance kill kiya,
  sibling processes ko haath nahi lagaya. Finder cmdrop window close kiya (bahar ka /tmp setup).
- Window frame dono instances me 320,91 800x632 — cj round-4 jaisa exact.

## Result

ALL GATES PASS (gates 1-5). Item-level verdicts REPORT.md me (ek PARTIAL hai — Text content rows,
input-injection env-block par; gate nahi verdict).
