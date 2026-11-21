# REPORT — exq-toolbar74 round-1: toolbar wiring (app.rs call-site)

OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-toolbar74/**`. NO-TOUCH sab respected
(views.rs / home.rs / theme.rs / baaki crates + STATUS.md untouched; stray
`checks/gui-sessionpaint-by` PNG dirt revert kiya, `crates/sessions` clean confirm).

## 1. Gap

views.rs `paint_toolbar_glyph` live ink `dim_for(dark)` rakhta hai (views.rs:611) —
active/disabled dono ek hi ink, koi app-side contract nahi. App call-site
`paint_session_action_bar_live` me `enabled` inline `self.tab != Tab::Home` se banta tha.
Is wave me app.rs side contract + call-site wiring, views consumption agle wave.

## 2. Adoption (ponytail, 3 chhote diffs, zero naya token/hue/size)

- `App::toolbar_active()` — Home par false, baaki 9 tabs par true. `paint_session_action_bar_live`
  ab isi se `enabled` banata hai (inline `!=` hata).
- `toolbar_glyph_ink_for(active, dark)` — active → `theme::dim_for` (live painter
  byte-identical), disabled → `theme::track_for` (frozen disabled tone). Mode flag seedha
  `*_for` me route (inline-accent precedent), koi hardcoded Color32 nahi. Views consumption
  agle wave (lightvisual68 unwired-readiness precedent); behavior unchanged.
- 3 headless kittests (pure, koi painter nahi): `toolbar74_active_home_off_others_on`,
  `toolbar74_glyph_ink_active_matches_live_painter`, `toolbar74_glyph_ink_disabled_uses_track`.

## 3. Evidence

- `cargo test -p autocompare-gui toolbar74` → 3 passed.
- `cargo test -p autocompare-gui` → 1143 passed, 0 failed (+2 integration).
- `cargo fmt --check -p autocompare-gui` → clean.
- Diff scan: banned (`unwrap`/`expect(`/`panic!`/`todo!` added lines) 0, `Color32::from_rgb`/`Visuals::` 0, naya float literal 0.
- `tools/exquisite/run.sh checks/gui-exq-toolbar74/round-1/` → exit 0, `RESULT: PASS`.
- `git status` gate se pehle: sirf `M crates/gui/src/app.rs` + `?? checks/gui-exq-toolbar74/`.

## 4. Loop suggestion

Agli wave (views.rs OWNS): `paint_toolbar_glyph` me `enabled` param jodo —
ink `crate::app::toolbar_glyph_ink_for(enabled, dark)` se aaye; call-sites
(`paint_toolbar_action_buttons`, `paint_action_nav_row`) `enabled` pass karen.
Geometry same (slot/rects untouched), typo same, colors frozen reuse.
