# Wave Y-SPLIT — GATES (Gap #1: draggable splitter, viewmodel single-source)

OWNS: `crates/gui/src/views.rs` + `crates/gui/src/viewmodel.rs` + `checks/gui-split-y/**` ONLY.
KEVL `theme.rs` / `app.rs` / `screenshot.rs` / `tree.rs` mat chhoona (read-only, grep-proof only).
Colors FROZEN (koi hue change nahi — handle sirf existing `bg_fill`, koi `.size()`/hue nahi).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: drag handle `paint_paths` / dual-tree / picture-SideBySide me pehle se hai (Wave R).
Ye wave pure split math (`SPLIT_MIN/MAX/DEFAULT` 0.20/0.80/0.50 + `clamp_split_fraction`
+ `split_apply_drag`) ko headless test surface `viewmodel.rs` me single-source karta hai;
`views.rs` wahi API delegate karta hai taaki `app.rs` (KEVL) bina touch ke chalta rahe.
`split_field_widths` / `split_picture_widths` / `paint_split_handle` ke signatures same.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `271 passed; 0 failed` (recorded 2026-11-20, `main@be04eb5`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views|viewmodel).rs|checks/gui-split-y/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "theme.rs|app.rs|screenshot.rs|tree.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/viewmodel.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views.rs` + `viewmodel.rs` + `checks/gui-split-y/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Split fraction viewmodel single-source, clamp unit-tested

- CHECK: `cargo test -p autocompare-gui split`
- EXPECT: green, kam se kam ye naye viewmodel tests:
  + `split_fraction_clamps_to_drag_range` (0.50 passthrough; 0.0/-5.0 -> 0.20; 1.0/9.9 -> 0.80)
  + `split_drag_delta_moves_fraction` (dx/avail delta + clamp + zero-avail guard)
  + purane views split tests (`split_field_widths_follow_fraction`,
    `split_picture_widths_sum_to_row`, dual-tree share) same values par pass (delegation)
- WHAT: `viewmodel.rs` me canonical `SPLIT_MIN_FRACTION` / `SPLIT_MAX_FRACTION` /
  `SPLIT_DEFAULT_FRACTION` + `clamp_split_fraction` + `split_apply_drag` (pure, no egui);
  `views.rs` me `pub use` delegation, baaki widths/handle untouched (GEO tokens reuse)

## G4 — MS Rust guidelines + fmt clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- --check; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs crates/gui/src/viewmodel.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -8 crates/gui/src/viewmodel.rs | grep -q "forbid(unsafe_code)" || grep -rq "forbid(unsafe_code)" crates/gui/src/lib.rs && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing/slicing nahi, `.get()` only jahan applicable — yahan koi indexing hai hi nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-split-y/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-split-y/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-split-y/REPORT.md checks/gui-split-y/round-1/after.png checks/gui-split-y/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs crates/gui/src/viewmodel.rs checks/gui-split-y/GATES.md checks/gui-split-y/REPORT.md checks/gui-split-y/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
