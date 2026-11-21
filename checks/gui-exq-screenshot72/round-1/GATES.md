# GATES — exq-screenshot72 round-1: fixture light-wash parity in screenshot.rs

Base: 24795e2. Worktree: /tmp/wt-screenshot72.
OWNS: `crates/gui/src/screenshot.rs` + `checks/gui-exq-screenshot72/**` only.
RULES: Hinglish-Roman chat, English-only commits, NO push, 600s budget.
CLEAN-ROOM: no copied assets. Geometry FROZEN, typo FROZEN.
Colors FROZEN except routing via existing theme `*_for()` helpers — zero new Color32, zero new sizes.

## Context
shot46 wired FixturePalette via theme helpers; light56 softened the light
inline accent. Audit: every fixture hue already routes via `fixture_palette()`
(zero `Color32::from_rgb`, zero wash literals in screenshot.rs), but the
shot46 light test pins fields back to the SAME helpers the impl uses
(tautological — a helper drift would stay green). Tightening: pin every
Light `FixturePalette` field directly to the `LIGHT_*` consts
(`LIGHT_BG_RGB`, `LIGHT_FG_RGB`, `LIGHT_DIM`, `LIGHT_TRACK`,
`LIGHT_DELETED_BG`, `LIGHT_INSERTED_BG`, `LIGHT_CHANGED_BG`,
`LIGHT_CHANGED_FG`, `LIGHT_RED`, `LIGHT_GREEN`) so the test proves
value-identity with live `LIGHT_*`, not self-consistency. Prod routing
unchanged if already identical (ponytail: smallest diff); any mismatch
found gets routed via existing `*_for()` only.

## Gates
- [x] G1 baseline-harness-pass: unmodified tree harness RESULT PASS.
  CHECK: tools/exquisite/run.sh checks/gui-exq-screenshot72/round-1
  EXPECT: RESULT: PASS
- [x] G2 light-parity-test-green: new screenshot72 light-wash const-pinning test passes.
  CHECK: cargo test -p autocompare-gui screenshot72 2>&1 | tail -5
  EXPECT: test result: ok
- [x] G3 full-suite-green: full gui suite green.
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: test result: ok
- [x] G4 fmt-clean: cargo fmt check clean.
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: exit 0
- [x] G5 banned-scan-zero: non-test code has no unwrap/expect/panic/todo/unreachable/unsafe; zero new Color32::from_rgb in screenshot.rs.
  CHECK: python3 -c "import re;src=open('crates/gui/src/screenshot.rs').read().split('#[cfg(test)]')[0];hits=[l for l in src.split('\n') if re.search(r'\bunwrap\b|expect\(|panic!|todo!|unreachable!|\bunsafe\b',l) and not l.strip().startswith('/')];rgb=src.count('Color32::from_rgb');print('BANNED-SCAN-CLEAN' if not hits and rgb==0 else 'DIRTY:'+str(len(hits))+' RGB:'+str(rgb))"
  EXPECT: BANNED-SCAN-CLEAN
- [x] G6 harness-rerun-pass: post-fix harness PASS, dark byte-stable, light delta documented.
  CHECK: tools/exquisite/run.sh checks/gui-exq-screenshot72/round-1
  EXPECT: RESULT: PASS
- [ ] G7 owns-only-commit: commit touches OWNS only, NO push.
  CHECK: git status --short
  EXPECT: OWNS-ONLY

## Parity appendix — EVIDENCE (step 5)
- Dark byte-stable: baseline-dark.png e75216dc (== light56 baseline-dark, untouched dark path).
- Light byte-identical pre/post: after.png 1c982b7e == baseline-light.png (test-only diff, zero render change).
- Note: run.sh default `after.png` renders the Light arm (binary defaults Light without
  `--screenshot-mode` unless dark_ref env seam fires — app.rs:1771, OWNS bahar, read-only note).
- Geometry FROZEN: SPACE [LG,MD,SM,XL,XS], margin/gap x743, Grid x16, row-h x375, widths x85,
  gutter x1989 — bit-identical pre/post; zero const changes.
- Typo FROZEN: TypoRole x1183, monospace x26, strong x45, .size [14.0],
  weights MEDIUM/REGULAR/SEMIBOLD, line-heights BODY/CHROME/MONO/SOURCE — unchanged.
- Colors frozen: Color32/Visuals touches 35 pre/post; screenshot.rs non-test has zero
  `Color32::from_rgb` (all hues via `fixture_palette()` -> existing `*_for()` + `palette()`).
- Kittest (headless proof, no window): `screenshot::tests::screenshot72_light_wash_matches_live_light_consts`
  pins all 11 Light palette fields to `LIGHT_*` consts; plus existing
  `shot46_palette_traces_to_live_theme_helpers`, `shot46_light_palette_meets_aa_on_light_wash`,
  `light56_*` (3) — full suite 1137 passed, 0 failed.
- Edge fixture: none added this wave (ponytail) — narrow-720 fallback route already covers
  the edge geometry and light wash is covered by the const-pinning kittest above.

## Gate evidence log
- G1: run.sh exit 0, RESULT: PASS (10946B magic-ok distinct-256).
- G2: `screenshot72_light_wash_matches_live_light_consts ... ok` (1 passed, 1136 filtered).
- G3: cargo test -p autocompare-gui: 1137 passed, 0 failed (+2 doc/integration binaries ok).
- G4: cargo fmt --check -p autocompare-gui exit 0.
- G5: banned scanner (non-test split): BANNED-SCAN-CLEAN (0 hits, 0 from_rgb).
- G6: run.sh post-fix RESULT: PASS (10946B 1c982b7e == baseline-light; geometry/typo/color signals identical).
- G7: (pending commit)
