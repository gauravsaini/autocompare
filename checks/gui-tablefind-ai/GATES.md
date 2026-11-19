# Wave AI-TABLEFIND — GATES (Gap #12 table hissa: find box + Next/Prev + N/M count)

OWNS: `crates/gui/src/views_table.rs` + `crates/gui/src/app.rs` + `checks/gui-tablefind-ai/**` ONLY.
KEVL `views.rs` / `viewmodel.rs` / `tree.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only, grep-proof only; `views.rs` caller me koi edit nahi — caller signature same).
Colors FROZEN (koi hue change nahi — sirf existing small/weak typo, koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi).
Geometry first, typography second. Clean-room: Beyond Compare bahar se observe only, apne shabdon me.
MS rust guidelines: `forbid(unsafe_code)` kept; non-test code me `unwrap`/`expect`/`panic`/`todo` mana hai (`.get()`/saturating/`unwrap_or` only).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. `git stash/push` kabhi nahi.

Scope: table find box + Next/Prev + N/M count (text/folder precedent reuse).
Table window ke andar find row paint karo (`views_table` fn ke andar, caller signature same):
`views_table.rs` me nayi pure find model (`table_find_match_rows` + `table_find_badge_label`) aur nayi paint fn
(`paint_table_find_bar`: needle field + Prev/Next + badge, text precedent geometry/typo reuse);
`app.rs` me App state (`table_find_cursor` + matches/total/position/badge/set_needle/next_row/prev_row, text precedent mirror)
aur table tab wiring (find row `views_table::paint_table_find_bar` se, pager buttons alag row me).
Legacy `table_find_next_paged` + `table_find_from` compat untouched (existing test green rehta hai).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `355 passed; 0 failed` (recorded 2026-11-20, `main@bbfad4b`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views_table|app).rs|checks/gui-tablefind-ai/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|viewmodel.rs|tree.rs|views_hex.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_table.rs crates/gui/src/app.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views_table.rs` + `app.rs` + `checks/gui-tablefind-ai/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Table find + count kaam karta hai, unit-tested

- CHECK: `cargo test -p autocompare-gui table_find`
- EXPECT: green, kam se kam ye tests:
  + `table_find_scans_cells_row_major_with_wrap` (purana, same shape pass)
  + `table_find_match_rows_collects_all_hits` (naya, views_table.rs: sab cells row-major, khali needle khali, case flag)
  + `table_find_badge_labels_position_over_total` (naya, views_table.rs: `Some(2),5 -> "2/5"`, `None,0 -> "0/0"`)
  + `table_find_bar_tracks_matches_and_badge` (naya, app.rs: Next/Prev wrap, badge N/M, khali needle `0/0` + no nav)
  + `table_pager_syncs_and_find_brings_into_view` (purana, legacy `table_find_next_paged` compat pass)
- WHAT: `views_table.rs` me nayi pure `table_find_match_rows(rows, needle, ignore_case) -> Vec<(usize, usize)>`
  (har matching cell row-major, khali needle par khali, `.get()`/saturating only, koi indexing nahi)
  aur `table_find_badge_label(position: Option<usize>, total: usize) -> String`
  (`Some(pos) -> "pos/total"`, `None -> "0/total"`, text precedent mirror);
  nayi paint `paint_table_find_bar(ui, needle: &mut String, badge: &str) -> (bool, bool)`
  (table window ke andar find row: `horizontal_wrapped`, min-size `avail x 28.0`, `Find:` small weak label,
  needle `desired_width(120.0)` + hint `needle`, `small_button("◀ Prev")` / `small_button("Next ▶")`,
  badge right-end `small().weak()` + `4.0` pad; GEO `28.0/120.0/4.0` text precedent reuse, koi naya token/color nahi);
  `app.rs` me nayi App state `table_find_cursor: usize` + `table_find_matches/total/position/badge/set_needle/next_row/prev_row`
  (text precedent mirror: matches model order, cursor wrap, `bring_into_view`, status `table find: row R (N/M)`),
  table tab wiring find row ko `views_table::paint_table_find_bar` se chalati hai (caller signature same, `views.rs` untouched),
  pager Prev/Next-page buttons alag row me rehte hain

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views_table.rs crates/gui/src/app.rs; echo fmt:$?`
  (sirf OWNS files; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_table.rs crates/gui/src/app.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views_table.rs crates/gui/src/app.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablefind-ai/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tablefind-ai/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablefind-ai/REPORT.md checks/gui-tablefind-ai/round-1/after.png checks/gui-tablefind-ai/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_table.rs crates/gui/src/app.rs checks/gui-tablefind-ai/GATES.md checks/gui-tablefind-ai/REPORT.md checks/gui-tablefind-ai/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
