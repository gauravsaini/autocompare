# GATES tree41 — folder tree round-2 (written BEFORE implementing per $unlazy)

Base: 9e0b1db. Worktree: /tmp/wt-tree41.
OWNS: `crates/gui/src/tree.rs` + `checks/gui-exq-tree41/**` ONLY.
NO-TOUCH: other gui files (`views.rs`, `screenshot.rs`, `viewmodel.rs`, `theme.rs`, `app.rs`, `home.rs`), `crates/sessions/**`, `crates/compare/**`, `crates/diff/**`, `crates/vfs/**`. NEVER push.
COLORS FROZEN: zero Color32 / Visuals additions.
MS Rust: forbid unsafe, no unwrap/expect/panic/todo/unreachable/indexing in non-test code. English-only code comments and commit message.

## Intent (clean-room, own words)
In a high-quality comparative folder tree, deep folder structures need a disciplined geometric rhythm:
1. Indentation progresses across nesting levels by a standardized small spacing quantum (`SPACE_SM`, 8.0px), preserving hierarchy without wasting horizontal space on deep trees.
2. The disclosure expander (the chevron target) maintains a 16x16 square interaction cell that is centered vertically within the row height (Mono line box 15.6px + 8.0px padding = 23.6px), making click targets consistent and predictable.
3. Vertical guide lines track the center of ancestor expander cells, remaining strictly aligned and spaced evenly apart, while status marks and path labels start from a fixed column origin clear of all guide lines.
4. On a wide 1200px canvas, even deep directory nestings fit comfortably without triggering horizontal scrolling, while on a narrow 700px viewport, guide lines remain distinct and uncollapsed.
5. Typography reuses canonical TypoRoles (`Mono` for path content and `Small` for weak/dimmed orphan dashes) with zero new size literals or color overrides.

## Gates & Checks (geometry first, typography second, colors frozen)

### G1: Baseline harness execution
- CHECK: `tools/exquisite/run.sh checks/gui-exq-tree41/round-1` exits 0 and produces `after.png` + `exquisite-report.md`.
- EXPECT: `after.png` valid PNG magic `89504e470d0a1a0a`, size >= 8192B, distinct colors > 64; report ends `RESULT: PASS`.

### G2: Indent rhythm reuses SPACE_SM
- CHECK: `tree41_indent_step()` == `crate::theme::SPACE_SM` (8.0px).
- CHECK: `tree41_indent_for_depth(depth)` == `depth as f32 * 8.0px`. Depths 0..=5 produce 0.0, 8.0, 16.0, 24.0, 32.0, 40.0.
- EXPECT: Pure, saturating-safe, unit-tested.

### G3: Expander hitbox 16px centered
- CHECK: `tree41_expander_w()` == `FOLDER_EXPANDER_W` (16.0px).
- CHECK: `tree41_row_line_height_px()` == `TypoRole::Mono.line_height_px()` (15.6px).
- CHECK: `tree41_row_h()` == 15.6px + `SPACE_SM` (23.6px).
- CHECK: `tree41_expander_box(depth, row_top)` returns a 16x16 rect with `x0 = indent`, `y0 = row_top + (row_h - 16.0) / 2.0`.
- CHECK: `tree41_expander_contains(depth, row_top, x, y)` true only inside the centered box; non-finite floats safely return false.
- EXPECT: Chevron hitbox centered vertically; pure; unit-tested.

### G4: Guide lines alignment and path origin
- CHECK: `tree41_guide_x(level)` == `level * step + expander_w / 2.0` (8.0, 16.0, 24.0, 32.0, 40.0).
- CHECK: `tree41_guide_xs(depth)` returns vector of length `depth` of strictly increasing ancestor guide positions.
- CHECK: `tree41_path_x(depth)` == `indent + expander_w + SPACE_SM` (depth * 8.0 + 24.0px).
- EXPECT: Path and status labels start strictly to the right of all ancestor guides and current expander.

### G5: 1200px no h-scroll + deepnest edge fixture + narrow 700 edge
- CHECK: `tree41_avail_w()` == 1176.0px (1200.0 - 2 * 12.0 outer margins).
- CHECK: `tree41_narrow_avail_w()` == 676.0px (700.0 - 2 * 12.0 outer margins).
- CHECK: `tree41_text_w(depth, name_len)` == `path_x + name_len * FOLDER_MONO_ADVANCE`.
- CHECK: `tree41_needs_hscroll(avail_w, text_w)` returns false for deepnest chain on 1200px canvas.
- CHECK: Fixtures `checks/gui-exq-tree41/round-1/comparison/deepnest-edge.json` and `checks/gui-exq-tree41/comparison/deepnest-edge.json` present and validated via headless test with `include_str!`.
- EXPECT: Deepnest edge fits 1200px without horizontal scrolling; narrow 700px keeps guides distinct.

### G6: Typography TypoRole reuse, no new sizes, colors frozen
- CHECK: `tree41_body_role()` == `TypoRole::Mono` (size 12.0px).
- CHECK: `tree41_dim_role()` == `TypoRole::Small` (size 11.0px).
- CHECK: Zero new `.size(` literals; colors frozen (0 new `Color32` / `Visuals`).

### G7: MS Rust + touch/scope + cargo fmt
- CHECK: `cargo fmt --check` clean.
- CHECK: Zero banned tokens outside tests (`unwrap/expect/panic/todo/unreachable/unsafe`).
- CHECK: Touch ONLY `crates/gui/src/tree.rs` and `checks/gui-exq-tree41/**`.
- CHECK: Re-run `tools/exquisite/run.sh checks/gui-exq-tree41/round-1` reports `RESULT: PASS`.
- EXPECT: All tests green, ready for commit.
