# Wave AH-SPLITPAINT — GATES (Gap #1 paint half: splitter drag-handle paint)

OWNS: `crates/gui/src/views.rs` + `checks/gui-splitpaint-ah/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only use ok, edit nahi — viewmodel splitdrag model sirf call karo).
Colors FROZEN (handle sirf existing `bg_fill`, koi hue/size change nahi). Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: viewmodel split-handle geometry MODEL pehle se landed hai (Wave AG: `SPLIT_HANDLE_HIT_W` + `split_handle_x_range` + `split_handle_contains_x` + `split_fraction_from_x` + `split_drag_delta_fraction` + `split_cursor_affordance` + `split_accessibility_label`).
Ye wave us model ko PAINT me wiring karta hai — sirf `views.rs` me: shared `paint_split_handle` (jo text-tab `paint_paths` + picture-SideBySide + `paint_folder_dual` teeno me lagta hai) model consume karega: hit-width parity, drag-delta helper, resize-column cursor affordance, a11y hover label. Fraction clamp reuse (`clamp_split_fraction` path se). Koi signature change nahi (App KEVL untouched rehta hai).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `344 passed; 0 failed` (recorded 2026-11-20, `main@9cc197a`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-splitpaint-ah/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "viewmodel.rs|app.rs|tree.rs|views_table.rs|views_hex.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views.rs` + `checks/gui-splitpaint-ah/**`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Split-handle paint wiring, unit-tested

- CHECK: `cargo test -p autocompare-gui split_handle`
- EXPECT: green, kam se kam ye naye views tests:
  + `split_handle_strip_matches_model_hit_width` (SPLIT_HIT_W == SPLIT_HANDLE_HIT_W == 8.0)
  + `split_handle_drag_uses_delta_and_clamp` (0.50+80/800 -> 0.60; edges clamp; zero-avail/NaN guard)
  + `split_handle_label_reports_percent_and_bounds` (label me percent + 20/80 bounds)
  + `split_handle_cursor_is_resize_column` (model `resize-column` -> `CursorIcon::ResizeColumn`)
  + `split_handle_hit_range_centers_on_fraction` (800/0.50 -> 396..404 parity)
- WHAT: `views.rs` me `paint_split_handle` upgrade — `split_drag_delta_fraction` + `clamp_split_fraction` se drag, `on_hover_cursor(ResizeColumn)` (model `split_cursor_affordance` se mapped), `on_hover_text(split_accessibility_label)`; thin pure wrappers `split_handle_drag_fraction` / `split_handle_label` / `split_handle_cursor`; strip/model width parity `debug_assert` + test

## G4 — MS Rust guidelines + fmt clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views.rs -- --check; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `grep -rn "unsafe" crates/gui/src/views.rs || echo NO-UNSAFE`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing/slicing nahi, `.get()` only jahan applicable)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-splitpaint-ah/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-splitpaint-ah/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-splitpaint-ah/REPORT.md checks/gui-splitpaint-ah/round-1/after.png checks/gui-splitpaint-ah/round-1/geo-typo-report.md`
  + `git add <sirf apne OWNS paths> && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
