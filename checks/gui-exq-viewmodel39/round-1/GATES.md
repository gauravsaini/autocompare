# GATES viewmodel39 — live text row rhythm round-2 (written BEFORE implementing per $unlazy)

Base: 2af9e50. Worktree: /tmp/wt-viewmodel39.
OWNS: `crates/gui/src/viewmodel.rs` + `checks/gui-exq-viewmodel39/**` ONLY.
NO-TOUCH: other gui files (`views.rs`, `screenshot.rs`, `theme.rs`, `app.rs`, `gutter.rs`, `tree.rs`, `copy.rs`, `take.rs`), `crates/sessions/**`, `crates/compare/**`, `crates/diff/**`, `crates/vfs/**`. NEVER push.
COLORS FROZEN: zero Color32 / Visuals additions.
MS Rust: forbid unsafe, no unwrap/expect/panic/todo/unreachable/indexing in non-test code. English-only code comments.

## Intent (clean-room, own words)
Twin text panes read as one mono rhythm when every row box clears its
glyph box: the mono line box sets the floor, a small fixed pad opens
breathing room, and the row height is exactly line plus pad. The wide
canvas must hold both panes plus chrome with no sideways scroll; the
narrow 700 window must wrap inside its smaller panes with no clipped
glyphs. This round locks that contract headless: row heights delegate
to TypoRole Mono/Body line boxes plus SPACE_XS/SM pads, and the two
width edges (1200 wide, 700 narrow) prove zero clip/scroll.

## Gates & Checks (geometry first, typography second, colors frozen)

### G1: Baseline harness execution
- CHECK: `tools/exquisite/run.sh checks/gui-exq-viewmodel39/round-1` exits 0 and produces `after.png` + `exquisite-report.md`.
- EXPECT: `after.png` valid PNG magic `89504e470d0a1a0a`, >= 8192B, distinct > 64; report ends `RESULT: PASS`.

### G2: Row heights reuse TypoRole line-height + SPACE_XS/SM pads (viewmodel.rs ONLY)
- CHECK: `TEXT_ROW_HEIGHT_COMFORTABLE` spells `crate::theme::SPACE_XL`; `TEXT_ROW_HEIGHT_SPACIOUS` spells `crate::theme::CHROME_HEADER_BAR_H`; `FOLDER_ROW_HEIGHT_COMPACT` spells `crate::theme::SPACE_LG + crate::theme::SPACE_XS`; `FOLDER_ROW_HEIGHT_COMFORTABLE` spells `crate::theme::SPACE_XL`; `FOLDER_ROW_HEIGHT_SPACIOUS` spells `crate::theme::CHROME_HEADER_BAR_H`; values stay 20.0/24.0/28.0.
- CHECK: new headless section exposes `viewmodel39_text_line_h()` == `TypoRole::Mono.line_height_px()` (15.6), `viewmodel39_folder_line_h()` == `TypoRole::Body.line_height_px()` (18.2), pad helpers reusing `SPACE_XS`/`SPACE_SM`, and `viewmodel39_text_row_h(density)` / `viewmodel39_folder_row_h(density)` delegating to existing density heights; no new size/token/color, no banned calls.
- EXPECT: `cargo test -p autocompare-gui --lib viewmodel::tests::viewmodel39_row_rhythm_reuses_typo_and_space` green; `grep -n SPACE_XL viewmodel.rs` shows token reuse; values unchanged.

### G3: 1200px no h-scroll + narrow700 wrap zero-clip
- CHECK: `viewmodel39_wide1200_no_hscroll()` true via `textcenter_pane_widths(1200, 0.5)` == (556, 556) with 88 chrome fitting exactly, and `viewmodel39_narrow700_wrap_zero_clip()` true via `textcenter_pane_widths(700, 0.5)` == (306, 306), `live_text_wrap_chars_for_width(306, 7.2)` == 42 < 80, all densities above mono-max 16.9 with `row_height_has_clip_risk` false.
- CHECK: fixtures `round-1/comparison/viewmodel39-wide-1200.json` and `round-1/comparison/viewmodel39-narrow-700.json` match the pure helpers via one headless test with `include_str!`.
- EXPECT: headless test green; both fixtures `needs_hscroll: false`, `zero_clip: true`.

### G4: Harness re-run PASS + MS Rust + scope + fmt
- CHECK: re-run `tools/exquisite/run.sh checks/gui-exq-viewmodel39/round-1` reports `RESULT: PASS`.
- CHECK: `cargo test -p autocompare-gui` green; `cargo fmt --check` clean; banned grep outside `#[cfg(test)]` for `unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe` empty; `git status --short` shows ONLY `crates/gui/src/viewmodel.rs` + `checks/gui-exq-viewmodel39/**`; colors count `grep -rn Color32::from_rgb crates/gui/src/viewmodel.rs | wc -l` == 0.
- EXPECT: full suite green; ZERO-BANNED; OWNS-clean; PASS.
