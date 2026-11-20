# GATES tree31 — folder tree geometry first, typography second (written BEFORE implementing per leaf prompt)

OWNS: crates/gui/src/tree.rs, checks/gui-exq-tree31/**
NO-TOUCH: views.rs, screenshot.rs, viewmodel.rs, views_table.rs, home.rs, theme.rs. NEVER push.
COLORS FROZEN. MS Rust: forbid unsafe, no unwrap/expect/panic/todo/unreachable/indexing in non-test.

Scope: Folder tree geometry parity wave. Live folder tree headless spec in tree.rs only
(paint stays with orchestrator in views.rs). Spacious indent rhythm per depth reusing
SPACE_SM, row height reusing TypoRole::Mono line-height, 16px expander hitbox centered
in row, icon/label column alignment, 1200px canvas with no h-scroll + narrow-700 edge.
Typography SECOND: TypoRole reuse only, no new sizes.
Geometry FIRST, typography SECOND, colors FROZEN.

Clean-room (own words): In the reference comparative tool, the folder pane shows a
roomy indented hierarchy: each nesting level steps right by one small spacing unit,
every row is tall enough to breathe around a single text line, the small disclosure
mark beside each directory sits in a fixed square target centered on its row so it
is easy to hit, status marks and names start from stable column origins so icons and
labels never collide, and a wide window holds even deeply nested names without
sideways scrolling while a narrow window still keeps guides distinct and rows apart.

## CHECK: baseline harness runs on fresh round-1 dir
- CHECK: `tools/exquisite/run.sh checks/gui-exq-tree31/round-1` exits 0 and writes `after.png` + `exquisite-report.md` with `RESULT: PASS`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`.

## CHECK: indent per depth reuses SPACE_SM
- CHECK: `tree31_indent_step()` == `crate::theme::SPACE_SM` (8.0); `tree31_indent_for_depth(d)` == d * 8.0.
- EXPECT: depth 0..4 gives 0/8/16/24/32; pure, saturating-safe, unit-tested.

## CHECK: row height reuses TypoRole::Mono line-height
- CHECK: `tree31_row_line_height_px()` == `TypoRole::Mono.line_height_px()` (15.6); `tree31_row_h()` == line-height + `SPACE_SM` (23.6).
- EXPECT: no new size literal; spacious row box around one mono line; unit-tested.

## CHECK: expander hitbox 16px centered
- CHECK: `tree31_expander_w()` == `FOLDER_EXPANDER_W` (16.0); `tree31_expander_box(depth,row_top)` is a 16x16 rect with x0 = indent, y0 = row_top + (row_h - 16)/2.
- CHECK: `tree31_expander_contains(depth,row_top,x,y)` true only inside box; non-finite never hits.
- EXPECT: chevron target centered vertically in row; pure; unit-tested.

## CHECK: icon + label alignment
- CHECK: `tree31_guide_x(level)` == level * step + 8.0 (expander center); `tree31_guide_xs(depth)` == 0..depth guides strictly increasing.
- CHECK: `tree31_path_x(depth)` == indent + 16.0 + 8.0 (expander cell + SM gap); status/label origins never overlap guides.
- EXPECT: icons and labels share one column origin per depth; unit-tested.

## CHECK: 1200px no h-scroll + narrow700 edge
- CHECK: `tree31_avail_w()` == 1176.0 (1200 minus two 12.0 outer margins); `tree31_narrow_avail_w()` == 676.0 (700 minus same margins).
- CHECK: `tree31_text_w(depth,name_len)` == path_x + len * `FOLDER_MONO_ADVANCE`; `tree31_needs_hscroll(avail,text_w)` false when text fits.
- EXPECT: deep-nest-4 fixture fits 1200 without h-scroll; narrow 700 keeps guides distinct.

## CHECK: typography TypoRole reuse, no new sizes
- CHECK: `tree31_body_role()` == `TypoRole::Mono`, `tree31_dim_role()` == `TypoRole::Small`; `tree31_body_size()` == `TYPO_MONO_SIZE`, dim == `TYPO_SMALL_SIZE`.
- EXPECT: zero `.size(` literals in tree31 code; roles only; unit-tested.

## CHECK: edge deep-nest-4 fixture + headless test
- CHECK: create `checks/gui-exq-tree31/comparison/deep-nest-4.json` and `checks/gui-exq-tree31/round-1/comparison/deep-nest-4.json` (depth-4 chain, 1200 fit + 700 narrow, needs_hscroll false).
- CHECK: headless test `tree31_deepnest_no_overlap` in `tree.rs` verifies indent rhythm, centered expander, guide/path non-overlap, 1200 fit, 700 narrow, TypoRole reuse, fixture parity via `include_str!`.
- EXPECT: test passes in `cargo test -p autocompare-gui`.

## CHECK: MS Rust + touch/scope + colors + fmt
- CHECK: `#![forbid(unsafe_code)]` holds; zero `unwrap/expect/panic/todo/unreachable` in non-test code; no `[i]` indexing in new non-test code (use `.get()`); touch ONLY `crates/gui/src/tree.rs` + `checks/gui-exq-tree31/**`.
- CHECK: `cargo fmt --check` clean; banned-token grep clean; harness re-run `RESULT: PASS`.
- EXPECT: commit `feat(gui): exq-tree31 live folder tree` in /tmp/wt-tree31, hash to /tmp/wave-tree31.done, NEVER push.
