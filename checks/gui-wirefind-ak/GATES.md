# Wave AK-WIREFIND — GATES (Gap #12 final wiring: hex + table find bars `views.rs` caller me)

OWNS: `crates/gui/src/views.rs` + `checks/gui-wirefind-ak/**` ONLY.
KEVL `app.rs` / `views_hex.rs` / `views_table.rs` / `home.rs` / `viewmodel.rs` / `tree.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read bhi avoid karo; sirf `views_hex::paint_hex_find_bar` / `hex_find_count_label` aur `views_table::paint_table_find_bar` / `table_find_match_rows` / `table_find_badge_label` read-only call karo, signature same rakho).
Colors FROZEN (koi hue change nahi — sirf existing small/weak typo via callee fns, koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals`/`.size(` add nahi).
Geometry first, typography second. Clean-room: Beyond Compare bahar se observe only, apne shabdon me.
MS rust guidelines: `forbid(unsafe_code)` kept; non-test code me `unwrap`/`expect`/`panic`/`todo` mana hai (`.get()`/saturating/`unwrap_or` only).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. `git stash/push` kabhi nahi.

Scope: Gap #12 final wiring — caller `views.rs` me:
(1) hex `paint_hex` se `views_hex::paint_hex_find_bar` ko call karo (signature same);
(2) table `paint_table` (covers `paint_table_keyed`) me `views_table::paint_table_find_bar` adoption (tablefind REPORT `checks/gui-tablefind-ai/REPORT.md` precedent).
Dono me local needle + cursor egui temp store me (signature same, App struct untouched — merge-pick `merge_pick_id` precedent), count kaam karta hai, Next/Prev selection + `bring_into_view` chalata hai.
Caller wiring ke baad `dead_code` allows hatao jahan wiring complete ho (`table_model_text_rows` ka allow hatata hai — ab `paint_table` me used; callee files KEVL hain isliye unke allows untouched rehte hain).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `373 passed; 0 failed` (recorded 2026-11-20, `main@293e20c`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-wirefind-ak/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|views_hex.rs|views_table.rs|home.rs|viewmodel.rs|tree.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views.rs` + `checks/gui-wirefind-ak/**`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Hex + table find bars wired, count kaam karta hai, unit-tested

- CHECK: `cargo test -p autocompare-gui hex_find` + `cargo test -p autocompare-gui table_find` + full `cargo test -p autocompare-gui`
- EXPECT: green, kam se kam ye tests:
  + `hex_find_hunk_hits_search_displayed_hunk_lines` (naya, views.rs: formatted hunk line substring, khali needle khali, missing khali)
  + `find_cursor_step_wraps_next_prev` (naya, views.rs: khali/zero par None, no-click None, next/prev wrap)
  + `table_wirefind_matches_painted_text_and_badge` (naya, views.rs: `table_model_text_rows` + `table_find_match_rows` + `table_find_badge_label` N/M)
  + purane: `table_model_text_rows_carry_painted_text_for_find`, `hex_find_*` (views_hex KEVL untouched, same values pass), `table_find_*` (views_table KEVL untouched, same values pass)
- WHAT: `views.rs` me naye pure helpers (`hex_find_hunk_hits`, `find_cursor_step`, temp ids `hex/table_find_needle/cursor_id`) + `paint_hex` me `paint_hex_find_bar` call (needle/cursor temp, badge `hex_find_count_label`, Next/Prev `view.selected` + `pager.bring_into_view`) + `paint_table` me `paint_table_find_bar` adoption (needle/cursor temp, `table_model_text_rows` + `table_find_match_rows`, badge `table_find_badge_label`, Next/Prev `pager.bring_into_view`); signatures same (`paint_hex(ui, view, pager, lines)`, `paint_table(ui, model, pager)` unchanged); callee signatures same; `.get()`/saturating only, koi indexing nahi

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views.rs && rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-wirefind-ak/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-wirefind-ak/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-wirefind-ak/REPORT.md checks/gui-wirefind-ak/round-1/after.png checks/gui-wirefind-ak/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-wirefind-ak/GATES.md checks/gui-wirefind-ak/REPORT.md checks/gui-wirefind-ak/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed), sirf OWNS paths staged, working tree clean after commit, NO push
