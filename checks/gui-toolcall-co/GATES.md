# CO-TBARCALL GATES — App toolbar call-site adoption (owns crates/gui/src/app.rs + checks/gui-toolcall-co/**)

Scope: app.rs toolbar row -> `views::paint_session_toolbar` call-site only. main.rs / views.rs /
theme.rs / screenshot.rs / STATUS.md READ-ONLY (zero edit). Colors FROZEN: NO Color32/Visuals/hue
touch. Geometry/typo numbers: koi naya literal nahi, sirf wiring. MS rust: forbid(unsafe_code)
kayam, non-test me unwrap/expect/panic/todo MANA (diff-added lines zero). Commit LOCALLY
(`feat(gui): adopt paint_session_toolbar call-site in App (CO-TBARCALL)`), PUSH MANA. Koi live
process chhodna MANA.

Gap (checks/gui-toolwire-cl/REPORT.md section 5 loop): App toolbar row (app.rs toolbar block)
plain-text hai — `"Swap Sides"` / `"Reload / Recompare"` default buttons, jabki views.rs me
glyph+role+floor contract + live paint fn ready hai (`paint_session_toolbar`, `ToolbarAction`).

Adoption (ponytail, smallest wiring diff, naya painter MANA):
- `views::paint_session_toolbar(ui, toolbar_active)` call-site (`enabled = (tab != Home)`
  reuse); hit par `Swap` -> `swap_sides + run_active_tab`, `Reload` -> `run_active_tab`.
- Views fn reuse: glyph labels + chrome role + 120x28 floor andar hi; App me koi naya
  size/typo/color literal nahi. Toggles + gaps + Home hint untouched.

- [ ] G1: Toolbar row views call-site par hai, purana plain-text row gaya
  CHECK: grep -n "paint_session_toolbar\|ToolbarAction" crates/gui/src/app.rs
  EXPECT: call-site + `Swap`/`Reload`/`None` match arms; `egui::Button::new("Swap Sides")`
  aur `egui::Button::new("Reload / Recompare")` zero hits (sirf Home hint sentence me
  descriptive text rehta hai, row me nahi)

- [ ] G2: Full suite green + clippy + fmt
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -2
  EXPECT: 673 passed, 0 failed
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: zero warnings
  CHECK: rustfmt --edition 2024 --check crates/gui/src/app.rs
  EXPECT: clean (FMT-CLEAN)

- [ ] G3: Banned + colors zero new
  CHECK: git diff crates/gui/src/app.rs | grep "^+" | grep "unwrap\|expect(\|panic!\|todo!" || echo DIFF-BANNED-CLEAN
  EXPECT: DIFF-BANNED-CLEAN (pre-existing `unwrap_or` lines untouched, diff adds none)
  CHECK: git diff crates/gui/src/app.rs | grep -c "Color32\|Visuals::"
  EXPECT: 0

- [ ] G4: Fixture PNGs byte-stable + magic ok
  CHECK: file checks/gui-toolcall-co/before.png checks/gui-toolcall-co/after.png
  EXPECT: dono `PNG image data, 960 x 600` magic ok
  CHECK: cmp checks/gui-toolcall-co/before.png checks/gui-toolcall-co/after.png && echo BYTE-IDENTICAL
  EXPECT: BYTE-IDENTICAL (wajah REPORT section 3 me: screenshot scene Home hai, toolbar
  row us scene me paint nahi hota; glyph live-verify round-7 scope)
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-toolcall-co/round-1/
  EXPECT: exit 0, geo-typo-report.md me `RESULT: PASS`

- [ ] G5: Hygiene — OWNS only, STATUS untouched, local commit NO push
  CHECK (BEFORE commit): git status --short
  EXPECT: sirf `M crates/gui/src/app.rs` + `?? checks/gui-toolcall-co/`
  CHECK: git diff --name-only | grep -vE "^(crates/gui/src/app.rs|checks/gui-toolcall-co/)" || echo OWNS-CLEAN
  EXPECT: OWNS-CLEAN
  CHECK: git status --short STATUS.md; git diff --name-only -- STATUS.md || echo STATUS-UNTOUCHED
  EXPECT: STATUS par koi entry nahi (STATUS-UNTOUCHED)
  CHECK: git show --stat HEAD --oneline (commit ke BAAD verify)
  EXPECT: stat me sirf `crates/gui/src/app.rs` + `checks/gui-toolcall-co/**`
  CHECK: git add crates/gui/src/app.rs checks/gui-toolcall-co && git commit -m "feat(gui): adopt paint_session_toolbar call-site in App (CO-TBARCALL)" && git log --oneline -1 && git status --short
  EXPECT: wahi pathspec ONLY, wahi English-only message, local only, NO push
  CHECK: REPORT/GATES Hinglish-Roman, Devanagari zero
  EXPECT: grep -P "[\x{0900}-\x{097F}]" checks/gui-toolcall-co/REPORT.md checks/gui-toolcall-co/GATES.md -> no hits
