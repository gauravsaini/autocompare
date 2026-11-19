# Wave BA-TYPOTOKENS — GATES (typography token side: TypoRole→FontId single-source)

OWNS: `crates/gui/src/theme.rs` + `checks/gui-typotokens-ba/**` ONLY.
`views.rs` / `views_table.rs` / `screenshot.rs` KO HAATH MAT LAGANA (parallel wave chal rahi hai;
paint call-sites sirf read karke REPORT me EXACT wiring spec likhna — paint calls ko mat badlo).
Colors FROZEN (koi hue nahi — koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi).
Geometry-free wave — typo only (koi width/margin/padding/alignment edit nahi).
NO push (orchestrator pushes). Conventional commit (`feat(gui): ...`).

Scope: `theme.rs` me `TypoRole`/`font_stack` contract poora karo — TypoRole→FontId
resolution helpers (pure, tests samet), proportional stack (UI sans) non-diff UI
ke liye vs mono diff panes; har role ka exact FontId/px/weight mapping single-source
(`TypoSpec`) + REPORT me `views.rs` call-sites ke liye EXACT wiring spec
(kaun si line me kaun sa helper — paint unchanged).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green, `0 failed` (count record karo `round-1/baseline-tests.out` me)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors + geometry untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/theme.rs|checks/gui-typotokens-ba/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|views_table.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/theme.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|add_space|set_width|desired_width)" || echo NO-COLOR-GEO-ADD`
- EXPECT: only `theme.rs` + `checks/gui-typotokens-ba/**`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-GEO-ADD

## G3 — TypoRole→FontId resolution helpers, pure + single-source + unit-tested

- CHECK: `cargo test -p autocompare-gui typo`
- EXPECT: green, kam se kam ye tests:
  + `typo_font_ids_carry_exact_px_and_family` (har role: `font_id().size == size()`,
    Mono → `FontFamily::Monospace`, baaki sab → `Proportional`; `font_stack` twin consistent)
  + `typo_spec_bundles_each_role_single_source` (har role: `spec()` ke har field ka
    matching accessor se barabari; ladder Small < Mono < Body < HomeTitle < Heading intact)
  + `typo_font_id_zoomed_follows_zoom_table` (level 0 == `font_id()`; ±1px steps;
    family unchanged; clamp MIN..MAX; full `ZOOM_LEVELS` band)
  + `typo_resolved_font_id_uses_role_text_style` (`Style::default` par Mono monospace,
    Body proportional; har role `text_style().resolve(style)` ke barabar)
- WHAT: `theme.rs` me naya `FontId resolution` section (sab pure/headless, koi `Ui`/`Context` nahi):
  `font_family()` (const, Mono→Monospace else Proportional),
  `font_id()` (const, `FontId::new(size, family)`),
  `font_id_zoomed(level)` (`zoomed_size` + family),
  `resolved_font_id(style)` (`text_style().resolve(style)` — `views.rs:2290` precedent),
  `TypoSpec` struct + `spec()` (const single-source bundle: role/size/font_id/weight/
  weight_value/line_height/line_height_px/letter_spacing_em/is_tabular).
  Koi naya token size/color nahi — sirf resolution + bundle.

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/theme.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/theme.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -55 crates/gui/src/theme.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code,
  `forbid(unsafe_code)` kept, naye helpers pure fns

## G5 — Harness PASS + PNG magic-ok (paint-unchanged proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-typotokens-ba/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-typotokens-ba/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64 (paint untouched → PNG bit-stable vs baseline)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-typotokens-ba/REPORT.md checks/gui-typotokens-ba/round-1/after.png checks/gui-typotokens-ba/round-1/geo-typo-report.md`
  + `git add crates/gui/src/theme.rs checks/gui-typotokens-ba/GATES.md checks/gui-typotokens-ba/REPORT.md checks/gui-typotokens-ba/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
- REPORT must contain: exact FontId/px/weight mapping table (har role),
  proportional-vs-mono contract, zoom behavior, `views.rs` call-sites EXACT wiring
  spec (line numbers + kaun sa helper, paint unchanged note), gates evidence.
