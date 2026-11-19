# BT-CARDCTA round-1 — Home CTA single-line budget (geometry only)

## 1. Summary

`checks/gui-localrun-bs/REPORT.md` P1-4: live-home me "Open Text / Compare"
jaise card CTA labels 2 lines me wrap hote hain — button width budget tight
tha (120px = card aadha, sabse lamba label "Open Picture Compare" 20 chars
~164px need karta hai). Fix: `crates/gui/src/home.rs` me naya headless
contract — CTA width full inner (224px), purana 120 floor untouched, font
size change nahi (HomeButton body 13 reuse). Harness round-1 me
`RESULT: PASS`. Paint adoption (`views.rs` `paint_home` `add_sized`)
paint-owner wave ka follow-up hai.

## 2. Geometry table (before → after, px)

| Item | Before | After | Kahan |
|---|---|---|---|
| CTA width budget | 120 (`HOME_CARD_BUTTON_MIN_W`, card aadha) — 20-char label wrap | 224 (`HOME_CARD_CTA_W` = 240 − 2×8 pad, full inner) — single line | `home.rs:195` |
| Label need (longest) | "Open Picture Compare" 20 chars ≈ 164 (20×7 + 24) > 120 → wrap | 164 ≤ 224 → fit, `home_card_cta_single_line_ok()` 9/9 true | `home.rs:216` |
| Hit area | 120×28 = 3360 (`home_card_button_hit_area`, untouched) | 224×28 = 6272 (`home_card_cta_hit_area`, paint adoption ke baad) | `home.rs:227` |
| Align / pad | centered (`vertical_centered` parity) | same, koi change nahi — full-inner width se moot | — |
| Font size | HomeButton body 13 | same, zero change (TypoRole alias reuse nahi chahiye tha) | — |
| Card min-h 99.6, grid 3-col, pad/gap tokens | untouched | untouched | — |

## 3. Exact lines (`crates/gui/src/home.rs`)

- `195`: `pub const HOME_CARD_CTA_W: f32 = HOME_CARD_MIN_W - HOME_CARD_PAD * 2.0;` (= 224)
- `198`: `pub const HOME_CARD_CTA_ADVANCE_PX: f32 = 7.0;` (kind-column precedent reuse)
- `200`: `pub const HOME_CARD_CTA_FRAME_PAD: f32 = 24.0;` (16 text pad + 8 frame)
- `203`: `pub fn home_card_cta_width() -> f32`
- `209`: `pub fn home_card_cta_text(title: &str) -> String` ("Open {title}", paint parity)
- `216`: `pub fn home_card_cta_single_line_ok() -> bool` (9/9 labels fit check)
- `227`: `pub fn home_card_cta_hit_area() -> f32` (= 6272)
- `1307–1334`: test `homecta_single_line_budget_fits_all_nine_labels`
  (224 == inner, longest == 20 chars, 6272 > 3360, purana 120 floor intact)

Purana `HOME_CARD_BUTTON_MIN_W` (120) aur `home_card_button_hit_area`
(3360) untouched hain taaki `views.rs` ke paint-lock tests green rahen.

## 4. Gates actual

- `cargo test -p autocompare-gui`: 629 passed, 0 failed (naya CTA test samait).
- Clippy: OWNS file (`home.rs`) zero hits; repo-wide `-D warnings` abhi bhi
  sibling waves ke uncommitted `views.rs`/`viewmodel.rs` dead_code (`filler_*`,
  `find_segment_format`, `text_side_is_filler`) par fail hai — KEVL hai,
  haath nahi lagaya. Do `doc_lazy_continuation` hits apne the (list-marker
  `+` aur `(` line-start), dono fix kar diye.
- `rustfmt --edition 2024 --check crates/gui/src/home.rs`: clean.
- Banned (non-test + tests): zero (`forbid(unsafe_code)` attribute line hi
  ekmatra "unsafe" text hai).
- Colors: diff me zero `Color32`/`Visuals` line.
- Harness `run-geo-typo.sh checks/gui-cardcta-bt/round-1`: `RESULT: PASS`
  (1 round, limit 6) — PNG 9765 bytes, magic ok, distinct 256.
- `checks/opencode-waves/STATUS.md`: untouched. Baaki checks dirs aur
  KEVL crate files: untouched (sibling waves ka kaam unhi ka hai).

## 5. Harness signals (round-1, read-only)

- Geometry: Grid x14, `text_style_height` x4, 28 width calls.
- Typography: monospace x50, strong x26, small x52, `.size` values ['14.0'] —
  koi naya size nahi aaya (CTA HomeButton body 13 reuse).
- Colors: 22 touches, info-only (scope bahar, zero new).

## 6. Follow-up (paint-owner wave, exact spec)

- `crates/gui/src/views.rs` `paint_home` (~line 452): `add_sized` CTA size
  `home_paint_cta_size()` (120×28) se naye `home_card_cta_width()` (224) ×
  `HOME_CARD_BUTTON_MIN_H` (28) par le jao; `home_paint_cta_size` doc
  (120x28/3360 wali lines ~345-350, 414-418) update karo; lock tests
  (~line 6496 `(120.0, 28.0)`/3360, ~line 7956) naye budget par lao.
- Toolbar/path-row floor (`session_toolbar_button_size`,
  `path_browse_button_size`) usi wave me decide ho — CTA widen hone se
  unka 120 floor chhota lagega ya nahi, paint owner dekhe.
- Is wave me paint screenshot same hai (contract-only change), isliye
  visual proof paint adoption wave ke harness me aayega.

## 7. Notes

- Rounds used: 1/6. Commit `feat(gui): ...` local, PUSH MANA.
- Beech me 2 `views.rs` test failures aur E0425 dikhe the — sibling wave ka
  live churn tha, meri file se unrelated (color-tuple asserts); dobara run
  par 629/0 green.
