# CL-TOOLWIRE P2-1 GATES — toolbar call-site wiring (owns crates/gui/src/views.rs + checks/gui-toolwire-cl/**)

Scope: views.rs me toolbar paint call-site only. app.rs / home.rs / main.rs / theme.rs / screenshot.rs /
baaki crates/gui/src/* READ-ONLY (zero edit). Colors frozen: NO Color32/Visuals/hue changes.
MS rust: forbid(unsafe_code) kept, non-test me unwrap/expect/panic/todo MANA. Commit locally (`feat(gui): ...`), PUSH MANA.

Gap (checks/gui-toolbar-ch/REPORT.md section 5 loop): App toolbar row (app.rs:~5096-5118)
text-only hai — "Swap Sides" / "Reload / Recompare" me koi glyph/role nahi. views.rs me
glyph+role contract hai (`TOOLBAR_*_GLYPH`, `toolbar_*_label`, `toolbar_button_role`) par koi
live paint call-site nahi — `toolbar_reload_label` (aur 5 siblings) par `#[allow(dead_code)]` hai.

Adoption (ponytail, smallest wiring diff, zero new assets/tokens):
- `ToolbarAction` enum (`None` default / `Swap` / `Reload`) — `SessionAction` precedent
  (paint emit, caller mutate; App hit par swap/run agle wave me chalata hai).
- `paint_session_toolbar(ui, enabled) -> ToolbarAction` — labels `toolbar_swap_label()` /
  `toolbar_reload_label()`, typo `toolbar_button_role().rich(..)`, floor
  `session_toolbar_button_size()` min_size (120x28), rhythm `TIGHT_GAP`
  (App `toolbar_item_gap` mirror, same value — views side se `TIGHT_GAP` reuse).
- 5 `#[allow(dead_code)]` hatao (2 glyph const + role + 2 labels — paint fn live call-site hai);
  `toolbar_icon_gap` ka allow rehta hai (glyph label string ke andar hai, alag icon widget nahi);
  naye paint items (enum + fn) par `SessionAction` precedent wala allow (App wiring agle wave).
  Net dead_code: 42 -> 39. Geometry unchanged (koi naya float literal/size nahi).

- [ ] G1: Reload label ka live call-site views.rs me, role + floor reuse, geometry same
  CHECK: grep -n "paint_session_toolbar\|ToolbarAction\|toolbar_reload_label\|toolbar_button_role\|session_toolbar_button_size" crates/gui/src/views.rs
  EXPECT: paint fn dono labels + role + floor se bandha hai; `egui::Button::new(..rich(..)).min_size(vec2(tool_w, tool_h))`
  shape (App row mirror, sirf label/typo icon-led); `enabled` par `add_enabled` (App `toolbar_active` mirror)

- [ ] G2: Headless tests lock, full suite green
  CHECK: cargo test -p autocompare-gui toolbar_ + cargo test -p autocompare-gui 2>&1 | tail -2
  EXPECT: naya `toolbar_action_defaults_to_none_with_swap_and_reload` green + existing parity test green, full suite 0 failed
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: zero warnings (dead_code allows: 5 hatao, 2 precedent wale lagao — net 42 -> 39)
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: clean

- [ ] G3: Harness PASS (paint output unchanged — App abhi purana row paint karta hai, isliye trivially stable)
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-toolwire-cl/round-1/
  EXPECT: exit 0, geo-typo-report.md contains RESULT: PASS (max 6 rounds, same round dir on re-run)

- [ ] G4: Hygiene — owned files only, zero banned patterns, local commit NO push
  CHECK (BEFORE test/gate): git status --short
  EXPECT: sirf `M crates/gui/src/views.rs` + `?? checks/gui-toolwire-cl/`
  CHECK: grep -n "unwrap\|expect(\|panic\|todo!" crates/gui/src/views.rs | grep -v test ; echo BANNED:$?
  EXPECT: 0 non-test hits (diff adds none)
  CHECK: git diff --stat; git diff crates/gui/src/views.rs | grep -c "Color32\|Visuals::"
  EXPECT: colors zero new (diff me sirf button-label/typo wiring, koi hue/style nahi)
  CHECK: grep -c "allow(dead_code)" crates/gui/src/views.rs
  EXPECT: 39 (pehle 42 — 5 hatao + 2 precedent wale = net -3, ghata prove)
  CHECK: git status --short STATUS.md checks/opencode-waves/ 2>/dev/null; git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-toolwire-cl/)" || echo OWNS-CLEAN
  EXPECT: STATUS untouched, OWNS-CLEAN
  CHECK: git add crates/gui/src/views.rs checks/gui-toolwire-cl && git commit -m "feat(gui): ..." && git log --oneline -1 && git status --short
  EXPECT: English-only conventional commit, sirf OWNS paths staged, local only, NO push
  CHECK: REPORT checks/gui-toolwire-cl/REPORT.md Hinglish-Roman, Devanagari zero
  EXPECT: grep -P "[\x{0900}-\x{097F}]" REPORT → no hits
