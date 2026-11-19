# Wave AS-TABLEKEY — GATES (table key-col UX: select/deselect affordance + positional-note clarity)

OWNS: `crates/gui/src/views_table.rs` + `checks/gui-tablekey-as/**` ONLY.
KEVL `views.rs` / `app.rs` / `viewmodel.rs` / `tree.rs` / `views_hex.rs` / `home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read bhi avoid karo).
Colors FROZEN (koi hue change nahi — sirf existing palette/tokens, koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
Repo ke bahar koi copy/sandbox nahi, `git stash/push` kabhi nahi. Clean-room: Beyond Compare bahar se observe only, apne shabdon me.

Scope: existing `TableKeyView` model reuse (`toggle_key_column`, `positional_note`, `status_line`, `POSITIONAL_NOTE` same rahenge).
`views_table.rs` me key-col header affordance + positional-note clarity: har header column par select/deselect
toggle (selected vs deselected marker, fixed-width indicator alignment), suggested-key hint, aur positional hone
par honest note (`POSITIONAL_NOTE`) badge/note row me. Geometry precedent reuse (find-bar `28.0/120.0/4.0`
rhythm, koi naya spacing token value nahi — sirf key-bar alias consts), typography me sirf
`.monospace()`/`.strong()`/`.small()`/`.weak()` (koi `.size()`/color add nahi). Paint wiring `views.rs` caller
untouched rehta hai — `paint_table_key_bar` future wiring ke liye ready, signature stable.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `473 passed; 0 failed` (recorded 2026-11-20, `main@05d9e1f`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views_table.rs|checks/gui-tablekey-as/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|app.rs|viewmodel.rs|tree.rs|views_hex.rs|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_table.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views_table.rs` + `checks/gui-tablekey-as/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Key-col affordance + honest note, pure helpers + paint, unit-tested

- CHECK: `cargo test -p autocompare-gui table_key_`
- EXPECT: green, kam se kam ye tests:
  + `table_key_marker_distinguishes_selected` (naya: selected vs deselected marker alag, single-char alignment)
  + `table_key_badge_labels_keyed_vs_positional` (naya: keyed par `key: ...`, positional par `positional`, khali header safe)
  + `table_key_bar_state_helpers_are_honest` (naya: `is_key_column`/`is_suggested_key` toggle/suggest ke saath sync, note sirf positional par)
  + purane `key_selector_toggles_columns_sorted`, `empty_keys_fall_back_to_positional_with_note` same shape pass
- WHAT: `views_table.rs` me pure helpers (koi egui/I-O nahi, `.get()`/saturating only, koi indexing/slicing nahi):
  + `key_column_marker(selected: bool) -> &'static str` (fixed 1-char indicator, alignment ke liye)
  + `is_key_column(view, col)` + `is_suggested_key(view, col)` (state query, view reuse)
  + `table_key_badge_label(view) -> String` (`key: 0,2` vs `positional`, khali header safe)
  + geometry consts `TABLE_KEY_BAR_H = 28.0` / `TABLE_KEY_COL_GAP = 8.0` / `TABLE_KEY_BADGE_PAD = 4.0`
    (bar height + badge pad find-bar precedent reuse: `TABLE_FIND_BAR_H`/`TABLE_FIND_BADGE_PAD` wahi values;
    col gap default item-spacing scale, koi naya rhythm token nahi)
  + `paint_table_key_bar(ui, view, header)` — header affordance row: har column `selectable_label(marker + name)`
    (selected par `.strong().monospace()`, deselected par `.monospace()`), click par `toggle_key_column(col)`;
    suggested par `(suggested)` suffix `.small().weak()` me; right-end badge `table_key_badge_label` `.small().weak()` me
    (`right_to_left` + `BADGE_PAD`, find-bar badge geometry reuse); positional hone par `POSITIONAL_NOTE`
    `.small().weak()` note line me (honest, `positional_note()` reuse, koi naya wording nahi)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views_table.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_table.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views_table.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablekey-as/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tablekey-as/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablekey-as/REPORT.md checks/gui-tablekey-as/round-1/after.png checks/gui-tablekey-as/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_table.rs checks/gui-tablekey-as/GATES.md checks/gui-tablekey-as/REPORT.md checks/gui-tablekey-as/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
