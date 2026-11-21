# GATES — exq-icons50 round-1 (vector toolbar icons + active tab pill + softer light accent, app.rs ONLY)

Base: ef893cd. Worktree: /tmp/wt-icons50.
OWNS: `crates/gui/src/app.rs` + `checks/gui-exq-icons50/**` ONLY (no other crates/gui files, no other checks dirs).
Clean-room: Lucide-style glyphs drawn with egui vector primitives (circle/line/path), zero new colors, zero new sizes.

Scope: app.rs me vector toolbar-icon geometry + active-tab pill rect math + softer light-mode
inline accent (existing LIGHT_* helpers reuse) + egui painter helpers + unit tests.

## Baseline (pre-edit, recorded BEFORE implementing)
- Harness: `tools/exquisite/run.sh checks/gui-exq-icons50/round-1` → `RESULT: PASS`.
- Report: PNG 10973 bytes (min 8192 ok), magic ok, distinct 256 (ok); COLORS 35.

## Gates (CHECK → EXPECT; all must hold post-edit)

- G1 harness green
  CHECK: `tools/exquisite/run.sh checks/gui-exq-icons50/round-1` exits 0
  EXPECT: `exquisite-report.md` ends with `RESULT: PASS` (PNG >= 8192B, magic ok, non-blank)
- G2 dark+light PNGs render
  CHECK: `cargo run -q -p autocompare-gui -- --screenshot $ROUND/dark.png --screenshot-mode dark` exits 0 (plus default light after.png)
  EXPECT: both PNGs exist, >= 8192B, magic ok; byte delta doc confined to icon rasterization (fixture paint FROZEN in screenshot.rs → expect IDENTICAL, delta doc me honest record)
- G3 vector icon geometry (no fill_rect, no new tokens)
  CHECK: `cargo test -p autocompare-gui icons_exq_icons50` passes
  EXPECT: ok — mark box 12.0 via SPACE_MD, ring/bars/rails/wedge segments pure + in-box, kinds cycle mirrors 4 fixture variants
- G4 active tab pill (rounded rect, existing SPACE tokens, no new sizes)
  CHECK: `cargo test -p autocompare-gui toolbar_exq_icons50` passes
  EXPECT: ok — pill rect = tab rect inset SPACE_XS/2, rounding = SPACE_XS, pill inside tab, height unchanged
- G5 softer light accent reuses existing LIGHT_* helpers, zero new Color32
  CHECK: `cargo test -p autocompare-gui icons_exq_icons50_light_accent` passes
  EXPECT: ok — accent == changed_bg_for(false) == LIGHT_CHANGED_BG (wash, harsh ink nahi)
- G6 full suites green
  CHECK: `cargo test -p autocompare-gui icons` + `cargo test -p autocompare-gui toolbar` exit 0
  EXPECT: 0 failures both
- G7 fmt clean
  CHECK: `cargo fmt --check` exits 0
  EXPECT: clean
- G8 banned scan 0 on non-test code
  CHECK: `git diff -U0 -- crates/gui/src/app.rs | grep '^+' | grep -v '^+++' | grep -cE 'unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe'` equals 0
  EXPECT: 0
- G9 colors frozen (documented softer light accent除外, no new palette tokens)
  CHECK: `rg -c 'Color32::from_rgb|Visuals::' crates/gui/src/*.rs` total vs baseline
  EXPECT: total unchanged (helpers take color params / reuse theme fns; zero new Color32::from_rgb)

## Post-edit results
- G1 harness re-run: `RESULT: PASS` — PNG 10973 bytes, magic ok, distinct 256; COLORS 35 (unchanged).
- G2 dark+light: light after.png 10973B + dark /tmp/icons50-dark.png 10928B, dono magic ok, distinct 256.
  Byte delta: `pxdiff.py before.png after.png` → `diff_pixels: 0 of 576000 / VERDICT: IDENTICAL`.
  Honest reason: fixture toolbar paint `screenshot.rs` me hai (dusre leaf ka OWNS, FROZEN) —
  is wave ka vector layer `app.rs` me hai (geometry + painter helpers + tests); raster adoption
  agle wave me views/screenshot owner karega. Isliye delta zero = confined, koi stray raster change nahi.
- G3/G4/G5: `icons_exq_icons50_vector_shapes`, `icons_exq_icons50_toolbar_mark_geometry`,
  `toolbar_exq_icons50_tab_pill_rect`, `icons_exq_icons50_light_accent_reuses_tokens` — sab pass.
- G6: `cargo test -p autocompare-gui` → 1103 passed, 0 failed; icons suite 5 pass, toolbar suite 29 pass.
- G7: `cargo fmt -p autocompare-gui -- --check` clean (workspace-wide sessions diffs pre-existing, untouched).
- G8: added non-test lines banned-token count 0 (unwrap/expect/panic/todo/unreachable/unsafe).
- G9: Color32/Visuals touches 35 = baseline 35 (2 main + 2 app + 27 theme + 4 views); zero naya Color32.
