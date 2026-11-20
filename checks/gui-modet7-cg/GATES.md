# GATES — gui-modet7-cg (mode-tab adopt + T7 rename)

Scope: `crates/gui/src/app.rs` caller wiring + `crates/gui/src/home.rs`
doc-note. MODEL (`SessionList::rename` T7) + PAINT (`paint_tabs`,
`paint_session_tab_strip`, `SessionAction`) READ-ONLY. CALLER `07a8db8`
follow-up: (1) mode-tab click session adopt, (2) T7 rename bridge.

## Must-pass

1. `cargo test -p autocompare-gui` green. Naye tests:
   - rename bridge: trim ok + status, blank false, unknown false,
     slots/tab untouched.
   - mode-adopt: same-kind click par switch (duplicate open nahi, paths
     load), fresh-kind click par open (len+1).
2. `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero.
3. `cargo fmt --check -p autocompare-gui`: clean.
4. Banned strict non-test me zero: `unwrap(`/`expect(`/`panic!`/`todo!`/`unsafe`.
5. Colors: diff me `Color32::from_rgb` / `Visuals::` zero new.
6. Harness `tools/gui-polish/run-geo-typo.sh checks/gui-modet7-cg/round-1/`:
   `RESULT: PASS` (max 6 rounds).
7. `checks/opencode-waves/STATUS.md` untouched. `git status --short` me
   sirf OWNS (`crates/gui/src/app.rs`, `crates/gui/src/home.rs`,
   `checks/gui-modet7-cg/`). Sibling `views.rs` dirty hai (CF-P14) —
   chhuna MANA.

## Fixtures

- `round-1/after.png` fixture renderer se aata hai (`screenshot.rs`
  NO-TOUCH, live `App::update` se nahi), isliye CB wave se byte-identical
  hona EXPECTED hai — no-regression proof. Mode-tab adopt + rename wiring
  headless tests se lock hai (gate 1).
- Stable rehna chahiye: magic ok, >=8192B, distinct>64.
