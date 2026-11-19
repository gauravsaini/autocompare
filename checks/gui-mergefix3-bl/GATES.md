# Wave BL-MERGEFIX3 — GATES (three-way tree fixture scene)

OWNS: `crates/gui/src/screenshot.rs` (fixture + dispatcher + tests only) + `checks/gui-mergefix3-bl/**` ONLY.
KEVL: `views.rs`, `app.rs`, `tree.rs` — paint/model live hain, koi paint edit MANA.
`checks/opencode-waves/STATUS.md` ko CHHOONA MANA. Colors FROZEN (koi hue/token change nahi).
MS rust guidelines: `forbid(unsafe_code)`, non-test me `unwrap()`/`expect()`/`panic!`/`todo!` MANA.
Skill: `skills/gui-polish/SKILL.md`; harness: `tools/gui-polish/run-geo-typo.sh` PASS.
NO push (orchestrator verify karke push karega).

Scope: BK-MERGEMULTI ke §4 callsite/fixture spec ka fixture half — dispatcher me naya
`three-way`/`threeway` route + `render_three_way_fixture_png` jo `paint_three_way_tree`
ka headless proof de (conflict rows + Take buttons + summary header). Model
`tree::three_way_*` reuse, paint me ZERO change (accent-scene precedent).

## G1 — Baseline green (edit se pehle)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (601 passed, 0 failed, `main@bd7df55`)
- EVIDENCE: edit se pehle full suite 601 green tha; final count G3 me

## G2 — OWNS-only diff, KEVL + colors + STATUS untouched

- CHECK: `git status --short` + `git diff --name-only`
  + `git diff --name-only | grep -vE "^(crates/gui/src/screenshot.rs|checks/gui-mergefix3-bl/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|app.rs|tree.rs$" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff --name-only | grep -E "opencode-waves/STATUS" && echo STATUS-TOUCHED || echo STATUS-CLEAN`
  + `git diff -- crates/gui/src/screenshot.rs | grep -E "^\+" | grep -E "Color32::|from_rgba|Visuals::" || echo NO-COLOR-ADD`
- EXPECT: sirf `crates/gui/src/screenshot.rs` + `checks/gui-mergefix3-bl/**`;
  KEVL-CLEAN; STATUS-CLEAN; NO-COLOR-ADD (sirf frozen palette reuse:
  `BG_DARK`/`BG_PANEL`/`FG_TEXT`/`FG_DIM`/`CHANGED_BG` + chrome grays)
- EVIDENCE: commit me keval OWNS paths (`git show --stat`)

## G3 — Full test suite green (2 naye tests samet)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: 603 passed, 0 failed (601 baseline + `three_way_scene_proves_live_tree`
  + `dispatcher_routes_threeway_scene`)
- EVIDENCE: `round-1/tests.out` (tail: `test result: ok. 603 passed; 0 failed`)

## G4 — clippy zero, fmt OWNS par clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` (exit 0)
  + `rustfmt --edition 2024 --check crates/gui/src/screenshot.rs` (exit 0)
- EXPECT: zero warnings; fmt clean (naye code me `.get()`/`first()` only, koi indexing nahi)
- EVIDENCE: clippy `Finished` with no errors; fmt-check exit 0

## G5 — Banned grep zero new + unsafe forbid intact

- CHECK: `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/screenshot.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `grep -n "forbid(unsafe_code)" crates/gui/src/screenshot.rs`
- EXPECT: ZERO-BANNED (`unwrap_or`/`map_err`/`ok_or_else` allowed — banned sirf `unwrap()`/`expect()` hain);
  `forbid(unsafe_code)` line 6 intact
- EVIDENCE: command output ZERO-BANNED

## G6 — Harness PASS + PNGs magic + distinct + naya scene purane twins se byte-distinct

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-mergefix3-bl/round-1` (exit 0)
  + `python3` magic/size/distinct check on `after.png`, `after-threeway.png`
  + purane scenes (text/folder/merge/table/home/sync/merge-take/allsame) se byte-compare
- EXPECT: `RESULT: PASS` (magic `89504e470d0a1a0a` ok, >= 8192B, distinct > 64);
  `after-threeway.png` (9919B, 256) != `after.png` (9695B, 255) aur baaki sab scenes se alag;
  `three-way` aur `threeway` spelling same scene par (twin by design, compat assert)
- EVIDENCE: `round-1/geo-typo-report.md` (RESULT: PASS), PNG files + md5 list REPORT me
