# Wave AU-TABLEFIND report (table/compare find highlight: current vs others + grid geometry)

## Scope
- OWNS: `crates/gui/src/views_table.rs` + `checks/gui-tablefind-au/**`.
- KEVL `app.rs` / `tree.rs` / `views.rs` / `viewmodel.rs` / `home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` untouched (read bhi minimal rakhi — sirf `paint_table` grid region + AT precedent signatures, grep-proof).
- Working tree me sibling waves ka in-progress diff hai (`views.rs` 387 lines folderfilter/AT ka, staged `checks/binary-au/**`, untracked `checks/gui-filterpaint-au/`) — haath nahi lagaya, stage nahi karunga.
- Colors FROZEN: koi naya hue/token nahi — sirf caller ke apne `fg/bg` + `TABLE_FIND_UNDERLINE_W` underline reuse, koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi, koi `.size(` nahi.
- NO push (orchestrator pushes).

## Problem
- Table/compare find bar sirf cursor/badge chalata tha (`table_find_match_rows` + `table_find_badge_label`), per-match block nahi dikhta tha.
- Current-match vs other-matches me koi visual farak nahi tha (Beyond Compare jaisa distinction missing).

## Delta (views_table.rs, +416/-0 pure addition)
- Pure helpers (koi egui ctx nahi, koi indexing nahi `.get()` only, saturating arithmetic):
  + `table_find_cell_spans(cell, needle, ignore_case)` — char-index spans, khali needle par empty, ascii-fold case-insensitive, overlap allow.
  + `table_find_row_spans(cells, ...)` — `(col,start,end)`, column order.
  + `table_find_grid_spans(rows, ...)` — `(row,col,start,end)`, row-major; row mapping Grid paint order jaisi.
  + `table_find_is_current(found, current)` + `table_find_current_cell(matches, cursor)` — N/M cursor se current mapping, bada cursor wrap (modulo).
  + `table_find_span_in_padded(start, end, width)` — trailing-pad clamp only (padding trailing-only hai isliye andar ke spans nahi hilte; cap ke bahar `None`).
  + `table_find_hit_in_key_col(view, col)` — `is_key_column` reuse (key hits ko wahi highlight, alignment grid se).
  + `TABLE_FIND_UNDERLINE_W = 1.0` — `INTRA_SPAN_UNDERLINE_W` precedent mirror, same value.
  + `table_find_hit_format(font_id, fg, bg, is_current)` — current = inverted block (no underline), other = underline-only line (no block). Same mono `FontId`, caller ke `fg/bg` reuse — block-vs-line se distinct.
  + `table_find_cell_job(cell, needle, ignore_case, is_current, font_id, fg, bg)` — ctx-free `LayoutJob`; overlap me consumed range skip taaki job text cell ke barabar rahe.
- Naye pub helpers `#[allow(dead_code)]` test-only precedent par (file me `table_column_widths`/`TABLE_MAX_COL_CHARS` wahi pattern) — Grid paint wiring (`views.rs` `paint_table`) orchestrator ke paas OPEN hai, caller signature stable.
- Self-review fix: clippy `manual_ignore_case_cmp` (`eq_ignore_ascii_case`), 2x `collapsible_if` (let-chains), test me `get_first` (`rows.first()`), test indexing `.get()` se replace.

## Tests (G3)
- Naye 8 (sab `tablefind_` prefix): empty-needle, case-modes (+multibyte char indices), overlapping, grid-spans-row-mapping, current-cell-cursor (+wrap + is_current), current/other-format-distinct (block-vs-line + same FontId), padded-spans-aligned-with-key-cols, cell-job-marks-current-vs-others (job text == cell, overlap me pehla hit jeetta hai).
- Full: `cargo test -p autocompare-gui` 515 passed, 0 failed. Hisaab: pre-edit baseline run me 498 note hua tha; meri diff se filter-proof 8 judte hain (`cargo test tablefind` = 8 passed, `tablefind_` prefix sirf mere tests me hai); baaki +9 parallel sibling activity ka delta hai (same checkout me waves parallel chal rahi hain — meri diff `views_table.rs` me +416/-0 pure addition hai, isliye mere naam par sirf 8 judte hain). Zero failures throughout.

## Gates
- G1 baseline: 498 passed, 0 failed (pre-edit run, `main@700669d`).
- G2 OWNS: mera diff sirf `views_table.rs` (+416/-0); `views.rs` diff sibling ka hai (zero AU symbols, `git diff views.rs | grep AU-symbols` = 0); stage sirf OWNS paths; NO-COLOR-TYPO-ADD (diff me `from_rgb`/`from_rgba`/`Visuals`/`SetVisuals`/`.size(` zero).
- G4 MS rust: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` exit 0 (zero warnings); `cargo fmt -- crates/gui/src/views_table.rs` only (bare fmt nahi); `rustfmt --check` clean; non-test me ZERO unwrap/expect/panic/todo; `forbid(unsafe_code)` kept.
- G5 harness: `run-geo-typo.sh checks/gui-tablefind-au/round-1` RESULT PASS; after.png 9624 bytes, magic `89504e470d0a1a0a` ok, distinct 256.
- G6: ye REPORT + GATES.md + round-1 artifacts; conventional commit, NO push.
