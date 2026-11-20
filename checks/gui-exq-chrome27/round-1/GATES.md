# GATES — exq-chrome27 round-1 (toolbar 12px slots -> 16px icon boxes)

Base: 7bdf2ea. Worktree: /tmp/wt-chrome27.
OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-chrome27/**` ONLY.
NEVER touch `screenshot.rs` / `theme.rs` / `app.rs`. Colors frozen.
Clean-room: reference shots described in own words only — native light
capture shows a top action row where small square marks sit left of short
labels; early-feedback geometry shows the two-row action band with even
vertical rhythm. No copied strings/icons/assets. Tokens reused from
`theme.rs` (`SPACE_*` family already in views via `TIGHT_GAP`/`SECTION_GAP`).

## CHECK: baseline harness on fresh round-1 dir
- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-chrome27 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-chrome27/round-1 crates/gui/src` exits 0; then `cp round-1/after.png round-1/before.png`.
- EXPECT: `after.png` valid PNG magic `89504e470d0a1a0a`, >= 8192B, distinct > 64; report ends `RESULT: PASS`; `before.png` byte-identical.

## CHECK: toolbar icon geometry 12 -> 16, band stable (views.rs ONLY)
- CHECK: in `crates/gui/src/views.rs` ONLY: `TOOLBAR_GLYPH_W/H` 12.0 -> 16.0; docs `(24-16)/2=4` vertical-center parity; `toolbar_glyph_rects` slot-local coords rescaled to 16x16 origin (v0 solid 16x16; v1 twin 16x4 bars at y0/y12; v2 side 4x16 bars at x0/x12; v3 wedge 16x5 + centered 10x6 + centered 4x5; v4/v5 chevron 2x2 cells span 6x10 recentered by +2/+2 in 16 box); `toolbar_glyph_slot` unchanged shape (lead `TIGHT_GAP` 4, centered via `TOOLBAR_GLYPH_H/2`); no button min-size / gap / row-height change.
- EXPECT: suite green; `cargo fmt --check` clean; banned grep empty; colors count unchanged (33); band helpers untouched: `action_bar_row_height` 24, `action_bar_row_gap` 8, band 56.

## CHECK: exquisite re-run PASS + pxdiff recorded
- CHECK: re-run same harness into same round dir; paste `exquisite-report.md`; run `python3 tools/exquisite/pxdiff.py before.png after.png --expect-diff`.
- EXPECT: `RESULT: PASS`; pxdiff output recorded (egui-only paint change; fixture renderer separate path so IDENTICAL is acceptable with reason, DIFFERENT also acceptable if fixture picks up glyph const — either way paste verbatim).

## CHECK: edge fixture + headless test (1200px no h-scroll, band stable)
- CHECK: add `round-1/comparison/toolbar-16-1200.json` (avail 1200: glyph 16, band 56, `needs_hscroll_1200` false, `side_air_1200` 500, pair 200, row_h 24, row_gap 8) + one headless test `chrome27_toolbar16_band1200` proving it (slot 16 box at lead 4 centered, rects span inside 16, band 24+8+24, 1200 no h-scroll).
- EXPECT: test green; fixture `include_str!` path correct; existing `toolbar22_live_glyph_slots` updated to 16 parity (same file, no new file).

## CHECK: MS Rust + touch/scope + colors + fmt
- CHECK: `#![forbid(unsafe_code)]` intact; `awk '/^#\[cfg(test)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE 'unwrap\(|expect\(|panic!|todo!|unreachable!'` empty; no `screenshot.rs`/`theme.rs`/`app.rs` in `git status`; `cargo fmt`; colors `grep -rn 'Color32::from_rgb|Visuals::' crates/gui/src/ | wc -l` stays 33.
- EXPECT: full `autocompare-gui` suite green; `cargo fmt --check` clean; ZERO-BANNED; OWNS-clean (`views.rs` + `checks/gui-exq-chrome27/**` only).
