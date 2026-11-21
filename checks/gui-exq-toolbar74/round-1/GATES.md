# GATES — exq-toolbar74 round-1: toolbar wiring in app.rs ONLY

OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-toolbar74/**`.
NO-TOUCH: views.rs / home.rs / theme.rs / main.rs / screenshot.rs / baaki crates + STATUS.md.
Colors FROZEN (koi naya hue nahi, sirf existing `theme::*_for` reuse).
Geometry FROZEN (koi naya size/float nahi, sirf existing tokens reuse).
MS rust: forbid(unsafe) kept, non-test me unwrap/expect/panic/todo MANA. Commit local, NO PUSH.

Gap: views.rs `paint_toolbar_glyph` live ink `dim_for(dark)` (views.rs:611) rakhta hai —
active/disabled dono ek hi ink. App call-site (`paint_session_action_bar_live`, app.rs:5674)
`enabled` inline `self.tab != Tab::Home` se deta hai, koi named contract nahi. Is wave me
app.rs side contract: `toolbar_active()` + `toolbar_glyph_ink_for(active, dark)` —
active → `dim_for` (live painter byte-identical), disabled → `track_for` (frozen disabled
tone); views consumption agle wave (lightvisual68 unwired-readiness precedent).

- [ ] G1: App toolbar call-site named contract par wired hai
  CHECK: grep -n "toolbar_active\|toolbar_glyph_ink_for" crates/gui/src/app.rs
  EXPECT: `toolbar_active()` Home par false / baaki tabs par true deta hai; `paint_session_action_bar_live` isi se `enabled` banata hai (inline `!=` nahi); `toolbar_glyph_ink_for` active arm `dim_for` + disabled arm `track_for` se bandha hai

- [ ] G2: Headless tests lock, full suite green, fmt clean
  CHECK: cargo test -p autocompare-gui toolbar_ 2>&1 | tail -3
  EXPECT: naye toolbar74 tests green
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -2
  EXPECT: 0 failed
  CHECK: cargo fmt --check -p autocompare-gui && echo FMT-CLEAN
  EXPECT: FMT-CLEAN

- [ ] G3: Banned 0, colors/geometry frozen
  CHECK: grep -n "unwrap\|expect(\|panic!\|todo!" crates/gui/src/app.rs | grep -v "mod tests" | grep -v "^\s*//" ; echo BANNED:$?
  EXPECT: BANNED:1 (zero hits; diff adds none — pre-existing file me bhi zero hone chahiye)
  CHECK: git diff crates/gui/src/app.rs | grep -c "Color32::from_rgb\|Visuals::"
  EXPECT: 0 (koi naya hue/style nahi; `*_for` reuse diff me `dim_for`/`track_for` naam se dikhe)
  CHECK: git diff crates/gui/src/app.rs | grep -cE "^\+\s*[0-9]+\.[0-9]"
  EXPECT: 0 (koi naya float literal nahi)

- [ ] G4: Harness PASS + kittest documented
  CHECK: tools/exquisite/run.sh checks/gui-exq-toolbar74/round-1/ && grep RESULT checks/gui-exq-toolbar74/round-1/exquisite-report.md
  EXPECT: exit 0 + RESULT: PASS
  CHECK: ls checks/gui-exq-toolbar74/round-1/ && grep -c "kittest\|headless" checks/gui-exq-toolbar74/REPORT.md
  EXPECT: REPORT me headless kittest documented

- [ ] G5: Hygiene — OWNS only, local commit NO push, done marker
  CHECK: git status --short
  EXPECT: sirf `M crates/gui/src/app.rs` + `?? checks/gui-exq-toolbar74/`
  CHECK: ls checks/gui-sessionpaint-by/ crates/sessions/ 2>&1; git diff --name-only | grep -E "sessionpaint-by|sessions/" || echo STRAYS-CLEAN
  EXPECT: STRAYS-CLEAN
  CHECK: git add crates/gui/src/app.rs checks/gui-exq-toolbar74 && git commit -m "feat(gui): exq-toolbar74 toolbar wiring" && git log --oneline -1 && git status --short && test -f /tmp/wave-toolbar74.done && echo DONE-MARKED
  EXPECT: English conventional commit, local only (push MANA), done marker maujood
