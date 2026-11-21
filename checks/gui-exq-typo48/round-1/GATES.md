# GATES — exq-typo48 round-1: live font wiring in crates/gui/src/app.rs

OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-typo48/**` only.
RULES: Hinglish-Roman chat, English-only commits, NO push, 600s budget.

## Context
`theme.rs` `TypoRole` has `font_stack()` / `font_id()` intent (`TYPO_MONO_STACK`, `TYPO_UI_SANS_STACK`)
with unit tests, but the live eframe app never installs real fonts — it renders stock egui fonts.
CLEAN-ROOM RULE: no font binaries copied into the repo, no proprietary names embedded beyond system
font-family lookup by generic role (Menlo/monospace stack, system sans stack resolved at runtime, never shipped).
Harness screenshot.rs keeps its 5x7 headless glyphs (harness-only, documented) — your wave is the LIVE app only.
Geometry untouched, colors FROZEN zero touches.

## Gates
- [x] G1 baseline-harness-pass: unmodified tree par harness RESULT PASS baseline.
  CHECK: tools/exquisite/run.sh checks/gui-exq-typo48/round-1
  EXPECT: RESULT: PASS
- [x] G2 live-font-install-plan: pure font-install plan (which egui FontFamily slots map to which TypoRole stacks, system-first fallback chain, zero new sizes/weights/tokens, reuse TYPO_MONO_STACK/TYPO_UI_SANS_STACK strings as single source) + guarded app setup wiring in app.rs.
  CHECK: cargo test -p autocompare-gui typo_exq_typo48 2>&1 | tail -3
  EXPECT: test result: ok
- [x] G3 font-adjacent-tests: all typo + font-adjacent unit test suites pass.
  CHECK: cargo test -p autocompare-gui typo 2>&1 | tail -3
  EXPECT: test result: ok
- [x] G4 harness-rerun-pass: post-fix harness re-run RESULT PASS (dark + light PNGs still render, byte deltas vs before confined to glyph rasterization and documented); plus a runtime log line at startup listing installed families visible in screenshot stderr.
  CHECK: tools/exquisite/run.sh checks/gui-exq-typo48/round-1 && grep -q "installed font families" checks/gui-exq-typo48/round-1/screenshot.out && echo "SCREENSHOT-LOG-OK"
  EXPECT: SCREENSHOT-LOG-OK
- [x] G5 fmt-clean: cargo fmt check clean.
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: exit 0
- [x] G6 banned-scan-zero: non-test code me unwrap/expect/panic/todo/unreachable/unsafe zero (naya code).
  CHECK: python3 -c "import re;src=open('crates/gui/src/app.rs').read();body=src.split('#[cfg(test)]')[0];hits=[l for l in body.split('\n') if re.search(r'\bunwrap\b|expect\(|panic!|todo!|unreachable!|\bunsafe\b',l) and 'forbid(unsafe_code)' not in l];print('HITS:'+str(len(hits)));[print(l.strip()[:100]) for l in hits[:10]];print('BANNED-SCAN-CLEAN' if not hits else 'DIRTY')"
  EXPECT: BANNED-SCAN-CLEAN
- [x] G7 owns-only: commit OWNS only touch karta hai; strays reverted.
  CHECK: git status --short
  EXPECT: OWNS-ONLY

## Parity appendix — EVIDENCE (step 5)
- Harness PNGs (screenshot.rs untouched): dark + light PNGs render, valid PNG magic, >= 8192B.
- Runtime log line at startup listing installed families visible in --screenshot stderr (machine evidence).
- Pure font install plan maps every TypoRole to an installed family with system-first fallback chain.
- Geometry tokens and colors FROZEN (zero new sizes, weights, tokens, or palette edits).
- VERDICT: TOKEN-SOURCED-LIMITED (pure clean-room system font install plan wired in app.rs).
- EVIDENCE round-1: after.png md5 94a35b85b7ef0dc5fd42b4dbcecc00d4 byte-identical vs
  baseline (screenshot.rs untouched, zero raster deltas); after-dark.png 10928B valid PNG;
  screenshot.out + screenshot-dark.out carry the installed-families log line.
