# GATES gutter28 — overview minimap (written BEFORE implementing per leaf prompt)

OWNS: crates/gui/src/screenshot.rs, checks/gui-exq-gutter28/**
NO-TOUCH: views.rs, theme.rs, all other crates. NEVER push.

Scope: fixture-only wave. Live paint owns views.rs/theme.rs; this wave locks
the headless fixture mirror: center gutter between panes + right overview
strip share one live row_y mapping; 720px narrow fallback stays intact.
Geometry FIRST, typography SECOND, colors FROZEN.

Clean-room (own words): the reference tool shows two side-by-side text panes
with a slim middle lane that holds one marker per changed block, plus a thin
right-hand strip that compresses the whole file into a miniature map where
each file row maps to a proportional vertical position. This wave mirrors
that geometry in the headless PNG only: middle lane stays fixed width with
markers on block heads; right strip shows every band (same + diff) inside a
small top/bottom inset so edge bands never clip; row<->y uses the live
mapping helpers, never ad-hoc math.

## CHECK: baseline harness runs on fresh round-1 dir
- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-gutter28 tools/exquisite/run.sh checks/gui-exq-gutter28/round-1 crates/gui/src` exits 0 and writes `after.png` + `exquisite-report.md` with `RESULT: PASS`; then `cp round-1/after.png round-1/before.png`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`; `before.png` byte-identical to `after.png` at baseline.

## CHECK: center gutter intact (20px, hunk-head markers only)
- CHECK: `text_fixture_center_layout()` still `(12, 448, 464, 20, 488, 448)` at 960; `text_center_gutter_width()` 20.0 reused, no new literal; `TEXT_CENTER_MARK` still `=>` centered via `center_row_marker_inset`-style symmetric math; markers only where `row.kind != Equal && textcenter_hunk_head(rows, i)`.
- EXPECT: `cargo test -p autocompare-gui gutter28_center_layout_intact` green; center columns unchanged; equal rows blank.

## CHECK: right overview strip is a padded minimap (live helpers only)
- CHECK: new `paint_overview_strip()` in `screenshot.rs` uses `crate::gutter::gutter_overview_geometry_padded` + `GUTTER_SEGMENT_MIN_H` floor + `GUTTER_MINIMAP_TRACK_PAD` inset (reuse, no new literals); Equal bands paint `fixture_palette().panel`, diff bands reuse frozen `CHANGED/DELETED/INSERTED_BG`; wide (960, `GUTTER_X`/`GUTTER_W`) and narrow (720, `dual_pane_x_positions` gutter) routes call it; accent/wrap/hex routes untouched.
- EXPECT: segments live inside `[pad, height-pad]` inner track; edge bands never clip; harness still `RESULT: PASS`.

## CHECK: row_y mapping via live contracts (no ad-hoc math)
- CHECK: new pure helpers `overview_row_y(row, total, pane_top, pane_h)` via `gutter_padded_y_for_row` + pane_top offset, and `overview_row_at_y(y, total, pane_top, pane_h)` via `gutter_padded_row_at_y` with pane_top offset; both return `None` on bad input, zero `unwrap/expect/panic/indexing`.
- EXPECT: `cargo test -p autocompare-gui gutter28_overview_row_mapping` green; round-trip `row_at_y(row_y(r)) == r` holds for sample rows; `y_for_row` values equal live helper + offset.

## CHECK: 720px narrow intact
- CHECK: `dual_pane_widths(720, 0.50) == (340, 340)`, `dual_pane_x_positions(720, 0.50) == (12, 356, 696)`, `gutter_x + GUTTER_W + 8 == 720`; narrow route still calls `paint_overview_strip` with its own `gutter_x`; existing `narrow_fixture_*` tests green.
- EXPECT: narrow PNG still 720x600 valid; no overlap; text still `clip_to_width` ellipsis.

## CHECK: edge fixture + headless test
- CHECK: add `round-1/comparison/gutter28-overview-720.json` (padded minimap: pad 4, inner track, first/last row y, round-trip rows, narrow gutter_x 696) + headless test `gutter28_overview_edge_fixture` with `include_str!` proving path.
- EXPECT: test green; fixture path correct; tall/narrow numbers mirror live helpers.

## CHECK: MS Rust + touch/scope + colors + fmt
- CHECK: `#![forbid(unsafe_code)]` holds; non-test delta has zero `unwrap/expect/panic/todo/unreachable/[` indexing; GUI stays thin client; `cargo fmt` run; `git status` shows ONLY `crates/gui/src/screenshot.rs` + `checks/gui-exq-gutter28/**`; colors `Color32::from_rgb|Visuals::` count unchanged (info only, must not grow).
- EXPECT: full `autocompare-gui` suite green; `cargo fmt --check` clean; banned-token grep empty.
