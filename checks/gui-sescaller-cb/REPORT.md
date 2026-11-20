# CB-SESCALLER Report — App session caller wiring (strip + T5/T6)

Wave: CB-CALLER. DEPENDENCY SATISFIED: BY-SES-PAINT (`views.rs`
`paint_session_tab_strip` + `SessionAction`) + MODEL (`home.rs` T1-T7) +
APP bridge (`switch_session`/`close_session`/`save_slots_into_active`/
`load_active_into_slots`) sab READ-ONLY use hue, unme zero change.
Sirf `crates/gui/src/app.rs` me caller wiring judi hai.

## Caller wiring table

| Call-site | Kaam | Notes |
|---|---|---|
| Header ke neeche strip paint | `paint_session_tab_strip(ui, &self.sessions, &mut action)` mode tabs ke UPAR | `&self.sessions` immutable borrow, paint mutate nahi karta (PAINT contract) |
| `apply_session_action` | `Switch(id)` -> `switch_session`, `Close(id)` -> `close_session`, `None` -> no-op | Naya pub fn, match-only, headless-tested; `update` isi ko chalata hai |
| Switch par load (T5/T6 load side) | `switch_session` ke andar pehle save phir `load_active_into_slots` | Pehle se tha, ab strip hit se live chalta hai; slots + cursor restore |
| Type ke baad save (T5/T6 save side) | Har frame tab-UI ke baad `save_slots_into_active()` | Idempotent trimmed write; khaali par no-op; setter bypass (paint `&mut String`) ab session me utarta hai |

## Geometry / typo / colors

- Geometry: zero naya token. Strip apne frozen tokens par hai
  (`SESSION_TAB_MAX_W`, close edge, `TIGHT_GAP`, `TAB_BOTTOM_GAP` reuse).
  Ek extra frame-save call hai, koi layout shift nahi.
- Typo: koi label string change nahi.
- Colors: FROZEN. Diff me `Color32::from_rgb` / `Visuals::` zero new.

## Shot map (round-1)

| Shot | Scene | Bytes | Check |
|---|---|---|---|
| `after.png` | Text fixture (`screenshot.rs` NO-TOUCH renderer, live `App::update` nahi) | 9765 | magic ok, >=8192, distinct 256, BY wave se byte-identical (EXPECTED, no-regression) |

Note: live strip ka visual proof is round me fixture renderer se nahi aa
sakta (`screenshot.rs` NO-TOUCH tha). Strip scenes khud BY wave me PNG-proven
hain; caller wiring 4 headless tests se lock hai.

## Gates

- `cargo test -p autocompare-gui`: 659 passed, 0 failed (naye 4:
  switch-load, close-neighbour, T5 typed-save, T6 cursor round-trip).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero.
- `cargo fmt --check -p autocompare-gui`: clean.
- Banned strict non-test me zero. Colors diff me zero new.
- Harness `tools/gui-polish/run-geo-typo.sh
  checks/gui-sescaller-cb/round-1/`: `RESULT: PASS` (round-1, repeat nahi laga).
- `git status --short`: sirf `M crates/gui/src/app.rs` + untracked
  `checks/gui-sescaller-cb/`. STATUS untouched. Sibling
  (`checks/gui-darkroot-cc/`) ko chhua nahi.

## Follow-up

- Mode-tab click (`paint_tabs` direct `&mut self.tab`) abhi bhi session
  adopt nahi karta — strip switch se bahar ka scope, agle wave ka hai.
- `rename` hook T7 ready hai, paint me nahi joda (scope bahar, BY note se carried).
