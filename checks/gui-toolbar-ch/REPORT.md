# REPORT — CH-TOOLBAR P2-1 toolbar icon parity (views.rs contract)

OWNS: `crates/gui/src/views.rs` + `checks/gui-toolbar-ch/**`. NO-TOUCH: app.rs / home.rs / main.rs / theme.rs /
screenshot.rs / baaki crates/gui/src/* / STATUS.md (sab untouched, `git status` proof neeche).

## 1. Gap (round-3 REPORT section 5 P2-1: "Toolbar icon parity nahi")

- Round-3 row P2-1 ka follow-up `views.rs` me hai: toolbar actions (app.rs:5073-5088) text-only hain —
  "Swap Sides" / "Reload / Recompare" me koi leading glyph nahi, koi icon-label gap token nahi,
  koi chrome typo-role lock nahi (buttons default style par paint hote hain).
- BC reference: is wave me BC ka fresh capture nahi mila — round-3 notes ke mutabiq BC window region
  capture me nahi aayi thi (galat PNG delete hua tha), isliye BC pixels se direct tulna possible nahi thi.
  Gap apne toolbar audit + fixtures + round-3 row se nikala: baaki chrome me glyph+label pattern pehle
  se standard hai (gutter copy `→/←`, paging `◀/▶`, expand `▼/▶`, mode toggle `☀/🌙`, recent titles `↔`),
  sirf toolbar actions is pattern se bahar the. BC-style icon-led actions ka matlab yahi hai: har action
  ke aage ek chhota aligned glyph, glyph aur label ke beech fixed gap, label chrome type me.
- Fixtures (`checks/gui-localrun-ca/round-3/live-home.png`, `fixture-*.png`) me toolbar text-only dikhta hai —
  wahi gap confirm karta hai, koi naya visual claim nahi.

## 2. Adoption (ponytail, shortest diff — 48 insertions, 0 deletions, zero new assets)

`session_toolbar_button_size` ke neeche, usi rhythm me (views.rs:363-394):

- `TOOLBAR_SWAP_GLYPH = "↔"` (`\u{2194}` reuse — `home_recent_row_title` precedent) + `TOOLBAR_RELOAD_GLYPH = "▶"`
  (paging-nav `Next hunk ▶` precedent: forward/run affordance). Dono glyph codebase me pehle se hain.
- `toolbar_icon_gap() -> TIGHT_GAP` (geometry: icon-label gap toolbar item rhythm par).
- `toolbar_button_role() -> chrome_tab_role()` (typo: Body 13 proportional chrome reuse, mono nahi).
- `toolbar_swap_label()` / `toolbar_reload_label()` — glyph + space + existing English label
  (`"↔ Swap Sides"`, `"▶ Reload / Recompare"`; label text unchanged, sirf glyph prefix).
- Contract-only is wave me (App wiring agle wave — `paint_session_tab` / `paint_session_tab_strip`
  precedent, `#[allow(dead_code)]` + tests live). Paint output unchanged, isliye harness trivially stable.
- Geometry first, typo second, colors FROZEN: diff me `Color32`/`Visuals::` zero (views.rs 4 -> 4).

## 3. Evidence

- `cargo test -p autocompare-gui toolbar_` → 4 passed (1 naya `toolbar_icon_parity_reuses_chrome_and_glyphs` + 3 existing).
- `cargo test -p autocompare-gui` → 664 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` → zero warnings (pehle dead-code 6 the,
  file precedent wala `#[allow(dead_code)]` lagaya — App wiring agle wave).
- `cargo fmt --check -p autocompare-gui` → clean.
- `tools/gui-polish/run-geo-typo.sh checks/gui-toolbar-ch/round-1/` → exit 0, `RESULT: PASS` (round 1/6,
  PNG 9765B magic ok distinct 256; Grid x14, mono x50, strong x26, small x52 — tree unchanged).
- Banned scan (diff): `unwrap`/`expect(`/`panic`/`todo!` → 0 non-test hits.
- `git status` gate se pehle: sirf `M crates/gui/src/views.rs` + `?? checks/gui-toolbar-ch/` (proof section 4).
- `git diff --stat`: views.rs 48 insertions, baaki OWNS sirf naya dir.

## 4. Status proof (gate se pehle, shared-workdir savdhani)

```
M crates/gui/src/views.rs
?? checks/gui-toolbar-ch/
```

## 5. Loop suggestion

- Agli wave: App toolbar row (app.rs:5073-5088, NO-TOUCH is wave me) `toolbar_swap_label()` /
  `toolbar_reload_label()` + `toolbar_button_role().rich(..)` par switch kare, `min_size` floor
  (`session_toolbar_button_size`, 120x28) same rahe; tab-switch rule khule to live Text-tab capture
  me icon-led buttons verify ho.
