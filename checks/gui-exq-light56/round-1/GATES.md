# GATES — exq-light56 round-1: light-mode contrast polish in screenshot.rs

Base: f31c2ee. Worktree: /tmp/wt-light56.
OWNS: `crates/gui/src/screenshot.rs` + `checks/gui-exq-light56/**` only.
Parallel waves: typo54 OWNS theme.rs, icons55 OWNS app.rs — DO NOT touch.
RULES: Hinglish-Roman chat, English-only commits, NO push, 600s budget.
CLEAN-ROOM: no copied assets. Geometry FROZEN except AA legibility padding if needed (document honestly).
Zero new Color32 beyond existing theme *_for helpers; reuse LIGHT_* tokens.

## Context
light51 wired live mode-aware palette (dark byte-identical, light wash + dark ink).
hygiene52 fixed test-output PNG alpha. screenshot.rs FixturePalette already
routes via theme *_for helpers, but word-level inline accents still fully
invert in Light (dark-ink block + light-wash text) — harsh vs the softer
live precedent (underline-only for non-current spans). Fix: Light paints
softer underline-only accent (dark-ink text stays on light wash row, 1px
fg line at row base, no fill); Dark keeps byte-stable inverted block.
No geometry const changes, no new colors.

## Gates
- [x] G1 baseline-harness-pass-dark: unmodified tree harness RESULT PASS.
  CHECK: tools/exquisite/run.sh checks/gui-exq-light56/round-1
  EXPECT: RESULT: PASS
- [x] G2 baseline-light-render-valid: explicit light render valid PNG.
  CHECK: ./target/debug/autocompare-gui --screenshot checks/gui-exq-light56/round-1/baseline-light.png --screenshot-mode light && python3 -c "d=open('checks/gui-exq-light56/round-1/baseline-light.png','rb').read();print('MAGIC-OK' if d[:8]==bytes.fromhex('89504e470d0a1a0a') else 'MAGIC-FAIL', len(d))"
  EXPECT: MAGIC-OK
- [x] G3 light-accent-contrast-tests-green: new mode-aware contrast tests pass.
  CHECK: cargo test -p autocompare-gui light56 2>&1 | tail -3
  EXPECT: test result: ok
- [x] G4 full-suite-green: full gui suite green.
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: test result: ok
- [x] G5 fmt-clean: cargo fmt check clean.
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: exit 0
- [x] G6 banned-scan-zero: non-test code has no unwrap/expect/panic/todo/unreachable/unsafe.
  CHECK: python3 -c "import re;src=open('crates/gui/src/screenshot.rs').read().split('#[cfg(test)]')[0];hits=[l for l in src.split('\n') if re.search(r'\bunwrap\b|expect\(|panic!|todo!|unreachable!|\bunsafe\b',l) and 'forbid(unsafe_code)' not in l and not l.strip().startswith('///') and not l.strip().startswith('//')];print('BANNED-SCAN-CLEAN' if not hits else 'DIRTY:'+str(len(hits)))"
  EXPECT: BANNED-SCAN-CLEAN
- [x] G7 harness-rerun-pass-both: post-fix harness PASS dark+light, deltas documented.
  CHECK: tools/exquisite/run.sh checks/gui-exq-light56/round-1 && ./target/debug/autocompare-gui --screenshot checks/gui-exq-light56/round-1/after-light.png --screenshot-mode light
  EXPECT: RESULT: PASS
- [x] G8 owns-only: commit touches OWNS only (screenshot.rs + checks/gui-exq-light56/**), strays reverted.
  CHECK: git status --short
  EXPECT: OWNS-ONLY

## Parity appendix — EVIDENCE (step 5)
- Dark byte-identical: after-dark.png 10928B e75216dc == baseline-dark (mode guard restores dark, inverted block path untouched).
- Light delta honest: after-light.png 10946B 1c982b7e vs baseline-light 10973B ff5c187d (27B smaller — 6 accent blocks became 1px underlines, no fill rects).
- Geometry FROZEN: SPACE [LG,MD,SM,XL,XS], margin/gap x717, Grid x16, row-h x371, widths x85, gutter x1942 — bit-identical pre/post; zero const changes, underline sits inside existing row_h.
- Typo FROZEN: TypoRole x1145, monospace x26, strong x45, .size [14.0], weights MEDIUM/REGULAR/SEMIBOLD, line-heights BODY/CHROME/MONO/SOURCE — unchanged.
- Colors frozen: Color32/Visuals touches 35 pre/post (zero new — helper reuses row fg/bg via existing *_for tokens only).
- AA: light changed_fg (105,85,15) on changed_bg wash 6.48, on bg 6.80; dim/track + text/track pairs PASS (5.27/14.09); new tests light56_* 3/3 green.

## Gate evidence log
- G1: run.sh exit 0, exquisite-report RESULT: PASS (10973B magic-ok distinct-256).
- G2: MAGIC-OK 10973 baseline-light.png; baseline-dark.png 10928 e75216dc (explicit --screenshot-mode dark).
- G3: cargo test -p autocompare-gui light56: 3 passed (mode_branch, ink_on_wash_aa, accent_scene_distinct).
- G4: cargo test -p autocompare-gui: 1115 passed, 0 failed.
- G5: cargo fmt --check -p autocompare-gui exit 0 (after cargo fmt).
- G6: python banned scanner (non-test split): BANNED-SCAN-CLEAN.
- G7: run.sh post-fix RESULT: PASS (10946B); after-dark == baseline-dark; after-light 10946B documented delta.
- G8: git status --short: M crates/gui/src/screenshot.rs, ?? checks/gui-exq-light56/ (sessionpaint-by strays reverted).
