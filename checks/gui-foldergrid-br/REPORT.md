# REPORT — BR-FOLDERGRID (livecomp G-F2: folder dual-pane Name/Size/Modified grid)

Wave: BR-FOLDERGRID. OWNS: `crates/gui/src/views.rs` + `checks/gui-foldergrid-br/**`.
`tree.rs` NO-TOUCH (model sirf reuse). `app.rs` NO-TOUCH (live single-list wahi paint
karta hai — uska follow-up neeche). Commit locally, PUSH MANA.

## Gap (G-F2 P0, livecomp REPORT se)

Ours folder dual-pane rows plain text the (`R extra.txt`, `< f.txt` jaisi single
`selectable_label` strings via `dual_left_line` / `dual_right_line`). BC me dono
sides me 3 aligned columns hain (Name/Size/Modified, headers ke saath). Fix sirf
`views::paint_folder_dual` me hai — wahi live dual-pane paint hai.

## Geometry table (before → after, `paint_folder_dual` only)

| # | area | before | after |
|---|---|---|---|
| 1 | header row | 2 single labels (`left: Path Lsize \| Ltime` plain strings, `RichText.monospace.small.weak`) | 2 per-pane 3-col Grids (`folder_dual_header_left/right`, `num_columns(3)`, spacing `SECTION_GAP/TIGHT_GAP`), cells model headers (Path / L-R size / L-R modified), caption-strong |
| 2 | left row | 1 `selectable_label(dual_left_line)` plain string (glyph + naam + side cell ek me) | indent + glyph + 3-col Grid (`folder_dual_left_cells`, per-row id) — Name cell selectable, Size/Time labels; text `dual_grid_row_left` se |
| 3 | right row | 1 `selectable_label(dual_right_line)` plain string | indent + glyph + 3-col Grid (`folder_dual_right_cells`) — same shape, `dual_grid_row_right` se |
| 4 | ScrollArea | `ScrollArea::both().id_salt("folder_dual_rows")` + single `show_rows` | SAME (untouched — pehle wave ka stable-id kaam bana hua hai) |
| 5 | split/pager/select/menu | split handle, page bar, shared select, `···` stub menu, pending row | SAME (koi behavior change nahi) |
| 6 | colors | frozen palette + `glyph_color` | SAME (zero `Color32`/`Visuals` touch — grep clean) |
| 7 | typo | `.monospace()` direct + `.small().weak()` headers | cells `content_grid()` (Mono 12), headers `caption().rich().strong()` (caption-strong composition); koi `.size(` paint me nahi |

## Exact lines (`crates/gui/src/views.rs`, is commit me)

- Pure helpers (naye, unit-tested): `dual_grid_headers_left` (:4961),
  `dual_grid_headers_right` (:4972), `dual_grid_row_left` (:4984),
  `dual_grid_row_right` (:4999), `dual_grid_cell_role` (:5014),
  `dual_grid_header_role` (:5020). Text `tree::folder_column_defs` /
  `tree::folder_cell_text` / `tree::pad_folder_cell` se reuse; typo
  `TypoRole::content_grid()` / `TypoRole::caption()` alias reuse.
- Paint header grids: :5094 (`folder_dual_header_left`), :5107
  (`folder_dual_header_right`).
- Paint row grids: :5148-5149 (left cells), :5196-5197 (right cells).
- Frozen (rakhe, delete nahi): `dual_left_line` (:4930), `dual_right_line`
  (:4943) + unke purane tests — `#[allow(dead_code)]` contract note ke saath.
- Naye tests: `dual_grid_headers_match_column_defs` (:8710),
  `dual_grid_row_cells_carry_name_size_time` (:8735),
  `dual_grid_roles_reuse_typo_aliases` (:8766).

## Harness + gates (evidence, 2026-11-20 UTC)

- `cargo test -p autocompare-gui`: 614 passed, 0 failed (baseline 611 + 3 naye).
  Pure-OWNS state me verify (sibling dirty `gutter.rs`/`home.rs`/`views_table.rs`/
  `app.rs` stash karke; wapas pop — unko touch nahi kiya). Full workdir me 1 fail
  tha (`home::recent_row_typo...`) — sibling ke dirty `home.rs` se, HEAD par pass,
  mere diff se unrelated.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: pure-OWNS me
  zero warnings. Full workdir ke baaki errors (`home.rs` imports, `views_table.rs`
  borrow/dead) sibling files ke hain.
- `rustfmt --edition 2024 --check crates/gui/src/views.rs`: clean.
- Banned (non-test): `unwrap(`/`expect(`/`panic!`/`todo!` zero; `forbid(unsafe_code)` kept.
- Colors: diff me `Color32::from_rgb`/`from_rgba`/`Visuals::` zero.
- Harness `run-geo-typo.sh checks/gui-foldergrid-br/round-1`: `RESULT: PASS`
  (round 1/6), PNG 9695 B, magic `89504e470d0a1a0a`, distinct 255.
- `checks/opencode-waves/STATUS.md` untouched; commit me sirf OWNS paths.

## Follow-up (dusri waves ke liye, is wave me NO-TOUCH tha)

1. `app.rs` Folder branch ka live single-list (lines ~5028-5052: `glyph + rel_path`
   plain `selectable_label`) abhi bhi plain text hai — G-F2 ka baaki hissa wahi hai.
   Exact spec: us loop ko `folder_row_cells` / `folder_header_cells` wale 6-col Grid
   (`paint_folder` dead-code path jaisa, `views.rs` :4455 `folder_data_grid`
   precedent) par lao, ya dual-pane ko default banao. OWNS: `app.rs` (BR ke baad
   wali wave). `paint_folder` khud already grid me hai par `#[allow(dead_code)]`
   hai — App rewiring pending.
2. `tree.rs` model change is wave me zaroori nahi tha (existing
   `folder_column_defs`/`folder_cell_text`/`dual_left_cell` kaafi the) — isliye
   koi model spec nahi likha. Agar per-side Name column me sort/click chahiye to
   model wave me `DualTreeState` sort key add karna hoga.
3. Header me "left:"/"right:" prefix hata diya (model headers self-explanatory
   hain) — agar BC parity ke liye side tag chahiye to header Grid me 4th span cell
   nahi, pane title row wapas lao (sibling/style call).
