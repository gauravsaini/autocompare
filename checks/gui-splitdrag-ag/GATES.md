# Wave AG-SPLITDRAG — GATES (Gap #1 paint half: splitter drag-handle geometry model)

OWNS: `crates/gui/src/viewmodel.rs` + `checks/gui-splitdrag-ag/**` ONLY.
KEVL `views.rs` / `app.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only, grep-proof only).
Colors FROZEN (koi hue change nahi). Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: split fraction (`SPLIT_MIN/MAX/DEFAULT` 0.20/0.80/0.50 + `clamp_split_fraction` + `split_apply_drag`) pehle se hai (Wave Y).
Ye wave uske upar drag-handle geometry MODEL banata hai — sirf `viewmodel.rs` me pure headless helpers, koi egui paint nahi:
`SPLIT_HANDLE_HIT_W` + `split_handle_x_range` (avail+fraction se hit x-range) + `split_handle_contains_x`
+ `split_fraction_from_x` (absolute x se fraction) + `split_drag_delta_fraction` (dx/avail delta)
+ `split_cursor_affordance` + `split_accessibility_label`. `views.rs` paint agle wave me (scope BAHAR).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `333 passed; 0 failed` (recorded 2026-11-20, `main@239d680`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/viewmodel.rs|checks/gui-splitdrag-ag/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|app.rs|tree.rs|views_table.rs|views_hex.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/viewmodel.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `viewmodel.rs` + `checks/gui-splitdrag-ag/**`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD

## G3 — Drag-handle geometry model, unit-tested

- CHECK: `cargo test -p autocompare-gui split`
- EXPECT: green, kam se kam ye naye viewmodel tests:
  + `split_handle_hit_width_matches_paint_strip` (HIT_W 8.0, paint strip se match)
  + `split_handle_x_range_centers_on_fraction` (800/0.50 -> 396..404; clamp edges; degenerate avail -> 0,0)
  + `split_handle_contains_x_hits_only_strip` (center hit, bahar miss, NaN miss)
  + `split_fraction_from_x_clamps_and_guards` (400/800 -> 0.50; bahar clamp; NaN/0-avail guard)
  + `split_drag_delta_fraction_guards_and_scales` (80/800 -> 0.10; zero-avail/NaN -> 0.0)
  + `split_cursor_and_a11y_label_read_honestly` (cursor resize-column; label me percent + 20..80 bounds)
  + purane split tests same values par pass (`split_fraction_clamps_to_drag_range`, `split_drag_delta_moves_fraction`)
- WHAT: `viewmodel.rs` me canonical `SPLIT_HANDLE_HIT_W` (8.0, `views::SPLIT_HIT_W` mirror, geometry only)
  + `split_handle_x_range` / `split_handle_contains_x` / `split_fraction_from_x`
  + `split_drag_delta_fraction` / `split_cursor_affordance` / `split_accessibility_label` (pure, no egui)

## G4 — MS Rust guidelines + fmt clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/viewmodel.rs -- --check; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/viewmodel.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `grep -rn "unsafe" crates/gui/src/viewmodel.rs || echo NO-UNSAFE`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` workspace lint kept, naye helpers pure fns (koi indexing/slicing nahi, `.get()` only jahan applicable — yahan koi indexing hai hi nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-splitdrag-ag/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-splitdrag-ag/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a1a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-splitdrag-ag/REPORT.md checks/gui-splitdrag-ag/round-1/after.png checks/gui-splitdrag-ag/round-1/geo-typo-report.md`
  + `git add <sirf apne OWNS paths> && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
