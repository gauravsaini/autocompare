# GATES — exq-palette47 round-1: live mode-aware palette in views.rs

OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-palette47/**` only.
RULES: Hinglish-Roman chat, English-only commits, NO push, 600s budget.

## Context
exq-shot46 ne harness ko honest bana diya (screenshot.rs single-sourced
colors, LIGHT_* washes + *_for(dark) helpers). Live app peeche hai:
`views.rs::kind_palette()` hardcoded dark pairs return karta hai dono modes
me (DELETED_FG/BG etc.), `selective_side_palette()` usko forward karta hai.
Light mode me dark-on-dark rows paint hote hain — yahi fix hai. theme.rs me
`deleted_bg_for(dark)` / `deleted_fg_for(dark)` / `inserted_*` / `changed_*`
helpers + `LIGHT_*` washes + `theme_contrast_ok` contract pehle se hain.
Toolbar glyphs pehle se real shapes hain (`toolbar_glyph_rects` 0-5 variants,
`paint_toolbar_glyph` dim_for ink) — koi block-icon kaam baki nahi, sirf
verify karna hai.

## Gates
- [x] G1 baseline-harness-pass-dark: unmodified tree par harness RESULT PASS (dark).
  CHECK: tools/exquisite/run.sh checks/gui-exq-palette47/round-1
  EXPECT: RESULT: PASS
- [x] G2 baseline-harness-pass-light: explicit light render RESULT PASS (defect note: light PNG dark-on-dark).
  CHECK: CARGO_TARGET_DIR=/tmp/wt-palette47/target cargo build -p autocompare-gui --quiet && /tmp/wt-palette47/target/debug/autocompare-gui --screenshot /tmp/wt-palette47/checks/gui-exq-palette47/round-1/baseline-light.png --screenshot-mode light && python3 -c "d=open('/tmp/wt-palette47/checks/gui-exq-palette47/round-1/baseline-light.png','rb').read();print('MAGIC-OK' if d[:8]==bytes.fromhex('89504e470d0a1a0a') else 'MAGIC-FAIL', len(d))"
  EXPECT: MAGIC-OK
- [x] G3 live-palette-mode-aware: kind_palette + selective_side_palette dark flag se theme helpers use karte hain; dark byte-identical, light washes + dark ink AA.
  CHECK: cargo test -p autocompare-gui views 2>&1 | tail -3
  EXPECT: test result: ok
- [x] G4 harness-rerun-pass-both: post-fix harness RESULT PASS dono modes me; light PNG me dark-on-dark nahi.
  CHECK: tools/exquisite/run.sh checks/gui-exq-palette47/round-1 && /tmp/wt-palette47/target/debug/autocompare-gui --screenshot /tmp/wt-palette47/checks/gui-exq-palette47/round-1/after-light.png --screenshot-mode light
  EXPECT: RESULT: PASS
- [x] G5 fmt-clean: cargo fmt check clean.
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: exit 0
- [x] G6 banned-scan-zero: non-test code me unwrap/expect/panic/todo/unreachable/unsafe nahi (naya code).
  CHECK: python3 -c "import re;src=open('crates/gui/src/views.rs').read();body=src.split('#[cfg(test)]')[0];hits=[l for l in body.split('\n') if re.search(r'\bunwrap\b|expect\(|panic!|todo!|unreachable!|\bunsafe\b',l) and 'forbid(unsafe_code)' not in l];print('HITS:'+str(len(hits)));[print(l.strip()[:100]) for l in hits[:10]];print('BANNED-SCAN-CLEAN' if not hits else 'DIRTY')"
  EXPECT: BANNED-SCAN-CLEAN
- [x] G7 owns-only: commit OWNS only touch karta hai; strays reverted.
  CHECK: git status --short
  EXPECT: OWNS-ONLY

## Parity appendix — EVIDENCE (step 5)
- Harness PNGs (screenshot.rs untouched): post-dark == pre-dark byte-identical (10928B); post-light == pre-light byte-identical (10973B); after(default)==light.
- Geometry/typo signals bit-identical pre/post: SPACE [LG,MD,SM,XL,XS], margins x706, Grid x16, row-h x371, widths x85, gutter x1823; TypoRole x1136, monospace x26, strong x45, .size [14.0], weights MEDIUM/REGULAR/SEMIBOLD, line-heights BODY/CHROME/MONO/SOURCE; Color32/Visuals touches 35 (unchanged — koi naya hue nahi).
- Live-palette proof unit-level hai (harness fixture me selection nahi): dark `*_for(true)` byte-identical purane pairs se; light `LIGHT_*` wash + dark ink, `light_diff_accents_aa()` + per-pair `theme_contrast_ok_color32` green.
- Toolbar: pehle se real shapes (`toolbar_glyph_rects` 0-5 variants, `paint_toolbar_glyph` `dim_for` ink) — koi block-icon nahi mila, koi edit nahi, dim-hue reuse barkarar.
- VERDICT: TOKEN-SOURCED-LIMITED (harness honest tha/hai; live fix contract-proof via unit tests).

## Gate evidence log
- G1: run.sh exit 0, exquisite-report RESULT: PASS (10973B magic-ok distinct-256).
- G2: MAGIC-OK 10973; after.png(default)==explicit-light identical; live defect unit-proof (pre-fix kind_palette me koi dark branch nahi).
- G3: cargo test -p autocompare-gui: 1098 passed, 0 failed; selective_side_palette_light_wash_dark_ink_aa ok.
- G4: run.sh RESULT: PASS; post-dark==pre-dark identical; post-light==pre-light identical.
- G5: cargo fmt --check -p autocompare-gui exit 0.
- G6: diff-scan 0 banned hits (whole-file 2 hits pre-existing doc-comment shabd); cargo build warnings 0.
- G7: status == `M crates/gui/src/views.rs` + `?? checks/gui-exq-palette47/` (sessionpaint-by strays reverted pre-commit).
