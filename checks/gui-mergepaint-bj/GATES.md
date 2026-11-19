# Wave BJ-MERGEPAINT — GATES (BI-M1 global-take rewire + BI-M3 apply gate live)

OWNS: `crates/gui/src/views.rs` + `crates/gui/src/app.rs` + `checks/gui-mergepaint-bj/**` ONLY.
KEVL `tree.rs`/`theme.rs`/`views_table.rs`/`views_hex.rs`/`screenshot.rs` READ-ONLY
(read bhi sirf zaroori grep; koi edit nahi). `checks/opencode-waves/STATUS.md` ko CHHOONA MANA.
Colors FROZEN (koi hue/token change nahi). NO push (orchestrator pushes).
`git stash/push` kabhi nahi. Sequential order: BI-M1 pehle, BI-M3 baad me.

Scope: `checks/gui-mergesync-bi/REPORT.md` §spec table ke do follow-ups.
BI-M1 (P0): `paint_merge` ka global Take row `resolve_take(&model.lines, 0, choice)` se
SIRF index 0 resolve karta tha — multi-hunk me baaki hunks untouched rehte the.
Rewire: global row hatao, per-hunk + apply-all rakho (`stage_merge_pick_all` pure helper,
`apply_merge_picks` + App `merge_picks` adopt par). BI-M3 (P1): `paint_merge_apply`
(dead_code tha) ko Merge tab me wire karo (`merge_apply_enabled`,
`request_merge_apply`/`run_merge_apply` caller).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (591 passed, 0 failed, recorded 2026-11-20, `main@6fb7c7b`).
- EVIDENCE: baseline shell out (pre-edit tail copy).

## G2 — OWNS-only diff, KEVL + colors + STATUS untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views|app).rs|checks/gui-mergepaint-bj/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "tree.rs|theme.rs|views_table.rs|views_hex.rs|screenshot.rs|opencode-waves/STATUS.md" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/app.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `app.rs` + `checks/gui-mergepaint-bj/**`; KEVL-CLEAN; OWNS-CLEAN;
  NO-COLOR-ADD (paint strings + staging only, koi color/token change nahi).

## G3 — BI-M1: global row apply-all stage karta hai, model lines untouched (headless unit-tested)

- CHECK: `cargo test -p autocompare-gui merge`
- EXPECT: green, naya test `stage_merge_pick_all_covers_every_hunk` (1 pick + apply-all
  Right → har hunk Right, summary 2/2/0; khali total par khali) + purane merge tests green.
- WHAT: `paint_merge` me global Take Left/Right/Base/Both (index-0 `resolve_take`) hata;
  naya apply-all row "Take <side> (all)" har click par `stage_merge_pick_all` se sab
  hunks stage karta hai (temp store, model lines untouched). Naya pure helper
  `stage_merge_pick_all(picks, total, choice)` (stale out-of-range kept, adopt prunes).
  `reconcile_merge_picks` doc update (paint lines mutate nahi karta, aam frame adopt).

## G4 — BI-M3: Merge tab me Apply gate live (headless unit-tested core reuse)

- CHECK: `cargo test -p autocompare-gui merge_apply` + `cargo test -p autocompare-gui merge`
- EXPECT: green (purane `merge_apply_*` + `merge_*` tests, koi naya headless test nahi —
  core `run_merge_apply`/`merge_apply_enabled` pehle se tested; delta sirf paint call-site).
- WHAT: `Tab::Merge` branch me adopt ke baad `views::paint_merge_apply` call
  (`merge_apply_enabled`, `merge_dry_run`/`merge_output_path` bind, `merge_apply_state`,
  `merge_apply_last_summary`, `merge_apply_summary_line`); click par
  `request_merge_apply` phir `run_merge_apply`. `#[allow(dead_code)]` 5 helpers se hata
  (section title, path hint, state label, result format, `paint_merge_apply` live hain).

## G5 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs crates/gui/src/app.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/views.rs crates/gui/src/app.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs crates/gui/src/app.rs | grep -c "forbid(unsafe_code)"`
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/views.rs crates/gui/src/app.rs | grep -nE "\[[a-zA-Z_][a-zA-Z0-9_]*\s*\]" | grep -v "Vec\|String\|Option\|Result\|\[u8\]\|default" || echo NO-INDEXING`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS files par), ZERO-BANNED in non-test
  code (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept (2 files), naye helpers pure fns (koi indexing nahi — `.get()`
  nahi chahiye tha, range loop + `.push` only; `format!` nahi, sirf struct moves).
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen
  (sirf apne OWNS files par scoped fmt pattern).

## G6 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-mergepaint-bj/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-mergepaint-bj/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64.

## G7 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-mergepaint-bj/REPORT.md checks/gui-mergepaint-bj/round-1/after.png checks/gui-mergepaint-bj/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs crates/gui/src/app.rs checks/gui-mergepaint-bj/GATES.md checks/gui-mergepaint-bj/REPORT.md checks/gui-mergepaint-bj/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish-Roman body allowed,
  Devanagari MANA), sirf OWNS paths staged, working tree clean after commit, NO push.
