# GATES — ck-darkbug (live-dark FAIL root-cause + 1-jagah fix)

Scope: `crates/gui/src/app.rs` me 1 block (per-frame theme re-assert, `App::ui` top).
`crates/gui/src/main.rs` READ-ONLY (CE+CI code untouched). NO-TOUCH: views.rs, home.rs,
screenshot.rs, theme.rs, baaki crates/gui/src/*, baaki checks, STATUS.md (MANA).

## Root-cause table (eframe-0.37 source-verified, `~/.cargo/registry` READ-ONLY)

| # | Fact (source) | Asar |
|---|---|---|
| 1 | `Options.theme_preference` default = `System`, `fallback_theme` = Dark (egui-0.37 `memory/mod.rs`) | Flag `dark_mode=true` ka ctx se koi link nahi jab tak preference explicit na ho |
| 2 | `Context::theme()` = System par `system_theme.unwrap_or(fallback)` (egui-0.37 `memory/mod.rs`) | Light-Mode macOS par first real frame se `theme() == Light` |
| 3 | `Context::set_visuals(v)` = `style_mut_of(self.theme(), …)` (egui-0.37 `context.rs:2704`) — sirf ACTIVE slot likhta hai | Creation closure me `set_visuals(dark)` fallback-Dark slot me jata hai (jo pehle se default-dark hai); light slot untouched rehta hai |
| 4 | Creation closure kisi pass se PEHLE chalta hai → `system_theme=None` → `theme()=Dark(fallback)` | CE fix (9e7b25b) Light-Mode OS par no-op hai — ye asli wajah hai, persist-restore nahi |
| 5 | Disk par eframe storage dir mili nahi (cj REPORT sec-5) → `cc.storage=None` → default dark expected tha, phir bhi light | Persist-restore theory fit nahi hoti; slot-miss theory fit hoti hai |
| 6 | views.rs toggle bhi `set_visuals` chalata hai (NO-TOUCH) — System preference par wo bhi galat slot corrupt karta hai | Per-frame re-assert ke bina double-toggle par desync wapas ata; fix ko har frame repair karna chahiye |

## Gate 1 — 1-jagah fix (app.rs, `App::ui` top)

- `ui.ctx().set_theme(...)` flag se (explicit Dark/Light — System follow khatm).
- Uske BAAD `ui.ctx().set_visuals(...)` (ab sahi active slot me likhta hai + toggle-corruption har frame repair).
- Sirf existing `egui::Theme` + `egui::Visuals::dark()/light()` reuse — naya palette zero.
- Non-test me banned zero: `unwrap`/`expect`/`panic!`/`todo!`/`unsafe` naya zero.

## Gate 2 — live-dark photo-proof (is wave ka core proof)

- Rebuild + live launch + `screencapture -R` region crop → `round-1/live-home.png`.
- Python pixel-check: content bg dark (panel gray ~27,27,27 ke aas-paas; white-nahi).
- Header badge photo: `dark_mode=true` + ctx dark (label "Light Mode" tabhi jab bg dark ho).
- Proof ke baad apna instance kill, sibling processes ko haath nahi.

## Gate 3 — rust/MS gates

- `cargo test -p autocompare-gui` green.
- `cargo clippy -p autocompare-gui --all-targets` zero warnings.
- `cargo fmt --check` clean (diff sirf OWNS).
- Banned zero (non-test grep). Colors zero new (diff me `Color32::from_rgb` zero;
  `Visuals::dark()/light()` lines sirf existing API reuse).
- PNG valid: harness report me magic ok + >=8192B + non-blank.

## Gate 4 — harness + hygiene

- `tools/gui-polish/run-geo-typo.sh checks/gui-darkbug-ck/round-1/` → `RESULT: PASS`.
- Har test/gate se PEHLE `git status --short`: sirf OWNS dirty
  (`crates/gui/src/app.rs`, `checks/gui-darkbug-ck/`).
- Commit pathspec-only (`git add crates/gui/src/app.rs checks/gui-darkbug-ck`), `git add -A` MANA.
- `STATUS.md` MANA. PUSH MANA.
