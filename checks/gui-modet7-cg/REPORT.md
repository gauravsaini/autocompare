# REPORT — gui-modet7-cg (mode-tab adopt + T7 rename)

CB-SESCALLER ka follow-up: mode-tab click session adopt nahi karta tha
(direct `&mut self.tab` bypass), aur T7 rename hook caller se judaa nahi
tha. Dono chhote diff me land hue, ek wave me.

## Kya badla

- `app.rs` mode-tab adopt: `paint_tabs` ab alag `picked_tab` buffer me
  paint karta hai; badla to `set_tab` se adopt hota hai (same-kind par
  switch, fresh-kind par open, switch se pehle T5/T6 save andar).
  CALLER `07a8db8` ke strip wiring par bana hai, usko chheda nahi.
- `app.rs` T7 bridge: naya `rename_session(id, title)` — model
  `SessionList::rename` reuse (T1-T6 precedent jaisa), blank/unknown par
  no-op false, success par status line. Slots untouched.
- `home.rs`: T7 doc-note update (caller wired, "future hook" line gayi).
  Logic zero change — model pehle se headless-ready tha.

## Caller wiring table

| Call-site | Kaam | Notes |
|---|---|---|
| Header strip ke neeche mode tabs | pick buffer + badalne par `set_tab` | same-kind switch (dup open nahi), fresh-kind open |
| `rename_session` | label rename via model T7 | trimmed, blank/unknown no-op, status set |

## Geometry / typo / colors

- Geometry: zero naya token. Ek local `picked_tab`, koi layout shift nahi.
- Typo: koi label string change nahi.
- Colors: FROZEN. Diff me `Color32::from_rgb` / `Visuals::` zero new.

## Shot map (round-1)

| Shot | Scene | Bytes | Check |
|---|---|---|---|
| `after.png` | Text fixture (`screenshot.rs` NO-TOUCH renderer) | 9765 | magic ok, >=8192, distinct 256, CB se byte-identical (EXPECTED, no-regression) |

## Gates

- `cargo test -p autocompare-gui`: 663 passed, 0 failed (naye 2:
  rename trim/blank/unknown, mode-adopt switch-vs-open).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero.
- `cargo fmt --check -p autocompare-gui`: clean.
- Banned strict non-test me zero. Colors diff me zero new.
- Harness `tools/gui-polish/run-geo-typo.sh
  checks/gui-modet7-cg/round-1/`: `RESULT: PASS` (round-1).
- `git status --short`: sirf OWNS (`app.rs`, `home.rs`,
  `checks/gui-modet7-cg/`). Sibling `views.rs` dirty (CF-P14) chhua nahi.
  STATUS untouched.
