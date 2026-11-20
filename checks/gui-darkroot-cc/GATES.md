# GATES — gui-darkroot-cc (static, no harness)

Scope: READ-ONLY investigate. Koi test/harness run nahi (sibling dirt se result bigadta).
Evidence sirf `git show HEAD:<path>` + `git log -S` se. Source me ZERO edit.

## Gate 1 — dark_mode flow table (HEAD committed code)

| # | Point | File:line (HEAD) | Kya hota hai |
|---|---|---|---|
| F1 | Flag field | `crates/gui/src/app.rs:256` | `pub dark_mode: bool` — App-owned intent flag |
| F2 | Default | `crates/gui/src/app.rs:373` | `dark_mode: true` — declared intent = dark |
| F3 | Startup create | `crates/gui/src/main.rs:52` | `Box::new(\|_cc\| Ok(Box::new(app::App::default())))` — `_cc` ignore, `set_visuals` kabhi nahi |
| F4 | Per-frame apply | `crates/gui/src/app.rs:4819` (`impl eframe::App for App::ui`) | Frame start par `dark_mode` se `ctx.set_visuals` ka koi call nahi (zoom/dnd/progress/rx + paint only) |
| F5 | Toggle def | `crates/gui/src/views.rs:435` | `pub fn paint_header(ui, dark_mode: &mut bool)` — flag ka ekmatra writer |
| F6 | Toggle write | `crates/gui/src/views.rs:450-455` | `set_visuals(dark/light)` SIRF button-click ke andar; first frame par kabhi nahi |
| F7 | Toggle callsite | `crates/gui/src/app.rs:5023` | `views::paint_header(ui, &mut self.dark_mode)` — har frame paint, write sirf click par |
| F8 | Palette selector | `crates/gui/src/theme.rs:741-743` | `pub const fn palette(dark: bool)` → `DARK_PALETTE` / `LIGHT_PALETTE` |
| F9 | Paint readers | `crates/gui/src/views.rs:1510,1997,2182,2703,4785,5156,5459,6012,6080,6242,6327` | Har site `let dark = ui.visuals().dark_mode` — LIVE ctx padhta hai, `App.dark_mode` nahi |
| F10 | Fixture frozen | `crates/gui/src/screenshot.rs:27-28,149` | `BG_DARK=[27,27,27,255]` hardcode + `fill_rect(..., BG_DARK)` — headless hamesha dark, flag/ctx se independent |

## Gate 2 — default-value chain

`App::default()` → `dark_mode=true` (F2) → koi startup `set_visuals` nahi (F3,F4)
→ first frame ctx = eframe default (Light-Mode macOS par light, `ui.visuals().dark_mode=false`)
→ flag(true) vs ctx(false) DESYNC janam se.

## Gate 3 — live-vs-screenshot divergence point

- Live: ctx light (false) → F9 sites `LIGHT_PALETTE` chunte hain → window light dikhta hai.
  Par flag true hai, isliye header button ulta label dikhata hai (`"☀ Light Mode"` jab already light).
- Fixture: F10 frozen dark → PNG hamesha dark.
- Natija: fixtures dark + live light — wahi mismatch jo PLAN sec-7 aur REPORT sec-6 me note hai.
- Divergence point ek line me: **F4 (koi startup/frame apply nahi) + F9 (paint ctx padhta hai) + F10 (fixture ctx ko bypass karta hai).**

## Gate 4 — history (git log -S, doubt mito)

- `git log -S "dark_mode" -- crates/gui/src/app.rs` → `7ffe91d` — flag + toggle-only `set_visuals` isi commit me janma, startup-apply kabhi tha hi nahi.
- `git log -S "set_visuals" -- crates/gui/src/` → sirf `7ffe91d` — aaj tak koi doosra apply-point add nahi hua.
- `git log -S "pub const fn palette" -- crates/gui/src/theme.rs` → `71fab3a` (wave G) — dual-palette paint baad me aaya, ctx-read (F9) par tikka; flag se wiring kabhi judi nahi.
- Matlab: do halves alag-alag waves me bane, beech ki wiring (flag→ctx at startup) kabhi likhi hi nahi gayi.

## Gate 5 — fix options (file:line + risk note)

| Opt | Change | File:line | Risk |
|---|---|---|---|
| A (laziest, recommended) | Startup par flag apply: `cc.egui_ctx.set_visuals(egui::Visuals::dark())` ya first-frame `if self.dark_mode { ctx.set_visuals(...) }` guard | `crates/gui/src/main.rs:52` ya `crates/gui/src/app.rs:4819` | LOW code-risk (1-2 lines); VISIBLE change — live light→dark hoga; follow-up fixture-vs-live re-proof chahiye |
| B | Default flip: `dark_mode: true` → `false` (live OS-light se match) | `crates/gui/src/app.rs:373` + label logic `crates/gui/src/views.rs:444` check | LOW code-risk (1 line); par fixtures (frozen dark, F10) ab bhi diverge — mismatch aadha bachta hai |
| C (full) | A + persist (per-run se aage) + fixture mode param (dark/light switch) | `main.rs:52`, `app.rs:4819`, `screenshot.rs:83` (`render_fixture_png`) | MEDIUM — sahi long-term, par bada diff + harness proof mangta hai; is wave ke bahar |

## Gate 6 — hygiene

- [x] `git status --short` me sirf `checks/gui-darkroot-cc/` (untracked) — no source touch
- [x] `STATUS.md` untouched
- [x] Koi test/harness run nahi (static only)
- [x] Commit pathspec-only (`git add checks/gui-darkroot-cc`), `git add -A` MANA
- [x] PUSH MANA (orchestrator verifies + pushes)
