# Wave BJ-MERGEFIX — GATES (BI-M4 take-row fixture + BI-S4 sync dispatcher)

OWNS: `crates/gui/src/screenshot.rs` (fixture + dispatcher + tests only) + `checks/gui-mergefix-bj/**` ONLY.
KEVL: `views.rs`, `app.rs`, `tree.rs`, paint code — koi paint edit MANA (paint live hai).
`checks/opencode-waves/STATUS.md` ko CHHOONA MANA. Colors FROZEN (koi hue/token baat nahi).
MS rust guidelines: `forbid(unsafe_code)`, non-test me `unwrap()`/`expect()`/`panic!`/`todo!` MANA.
Skill: `skills/gui-polish/SKILL.md`; harness: `tools/gui-polish/run-geo-typo.sh` PASS.
NO push (orchestrator verifies then pushes).

Scope: BI audit (`checks/gui-mergesync-bi/REPORT.md` §spec table) ke do P2 follow-ups —
BI-M4 (merge fixture me take-row strip + decision line, ya alag merge-take route) aur
BI-S4 (dispatcher me `sync` branch + `render_sync_fixture_png`, AW2 picture-precedent twin se bachao).

## G1 — Baseline green (edit se pehle)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (592 passed, 0 failed, `main@6fb7c7b`)
- EVIDENCE: tests green the; final count G3 me (592 + 3 naye = 595)

## G2 — OWNS-only diff, KEVL + colors + STATUS untouched

- CHECK: `git status --short` + `git diff --name-only`
  + `git diff --name-only | grep -vE "^(crates/gui/src/screenshot.rs|checks/gui-mergefix-bj/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|app.rs|tree.rs$" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff --name-only | grep -E "opencode-waves/STATUS" && echo STATUS-TOUCHED || echo STATUS-CLEAN`
  + `git diff -- crates/gui/src/screenshot.rs | grep -E "^\+" | grep -E "Color32::|from_rgba|Visuals::" || echo NO-COLOR-ADD`
- EXPECT: sirf `crates/gui/src/screenshot.rs` + `checks/gui-mergefix-bj/**`;
  KEVL-CLEAN (working tree me `app.rs`/`views.rs` ke diffs parallel agent ke hain —
  is commit me shaamil nahi); STATUS-CLEAN; NO-COLOR-ADD (sirf frozen palette reuse:
  `BG_DARK`/`BG_PANEL`/`FG_TEXT`/`FG_DIM`/`CHANGED_*`/`DELETED_BG`/`INSERTED_BG` + chrome grays)
- EVIDENCE: commit me keval OWNS files (`git show --stat`)

## G3 — Full test suite green (3 naye tests samet)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: 595 passed, 0 failed (592 baseline + `merge_take_scene_proves_live_take_row`
  + `sync_scene_matches_checklist_gate_contract` + `dispatcher_routes_sync_scene`)
- EVIDENCE: `round-1/tests.out` (tail: `test result: ok. 595 passed; 0 failed`)

## G4 — clippy zero, fmt OWNS par clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` (exit 0)
  + `cargo fmt -p autocompare-gui -- --check` (exit 0; fmt run `cargo fmt -p autocompare-gui` se)
- EXPECT: zero warnings; fmt clean (indexing lints se bachne ke liye `.first()` + `match`,
  naye helpers me `.get()` only, koi indexing nahi)
- EVIDENCE: clippy `Finished` with no errors; fmt-check exit 0

## G5 — Banned grep zero new + unsafe forbid intact

- CHECK: `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/screenshot.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `grep -n "forbid(unsafe_code)" crates/gui/src/screenshot.rs`
- EXPECT: ZERO-BANNED (`unwrap_or`/`map_err` allowed — banned sirf `unwrap()`/`expect()` hain);
  `forbid(unsafe_code)` line 6 intact
- EVIDENCE: command output ZERO-BANNED

## G6 — Harness PASS + PNGs magic + distinct + naye scenes purane twins se byte-distinct

- CHECK: `bash tools/gui-polish/run-geo-typo.sh checks/gui-mergefix-bj/round-1` (exit 0)
  + `python3` magic/size/distinct check on `after.png`, `after-merge-take.png`,
    `after-sync.png`, `after-merge.png`
- EXPECT: `RESULT: PASS` (magic `89504e470d0a1a0a` ok, >= 8192B, distinct > 64);
  `after-merge-take.png` (8848B, 256) != `after-merge.png` (9963B, 256);
  `after-sync.png` (10732B, 256) text/folder/merge/table/home se byte-alag (AW2 twin-guard);
  `foldermerge` naam ab bhi merge route ka twin by design (compat assert)
- EVIDENCE: `round-1/geo-typo-report.md` (RESULT: PASS), PNG files + md5 list REPORT me
