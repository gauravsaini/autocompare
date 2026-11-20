# CK-DARKBUG — live-dark FAIL root-cause + 1-jagah fix (round-1 PASS)

## 1. Asli wajah (eframe-0.37 source-verified, registry READ-ONLY)

CE (9e7b25b) + CI (262eb58) land ke baad bhi live LIGHT khulta tha. Persist-restore
theory galat thi — is machine par eframe storage dir mili hi nahi (cj REPORT sec-5
item 1), to restore hone ke liye kuch tha hi nahi. Asli wajah apply-path me hai:

- egui-0.37 default `Options.theme_preference = System`, `fallback_theme = Dark`
  (`memory/mod.rs`). `Context::theme()` System par
  `system_theme.unwrap_or(fallback)` deta hai — Light-Mode macOS par first real
  frame se `theme() == Light`.
- `Context::set_visuals(v)` = `style_mut_of(self.theme(), …)` (`context.rs:2704`) —
  sirf ACTIVE theme slot likhta hai, dono nahi.
- `main.rs` creation closure kisi pass se PEHLE chalta hai: `system_theme` abhi
  `None` hai, isliye `theme() == Dark(fallback)` aur `set_visuals(dark)` dark slot
  me jata hai (jo pehle se default-dark hai). Light slot untouched rehta hai.
- First real frame me winit `system_theme=Light` report karta hai → live ui
  light slot (default light) use karta hai → LIGHT window, jabki flag true.
  Badge "Light Mode" flag-true par ctx-light ke upar = desync barkarm.
- `views.rs` toggle (NO-TOUCH) bhi `set_visuals` chalata hai — System preference
  par wo bhi galat slot likhta hai, isliye double-toggle par desync wapas ata.
  Fix ko har frame repair karna chahiye, startup-ek-bar nahi.

## 2. Fix (1 jagah, sirf app.rs)

`crates/gui/src/app.rs` — `App::ui` ke top par per-frame re-assert block:

```rust
ui.ctx().set_theme(if self.dark_mode { egui::Theme::Dark } else { egui::Theme::Light });
ui.ctx().set_visuals(if self.dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() });
```

Pehle preference explicit (System-follow khatm), phir visuals — ab `set_visuals`
hamesha sahi active slot me likhta hai aur toggle-corruption har frame repair
hota hai. `main.rs` untouched (CE+CI code jaisa tha waisa). Naya palette zero —
sirf existing `egui::Theme` + `Visuals::dark()/light()` reuse. Banned zero.

## 3. Live proof (Light-Mode OS, dark mode=false verify karke)

- OS Light-Mode par final-tree binary launch (PID 65386, window 320,91 800x632 —
  cj round-4 jaisa exact frame), `screencapture -R` → `round-1/live-home.png`
  (1600x1264 @2x, PNG magic ok).
- Photo me: dark title bar (sync_window_theme ne native chrome bhi dark kiya),
  dark bg + dark cards, header badge "Light Mode" flag-true + ctx-dark ke sath —
  sahi toggle affordance, desync khatm. Apna instance kill kiya, sibling ko hath nahi.
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-darkbug-ck/round-1/` →
  `RESULT: PASS` (after.png 9765 B, magic ok, non-blank; tests.out samet).

## 4. Gates (round-1)

- `cargo test -p autocompare-gui`: 670 passed, 0 failed (cj 669 + landed waves ka +1).
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings.
- `cargo fmt --check`: clean (diff sirf OWNS).
- Banned zero: diff me `unwrap`/`expect`/`panic!`/`todo!`/`unsafe` naya zero.
- Colors zero new: diff me `Color32::from_rgb` zero; 2 `Visuals::` lines existing API reuse.
- Har test/gate se PEHLE `git status --short`: beach me sibling `views.rs` dirt dikhi
  (sibling ne 48946aa me commit kar diya) — usko touch/stash nahi kiya, sirf apne
  OWNS (`crates/gui/src/app.rs`, `checks/gui-darkbug-ck/`) verify karke aage badha.
- `STATUS.md` untouched. PUSH nahi kiya.

## 5. Follow-up

- Light-mode live visual (toggle→light) ka photo-proof alag wave me (click-rule chahiye).
- `main.rs` creation-time `set_visuals` ab redundant-harmless hai — chheda nahi
  (ponytail: chalti cheez ko dobara mat chhedo); hataana ho to alag cleanup wave me.
- Fixture CLI me `--screenshot-mode light` flag ab bhi khula hai (cj sec-6 item 5).
