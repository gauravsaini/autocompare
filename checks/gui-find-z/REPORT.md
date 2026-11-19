# Wave Z-FIND report (Gap #12 model half: text-view find + count badge)

## Scope
- OWNS: `crates/gui/src/viewmodel.rs` + `checks/gui-find-z/**`. `app.rs` verify-only (zero diff).
- KEVL `views.rs` / `views_hex.rs` / `theme.rs` / `screenshot.rs` / `tree.rs` untouched.
  `views.rs` me sibling Z-HEX ka in-progress diff hai — haath nahi lagaya, stage nahi karunga.
- Paint wiring is scope se BAHAR (agla wave) — sirf model + pure helpers + tests.
- Colors FROZEN, koi paint/geo/typo token change nahi (pure model wave).

## Delta (viewmodel.rs 46+/1-)
- `find_count_label(position: Option<usize>, total: usize) -> String` (pure):
  `Some(n)` par `"n/m"`, `None` par `"0/m"` (empty par `"0/0"`).
  Paint-side `views::text_find_badge_label` ka model-side judvaan (KEVL copy nahi,
  agla paint wave isko badge label ke liye use karega). `#[allow(dead_code)]`
  wiring-pending file precedent ke saath.
- `FindState::count_label(&self) -> String`: `position()` over `match_count()`.
- WIRING comment update: `count_label()` badge feed karega (agla paint wave).
- App side (`text_find_needle`/`cursor`/`ignore_case` + match/total/position/badge/
  set_needle/next_row/prev_row) Wave Q/S/T se landed — verify-only, koi edit nahi.

## Tests (G3)
- Naye (2): `find_count_label_formats_n_of_m` ("2/5", "1/1", "0/5", "0/0");
  `find_state_count_label_tracks_cursor` (set_query "1/2" -> next "2/2" ->
  wrap "1/2" -> empty "0/0" + next None).
- Purane same-values pass: `find_state_cycles_next_prev_with_wrap`,
  `find_state_empty_query_has_no_position`, `text_find_box_tracks_matches_and_badge`,
  `text_find_next_prev_wrap_over_many_matches`.
- Full: `cargo test -p autocompare-gui` 278 passed, 0 failed
  (275 baseline + 2 naye + 1 sibling Z-HEX ka views.rs test; sibling diff unstaged rakha).

## Gates
- G1 baseline: 275 passed (round-1/baseline-tests.out).
- G2 OWNS: staged sirf viewmodel.rs + checks/gui-find-z/**; KEVL-CLEAN (views.rs
  diff sibling ka hai — unstaged); NO-COLOR-ADD.
- G4 MS rust: `cargo fmt -- <OWNS files>` only (bare fmt nahi);
  `rustfmt --check` OWNS clean; non-test me ZERO unwrap/expect/panic/todo;
  `forbid(unsafe_code)` kept; naye helpers pure (koi indexing nahi, `.get()` only).
- Clippy note: `-D warnings` run me 1 dead-code error `views.rs:485`
  `format_hex_grid_row` — sibling Z-HEX ka mid-wiring helper (KEVL, mera scope bahar).
  Mera OWNS (viewmodel.rs) zero-warning; sibling ke wire/allow ke baad retry par green.
  Wave Q precedent (wait+retry, fix mat karo).
- G5 harness: `run-geo-typo.sh checks/gui-find-z/round-1` RESULT PASS;
  after.png 9624 bytes, magic 89504e470d0a1a0a ok, distinct 256
  (model-only wave — PNG paint-unchanged rehna expected hai).

## Commit
- Local only, no push. Sibling `checks/gui-hex-z/` untracked rehta hai.
