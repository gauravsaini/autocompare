# REPORT — Wave BL-MERGEFIX3 (three-way tree fixture scene)

Scope: BK-MERGEMULTI §4 ka fixture half — dispatcher me naya three-way route +
`render_three_way_fixture_png` jo live `paint_three_way_tree` ka headless proof de.
Koi paint/model edit nahi, sirf `crates/gui/src/screenshot.rs` (fixture + dispatcher +
tests) aur `checks/gui-mergefix3-bl/**`. Model `tree::three_way_*` reuse hai, paint
`views.rs` untouched hai. Accent-scene precedent par bana hai.

## Kya badla (screenshot.rs only)

1. **Dispatcher me `three-way`/`threeway` branch** (`render_fixture_png`): `table` branches
   ke baad, `merge-take` se pehle. `after-threeway.png` (hyphen wali spelling) isi par
   jata hai; bina-hyphen `threeway` bhi same scene par (compat twin by design).
   Doc comment me route list update.
2. **`render_three_way_fixture_png` (naya)**: snapshots headless truth se —
   base `[a.txt, del.txt, src]`, left `[a.txt, new.txt, one.txt, src]`,
   right `[a.txt, del.txt, new.txt, src]`; `three_way_tree_rows` se 5 rows
   (`a/del/new/one/src` sorted), `stage_three_way_decision(&[], "del.txt", Right)`
   se 1 staged pick. Header live `three_way_header_line` se
   (`5 path(s), 1 decided, 4 pending, 2 conflict(s) (unresolved)`), har row par live
   `three_way_presence_marks` (`BLR`/`B-R`/`-LR`/`-L-`/`BLR`) + `three_way_row_line`
   (dir par `src/`) + `three_way_row_decision_text` (`take right` / `needs a decision` /
   `clean`). Conflict rows (`del.txt`, `new.txt`) CHANGED band me, neeche `Take:` strip
   me teen live buttons (`ThreeWaySide::label()`: Take Left / Take Base / Take Right,
   paint order Left/Base/Right). Chrome `paint_chrome` merge family (`active=4`),
   status `threeway: <header> | take left/base/right live`. Colors frozen.
3. **2 naye tests**: `three_way_scene_proves_live_tree` (rows/conflicts/marks/decisions/
   header/labels/width/PNG-valid/merge-se-alag) + `dispatcher_routes_threeway_scene`
   (hyphen/bina-hyphen same scene, 8 purane scenes se byte-alag twin-guard).

## Live-proof (koi mock string nahi)

- Rows `three_way_tree_rows` se, conflicts `three_way_conflicts` se
  (`["del.txt", "new.txt"]`).
- Marks/path/decision/header sab live helpers se (`three_way_presence_marks`,
  `three_way_row_line`, `three_way_row_decision_text`, `three_way_header_line` ==
  `three_way_tree_summary().summary_line()`).
- Take labels live `ThreeWaySide::label()` se, paint wale order me.
- Staged pick live `stage_three_way_decision` se (del.txt -> Right).

## Harness evidence (round-1/)

- `zsh tools/gui-polish/run-geo-typo.sh` → EXIT 0, `RESULT: PASS`
  (PNG 9695B, magic ok, distinct 255; Grid x10, monospace x55, strong x23, small x53,
  `.size` sirf `['14.0']` — koi typo/color change nahi).
- `cargo test -p autocompare-gui` → 603 passed, 0 failed (`tests.out` tail:
  601 baseline + 2 naye).
- `cargo clippy --all-targets -- -D warnings` → zero warnings; `rustfmt --check` clean.
- Banned grep → ZERO-BANNED; `forbid(unsafe_code)` intact.
- PNGs (magic `89504e470d0a1a0a` sab me ok, sab >= 8192B, distinct > 64):
  - `after.png` (9695B, md5 `35d69828f03d`, distinct 255) — harness default (text scene)
  - `after-threeway.png` (9919B, md5 `ba06b7cfb6b8`, distinct 256) — naya three-way scene
- Twin-guard: `after-threeway.png` text/folder/merge/table/home/sync/merge-take/allsame
  sab se byte-alag; `three-way` == `threeway` spelling twin by design (compat assert).

## Scope / DoD

- OWNS only: `crates/gui/src/screenshot.rs` + `checks/gui-mergefix3-bl/**`.
- `views.rs`/`app.rs`/`tree.rs` me ZERO paint edit; `checks/opencode-waves/STATUS.md` untouched.
- Colors FROZEN — sirf frozen palette + chrome grays reuse, koi naya hue nahi.
- DoD: GATES.md ke G1–G6 sab green; commit local only, NO push.

## Files (sirf OWNS)

- `crates/gui/src/screenshot.rs` (dispatcher + three-way fixture + 2 tests)
- `checks/gui-mergefix3-bl/GATES.md` (yeh gates)
- `checks/gui-mergefix3-bl/REPORT.md` (yeh file)
- `checks/gui-mergefix3-bl/round-1/` (`after.png`, `after-threeway.png`,
  `geo-typo-report.md`, `tests.out`, `screenshot.out`)
