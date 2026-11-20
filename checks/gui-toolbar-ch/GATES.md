# CH-TOOLBAR P2-1 GATES — toolbar icon parity (owns crates/gui/src/views.rs + checks/gui-toolbar-ch/**)

Scope: views.rs toolbar icon contract only. app.rs / home.rs / main.rs / theme.rs / screenshot.rs READ-ONLY (no edits).
Colors frozen: NO Color32/Visuals/hue changes. MS rust: forbid unsafe, no unwrap/expect/panic/todo in non-test code.
Commit locally (`feat(gui): ...`), PUSH MANA.

Gap (round-3 REPORT section 5 P2-1: "Toolbar icon parity nahi"):
- Toolbar actions (app.rs:5073-5088) text-only hain — "Swap Sides" / "Reload / Recompare" me koi
  leading glyph nahi, jabki baaki chrome me glyph+label pattern pehle se hai (gutter copy `→/←`,
  paging `◀/▶`, expand `▼/▶`, mode toggle `☀/🌙`, recent titles `↔`).
- Koi icon-label gap token nahi, koi chrome typo-role lock nahi (buttons default style par hain).

Adoption (ponytail, shortest diff, zero new assets):
- `TOOLBAR_SWAP_GLYPH = "↔"` (`\u{2194}` reuse, home_recent_row_title precedent) + `TOOLBAR_RELOAD_GLYPH = "▶"`
  (paging-nav precedent) — dono glyph codebase me pehle se hain.
- `toolbar_icon_gap() -> TIGHT_GAP` (geometry), `toolbar_button_role() -> chrome_tab_role()` (typo chrome reuse).
- `toolbar_swap_label()` / `toolbar_reload_label()` — glyph + space + existing English label (labels unchanged).
- Contract-only is wave me (App wiring agle wave, session-strip precedent); paint output unchanged.

- [ ] G1: Icon contract reuses existing glyphs + geo/typo tokens, zero new literals/assets
  CHECK: grep -n "toolbar_icon_gap\|toolbar_button_role\|toolbar_swap_label\|toolbar_reload_label\|TOOLBAR_SWAP_GLYPH\|TOOLBAR_RELOAD_GLYPH" crates/gui/src/views.rs
  EXPECT: helpers defined near session_toolbar_button_size; gap reuses TIGHT_GAP, role reuses chrome_tab_role,
  glyphs reuse ↔ (U+2194) + ▶, labels keep existing English text, zero new float literals / Color32 / Visuals

- [ ] G2: Headless tests lock the contract, full suite green
  CHECK: cargo test -p autocompare-gui toolbar_ + cargo test -p autocompare-gui 2>&1 | tail -2
  EXPECT: new toolbar icon test green, full suite 0 failed
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: zero warnings
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: clean

- [ ] G3: Harness PASS (paint output unchanged, PNG valid)
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-toolbar-ch/round-1/
  EXPECT: exit 0, geo-typo-report.md contains RESULT: PASS (max 6 rounds, same round dir on re-run)

- [ ] G4: Hygiene — owned files only, zero banned patterns, local commit NO push
  CHECK (BEFORE test/gate): git status --short
  EXPECT: only crates/gui/src/views.rs + checks/gui-toolbar-ch/**
  CHECK: grep -n "unwrap\|expect(\|panic\|todo!" crates/gui/src/views.rs | grep -v test ; echo BANNED:$?
  EXPECT: 0 non-test hits (diff adds none)
  CHECK: git diff --stat; git diff crates/gui/src/views.rs | grep -c "Color32\|Visuals::"
  EXPECT: colors zero new
  CHECK: git log --oneline -1
  EXPECT: English-only conventional commit feat(gui): ..., local only, NO push
  CHECK: REPORT checks/gui-toolbar-ch/REPORT.md Hinglish-Roman, Devanagari zero
