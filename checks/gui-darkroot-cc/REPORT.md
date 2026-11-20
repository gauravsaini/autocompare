# CC-DARKROOT — Light-vs-dark root-cause (static, P2)

## 1. Summary

Root cause lock: `App.dark_mode=true` (declared intent) kabhi live `egui` ctx par startup me apply hi nahi hota.
Paint `App.dark_mode` nahi, live `ui.visuals().dark_mode` padhta hai; fixture renderer dono ko bypass karke frozen-dark paint karta hai.
Isliye fixtures dark + live app light — flag aur ctx janam se desync hain.
Source me ZERO change; fix ke liye 3 options (A laziest) neeche.

## 2. Flow diagram (HEAD committed code se)

```
App::default()                     main.rs create              per-frame ui()
 dark_mode = TRUE  ────────────►  |_cc| ignore, ────────────►  ctx.set_visuals()
 (app.rs:373)                      set_visuals KABHI NAHI       KABHI NAHI (app.rs:4819)
 (field app.rs:256)                (main.rs:52)                 sirf zoom/dnd/rx/paint
        │                                                              │
        │ flag(true)                                                   │ ctx = eframe default
        │                                                              │ (Light-Mode macOS → light=false)
        ▼                                                              ▼
paint_header(ui, &mut dark_mode) ◄── app.rs:5023          paint sites: let dark = ui.visuals().dark_mode
 (views.rs:435)                                     (views.rs x11: 1510,1997,2182,2703,4785,
  click par hi set_visuals                          5156,5459,6012,6080,6242,6327)
  (views.rs:450-455)                                         │
        │                                                    ▼
        │                                              palette(dark) → LIGHT_PALETTE
        │                                              (theme.rs:741-743)
        │                                                    │ live = LIGHT
        │
        └─► label ulta: flag true → "☀ Light Mode"
            dikhta hai jab ctx already light hai

Fixture alag raasta (ctx bypass):
  render_fixture_png → fill_rect(BG_DARK=[27,27,27])
  (screenshot.rs:27-28,149) → PNG HAMESHA DARK
```

## 3. Root-cause (ek paragraph me)

Do halves do waves me bane aur beech ki wiring kabhi likhi nahi gayi:
`7ffe91d` ne `dark_mode=true` default + toggle-ke-andar `set_visuals` diya (startup-apply kabhi tha hi nahi);
`71fab3a` (wave G) ne dual-palette paint diya jo live ctx (`ui.visuals().dark_mode`) padhta hai, flag nahi.
Pehle frame se flag(true) vs ctx(false) desync hai: live window OS-default light me khulta hai, header toggle ulta label dikhata hai,
aur frozen-dark fixture renderer (screenshot.rs) is khaai ko pakka karta hai — fixtures dark, live light.

## 4. Evidence snippets (HEAD se, working-tree nahi)

- `git show HEAD:crates/gui/src/app.rs | sed -n '373p'` → `dark_mode: true,`
- `git show HEAD:crates/gui/src/main.rs | sed -n '52p'` → `Box::new(|_cc| Ok(Box::new(app::App::default()))),`
- `git show HEAD:crates/gui/src/views.rs | sed -n '450,455p'` →
  `ui.ctx().set_visuals(if *dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() });` (click-block ke andar)
- `git show HEAD:crates/gui/src/theme.rs | sed -n '742,743p'` → `if dark { DARK_PALETTE } else { LIGHT_PALETTE }`
- `git show HEAD:crates/gui/src/screenshot.rs | sed -n '27p'` → `const BG_DARK: [u8; 4] = [27, 27, 27, 255];`

## 5. Fix options

- **A (laziest, recommended):** startup par flag apply karo — `main.rs:52` me
  `cc.egui_ctx.set_visuals(egui::Visuals::dark())`, ya `app.rs:4819` me first-frame guard.
  1-2 lines, live dark hokar Default + fixtures se align hoga. Risk LOW (code), par live light→dark
  user-visible change hai — follow-up me live re-capture + fixture diff-proof chahiye.
- **B:** default flip (`app.rs:373` true→false) — live OS-light se match, 1 line.
  Par fixtures frozen-dark rehte hain, mismatch aadha bachta hai. NOT recommended akela.
- **C (full):** A + persist + fixture mode-param (`screenshot.rs:83` `render_fixture_png` me dark/light switch).
  Sahi long-term, par bada diff + harness proof — alag wave ka kaam.

## 6. Follow-up

- A land karne wali wave OWNS rakhe: `main.rs` ya `app.rs` (sirf ek apply-point, dono nahi).
- Proof: live re-capture (Light-Mode OS par bhi dark khulna chahiye) + fixture byte-compare + header-label photo
  (`dark_mode=true` par `"☀ Light Mode"` tabhi jab ctx bhi dark ho).
- `STATUS.md` touch nahi kiya (MANA) — orchestrator apne pass update kare.
- PusH MANA — commit local, orchestrator verifies + pushes.
