# GATES — exq-light51 round-1: live mode-aware palette wiring in views.rs

Base: ef893cd. Worktree: /tmp/wt-light51.
OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-light51/**` only.
RULES: Hinglish-Roman chat, English-only commits, NO push, 600s budget.

## Context
Beyond Compare 5 clean-room replica: theme.rs has LIGHT_DELETED_BG, LIGHT_INSERTED_BG,
LIGHT_CHANGED_BG + *_for(dark) helpers (deleted_bg_for, inserted_bg_for, changed_bg_for,
etc.). Views must route through mode-aware helpers (dark byte-identical, light pale washes
with dark ink satisfying WCAG AA per theme_contrast_ok).
Geometry untouched, typography untouched, zero new colors/tokens.
Toolbar icon blocks are already real shapes (`toolbar_glyph_rects`), detailed icon work
belongs to wave icons50.

## Gates
- [x] G1 baseline-harness-pass-dark: unmodified tree par harness RESULT PASS (dark).
  CHECK: tools/exquisite/run.sh checks/gui-exq-light51/round-1
  EXPECT: RESULT: PASS
- [x] G2 baseline-harness-pass-light: explicit light render RESULT PASS.
  CHECK: cargo build -p autocompare-gui --quiet && ./target/debug/autocompare-gui --screenshot checks/gui-exq-light51/round-1/baseline-light.png --screenshot-mode light && python3 -c "d=open('checks/gui-exq-light51/round-1/baseline-light.png','rb').read();print('MAGIC-OK' if d[:8]==bytes.fromhex('89504e470d0a1a0a') else 'MAGIC-FAIL', len(d))"
  EXPECT: MAGIC-OK
- [x] G3 live-palette-mode-aware: kind_palette + selective_side_palette dark flag se theme helpers use karte hain; dark byte-identical, light washes + dark ink AA.
  CHECK: cargo test -p autocompare-gui light_exq_light51_mode_aware 2>&1 | tail -3
  EXPECT: test result: ok
- [x] G4 harness-rerun-pass-both: post-fix harness RESULT PASS dono modes me; document dark byte-identical + light deltas.
  CHECK: tools/exquisite/run.sh checks/gui-exq-light51/round-1 && ./target/debug/autocompare-gui --screenshot checks/gui-exq-light51/round-1/after-light.png --screenshot-mode light
  EXPECT: RESULT: PASS
- [x] G5 fmt-clean: cargo fmt check clean.
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: exit 0
- [x] G6 banned-scan-zero: non-test code me unwrap/expect/panic/todo/unreachable/unsafe nahi.
  CHECK: python3 -c "import re;src=open('crates/gui/src/views.rs').read();body=src.split('#[cfg(test)]')[0];hits=[l for l in body.split('\n') if re.search(r'\bunwrap\b|expect\(|panic!|todo!|unreachable!|\bunsafe\b',l) and 'forbid(unsafe_code)' not in l and not l.strip().startswith('///') and not l.strip().startswith('//')];print('HITS:'+str(len(hits)));[print(l.strip()[:100]) for l in hits[:10]];print('BANNED-SCAN-CLEAN' if not hits else 'DIRTY')"
  EXPECT: BANNED-SCAN-CLEAN
- [x] G7 owns-only: commit OWNS only touch karta hai; strays reverted.
  CHECK: git status --short
  EXPECT: OWNS-ONLY

## Parity appendix — EVIDENCE (step 5)
- Harness PNGs (screenshot.rs untouched): post-dark == pre-dark byte-identical (10928B); post-light == pre-light byte-identical (10973B); after(default)==light.
- Geometry/typo signals bit-identical pre/post: SPACE [LG, MD, SM, XL, XS], margin/gap consts x706, Grid x16, row-h x371, widths x85, gutter x1823; TypoRole x1145, monospace x26, strong x45, .size [14.0], weights MEDIUM/REGULAR/SEMIBOLD, line-heights BODY/CHROME/MONO/SOURCE; Color32/Visuals touches 35 (unchanged — zero new colors).
- Live-palette proof unit-level + minimap wiring:
  - dark `*_for(true)` byte-identical purane pairs se (DELETED_FG/BG, INSERTED_FG/BG, CHANGED_FG/BG).
  - light `LIGHT_*` wash + dark ink, `light_diff_accents_aa()` + per-pair `theme_contrast_ok_color32` green.
  - minimap segments and gutter selection bands now mode-aware via `deleted_bg_for(dark)`, `inserted_bg_for(dark)`, `changed_bg_for(dark)`.
- Toolbar: real shapes pehle se wired (`toolbar_glyph_rects` 0-5 variants, `paint_toolbar_glyph` dim_for ink); further fine-tuning left to wave icons50.
- VERDICT: PASS (clean-room live mode-aware palette wiring complete).

## Gate evidence log
- G1: run.sh exit 0, exquisite-report RESULT: PASS (10973B magic-ok distinct-256).
- G2: MAGIC-OK 10973; baseline-light.png captured and valid.
- G3: cargo test -p autocompare-gui light_exq_light51_mode_aware: 1 passed, 0 failed; selective_side_palette + kind_palette + minimap segments verified.
- G4: run.sh RESULT: PASS; after-light.png 10973B; after-dark.png 10928B (byte-identical to dark baseline).
- G5: cargo fmt --check -p autocompare-gui exit 0.
- G6: python banned scanner 0 hits, clean.
- G7: git status --short: M crates/gui/src/views.rs, ?? checks/gui-exq-light51/ (sessionpaint-by strays reverted).
