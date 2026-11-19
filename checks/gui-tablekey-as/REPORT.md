# Wave AS-TABLEKEY REPORT (table key-col UX: select/deselect affordance + positional-note clarity)

Scope: `crates/gui/src/views_table.rs` + `checks/gui-tablekey-as/**` only.
KEVL `views.rs` / `app.rs` / `viewmodel.rs` / `tree.rs` / `views_hex.rs` / `home.rs` / `gutter.rs` / `theme.rs` /
`screenshot.rs` untouched (read bhi avoid kiya), `STATUS.md` untouched, no push. Clean-room: bahar se
observation only, koi string/asset copy nahi. Existing `TableKeyView` model reuse (`toggle_key_column`,
`positional_note`, `status_line`, `POSITIONAL_NOTE` same rahe — koi model change nahi).

## Item-wise before/after

### (a) `views_table.rs`: pure key-col state queries + badge (naya)
- Before: `TableKeyView` me toggle/suggest/positional-note state tha, par header affordance ke liye
  koi marker/badge vocabulary nahi tha.
- After: pure helpers (koi egui/I-O nahi, `.contains()`/saturating only, koi indexing/slicing nahi):
  + `key_column_marker(selected) -> &'static str` — fixed 1-char indicator (`●` vs `○`),
    hamesha toggle text ke start me, isliye indicator column align rehta hai.
  + `is_key_column(view, col)` + `is_suggested_key(view, col)` — view-state queries, toggle/suggest ke saath sync.
  + `table_key_badge_label(view) -> String` — keyed par `key: 0,2`, positional par `positional`
    (khali header safe, sirf view state par).
- Geometry consts: `TABLE_KEY_BAR_H = 28.0` / `TABLE_KEY_COL_GAP = 8.0` / `TABLE_KEY_BADGE_PAD = 4.0`
  (bar height + badge pad find-bar precedent reuse, col gap default item-spacing scale; koi naya rhythm nahi).
- Tests: `table_key_marker_distinguishes_selected`, `table_key_badge_labels_keyed_vs_positional`,
  `table_key_bar_state_helpers_are_honest` (marker single-char alignment, badge keyed/positional,
  helpers toggle/suggest/note ke saath honest, geometry const values).

### (b) `views_table.rs`: `paint_table_key_bar` header affordance + honest note (naya)
- Before: key select/deselect ka koi header UI nahi tha; positional note sirf status-line string me tha.
- After: `paint_table_key_bar(ui, view, header)` — header affordance row: har column
  `selectable_label(marker + name)` toggle (selected par `.strong().monospace()`, deselected par
  `.monospace()`), click par `toggle_key_column(col)` (click collect karke closure ke baad toggle —
  borrow-safe); suggested-par `(suggested)` hint `.small().weak()` me; right-end badge
  `table_key_badge_label` `.small().weak()` me (`right_to_left` + `BADGE_PAD`, find-bar badge
  geometry reuse); positional hone par `POSITIONAL_NOTE` note line `.small().weak()` me
  (`positional_note()` reuse, koi naya wording nahi).
- Reuse: `TableKeyView`, `POSITIONAL_NOTE`, find-bar geometry/typo precedent; koi naya token/color nahi,
  koi `.size()`/color call add nahi. Caller (`views.rs`) untouched — wiring orchestrator ka kaam;
  isliye `#[allow(dead_code)]` (file precedent: paint-wiring-landing-tak test-only).
- Tests: pure helpers upar (a) me covered; paint khud thin egui glue hai (koi logic nahi —
  saari state pure helpers me).

## Gates
- `cargo test -p autocompare-gui`: 481 passed, 0 failed (pre-edit baseline 473; +3 TABLEKEY naye,
  +5 sibling HEXGUTTER ke uncommitted — working-tree count; hamara diff sirf OWNS).
- `cargo test -p autocompare-gui table_key_`: 3 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0, zero warnings).
  (Beech me ek run par sibling ke dirty `views_hex.rs` par `collapsible_if` dikha tha — KEVL, chhua nahi;
  final commit-time run fully clean.)
- `rustfmt --edition 2024 --check` (sirf OWNS file): clean (exit 0).
- Banned grep (non-test code): ZERO-BANNED (`unwrap()`/`expect()`/`panic!`/`todo!`/`unimplemented!`/
  `unreachable!` nahi; `unwrap_or`/`unwrap_or_default` allowed; `.contains()` use kiya).
  `forbid(unsafe_code)` kept.
- Colors: NO-COLOR-TYPO-ADD (koi `from_rgb`/`from_rgba`/`Visuals::`/size-override add nahi).
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-tablekey-as/round-1`: RESULT PASS
  (after.png 9624B, magic-ok `89504e470d0a1a0a`, distinct 256).
- OWNS-clean: diff sirf `views_table.rs` + `checks/gui-tablekey-as/**`;
  KEVL-CLEAN (sibling HEXGUTTER ke `views_hex.rs` + `checks/gui-hexgutter-as/` ko chhua nahi —
  commit pathspec sirf OWNS paths par, taaki sibling ka kaam usi ka rahe; uske paths tree me
  uncommitted rahenge — parallel wave me expected, orchestrator dekhega).

Commit: local only, no push.
