# CF-CTAWDITH round-1 — Home CTA width paint adoption (P1-4)

## 1. Summary

`checks/gui-localrun-ca/REPORT.md` section 5 P1-4 (bu2 section 5 P1-4
same): headless contract `HOME_CARD_CTA_W`=224 landed tha (BT-CARDCTA),
par live paint me 120 floor tha — live-home me "Open Text / Compare"
2-line wrap. Fix: `crates/gui/src/views.rs` me `home_paint_cta_size()`
ab BT contract reuse karta hai — `(home_card_cta_width(), 28)` =
(224, 28), area 6272. Toolbar/path-row floor 120x28 par pin rehta hai
(chhote labels ko 224 nahi chahiye), isliye unko CTA se decouple kiya.
Font size change nahi (HomeButton body 13 reuse), align/spacing
untouched, colors zero. Harness round-1 me `RESULT: PASS`.

## 2. Geometry table (before → after, px)

| Item | Before | After | Kahan |
|---|---|---|---|
| CTA paint size | (120, 28) = 3360, longest label ~164 need → wrap | (224, 28) = 6272, 164 ≤ 224 → single line | `views.rs:home_paint_cta_size` |
| CTA width source | `HOME_CARD_BUTTON_MIN_W` (card aadha) | `home_card_cta_width()` = `HOME_CARD_CTA_W` (full inner 240 − 2×8) | `views.rs:349`, contract `home.rs:195` |
| Toolbar floor | `home_paint_cta_size()` via (120, 28) | `(HOME_CARD_BUTTON_MIN_W, HOME_CARD_BUTTON_MIN_H)` direct, same 120x28 | `views.rs:session_toolbar_button_size` |
| Path-row floor | `home_paint_cta_size()` via (120, 28) | `(HOME_CARD_BUTTON_MIN_W, HOME_CARD_BUTTON_MIN_H)` direct, same 120x28 | `views.rs:path_browse_button_size` |
| Card align/spacing | `vertical_centered` + `TIGHT_GAP`/`HOME_GRID_GAP` | same, zero change (full-inner width se centering moot) | `views.rs:paint_home` |
| Font/typo | HomeTitle 14 strong / HomeDesc small-weak / HomeButton body 13 | same, TypoRole reuse, koi naya size nahi | `views.rs:paint_home` |

## 3. Exact lines (`crates/gui/src/views.rs`)

- import: `home_card_cta_width` add (`use crate::home`, ~line 34).
- `home_paint_cta_size`: `(home_card_cta_width(), HOME_CARD_BUTTON_MIN_H)` = (224, 28), doc 120x28/3360 → 224x28/6272.
- `session_toolbar_button_size`: `(HOME_CARD_BUTTON_MIN_W, HOME_CARD_BUTTON_MIN_H)` direct (120x28 pin, CTA se decouple).
- `path_browse_button_size`: same direct 120x28 pin.
- `paint_home` doc: CTA "120x28 hit floor, area 3360" → "224x28 single-line budget, area 6272"; paint code unchanged (`add_sized` already `home_paint_cta_size()` se aata hai).
- test `home_paint_adopts_home_contract`: CTA assert (224.0, 28.0) + area 6272 + `home_card_cta_single_line_ok()`; unused `HOME_CARD_BUTTON_MIN_W` import drop.
- test `session_toolbar_and_path_buttons_reuse_home_cta_floor`: toolbar == path == (120, 28), area 3360 (CTA se equality assert hata, floor pin assert rakha).

## 4. Gates actual

- `cargo test -p autocompare-gui`: 661 passed, 0 failed (meri run); harness run me 663 passed, 0 failed (sibling wave ne beech me 2 tests add kiye, combined tree green).
- Clippy: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` zero warnings.
- `rustfmt --edition 2024 --check crates/gui/src/views.rs`: clean (ek assert ko single-line kiya).
- Banned (OWNS diff): zero (`unwrap/expect/panic/todo/unimplemented/unreachable/unsafe` koi hit nahi).
- Colors: diff me zero `Color32`/`Visuals` line.
- Harness `run-geo-typo.sh checks/gui-ctawidth-cf/round-1`: `RESULT: PASS` (1/6 rounds) — PNG 9765 bytes, magic ok, distinct 256.
- `checks/opencode-waves/STATUS.md`: untouched (`git status` me nahi hai). Sibling staged files (`app.rs`/`home.rs`/`checks/gui-modet7-cg/`) ko haath nahi lagaya.
- Commit `feat(gui): ...` local, PUSH MANA.

## 5. Harness signals (round-1, read-only)

- Geometry: Grid x14, `text_style_height` x4, 28 width calls, `add_space` values [] (koi naya gap nahi).
- Typography: monospace x50, strong x26, small x52, `.size` values ['14.0'] — koi naya size nahi (CTA HomeButton body 13 reuse).
- Colors: 23 touches, info-only (scope bahar, zero new).

## 6. Follow-up

- Koi code follow-up nahi — P1-4 paint adoption isi wave me complete hai. Agle live-capture round me live-home.png me CTA single line verify hoga ("Open Text Compare" ek line me).
- Toolbar/path-row floor 120x28 pin hai; agar kabhi unke labels lambe hon to alag wave me contract banega (is wave me koi change nahi).

## 7. Notes

- Rounds used: 1/6. Commit `feat(gui): ...` local, PUSH MANA.
- Unlazy note: task me `checks/gui-ctawidth-cf/GATES.md` pehle padhne ko tha, par dir exist nahi karta tha — BT-CARDCTA GATES pattern par naya GATES.md banaya, phir implement kiya.
- Shared-workdir note: harness run ke time sibling wave ke staged changes (`app.rs`/`home.rs`) tree me the — combined suite green (663/0). Mera commit sirf apne OWNS tak limited hai.
