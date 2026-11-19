# Wave AM-DIFFNAV — GATES (text tab diff navigation: Next/Prev jump + N/M count)

OWNS: `crates/gui/src/views.rs` + `checks/gui-diffnav-am/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` / `home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only, grep-proof only; caller signature same, App struct untouched).
Colors FROZEN (koi hue change nahi — sirf existing small/weak typo, koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi).
Geometry first, typography second. Clean-room: Beyond Compare bahar se observe only, apne shabdon me.
MS rust guidelines: `forbid(unsafe_code)` kept; non-test code me `unwrap`/`expect`/`panic`/`todo` mana hai (`.get()`/saturating/`unwrap_or` only).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. `git stash/push` kabhi nahi.

Scope: text tab diff navigation (find-bar precedent `checks/gui-tablefind-ai/REPORT.md` pattern reuse).
`views.rs` me nayi pure diff-nav model (`diff_nav_position` + `diff_nav_count_label`) aur nav-row paint wiring
(Prev/Next + N/M badge + bring-into-view scroll, existing `select_next/prev_diff` reuse):
nav row geometry/typo find-bar reuse (`HEADER_H`/`FIND_BAR_H` 28.0, `BADGE_PAD` 4.0, small+weak badge, right-end dock),
koi naya token/color nahi.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `393 passed; 0 failed` (recorded 2026-11-20, `main@8daf1ca`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-diffnav-am/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex.rs|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals)" || echo NO-COLOR-ADD`
- EXPECT: only `views.rs` + `checks/gui-diffnav-am/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD

## G3 — Diff nav kaam karta hai + count sahi, unit-tested

- CHECK: `cargo test -p autocompare-gui diff_nav`
- EXPECT: green, kam se kam ye tests:
  + `diff_nav_position_tracks_selected_among_diffs` (naya, views.rs: None/antiequal/1-based position, wrap ke baad stable)
  + `diff_nav_count_labels_position_over_total` (naya, views.rs: `Some(2),5 -> "2/5"`, `None,0 -> "0/0"`, find precedent mirror)
  + `diff_nav_total_counts_only_diffs` (naya, views.rs: equal-only par 0, mixed par diff count)
- WHAT: `views.rs` me nayi pure `diff_nav_total(rows: &[TextRow]) -> usize`
  (sirf `is_diff_row` count, khali par 0, `.get()`/saturating only, koi indexing nahi)
  aur `diff_nav_position(rows: &[TextRow], selected: Option<usize>) -> Option<usize>`
  (selected diff hai to 1-based position, nahi to `None`; out-of-range par `None`)
  aur `diff_nav_count_label(position: Option<usize>, total: usize) -> String`
  (`Some(pos) -> "pos/total"`, `None -> "0/total"`, `find_count_label`/tablefind badge precedent mirror);
  `paint_text_with_prefs` toolbar wiring nav row ko find-bar geometry se paint karti hai
  (`horizontal_wrapped`, min-size `avail x HEADER_H`, `◀ Prev` / `Next ▶` small_button,
  badge right-end `small().weak()` + `BADGE_PAD` pad; existing `select_next/prev_diff` + `scroll_to` bring-into-view reuse),
  needle edit jaisa koi state nahi — badge `diff_nav_position/total` se har frame recompute hota hai

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-diffnav-am/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-diffnav-am/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-diffnav-am/REPORT.md checks/gui-diffnav-am/round-1/after.png checks/gui-diffnav-am/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-diffnav-am/GATES.md checks/gui-diffnav-am/REPORT.md checks/gui-diffnav-am/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
