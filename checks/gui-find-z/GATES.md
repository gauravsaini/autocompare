# Wave Z-FIND — GATES (Gap #12 model half: text-view find + count badge)

OWNS: `crates/gui/src/app.rs` + `crates/gui/src/viewmodel.rs` + `checks/gui-find-z/**` ONLY.
KEVL `views.rs` / `views_hex.rs` / `theme.rs` / `screenshot.rs` / `tree.rs` mat chhoona
(read-only, grep-proof only — Z-HEX sibling `views_hex.rs`+`views.rs` par kaam kar raha hai).
Colors FROZEN (koi hue/token change nahi — ye wave pure model hai, paint agle wave me).
Geometry first, typography second (model me koi geo/typo token nahi — sirf state + pure helpers).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: Gap #12 ka model half. App side state machine Wave Q/S/T me landed hai
(`text_find_needle`/`cursor`/`ignore_case` + `text_find_match_rows`/`matches`/`total`/
`position`/`badge`/`set_needle`/`next_row`/`prev_row` + tests) — verify-only, App me
edit sirf tab jab test red ho. Is wave ka delta `viewmodel.rs` me hai: pure
`find_count_label(position, total)` helper + `FindState::count_label()` taaki agle
paint wave ke paas model-side N/M label ho (paint-side `views::text_find_badge_label`
KEVL hai, chhoona nahi). `views.rs` paint wiring is scope se BAHAR hai (agla wave).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `275 passed; 0 failed` (recorded 2026-11-20, `main@3e33d72`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(app|viewmodel).rs|checks/gui-find-z/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|views_hex|theme.rs|screenshot.rs|tree.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/app.rs crates/gui/src/viewmodel.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `app.rs` + `viewmodel.rs` + `checks/gui-find-z/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (model wave — koi paint token nahi)

## G3 — Count-label helper + next/prev wrap unit-tested

- CHECK: `cargo test -p autocompare-gui find`
- EXPECT: green, kam se kam ye tests:
  + `find_count_label_formats_n_of_m` ("2/5"; `None` par "0/5"; "0/0" empty par)
  + `find_state_count_label_tracks_cursor` (set_query/next/prev ke baad label badalta hai)
  + purane find tests same values par pass (`find_state_cycles_next_prev_with_wrap`,
    `find_state_empty_query_has_no_position`, `text_find_box_tracks_matches_and_badge`,
    `text_find_next_prev_wrap_over_many_matches`)
- WHAT: `viewmodel.rs` me pure `find_count_label(position: Option<usize>, total: usize) -> String`
  + `FindState::count_label(&self) -> String` (wiring-pending: paint agla wave use karega,
  `#[allow(dead_code)]` file precedent ke saath); App side verify-only

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/app.rs crates/gui/src/viewmodel.rs && git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/app.rs crates/gui/src/viewmodel.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/app.rs crates/gui/src/viewmodel.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -12 crates/gui/src/app.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS files par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only;
  saturating arithmetic; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling Z-HEX files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-find-z/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-find-z/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64 (model-only wave — PNG bit-same rehna expected hai)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-find-z/REPORT.md checks/gui-find-z/round-1/after.png checks/gui-find-z/round-1/geo-typo-report.md`
  + `git add crates/gui/src/app.rs crates/gui/src/viewmodel.rs checks/gui-find-z/GATES.md checks/gui-find-z/REPORT.md checks/gui-find-z/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit (sibling `checks/gui-hex-z/`
  untracked rehta hai — chhoona nahi), NO push
