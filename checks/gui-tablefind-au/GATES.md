# Wave AU-TABLEFIND — GATES (table/compare find highlight: current vs others + grid geometry)

OWNS: `crates/gui/src/views_table.rs` + `checks/gui-tablefind-au/**` ONLY.
KEVL: `app.rs` / `tree.rs` / `views.rs` / `viewmodel.rs` / `home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` KO HAATH MAT LAGANA (read bhi minimal rakhi — sirf `paint_table` grid region + AT precedent signatures, grep-proof).
Colors FROZEN (koi naya hue/token nahi — sirf caller ke apne `fg/bg` + `TABLE_FIND_UNDERLINE_W` underline reuse, koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi, koi `.size(` nahi).
Geometry first, typography second (match spans + row mapping + key-col alignment pehle, same mono `FontId` baad me, koi naya size nahi).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. Repo ke bahar koi copy/sandbox nahi. `git stash/push` kabhi nahi.

Scope: table/compare find match highlight parity. Aaj find bar sirf cursor/badge chalata hai (`table_find_match_rows` + `table_find_badge_label`), per-match block nahi dikhta aur current-vs-other me koi farak nahi hai. Is wave ka delta `views_table.rs` me hai: pure cell/row/grid span helpers + N/M cursor se current mapping + padded-width clamp + key-col hit query + current-vs-other `TextFormat` + headless `LayoutJob` builder. Grid paint wiring OPEN hai (orchestrator `views.rs` `paint_table` Grid me thread karega — caller signature stable, isliye naye helpers `#[allow(dead_code)]` test-only precedent par).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `498 passed; 0 failed` (recorded 2026-11-20, `main@700669d`, pre-edit run)
- EVIDENCE: run output se noted (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views_table.rs|checks/gui-tablefind-au/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views.rs|views_hex.rs|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_table.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: working tree me sirf `views_table.rs` mera diff hai (`views.rs` diff sibling wave ka hai — haath nahi lagaya, stage nahi karunga); stage sirf OWNS paths; KEVL-CLEAN (mere naam par zero); NO-COLOR-TYPO-ADD
- EVIDENCE: `round-1/owns-diff.out`

## G3 — Current vs others + grid geometry, unit-tested

- CHECK: `cargo test -p autocompare-gui tablefind`
- EXPECT: green, 8 naye tests (sab `tablefind_` prefix):
  + `tablefind_cell_spans_empty_needle_gives_no_hits` (khali needle/lambi needle par empty, panic nahi)
  + `tablefind_cell_spans_case_modes` (case-sensitive vs ignore-case, multibyte char indices)
  + `tablefind_cell_spans_overlapping_ok` (overlap allow, sorted, clamped)
  + `tablefind_grid_spans_map_rows_and_cols` (row-major `(row,col,start,end)`, row mapping, khali needle empty)
  + `tablefind_current_cell_tracks_cursor_with_wrap` (cursor mapping + wrap + `table_find_is_current`)
  + `tablefind_current_other_formats_differ` (current = inverted block no-underline, other = underline-only no-block, same mono `FontId`, no new size)
  + `tablefind_padded_spans_stay_aligned_with_key_cols` (trailing-pad alignment, cap clamp, key-col query, spans padded me nahi hilte)
  + `tablefind_cell_job_marks_current_vs_others` (job text cell ke barabar, sections block-vs-line, overlap me pehla hit jeetta hai)
- WHAT: `views_table.rs` me pure fns (koi egui ctx nahi, koi indexing nahi `.get()` only, saturating arithmetic):
  + `table_find_cell_spans(cell, needle, ignore_case) -> Vec<(usize,usize)>` (char indices, empty needle par empty, ascii-fold case-insensitive, overlap allow)
  + `table_find_row_spans(cells, needle, ignore_case) -> Vec<(col,start,end)>` (column order)
  + `table_find_grid_spans(rows, needle, ignore_case) -> Vec<(row,col,start,end)>` (row-major, Grid paint order jaisi row mapping)
  + `table_find_is_current(found, current) -> bool` + `table_find_current_cell(matches, cursor) -> Option<(row,col)>` (N/M cursor mapping, wrap modulo)
  + `table_find_span_in_padded(start, end, width) -> Option<(usize,usize)>` (trailing-pad clamp only)
  + `table_find_hit_in_key_col(view, col) -> bool` (`is_key_column` reuse — key hits ko wahi highlight, alignment grid se)
  + `TABLE_FIND_UNDERLINE_W = 1.0` (`INTRA_SPAN_UNDERLINE_W` precedent mirror, same value)
  + `table_find_hit_format(font_id, fg, bg, is_current) -> TextFormat` (current = inverted block, other = underline-only line; same mono `FontId`, caller ke `fg/bg` reuse)
  + `table_find_cell_job(cell, needle, ignore_case, is_current, font_id, fg, bg) -> LayoutJob` (ctx-free; overlap me consumed range skip taaki job text cell ke barabar rahe)
- EVIDENCE: `round-1/tablefind-tests.out`

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views_table.rs && git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/views_table.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_table.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views_table.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (sirf OWNS file par — bare `cargo fmt` mat chalao, sibling files untouched rahen), ZERO-BANNED in non-test code (`unwrap_or` allowed — banned sirf `unwrap()`/`expect()` hain), `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only; saturating arithmetic)
- NOTE: naye pub helpers `#[allow(dead_code)]` test-only precedent par hain (file me `table_column_widths`/`TABLE_MAX_COL_CHARS` wahi pattern) — Grid paint wiring orchestrator ke paas OPEN hai
- EVIDENCE: `round-1/clippy.out`, `round-1/fmt.out`

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablefind-au/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tablefind-au/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- EVIDENCE: `round-1/geo-typo-report.md`, `round-1/after.png`

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablefind-au/REPORT.md checks/gui-tablefind-au/round-1/after.png checks/gui-tablefind-au/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_table.rs checks/gui-tablefind-au/GATES.md checks/gui-tablefind-au/REPORT.md checks/gui-tablefind-au/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed), sirf OWNS paths staged (sibling `views.rs` diff + staged `checks/binary-au/**` ko haath nahi), working tree me meri files clean after commit, NO push
