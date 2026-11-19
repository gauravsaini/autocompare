# Wave AJ-HOME — REPORT (Home/session-start polish)

## Kya kiya (geometry first, typography second, colors frozen)

Geometry (headless contract, `crates/gui/src/home.rs`):
- `HOME_GRID_COLUMNS = 3` — 9 session cards / 3 poori rows; `home_grid_rows()`
  (ceil) + `home_grid_cell()` (row-major) pure fns, paint `Grid` order mirror.
- Card stack: `HOME_CARD_MIN_W = 240.0` (`views::CARD_W` parity),
  `home_card_min_height()` token-derived = title 1 line + desc 2 lines
  (role line-height px) + button 24px hit floor + 2×4 inner gap + 2×8 pad
  = 95.6px — purane 75px clip se mehfooz. Gaps section/tight rhythm (8/4) reuse.
- Session-start recent list: `RecentSessions` (cap 8, naya upar, dedupe,
  khali-ignore) + `App::open_home_session()` (tab switch + record + status),
  `recent_sessions()` / `clear_recent_sessions()`.

Typography (TypoRole precedent reuse, koi naya size nahi):
- Title HomeTitle (14 strong proportional, tracking 0.02em, chrome lh 1.2,
  medium 500), desc HomeDesc (11 small weak, regular 400), button HomeButton
  (13 body, medium 500) — layer-2 accessors + `home_card_roles_are_chrome_proportional()`.
- `home.rs` me koi `.size(` literal nahi; colors untouched (diff me 0 Color32/Visuals).

Paint note: `views.rs` KEVL hai — `paint_home` adoption (grid helpers + min
height + recent list UI) unki wave me; yeh wave headless contract + tests hai.

## Saboot

- `cargo test -p autocompare-gui`: 373 passed, 0 failed (6 naye: 5 home + 1 app).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0).
- `cargo fmt -- <owns files>` + workspace `fmt --check`: clean; diff purely additive.
- Banned: apne diff me 0 `unwrap(`/`.expect(`/`panic!`/`todo!`/`unsafe`; forbid kept.
- Harness: `tools/gui-polish/run-geo-typo.sh checks/gui-home-aj/round-1` → RESULT: PASS
  (PNG 9624B, magic ok, distinct 256).

## Commit

`feat(gui): polish home session-start geometry and typography` — sirf OWNS paths.
No push (orchestrator karega). `views_hex.rs` me dusre agent ka kaam hai — untouched.
