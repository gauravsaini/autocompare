# GATES — exq-text30 round-1 (live text typo: compact row 18 -> 20, mono roles)

Base: 7d15e3f. Worktree: /tmp/wt-text30.
OWNS: `crates/gui/src/viewmodel.rs` + `checks/gui-exq-text30/**` ONLY.
NEVER touch `views.rs` / `screenshot.rs` / `theme.rs` / `app.rs`. Colors frozen.
Clean-room: reference look described in own words only — the native light
capture shows airy side-by-side single-color rows with per-side line numbers
and a slim center split; ours renders dark and tight. No copied strings/assets.
Tokens reused from `theme.rs` (`SPACE_LG`+`SPACE_XS`, `TypoRole::Mono/Body/Small`,
`TYPO_LINE_HEIGHT_MONO`, `TYPO_LETTER_SPACING`, `TYPO_WRAP_PANE_CHARS`).

## CHECK: baseline harness on fresh round-1 dir
- CHECK: `tools/exquisite/run.sh checks/gui-exq-text30/round-1` exits 0 on base; then `cp round-1/after.png round-1/before.png`.
- EXPECT: `after.png` valid PNG magic `89504e470d0a1a0a`, >= 8192B, distinct > 64; report ends `RESULT: PASS`; `before.png` byte-identical. DONE base: 11059B PASS.

## CHECK: minimal typo fix (viewmodel.rs ONLY)
- CHECK: in `crates/gui/src/viewmodel.rs` ONLY: `TEXT_ROW_HEIGHT_COMPACT` 18.0 -> `crate::theme::SPACE_LG + crate::theme::SPACE_XS` (= 20.0) with docs updated (compact 20px text / 20px folder alignment); add headless live-text typo section reusing existing roles only — body `TypoRole::Mono` (`content_textcenter` alias), gutter `TypoRole::Mono` (`gutter_text` alias), meta/counts `TypoRole::Small` (`caption` alias), mono line-height `TypoRole::Mono.line_height()` (= 1.30), letter-spacing `TYPO_LETTER_SPACING` (= 0.0), wrap cap `TYPO_WRAP_PANE_CHARS` (80); no new size/token/color, no `unsafe`, no `unwrap/expect/panic/todo/unreachable/indexing` outside `#[cfg(test)]`.
- EXPECT: `Compact.text_row_height()` 20.0 == `SPACE_LG + SPACE_XS`; `text_row_pad` 2.2; mono line-height 1.30 (`12.0 * 1.30 = 15.6`); letter-spacing 0.0; capacities `476/20 = 23`, `326/20 = 16`; full suite green; `cargo fmt --check` clean; banned grep empty; colors count unchanged.

## CHECK: exquisite re-run PASS + pxdiff recorded
- CHECK: re-run same harness into same round dir; paste `exquisite-report.md`; run `python3 tools/exquisite/pxdiff.py before.png after.png --expect-diff` (or record IDENTICAL with reason: headless-only change, fixture renderer separate path).
- EXPECT: `RESULT: PASS`; pxdiff output recorded verbatim.

## CHECK: edge fixture + headless test (narrow700 wrap, zero clip)
- CHECK: add `round-1/comparison/narrow700-wrap.json` (avail 700 below 720: textcenter panes 306/306 after 88 chrome, wrap chars < 80 via `text_pane_char_capacity`, compact/comfortable/spacious rows all above mono max 16.9 line-height, `zero_clip` true) + one headless test `text30_wrap700_zero_clip` proving it via `include_str!` + headless helpers.
- EXPECT: test green; fixture `include_str!` path correct; existing density tests updated to 20px parity in same file (no new file).

## CHECK: MS Rust + touch/scope + colors + fmt
- CHECK: `#![forbid(unsafe_code)]` intact; `awk '/^#\[cfg(test)\]/ {intest=1} !intest' crates/gui/src/viewmodel.rs | grep -nE 'unwrap\(|expect\(|panic!|todo!|unreachable!|\[[0-9a-zA-Z_\"'"'"']+\]'` empty (no banned calls/indexing outside tests); `git status` shows ONLY `crates/gui/src/viewmodel.rs` + `checks/gui-exq-text30/**`; `cargo fmt`; colors `grep -rn 'Color32::from_rgb|Visuals::' crates/gui/src/ | wc -l` unchanged from base.
- EXPECT: full `autocompare-gui` suite green; `cargo fmt --check` clean; ZERO-BANNED; OWNS-clean.
