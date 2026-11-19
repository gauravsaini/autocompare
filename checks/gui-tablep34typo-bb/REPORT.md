# Wave BB-TABLEP34TYPO report (P3 data-grid + P4 overlays + typo adoption)

## Scope
- OWNS: `crates/gui/src/views.rs` + `checks/gui-tablep34typo-bb/**` ONLY.
- KEVL: `views_table.rs` / `views_hex.rs` / `theme.rs` / `tree.rs` / `screenshot.rs` / `app.rs` untouched (read-only). `views_table.rs` me koi change nahi — P3/P4 callee side stable hai.
- `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).
- Colors FROZEN: koi naya hue/token nahi, koi `.size(` add nahi (purane `.size(` calls sirf hatae hain). NO push (orchestrator pushes).

## Problem
- AY wave ne cell-grid geometry + paint helpers `views_table.rs` me single-source kar diye the, par `views.rs` data Grid purane literals par tha (`"table_data_grid"`, `[SECTION_GAP, TIGHT_GAP]`, plain `ui.label` loop) — key-col highlight + find underline ka paint-adoption missing tha. P2 (BA wave) sirf header Grid laaya tha; P3/P4 OPEN the.
- Typo wave ne `TypoRole` single-source kar diya tha, par paint purane `.monospace()` / `.small()` / `.size()` par tha — role adoption OPEN thi.

## Delta (views.rs only, 3 steps sequential — +331/-228)

### 1. P3 data-grid (AY REPORT §P3 exact)
- `paint_table(ui, model, pager)` → `paint_table(ui, view, model, pager)`; caller `paint_table_keyed` 1-line `view` thread (P1 spec ka baaki hissa already live tha).
- Grid: `egui::Grid::new("table_data_grid")` → `views_table::TABLE_CELL_GRID_ID`; spacing → `[TABLE_GRID_COL_GAP, TABLE_GRID_ROW_GAP]` (8.0/4.0, same); `.num_columns(table_grid_columns(&widths))` rakha; glyph block rakha (row-status/changed tint caller-side).
- Cell loop → indexed `for (col_idx, cell) in table_row_cells(...).iter().enumerate()` + `paint_table_data_cell(ui, view, cell, data_row_idx, col_idx, find_ctx, &style)`. `data_row_idx` naya pure helper `table_data_index_map(model)` se (hdr-filtered fwd map; model-row index NAHI).
- `find_ctx = TableCellFindCtx { needle, ignore_case: false, current }` (`current = table_find_current_cell(&matches, cursor)`); `style` per-row (`font` shared mono, `fg` = CHANGED_FG jab `row.changed` warna `visuals().text_color()` — Plain/KeyStrong arms caller fg reuse karte hain, `bg` = `visuals().panel_fill`).
- Find-source switch: `matches` ab `table_find_match_rows(&data_cells, &needle, false)` par (data/grid coords align; badge count wahi jo Grid dikhata hai). `data_cells`/`widths`/maps find-bar se PEHLE compute hote hain (pure move, paint order same). `bring_into_view` reverse map (`model_idx_of_data_row`) se model index par wapas.
- Side-effect: `table_model_text_rows` non-test me orphan hua → `#[allow(dead_code)]` test-only contract (2 tests isi par lock hain; Wave H precedent). Temp-store `needle` ab `clone()` hota hai (find_ctx borrow alive rehta hai).

### 2. P4 painter overlays (AY REPORT §P4 exact)
- `show_rows` closure ke andar Grid ke baad: key boxes (`table_key_box_rects` → `ui.painter().rect_stroke(..., CornerRadius::ZERO, Stroke::new(TABLE_FIND_UNDERLINE_W, fg), StrokeKind::Middle)`) + find underlines (`table_find_grid_spans` → `table_cell_rect` → `table_find_underline_rects_for_cell` → `ui.painter().line_segment(...)`).
- Measure paint-side: `char_w = ctx.fonts_mut(glyph_width(&mono_font, 'M'))`, `col_widths_px = char_w * widths[col]`, origin = pehle painted data cell (`ui.min_rect().min` + uska data-row index; `Cell<Option<...>>` se capture). Overlays ScrollArea content `ui` se paint hote hain → viewport clip free me milta hai.
- Kharab inputs graceful hain (helpers `None`/empty dete hain; `unwrap_or` fallback; koi `unwrap` nahi).

### 3. Typo adoption (TYPOTOKENS REPORT "views.rs EXACT wiring spec" — line numbers old main se content-match karke re-verify)
- Chrome aliases already wired the (verify only, koi change nahi).
- Home card: title → `HomeTitle.rich`, desc branch → `HomeDesc.rich`, button → `HomeButton.rich`. `home_title/button_paint_size` + `home_desc_paint_is_small_weak` ab test-only (`#[allow(dead_code)]`, tests contract lock karte hain).
- Mono panes → `TypoRole::Mono.rich(..)`: text lines, hex addr/hex/ascii, 3 merge panes, merge marker lines (`.color().strong()` condition rakha), table header glyph/key-cells(`.strong()`)/rule(`.weak()`), fallback + data glyph/body, dono `▶` (`.strong()`), teenon `" | "` + CONTENT_LEAD (`.color(gutter_dim)` rakha), `TextStyle::Monospace.resolve` ×2 → `Mono.resolved_font_id` (exact drop-in), folder header + rule (`Mono.rich.weak`), error `error:` (`Mono.rich.strong` + DELETED_FG rakha).
- Small captions → `TypoRole::Small.rich(..)` (`+.strong()` jahan aaj strong hai — strong weak par jeetta hai): hex/table/folder/sync/dual pager labels ×5, byte-grid note, merge_pick summary + decision, hunk title (strong), pane headers Left/Base/Right (strong), `(empty)` ×3, `Take:`, merge-apply section title (strong), table summary, `paint_note`, `Show:`, nav badge, rows status, dual preview note, dual pending, `Layout:`, left/right/overlap/verdict/overlay.
- SKIP (spec khud defer/boundary kehta hai): judgment calls (dual meta `.monospace().small().weak()`, default-size weak labels), boundary (folder_filter_rich, sync badge/summary sizes, views_table/screenshot internals), folder body mono rows, picture `{}x{}` mono, zoom/audio/find-bar sites (dusre waves ke domain).

## Pixel-delta note (honest)
- Runtime defaults (koi TextStyle override nahi hai): `.monospace()` = 13px mono, `.small()` = 9px proportional. Roles: Mono 12px, Small 11px. Isliye screenshots pixel-bit-stable NAHI hain (after.png 9627B vs purane baselines) — ye adoption ka intended effect hai (render ab headless math — gutter 12px — se align hota hai). Harness pixel-equality check nahi karta, isliye PASS sahi hai.

## Tests (G3)
- Naya 1 (`cargo test -p autocompare-gui tablep34` = 1 passed): `tablep34_data_index_map_skips_header_rows` (hdr → None, data order, reverse map).
- Full: `cargo test -p autocompare-gui` 569 passed, 0 failed. Hisaab: HEAD baseline 568 + meri 1.

## Gates
- G1 baseline: 568 passed, 0 failed (pre-edit run, `round-1/baseline-tests.out`).
- G2 OWNS: mera diff sirf `views.rs` (+ 331/-228) + `checks/gui-tablep34typo-bb/**`; KEVL-CLEAN; NO-COLOR-TYPO-ADD (diff me `from_rgb`/`from_rgba`/`Visuals`/`SetVisuals`/`.size(` additions zero).
- G4 MS rust: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` exit 0 (zero warnings); `rustfmt --edition 2024 --check crates/gui/src/views.rs` clean (sirf OWNS file); non-test me ZERO unwrap/expect/panic/todo; `forbid(unsafe_code)` kept.
- G5 harness: `run-geo-typo.sh checks/gui-tablep34typo-bb/round-1` RESULT PASS; after.png 9627B, magic `89504e470d0a1a0a` ok, distinct 256; `file` = PNG 960x600 RGBA.
- G6: ye REPORT + GATES.md + round-1 artifacts; conventional commit, NO push.
