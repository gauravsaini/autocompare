# Wave T CALL report (find-bar paint wiring)

## Study
- `crates/gui/src/views.rs:1992` `text_find_badge_label` + `:2003`
  `paint_text_find_bar` dono `allow(dead_code)` ke saath pade the, koi
  caller nahi tha (Wave Q hook, test-only note).
- App text-find state Wave Q se live hai (`app.rs:101-103,871-993`):
  `text_find_needle/cursor/ignore_case`, `text_find_matches/total/
  position/badge`, `text_find_next_row/prev_row` (wrap + selection +
  status). Text diff F3 loop `views.rs:1036-1052` untouched rakha.
- Zoom Cmd +/-/0 `app.rs:2183-2200` pehle se live hai (`zoom_in/out/
  reset` `:469-480`); yahan koi paint change nahi, sirf test se confirm.

## Build (sirf views.rs + app.rs, chhota diff)
- `views.rs:1986,1989-1992,2000-2003`: hook note hata kar "App text tab
  se called" likha, dono `allow(dead_code)` hataye. Koi color/geometry/
  token change nahi (wahi Find:/120.0/needle/Ignore case/Prev/Next/
  small().weak()).
- `app.rs:2526-2552`: Text tab ka inline find block hatakar views calls
  lagaye — `text_find_badge_label(position,total)` se badge,
  `paint_text_find_bar(ui, needle, ignore_case, badge)` se field+Prev/
  Next; `(prev,next)` par `text_find_prev/next_row` chalata hai, needle
  badalne par cursor 0. Badge/model parity test ke andar hai.
- Tests (2 naye, app.rs owned): `font_zoom_steps_clamp_and_reset`
  (zoom live confirm) + `text_find_badge_label_matches_app_badge`
  (paint badge == App `text_find_badge`, empty par 0/0).

## Call sites (file:line)
- Def: `views.rs:1992` badge, `views.rs:2003` bar.
- Call: `app.rs:2529` badge, `app.rs:2531` bar.
- Nav: `app.rs:2541` prev_row, `app.rs:2547` next_row.
- Zoom (no change): `app.rs:2183` keys, `app.rs:2192/2195/2198` handlers.

## Check (round-T-1, sab green)
- `cargo test -p autocompare-gui`: 231 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: exit 0.
- `cargo fmt -- --check`: clean.
- Harness `tools/gui-polish/run-geo-typo.sh
  checks/gui-paintcall-t/round-T-1`: RESULT PASS (PNG 9914B, magic ok).

## Files
- `crates/gui/src/views.rs`, `crates/gui/src/app.rs`,
  `checks/gui-paintcall-t/round-T-1/`, `checks/gui-paintcall-t/REPORT.md`.
- Touch nahi: theme/gutter/hex/table/tree/viewmodel/main/screenshot,
  STATUS.md. Local commit only, no push.
