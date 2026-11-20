# GATES — CJ-LOCALRUN round-4 (ca round-3 -> cj round-4 re-verify)

Scope: READ-ONLY re-verify. `crates/gui/src/*` me ZERO change — sirf rebuild + recapture.
OWNS: sirf `checks/gui-localrun-cj/`. Baaki checks + STATUS (repo me STATUS.md hai hi nahi) untouched.

## Gate 1 — PNG magic + size (round-4/, 7 PNGs)

`file` magic sab ok (PNG RGBA, non-interlaced), sab >= 8192 bytes:

| File | Dims | Bytes | Magic | Size gate |
|---|---|---|---|---|
| after.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-home.png | 960x600 | 15007 | PNG ok | PASS |
| fixture-text.png | 960x600 | 9765 | PNG ok | PASS |
| fixture-folder.png | 960x600 | 10149 | PNG ok | PASS |
| fixture-table.png | 960x600 | 9996 | PNG ok | PASS |
| live-desktop.png | 2880x1800 | 757096 | PNG ok | PASS |
| live-home.png | 1600x1264 (@2x = 800x632) | 295501 | PNG ok | PASS |

7/7 PASS.

## Gate 2 — harness RESULT:PASS

`tools/gui-polish/run-geo-typo.sh checks/gui-localrun-cj/round-4/` exit 0.
`geo-typo-report.md`: PNG 9765 bytes ok, magic ok, distinct 256 ok, `RESULT: PASS`.
Signals: Grid x14, monospace x50, strong x26, small x52, `.size(14.0)` — ca round-3 jaisa exact.
`Color32/Visuals` touches 27 (ca me 22 tha) — info-only; farak landed waves (CE/CI/CH) ka hai,
is run me source touch zero (`git status` me `crates/**` zero entries).

## Gate 3 — tests green

`cargo test -p autocompare-gui`: 669 passed, 0 failed (`round-4/tests.out`).
ca round-3 me 659 the; +10 beech ki waves (CD status, CE darkfix, CF CTA, CG mode-T7, CH toolbar, CI darkfull) ke.
Targeted proof bhi green (4/4): `chrome_selectors_follow_dark_mode`,
`dark_mode_storage_parse_trims_and_defaults_dark`, `dark_mode_storage_roundtrip_through_save`,
`screenshot::tests::fixture_mode_param_switches_hues`.

## Gate 4 — default-dark byte-stability (ca -> cj, `cmp` + sha256)

| File | ca round-3 sha12 | cj round-4 sha12 | `cmp` |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH |
| after.png | (ca: 9765 B) | 9765 B | MATCH ca + MATCH `gui-darkfix-ce/round-1/after.png` |

4/4 MATCH + after MATCH — default (dark) path byte-stable, CI guarantee hold karta hai.

## Gate 5 — hygiene + commit rule

- `git status --short` me launch se commit tak sirf `?? checks/gui-localrun-cj/` — source zero change.
- Commit sirf `git add checks/gui-localrun-cj` (pathspec-only). `git add -A` MANA.
- PUSH MANA — commit local rahega, orchestrator verify + push karega.
- BC app ko observe-only rakha (window title confirm, koi click/edit nahi); apna instance (PID 62872) kill kiya.

## Result

ALL GATES PASS (gates 1-5). Item-level verdicts REPORT.md me (ek FAIL hai — live dark, gate nahi verdict).
