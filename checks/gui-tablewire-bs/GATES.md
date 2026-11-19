# Wave BS-TABLEWIRE — GATES (P0: table dedup + grid polish, views_table.rs + app.rs)

OWNS: `crates/gui/src/views_table.rs` + `crates/gui/src/app.rs` (sirf Table branch toolbar hunk) + `checks/gui-tablewire-bs/**`.
KEVL NO-TOUCH: `views.rs` / `theme.rs` / `gutter.rs` / `home.rs` / `tree.rs` / `viewmodel.rs` / `screenshot.rs` / `scripts/` / `checks/opencode-waves/STATUS.md` (MANA) / baaki `checks/` dirs. `views.rs` BR wave owns karta hai — is wave me sirf REPORT me exact spec.
Colors FROZEN (koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi, koi `.size(` add nahi).
Geometry first, typography second (sirf existing tokens reuse: `TABLE_FIND_BAR_H`, `TABLE_FIND_NEEDLE_W`, `TABLE_FIND_BADGE_PAD`, `TABLE_KEY_COL_GAP`, `TABLE_GRID_COL_GAP`, `TABLE_GRID_ROW_GAP`, `TABLE_CELL_GLYPH_PAD`, `TypoRole::Mono`/`Small`).
NO push (orchestrator pushes). Repo ke bahar koi copy nahi. `git stash/push` kabhi nahi.

Scope (sirf P0 — P1/P2 mat chhuna):
- `views_table.rs`: Find controls ka single source — `paint_table_find_bar` body ek shared helper me, `paint_table_toolbar` wahi helper chalaye (do Find implementations nahi). Header Grid col count `table_cell_grid_columns(&widths)` reuse (data Grid wala single source). Naye pure helpers `table_column_letter` (A..Z, AA..) + `table_row_gutter_label` (1-based) — paint wiring BR ke paas (views.rs NO-TOUCH), yahan sirf helpers + tests.
- Dashed `----` remainder: `views_table.rs` code me koi dashed separator nahi (grep-proof: sirf doc-comment me G-B1 zikr). Asli remainder `views.rs paint_table` me hai (`format_table_header_rule` label) — NO-TOUCH, REPORT me BR spec.
- Single note: `views_table.rs` me note ek hi jagah (key bar ka `positional_note()`); live double-note `paint_table_keyed` (bar vs outer block) ka fix BR spec me (views.rs NO-TOUCH).
- `app.rs` Table branch: `paint_table_find_bar` + alag Prev/Next page row hata kar single `paint_table_toolbar` call (App state wiring: `table_find_next_row`/`table_find_prev_row`, `table_page_next`/`table_page_prev`, `format_table_page_label`, needle-change par cursor reset barkarar).

## G1 — Baseline green (before BS edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `611 passed; 0 failed` (recorded 2026-11-20, BQ-after HEAD)
- EVIDENCE: `round-1/baseline-tests.out`

## G2 — OWNS-only diff, KEVL + STATUS + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views_table.rs|crates/gui/src/app.rs|checks/gui-tablewire-bs/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|theme.rs|gutter.rs|home.rs|tree.rs|viewmodel.rs|screenshot|STATUS.md" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_table.rs crates/gui/src/app.rs | grep -E "^\+\.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
  + `git diff -- crates/gui/src/app.rs` sirf Table branch toolbar hunk (find_bar + page row → toolbar call, koi logic/wording change nahi)
- EXPECT: diff sirf `views_table.rs` + `app.rs` (1 hunk) + `checks/gui-tablewire-bs/**`; KEVL-CLEAN; NO-COLOR-TYPO-ADD
- EVIDENCE: `round-1/owns-diff.out`

## G3 — Dedup wiring, full suite green (+2 naye tests)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `613 passed; 0 failed` (baseline 611 + 2 naye: column-letter sequence + gutter-label/cols-reuse)
- WHAT: shared Find-controls helper (find_bar + toolbar same body); header Grid `table_cell_grid_columns` reuse; `table_column_letter` + `table_row_gutter_label` pure + tested
- EVIDENCE: `round-1/tablewire-tests.out`

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views_table.rs; echo fmt:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/app.rs; echo fmt:$?`
  (sirf OWNS files; bare `cargo fmt` mat chalao)
  + `git diff -- crates/gui/src/views_table.rs crates/gui/src/app.rs | grep -nE "^\+\.*(unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!)" || echo ZERO-BANNED-ADDED`
  + `head -25 crates/gui/src/views_table.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0`, `fmt:0` dono files, ZERO-BANNED-ADDED (added lines me banned nahi — pre-existing `unwrap_or_default`/`unwrap_or` allowed), `forbid(unsafe_code)` kept
- EVIDENCE: `round-1/clippy.out`, `round-1/fmt.out`

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablewire-bs/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tablewire-bs/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- EVIDENCE: `round-1/geo-typo-report.md`, `round-1/after.png`

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablewire-bs/REPORT.md checks/gui-tablewire-bs/round-1/after.png checks/gui-tablewire-bs/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_table.rs crates/gui/src/app.rs checks/gui-tablewire-bs/GATES.md checks/gui-tablewire-bs/REPORT.md checks/gui-tablewire-bs/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish-Roman body allowed), sirf OWNS paths staged, working tree clean after commit, NO push
