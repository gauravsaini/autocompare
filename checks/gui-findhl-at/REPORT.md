# Wave AT-FINDHL report (text find highlight: current vs others + wrap geometry)

## Scope
- OWNS: `crates/gui/src/views.rs` + `checks/gui-findhl-at/**`.
- KEVL `app.rs` / `viewmodel.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` /
  `home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` untouched (read bhi avoid kiya).
  `app.rs` + `tree.rs` me sibling folderfilter wave ka in-progress diff hai — haath nahi lagaya, stage nahi karunga.
- Colors FROZEN: koi naya hue/token nahi — sirf row ke apne `fg/bg` + `span_colors`
  inversion + `INTRA_SPAN_UNDERLINE_W` underline reuse.
- Geometry first (find spans char indices + `normalize_intra_spans` clamp/merge +
  `wrap_chunks`/`wrap_chunk_spans` reuse), typography second (same mono `FontId`,
  koi naya size nahi — emphasis sirf block-vs-line se).

## Problem
- `paint_text_with_find` sirf row select karta tha (`model.selected`), per-match
  block nahi dikhta tha; wrap-on continuations par match highlight kho jata tha.
- Current-match vs other-matches me koi visual farak nahi tha.

## Delta (views.rs)
- Pure helpers (koi egui ctx nahi, koi indexing nahi `.get()` only, saturating arithmetic):
  + `FindHl` enum (`Plain`/`Accent`/`Other`/`Current`) — find wins over accent.
  + `find_line_spans(line, needle, ignore_case)` — char-index spans, khali needle
    par empty, ascii-fold case-insensitive, overlap allow.
  + `find_is_current_row(row_idx, current)` — `Some(row) == current`.
  + `find_hit_format(...)` — current = inverted block (no underline), other =
    underline-only line (no block). Same mono `FontId`, same `fg/bg`, same
    underline width — block-vs-line se distinct, accent (block+line) se bhi alag.
  + `find_pane_segments(...)` — no-wrap panes, `pane_segments` wahi geometry
    (truncation + trailing pad).
  + `find_chunk_segments(...)` — wrap continuations, `wrap_chunk_spans` reuse.
  + `find_segment_format(...)` — kind se format router.
  + `FindPaintCtx` (frame ctx: needle + flag + current) + `FindRowPaint`
    (row input bundle — argument-count lint ke andar, `TextPaneLayout` precedent).
- Paint threading: `paint_text_with_prefs` ab thin wrapper hai, body
  `paint_text_body(..., find: Option<FindPaintCtx>)` me hai; `paint_text_with_find`
  find bar + advance ke baad `Some(ctx)` se body chalata hai (needle copy taaki
  double-`&mut` borrow na ho). Plain path byte-identical hai (find `None` par
  purane painters).
- Public signatures same (`paint_text_with_prefs`, `paint_text_with_find`),
  isliye App call-sites untouched.

## Tests (G3)
- Naye (6, sab `findhl_` prefix): empty-needle, case-modes, overlapping,
  current/other-format-distinct (block-vs-line + same FontId), wrap-chunks-keep-match
  (`"hi world"` width 5 par `"world"` continuation me milta hai), pane-segments-mark-current-row.
- Full: `cargo test -p autocompare-gui` 498 passed, 0 failed
  (baseline 481 + 6 naye + 11 sibling folderfilter ke; sibling diff unstaged rakha).

## Gates
- G1 baseline: 481 passed (round-1/baseline-tests.out, `main@22e5e3f`).
- G2 OWNS: stage sirf `views.rs` + `checks/gui-findhl-at/**`; KEVL files me mera
  zero diff (`app.rs`/`tree.rs` diff sibling ka hai — unstaged); NO-COLOR-ADD
  (diff me `Color32::from_rgb`/`from_rgba`/`Visuals::` zero).
- G4 MS rust: `cargo fmt -- crates/gui/src/views.rs` only (bare fmt nahi);
  `rustfmt --check` clean; non-test me ZERO unwrap/expect/panic/todo;
  `forbid(unsafe_code)` kept; clippy `--all-targets -D warnings` exit 0
  (2 self-review fix: `eq_ignore_ascii_case` + find-args bundle).
- G5 harness: `run-geo-typo.sh checks/gui-findhl-at/round-1` RESULT PASS;
  after.png 9624 bytes, magic `89504e470d0a1a0a` ok, distinct 256.

## Commit
- Local only, no push. Sibling `checks/gui-folderfilter-at/` untracked rehta hai.
