# Wave BR-FOLDERGRID — GATES (livecomp-BM FOLDER G-F2: dual-pane Name/Size/Modified grid columns)

OWNS: `crates/gui/src/views.rs` + `checks/gui-foldergrid-br/**` ONLY.
KEVL NO-TOUCH (read-only ok, kabhi edit nahi): `views_table.rs`, `theme.rs`, `gutter.rs`,
`home.rs`, `tree.rs`, `viewmodel.rs`, `app.rs`, `screenshot.rs`,
`checks/opencode-waves/STATUS.md` (MANA — orchestrator-owned), baaki `checks/**`
(sibling waves: `checks/gui-localrun-bs/` workdir me untracked reh sakta hai — pathspec
commit se bahar rakho).
Branch `main`, start me `git pull --ff-only`. Colors FROZEN (koi `Color32`/`Visuals`/
palette touch nahi). Typo: sirf `TypoRole::content_grid()` / `TypoRole::caption()`
alias reuse, koi `.size(` naya nahi. NO push (orchestrator pushes).

## Scope (livecomp REPORT G-F2 P0)

- G-F2 (P0): ours folder rows plain text hain (`R extra.txt`, `< f.txt`) — Name/Size/
  Modified COLUMNS nahi hain. BC me dono sides me 3 aligned columns hain (Name/Size/
  Modified headers ke saath). Fix: `views.rs` `paint_folder_dual` me har side ki row
  ko single `selectable_label` string (`dual_left_line` / `dual_right_line`) se nikal
  kar 3-column Grid me paint karo (Name + Size + Modified), har pane ke upar header
  row banao. Model `tree.rs` se reuse (`folder_column_defs`, `folder_cell_text`,
  `FolderColumn::LeftSize/RightSize/LeftTime/RightTime`) — `tree.rs` is wave me
  NO-TOUCH, model change chahiye to REPORT me exact spec likho.
- Pehle wave ka ScrollArea stable-id kaam bana rehna chahiye: single
  `ScrollArea::both().id_salt("folder_dual_rows")` + single `show_rows` call, koi
  naya scroll container nahi. Glyph colors, split handle, pager, select/menu/pending
  behavior same. `dual_left_line` / `dual_right_line` fns + unke tests frozen hain
  (rakho, delete mat karo).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: all green, 0 failed (count note karo — gutter+table waves ke baad ~611+)

## G2 — OWNS-only diff, KEVL + MANA + colors + typo-adds untouched

- CHECK: `git status --short && git diff HEAD --stat`
  + `git diff HEAD --name-only -- crates/gui/src/views_table.rs crates/gui/src/theme.rs crates/gui/src/gutter.rs crates/gui/src/home.rs crates/gui/src/tree.rs crates/gui/src/viewmodel.rs crates/gui/src/app.rs crates/gui/src/screenshot.rs | grep . && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff HEAD --name-only -- checks/opencode-waves/STATUS.md | grep . && echo MANA-TOUCHED || echo MANA-CLEAN`
  + `git diff HEAD -- crates/gui/src/views.rs | grep -E "^\ DeprecationWarning.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR`
  + `git diff HEAD -- crates/gui/src/views.rs | grep -F "+" | grep -F ".size("`
- EXPECT: diff me sirf `views.rs` + `checks/gui-foldergrid-br/**`; KEVL-CLEAN;
  MANA-CLEAN; NO-COLOR; `.size(` sirf 2 test-assert lines me
  (`dual_grid_*_role().size()` vs `TYPO_MONO/SMALL_SIZE` constants — koi naya paint
  size nahi, typo sirf `content_grid()`/`caption()` alias reuse)

## G3 — Dual-pane grid columns + header row, pure cells + unit-tested

- CHECK: `cargo test -p autocompare-gui folder_` + `cargo test -p autocompare-gui dual_`
- EXPECT: green, kam se kam ye tests:
  + `dual_grid_headers_match_column_defs` (naya: left == [Path, L size, L modified],
    right == [Path, R size, R modified], defs se reuse)
  + `dual_grid_row_cells_carry_name_size_time` (naya: naam + dir mark, missing side
    `-` dash, orphan align)
  + `dual_grid_roles_reuse_typo_aliases` (naya: cell role == content_grid == Mono 12,
    header role == caption == Small 11, koi naya size nahi)
  + purane `dual_lines_carry_glyph_name_and_side_cells`,
    `dual_lines_use_padded_side_cells`, `folder_header_cells_match_defs` same pass
- WHAT: `dual_grid_headers_left/right() -> [String; 3]`,
  `dual_grid_row_left/right(&FolderRow) -> [String; 3]` (name+dir-mark, size, time;
  text `tree::folder_cell_text` se), `dual_grid_cell_role()` (`content_grid`),
  `dual_grid_header_role()` (`caption`); paint me per-pane header Grid (3 cols,
  caption-strong composition) + per-row per-pane Grid (3 cols, name cell selectable,
  size/time labels); `ScrollArea::both().id_salt("folder_dual_rows")` single rehta hai

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code,
  `forbid(unsafe_code)` kept, naye fns `.get()`/saturating only jahan needed

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-foldergrid-br/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-foldergrid-br/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS` (max 6 rounds, same round dir par re-run),
  `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-foldergrid-br/REPORT.md checks/gui-foldergrid-br/round-1/after.png checks/gui-foldergrid-br/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-foldergrid-br && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish-Roman body allowed),
  sirf OWNS paths staged (pathspec commit — sibling untracked dirs untouched),
  REPORT Hinglish-Roman (Devanagari MANA): geometry table before/after + exact lines +
  follow-up, NO push
