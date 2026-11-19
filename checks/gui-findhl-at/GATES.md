# Wave AT-FINDHL — GATES (text find highlight: current vs others + wrap geometry)

OWNS: `crates/gui/src/views.rs` + `checks/gui-findhl-at/**` ONLY.
KEVL: `app.rs` / `viewmodel.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` / `home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read bhi avoid karo, grep-proof only).
Colors FROZEN (koi naya hue/token nahi — sirf existing palette reuse: row strip `fg/bg`, inversion + underline).
Geometry first, typography second (wrap mapping + bar geometry pehle, mono/strong/underline reuse baad me).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. Repo ke bahar koi copy/sandbox nahi. `git stash/push` kabhi nahi.

Scope: text find highlight polish. Aaj `paint_text_with_find` sirf row select karta hai (`model.selected`), per-match block nahi dikhta aur wrap continuations par match kho jata hai. Is wave ka delta `views.rs` me hai: pure `find_line_spans` + `find highlight format` + wrap-chunk mapping reuse + paint threading (`FindState` query/current row se panes me per-match blocks). Find-bar precedent reuse (28px `FIND_BAR_H`, 120px `FIND_NEEDLE_W`, 4px `BADGE_PAD`, N/M badge shape), existing palette reuse (accent `span_colors` inversion + `INTRA_SPAN_UNDERLINE_W` underline — koi naya `Color32` nahi). Wrap geometry: `wrap_chunks` + `wrap_chunk_spans` reuse taaki lambi lines par match continuations me bhi highlight rahe.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `481 passed; 0 failed` (recorded 2026-11-20, `main@22e5e3f`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-findhl-at/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex.rs|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `checks/gui-findhl-at/**`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (sirf existing `fg/bg` + inversion/underline reuse)

## G3 — Current vs others highlight + wrap geometry, unit-tested

- CHECK: `cargo test -p autocompare-gui findhl`
- EXPECT: green, kam se kam ye tests:
  + `findhl_line_spans_empty_needle_gives_no_hits` (khali needle par empty)
  + `findhl_line_spans_case_modes` (case-sensitive vs ignore-case)
  + `findhl_line_spans_overlapping_ok` (overlap/adjacent sane, sorted, clamped)
  + `findhl_current_other_formats_differ` (current = inverted+underline, other = underline-only, same mono FontId, no new size)
  + `findhl_wrap_chunks_keep_match` (lambi line + chhoti width par match continuations me `wrap_chunk_spans` se milta hai)
  + `findhl_pane_segments_mark_current_row` (current row ke segments flagged, other rows plain underline)
- WHAT: `views.rs` me pure fns (koi egui ctx nahi, koi indexing nahi `.get()` only, saturating arithmetic, `format!` nahi):
  + `find_line_spans(line, needle, ignore_case) -> Vec<(usize,usize)>` (char indices, empty needle par empty)
  + `find_is_current_row(row_idx, current: Option<usize>) -> bool`
  + `find_segment_format(font_id, fg, bg, is_current) -> TextFormat` (current = `span_text_format(acc true)` reuse, other = underline-only same fg/bg + `INTRA_SPAN_UNDERLINE_W`)
  + wrap reuse: `wrap_chunks` + `wrap_chunk_spans` par naya wrapper nahi, sirf find spans ko chunk me map karke test
  + paint threading: `paint_text_with_find` se `FindState::query/ignore_case` + `model.selected` ko pane jobs me pass (internal `FindPaintCtx`, public sig same)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views.rs && git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (sirf OWNS file par), ZERO-BANNED in non-test code (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain), `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only; saturating arithmetic)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-findhl-at/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-findhl-at/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-findhl-at/REPORT.md checks/gui-findhl-at/round-1/after.png checks/gui-findhl-at/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-findhl-at/GATES.md checks/gui-findhl-at/REPORT.md checks/gui-findhl-at/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed), sirf OWNS paths staged, working tree clean after commit, NO push
