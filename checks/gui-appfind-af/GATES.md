# Wave AF-APPFIND — GATES (Gap #12 FINAL wiring: App text tab → `paint_text_with_find`)

OWNS: `crates/gui/src/app.rs` + `checks/gui-appfind-af/**` ONLY.
KEVL `views.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`views_hex.rs`/`gutter.rs`/`theme.rs`/`screenshot.rs`
mat chhoona (read/grep-only). Colors FROZEN (koi hue/token change nahi).
Geometry first, typography second (sirf existing tokens: `FIND_BAR_H`/`FIND_NEEDLE_W`/
`BADGE_PAD` + small weak badge, `find_bar_geometry` dock). NO push (orchestrator pushes).
Dusre agent ki files kabhi touch mat karo (`checks/gui-mergepaint-af/` parallel wave hai).
`git stash/push` kabhi nahi.

Scope: Gap #12 ka FINAL wiring. Paint half (Wave AE-FINDPAINT) landed hai:
`viewmodel::FindState` + `views::paint_text_find_state_bar` + `views::paint_text_with_find`
(abhi `#[allow(dead_code)]` + App-unwired). Model half (Wave Q/S/T) landed hai:
App side `text_find_*` legacy state (compat ke liye rehta hai). Is wave ka delta
sirf `app.rs` me hai: App me `FindState` rakho, Text tab paint se
`views::paint_text_with_find` call karo, N/M badge visible ho. Purana
`paint_text_find_bar` + legacy `text_find_next_row` helpers compat ke liye untouched
rahenge (tests use karte hain); paint ka source of truth `FindState` hoga, legacy
fields needle/flag se seeded + mirrored.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (324 passed, recorded `round-1/baseline-tests.out` me)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/app.rs|checks/gui-appfind-af/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/app.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `app.rs` + `checks/gui-appfind-af/**`; KEVL-CLEAN; OWNS-CLEAN;
  NO-COLOR-ADD (frozen palette reuse, koi naya hue/token nahi)

## G3 — FindState App wiring + N/M badge unit-tested

- CHECK: `cargo test -p autocompare-gui find` + full `cargo test -p autocompare-gui`
- EXPECT: green, kam se kam ye naye tests (sab `app.rs` me, headless):
  + `text_find_state_prefs_follow_toggles` — prefs builder toggles se TextViewPrefs banata hai
  + `text_find_state_seeds_from_legacy_needle` — legacy needle/flag FindState me seed hota hai
  + `text_find_state_badge_matches_legacy_badge` — FindState count_label legacy N/M badge se milta hai
  + purane find tests same values par pass (Wave Q/S/T compat)
- WHAT: App me `pub text_find: viewmodel::FindState` field + Default init;
  Text tab paint `views::paint_text_with_find(ui, model, &prefs, &mut find)` call karta hai
  (bar + `paint_text_with_prefs` delegation + `model.selected` sync andar hi);
  `#[allow(dead_code)]` KEVL me rehta hai (dusre agent ka file) — wiring se function
  used hota hai isliye clippy dead-code warning nahi aata. Legacy `text_find_needle`
  fields compat ke liye rehte hain (seed + mirror).

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/app.rs && git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/app.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/app.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/app.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (sirf `app.rs` par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only;
  saturating arithmetic; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-appfind-af/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-appfind-af/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-appfind-af/REPORT.md checks/gui-appfind-af/round-1/after.png checks/gui-appfind-af/round-1/geo-typo-report.md`
  + `git add crates/gui/src/app.rs checks/gui-appfind-af/GATES.md checks/gui-appfind-af/REPORT.md checks/gui-appfind-af/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
