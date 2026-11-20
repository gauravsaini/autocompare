# REPORT — wave tree31 (live folder tree geometry, round-1)

Scope: Folder tree geometry parity at 1200px wide without horizontal scroll, plus
narrow-700 edge. Headless spec only in `crates/gui/src/tree.rs` (paint stays with
the orchestrator in `views.rs`, untouched).
Geometry FIRST, typography SECOND, colors FROZEN.

## Touched functions (all NEW in `crates/gui/src/tree.rs`, pure non-test helpers)

- `tree31_indent_step` — 8.0px (`SPACE_SM` reuse).
- `tree31_indent_for_depth` — depth * 8.0 (0/8/16/24/32 for 0..4).
- `tree31_row_line_height_px` — `TypoRole::Mono.line_height_px()` (15.6).
- `tree31_row_h` — line box + `SPACE_SM` (23.6, spacious row rhythm).
- `tree31_expander_w` — `FOLDER_EXPANDER_W` parity (16.0).
- `tree31_expander_box` — centered 16x16 hit box (empty box on non-finite input).
- `tree31_expander_contains` — inside-box only, non-finite never hits.
- `tree31_guide_x` / `tree31_guide_xs` — ancestor guides at expander centers.
- `tree31_path_x` — indent + 16.0 + 8.0 label origin, clear of guides.
- `tree31_body_role` / `tree31_dim_role` — Mono / Small, sizes from role tokens.
- `tree31_avail_w` — 1176.0 (1200 minus two 12.0 outer margins).
- `tree31_narrow_avail_w` — 676.0 (700 minus two 12.0 outer margins).
- `tree31_text_w` — path origin + chars * `FOLDER_MONO_ADVANCE`.
- `tree31_needs_hscroll` — finite-only overflow gate.

UNTOUCHED (frozen predecessors intact):
- `views.rs`, `screenshot.rs`, `viewmodel.rs`, `views_table.rs`, `home.rs`,
  `theme.rs` — NEVER touched. Legacy `tree_indent_step` (12.0),
  `FOLDER_ROW_HEIGHT` (24.0), `folder_row_x_offset` unchanged.

TESTS (in `crates/gui/src/tree.rs`, `tree31_tests`):
- `tree31_deepnest_no_overlap` — headless edge proof for `deep-nest-4.json`.

## Evidence

- Harness: `tools/exquisite/run.sh checks/gui-exq-tree31/round-1` exits 0 with `RESULT: PASS`.
- PNG: 11059 bytes, magic ok, non-blank. Colors frozen count unchanged (38).
- Full suite: 1063 passed, 0 failed.
- Edge fixture: `comparison/deep-nest-4.json` (+ round-1 mirror) verified by headless test.
- Zero banned tokens in new non-test code (`unwrap/expect/panic/todo/unreachable/unsafe`;
  `unsafe` appears only in the pre-existing `#![forbid(unsafe_code)]` line).
- `cargo fmt --check` clean for `crates/gui/src/tree.rs`.
