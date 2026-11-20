# REPORT — CL-TOOLWIRE P2-1 toolbar call-site wiring (views.rs paint fn)

OWNS: `crates/gui/src/views.rs` + `checks/gui-toolwire-cl/**`. NO-TOUCH: app.rs / home.rs / main.rs /
theme.rs / screenshot.rs / baaki crates/gui/src/* / baaki checks / STATUS.md (sab untouched, `git status` proof neeche).

## 1. Gap (toolbar-ch REPORT section 5 loop: App wiring pending, `toolbar_reload_label` par dead_code)

- Toolbar-ch wave ne glyph+role contract banaya tha (`TOOLBAR_*_GLYPH`, `toolbar_*_label`,
  `toolbar_button_role`, `toolbar_icon_gap`) par koi live paint call-site nahi tha — 6 items par
  `#[allow(dead_code)]` laga tha, App row (app.rs:~5096-5118) abhi bhi text-only paint karti hai.
- BC reference: is wave me BC ka fresh capture nahi liya (screenshot frozen/NO-TOUCH); gap apne
  toolbar audit + toolbar-ch fixtures se aaya — App row ke do buttons (`"Swap Sides"` /
  `"Reload / Recompare"`) default style par hain, jabki contract icon-led chrome ready hai.

## 2. Adoption (ponytail, shortest wiring diff — 71 insertions, 5 deletions, zero new tokens/assets)

- `ToolbarAction` enum (`None` default / `Swap` / `Reload`) — `SessionAction` precedent (paint emit,
  caller mutate; App hit par swap/run agle wave me chalayegi).
- `paint_session_toolbar(ui, enabled) -> ToolbarAction` — labels `toolbar_swap_label()` /
  `toolbar_reload_label()` (glyph prefix, English text unchanged), typo `toolbar_button_role().rich(..)`
  (Body 13 proportional), floor `session_toolbar_button_size()` min_size (120x28), rhythm `TIGHT_GAP`
  (App `toolbar_item_gap` mirror, same value), `enabled` par `add_enabled` (App `toolbar_active`
  mirror: Home par off).
- 5 `#[allow(dead_code)]` hatao (2 glyph const + role + 2 labels — paint fn live call-site hai);
  `toolbar_icon_gap` ka allow rehta hai (glyph label string ke andar hai, alag icon widget nahi);
  naye paint items (enum + fn) par `SessionAction` precedent wala allow (App wiring agle wave).
  Net dead_code: 42 -> 39 (ghata prove, section 4).
- Geometry unchanged (koi naya float literal/size nahi — floor + gap sab reuse), TypoRole reuse,
  colors FROZEN (diff me `Color32`/`Visuals::` zero).

## 3. Evidence

- `cargo test -p autocompare-gui toolbar_` → 5 passed (1 naya
  `toolbar_action_defaults_to_none_with_swap_and_reload` + 4 existing).
- `cargo test -p autocompare-gui` → 670 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` → zero warnings.
- `cargo fmt --check -p autocompare-gui` → clean.
- `tools/gui-polish/run-geo-typo.sh checks/gui-toolwire-cl/round-1/` → exit 0, `RESULT: PASS`
  (round 1/6, PNG 9765B magic ok distinct 256 — toolbar-ch wave se byte-identical, paint output
  unchanged kyunki App abhi purana row paint karti hai).
- Banned scan (non-test): `unwrap(`/`expect(`/`panic!`/`todo!` → 0 hits; `forbid(unsafe_code)` kept.
- Colors: `git diff crates/gui/src/views.rs | grep -c "Color32\|Visuals::"` → 0.
- `git status` gate se pehle: sirf `M crates/gui/src/views.rs` + `?? checks/gui-toolwire-cl/` (proof section 4).
- `git diff --stat`: views.rs 71 insertions, 5 deletions, baaki OWNS sirf naya dir.

## 4. Status proof (gate se pehle, shared-workdir savdhani)

```
M crates/gui/src/views.rs
?? checks/gui-toolwire-cl/
```

## 5. Loop suggestion

- Agli wave: App toolbar row (app.rs:~5096-5118, NO-TOUCH is wave me) `paint_session_toolbar` par
  switch kare — `enabled = (tab != Home)`, hit par `Swap` → `swap_sides + run_active_tab`,
  `Reload` → `run_active_tab`; `min_size` floor same rahe (120x28) to tab-switch rule khule par
  live Text-tab capture me icon-led buttons verify hon. `toolbar_icon_gap` ka allow tab hata jab
  glyph label se alag widget bane (abhi string ke andar hai).
