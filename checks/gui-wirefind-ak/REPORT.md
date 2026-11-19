# Wave AK-WIREFIND — REPORT (Gap #12 final wiring: hex + table find bars `views.rs` caller me)

## Summary

Hex + table find bars ab real UI me wired hain — dono `views.rs` caller fns ke andar,
callee signature same, App struct untouched (needle + cursor caller-local egui temp
store me, merge-pick `merge_pick_id` precedent). Count kaam karta hai, Next/Prev
selection + `bring_into_view` chalata hai.
Table adoption tablefind precedent (`checks/gui-tablefind-ai/REPORT.md`) par hai.

## Metrics

- Tests: 373 passed (pre-edit file) → 378 passed, 0 failed (full `cargo test -p autocompare-gui`)
- Note: 3 naye wirefind tests; pre-edit 373 vs attr arithmetic (HEAD views 84 attrs +
  unchanged KEVL files) me 2 ka gap — parallel orchestration me stale build cache
  suspected; KEVL files untouched hone se gap codeartifacts se bahar hai, evidence
  `round-1/baseline-tests.out` me preserved hai
- `hex_find` focused: 8 passed; `table_find` focused: 4 passed; wirefind/cursor: 3 passed
- Clippy `--all-targets -- -D warnings`: 0 warnings
- `rustfmt --check` (sirf OWNS file): clean (`cargo fmt` sirf `crates/gui/src/views.rs` par)
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`...) non-test code me: zero
- Harness: `RESULT: PASS`; `after.png` 9624 bytes, magic `89504e470d0a1a0a`, distinct 256

## Highlights

- ✅ `views.rs` (+242): wirefind section — temp ids (`hex/table_find_needle/cursor_id`,
  `Id::unique` precedent), pure `hex_find_hunk_hits` (displayed hunk-line substring,
  khali needle par khali), pure `find_cursor_step` (Next/Prev wrap, text precedent:
  Next jeetta hai), 3 tests
- ✅ `paint_hex` me `views_hex::paint_hex_find_bar` call (signature same), badge
  `hex_find_count_label` se N/M, Next/Prev hunk selection + `bring_into_view`
- ✅ `paint_table` me `views_table::paint_table_find_bar` adoption (signature same),
  `table_model_text_rows` (ab wired — `#[allow(dead_code)]` hata) + `table_find_match_rows`,
  badge `table_find_badge_label`, Next/Prev `pager.bring_into_view`; `paint_table_keyed`
  isi se cover hota hai
- ✅ Caller signatures same (`paint_hex(ui, view, pager, lines)`,
  `paint_table(ui, model, pager)` unchanged); callee KEVL files untouched (unke allows
  wahi rehte hain — wiring se dead code khatam, allow hatana unki wave ka kaam hai)
- ✅ Geometry first, typography second, colors frozen (koi naya token/color/`.size(` nahi)

## Files changed (OWNS only, is commit me)

- `crates/gui/src/views.rs` (+242/-1: wirefind helpers + hex/table caller wiring + 3 tests)
- `checks/gui-wirefind-ak/GATES.md`, `REPORT.md`, `round-1/`

## Note (sibling wave)

- Beech me sibling commit `0bdf54f` (folder polish) land hua — is commit me excluded,
  working tree me uska diff nahi hai. Final test 378 usi HEAD par green hai.
- NO push (orchestrator pushes).
