# GATES — wave theme26 (light theme tokens + screenshot light default, round-1)

OWNS: `crates/gui/src/theme.rs` + `crates/gui/src/app.rs` + `crates/gui/src/main.rs` + `checks/gui-exq-theme26/**` ONLY.
NEVER touch `crates/sessions/*`, `crates/gui/src/views.rs`, `crates/gui/src/screenshot.rs`, never `git add -A`.
Clean-room: describe reference behavior in own words only; never copy strings, icons, assets.
Evidence: `/tmp/bc-capture.png` (reference is light native: white panes, dark ink, subtle gray chrome, blue current-diff wash, yellow changed marker), `/tmp/live-early-feedback.png` (current fixture dark render, geometry reference only — all geometry identical, only palette remaps).
THIS WAVE LIFTS THE COLOR FREEZE (single theme wave per plan): geometry and typography stay byte-identical in layout (same rects, same fonts/sizes) — only the palette remaps. Both fixture shots (dark + light) re-baselined in this one wave: `after.png` is the light shot, `dark-ref.png` keeps the dark reference.

Scope: light-theme tokens in `theme.rs` (`LIGHT_*` mirrors of the fixed diff accents, same names/structure as the existing `DARK_*`/`LIGHT_*` chrome pairs) + `Visuals::light()` wiring in `app.rs`/`main.rs` so `--screenshot` renders light by default (explicit `--screenshot-mode dark` flag + `AUTOCOMPARE_DARK_REF` env keep the dark-ref path). Geometry/typo consts untouched.

## G1 — Baseline green (pre-edit)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-theme26 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-theme26/round-1 crates/gui/src` + `cp round-1/after.png round-1/before.png` (dark baseline) + `python3 tools/exquisite/pxdiff.py before.png after.png`
- EXPECT: harness `RESULT: PASS`; `after.png` magic ok, >= 8192 B, distinct > 64; pxdiff self-compare `VERDICT: IDENTICAL` (proves wiring only).
- EVIDENCE (verbatim, pasted post-run below).

## G2 — OWNS-only diff

- CHECK: `git status --short` + `git diff --name-only`
- EXPECT: only `checks/gui-exq-theme26/**` + `crates/gui/src/theme.rs` + `crates/gui/src/app.rs` + `crates/gui/src/main.rs`; no `screenshot.rs`/`views.rs`/`sessions/*`. REPORT.md lists every touched fn.

## G3 — Palette: LIGHT_* mirrors, reuse-first, AA-gated

- CHECK: `git diff -- crates/gui/src/theme.rs`
- EXPECT: minimal delta, same structure as existing chrome pairs (`*_RGB` + `Color32` + `*_for(dark)` selector):
  + `LIGHT_DELETED_BG_RGB=[255,222,222]`/`LIGHT_DELETED_BG` (ink reuses `LIGHT_RED`) — light-mirror of `DELETED_BG`;
  + `LIGHT_INSERTED_BG_RGB=[214,242,220]`/`LIGHT_INSERTED_BG` (ink reuses `LIGHT_GREEN`) — light-mirror of `INSERTED_BG`;
  + `LIGHT_CHANGED_BG_RGB=[255,242,204]`/`LIGHT_CHANGED_BG` + `LIGHT_CHANGED_FG_RGB=[105,85,15]`/`LIGHT_CHANGED_FG` (dark amber ink, also reused for accent spans) — light-mirror of `CHANGED_BG`/`CHANGED_FG`;
  + `LIGHT_ERROR_BG_RGB=[252,230,230]`/`LIGHT_ERROR_BG` (text reuses `LIGHT_RED`) — light-mirror of `ERROR_BG`;
  + selectors `deleted_bg_for`/`inserted_bg_for`/`changed_bg_for`/`changed_fg_for`/`error_bg_for`/`accent_fg_for` (dark arm returns the existing fixed const, light arm the mirror/reuse) + `light_diff_accents_aa()` gate fn.
  No geometry/typo move: `git diff` shows zero `.size(`/`FontId`/rect-const changes; hues of CHANGED/INSERT/DELETE families stay recognizable on light (red/green/amber washes + dark inks).
- EVIDENCE: pasted `git diff --stat` + fn list into REPORT.md after run.

## G4 — app.rs/main.rs wiring: light default, dark-ref preserved

- CHECK: `git diff -- crates/gui/src/app.rs crates/gui/src/main.rs`
- EXPECT: `App::resolve_screenshot_mode[_with_env]` pure core (explicit `--screenshot-mode` wins via frozen `screenshot_mode_from_args`; else `AUTOCOMPARE_DARK_REF=1/true/dark` forces dark; else light default) + thin env-reading wrapper; `main.rs` `--screenshot` path calls it instead of `screenshot_mode_from_args` directly (exit-2 on Err preserved). Live path (`dark_mode` default, `set_theme`/`set_visuals`) untouched.
- EVIDENCE: fn list + behavior table (no-flag→light, `--screenshot-mode dark`→dark, env→dark) pasted into REPORT.md.

## G5 — Re-run PASS + fixture pxdiff DIFFERENT + dark-ref

- CHECK: re-run harness into same round dir; `python3 tools/exquisite/pxdiff.py before.png after.png`; render dark-ref via `--screenshot-mode dark` to `round-1/dark-ref.png`; `ls -l` all three PNGs.
- EXPECT: harness `RESULT: PASS`; pxdiff `VERDICT: DIFFERENT` with diff bbox covering the panes (palette-only change: same 960x600, same geometry code path); `dark-ref.png` px-identical scene to `before.png`; PNG sizes recorded in REPORT.md.
- EVIDENCE (verbatim post-run): exquisite-report + pxdiff output + `ls -l` sizes.

## G6 — Edge fixture + headless test (light contrast at 720)

- CHECK: `round-1/comparison/theme-contrast-720.json` (720px narrow fallback reuses the identical light palette — width-independent hues; per-pane light text AA ratios recorded: base 17.35, dim 6.49, markers red 6.48/green 5.98/blue 6.91, row pairs deleted 5.49/inserted 5.32/changed 6.48, accent 6.48/6.8, error 5.77; min ratio asserted) + one headless test named `theme26_light_contrast`.
- EXPECT: fixture exists with ratios above (±0.05 rounding); test green; test proves every light text ink ≥ 4.5 on its pane surface.

## G7 — MS Rust + touch/scope + colors

- CHECK: `#![forbid(unsafe_code)]` holds; no `unwrap/expect/panic/todo/unreachable/indexing` in non-test delta; GUI stays thin client; `cargo fmt` on touched files; `git status` shows ONLY the four OWNS paths; NEVER touch `crates/sessions/*`, never `git add -A`; colors `Color32::from_rgb|Visuals::` grows 33 → 38.
- EXPECT: full `autocompare-gui` suite green; `cargo fmt --check` clean; banned-token grep on non-test delta empty; new count 38 with per-addition justification (each addition = light-mirror of a dark token: deleted/inserted/changed/error bg + changed fg; inks otherwise reused from existing `LIGHT_RED`/`LIGHT_GREEN`).
- EVIDENCE: suite tail + fmt output + grep output + count, pasted into REPORT.md.
