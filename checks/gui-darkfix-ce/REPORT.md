# CE-DARKFIX — option A startup visuals apply (round-1 PASS)

## 1. Flow (root-cause se fix tak)

- Root-cause (gui-darkroot-cc sec-5): `App.dark_mode=true` (app.rs:373) kabhi live ctx par
  apply nahi hota tha — `main.rs:52` me closure `_cc` ko ignore karta tha, aur per-frame
  `set_visuals` ka koi call nahi tha (app.rs:4819). Paint sites live `ui.visuals().dark_mode`
  padhte hain (views.rs x11), jabki fixture renderer frozen-dark paint karta hai
  (screenshot.rs `BG_DARK`). Natija: flag(true) vs ctx(false) desync janam se —
  live light khulta tha, fixtures dark.
- Unlazy order follow kiya: pehle `GATES.md` likha (startup visuals test + live-vs-fixture
  alignment note + VISIBLE change document), phir implement kiya.
- Pehla attempt `egui::Visuals::dark()` se fail hua (E0433 — `egui` crate direct scope me
  nahi hai). Codebase pattern check karke `eframe::egui::Visuals::dark()` use kiya
  (views.rs:452 / app.rs:18 wala pattern) — compile green.

## 2. Fix (1-line net, sirf main.rs)

`crates/gui/src/main.rs:52` — closure ab `cc` leta hai aur `App::default()` se PEHLE
`cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark())` lagata hai.
Live ctx first frame se dark hota hai aur `App.dark_mode=true` + fixtures se align hota hai.
Naya palette zero — sirf existing `Visuals::dark()` API reuse. `app.rs` / `theme.rs` /
`screenshot.rs` untouched (READ-ONLY rule).

## 3. Visible-change note (expected, regression nahi)

- EXPECTED: live window ab Light-Mode OS par bhi dark khulega (pehle light khulta tha).
- Header toggle label desync khatm: `dark_mode=true` par "Light Mode" tabhi jab ctx bhi dark ho.
- Proof is wave me: harness `RESULT: PASS` + PNG valid (9765 bytes, magic ok, non-blank)
  + 659 tests green. Live re-capture photo-proof follow-up wave ka kaam hai.

## 4. Gates (round-1)

- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-darkfix-ce/round-1/` → `RESULT: PASS`
  (round 1/6, aage round ki zaroorat nahi padi).
- `cargo test -p autocompare-gui`: 659 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings.
- `cargo fmt --check`: clean.
- Banned zero: `main.rs` me `unwrap`/`expect`/`panic!`/`todo!`/`unsafe` naya zero
  (sirf pre-existing `#![forbid(unsafe_code)]` line hai).
- Colors zero new: diff me `Color32::from_rgb` zero; ek `Visuals::dark()` line (existing API).
- Shared-workdir: har test/gate se pehle `git status --short` — sirf OWNS dirty
  (`crates/gui/src/main.rs`, `checks/gui-darkfix-ce/`); sibling `views.rs` ko touch nahi kiya.
- `STATUS.md` untouched. PUSH nahi kiya (orchestrator verifies + pushes).

## 5. Follow-up (alag wave, option C full fix)

- Persist (`dark_mode` per-run se aage) + fixture mode-param (`render_fixture_png` me
  dark/light switch) + live re-capture photo-proof — bada diff + harness proof mangta hai.
- Is wave ka scope yahin khatm: option A land, live Default + fixtures align.
