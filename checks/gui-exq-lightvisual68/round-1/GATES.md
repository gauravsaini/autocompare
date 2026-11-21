# GATES — exq-lightvisual68 round-1: light-mode polish in app.rs ONLY

Base: 67ca95d. Worktree: /tmp/wt-lightvisual68.
OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-lightvisual68/**` ONLY.
Disjoint: theme.rs, views.rs, screenshot.rs, tests/, NO other checks dirs.
RULES: Hinglish-Roman chat, English-only commits. Never push. 600s budget.
CLEAN-ROOM: no copied assets. Geometry first, colors FROZEN except routing
via existing theme `*_for()` helpers (zero new Color32, zero new sizes).

## Context
typo54 + typoinst61 landed, toolbar60 vector chevrons landed,
gutter/tree/hex/home geometry landed. Gap: app.rs still uses default egui
Visuals (skill-freeze: Visuals untouched) + weak active-tab affordance.
Diagnosis: views.rs `paint_tab_active_indicator` reuses app.rs-owned
`exq_icons50_inline_accent_for(dark)`; light arm returns LIGHT_CHANGED_BG
wash (255,242,204) on near-white chrome (248,248,248) = ~invisible
underline. Fix (app.rs only): light arm routes `accent_fg_for(false)` =
LIGHT_CHANGED_FG dark-amber ink (AA on light per
`theme::light_diff_accents_aa`, same hue family = softer than markers);
dark arm UNTOUCHED (byte-identical). Plus staged pill-geometry readiness
`exq_lightvisual68_tab_pill_rect_for(tab, dark)`: dark delegates to legacy
rect EXACT (value-identical), light reuses SPACE_XS/2 horizontal +
TEXT_INLINE_SPAN_UNDERLINE_W (1.0) vertical inset — taller light pill,
zero new sizes/colors, unwired (views wiring = later wave, icons52
precedent). Behavior/state unchanged; light tab pixels improve only.

## Gates
- [x] G1 baseline-harness-pass: unmodified tree harness RESULT PASS.
  CHECK: tools/exquisite/run.sh checks/gui-exq-lightvisual68/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: 2026-11-21 exit 0, report RESULT: PASS (PNG 10946B magic ok distinct 256)
- [x] G2 dark-ref-byte-identical: dark fixture PNG sha pre == post change.
  CHECK: ./target/debug/autocompare-gui --screenshot checks/gui-exq-lightvisual68/round-1/dark-ref.png --screenshot-mode dark && python3 -c "import hashlib;print('DARK-SHA:'+hashlib.sha256(open('checks/gui-exq-lightvisual68/round-1/dark-ref.png','rb').read()).hexdigest())"
  EXPECT: DARK-SHA:
  EVIDENCE: PRE e75216dcfa76b230fdd2cc12fbf78008ddf703ae804689a492f994c483614e98 == POST e75216dcfa76b230fdd2cc12fbf78008ddf703ae804689a492f994c483614e98 (identical)
- [x] G3 targeted-tests-green: new lightvisual68 + existing icons50 tests pass.
  CHECK: cargo test -p autocompare-gui lightvisual68 2>&1 | tail -3 && cargo test -p autocompare-gui icons50 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 lightvisual68 2 passed; icons50 4 passed (incl. updated accent test)
- [x] G4 full-suite-green: full gui suite green.
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 lib 1132 passed 0 failed; integration targets ok
- [x] G5 fmt-and-banned-clean: fmt clean + banned tokens 0 in non-test code.
  CHECK: cargo fmt --check -p autocompare-gui && python3 -c "import re;src=open('crates/gui/src/app.rs').read().split('#[cfg(test)]')[0];hits=[l for l in src.split(chr(10)) if re.search(r'\bunwrap\b|expect\(|panic!|todo!|unreachable!|\bunsafe\b',l) and not l.strip().startswith('/')];print('BANNED-SCAN-CLEAN' if not hits else 'DIRTY:'+str(len(hits)))"
  EXPECT: BANNED-SCAN-CLEAN
  EVIDENCE: 2026-11-21 FMT-OK + BANNED-SCAN-CLEAN; colors count 35 pre == post (frozen)
- [x] G6 harness-rerun-plus-kittest: post-change harness PASS + dual-theme kittest documented.
  CHECK: tools/exquisite/run.sh checks/gui-exq-lightvisual68/round-1 && cargo test -p autocompare-gui --test real_egui_shot 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 harness exit 0 RESULT: PASS (PNG 10946B); kittest 2 passed — dark bytes=53798 colors=503 ratio=50, light bytes=54408 colors=549 ratio=54, both 960x600, floors ok (>=8192B, >=500 colors, ratio>=10)
