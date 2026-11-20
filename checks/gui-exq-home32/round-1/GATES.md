# GATES home32 — Home sessions geometry: session cards grid reuse OUTER_MARGIN/SECTION_GAP + search field height 28px reuse SPACE_* + workspace rows alignment + 1200px no h-scroll (written BEFORE implementing per leaf prompt)

OWNS: crates/gui/src/home.rs, checks/gui-exq-home32/**
NO-TOUCH: views.rs, screenshot.rs, viewmodel.rs, tree.rs, views_table.rs, and all other crates. NEVER push.

Scope: Live Home sessions geometry parity wave. Home view owns `home.rs`;
this wave locks the 1200px wide Home sessions geometry without horizontal scroll:
session cards grid reusing OUTER_MARGIN and SECTION_GAP tokens, search field height locked to 28px
reusing SPACE_* tokens, workspace hierarchy rows aligned with consistent indentation and hit heights,
and zero horizontal scrolling across 1200px canvas.
Geometry FIRST, typography SECOND, colors FROZEN.

Clean-room (own words): In the reference comparative tool, the Home launcher serves as the primary
entry point and workspace organizer. Live session cards represent different comparison modalities
arranged in a multi-column responsive grid bounded by outer margin and section gap spacing.
The workspace sidebar organizes sessions into hierarchically indented rows with predictable item heights
and iconography. Above the sessions list, the quick search field matches the standard 28px header
height derived from base spacing tokens (SPACE_XL + SPACE_XS) for optical harmony.
On a 1200px desktop canvas, the multi-column session cards, workspace tree, and 8-tab session strip
fit comfortably with zero horizontal scrolling or clipping.

## CHECK 1: baseline harness runs on round-1 dir
- CHECK: `tools/exquisite/run.sh checks/gui-exq-home32/round-1` exits 0 and produces `after.png` (>= 8192 bytes, valid PNG magic, non-blank) + `exquisite-report.md` with `RESULT: PASS`.
- EXPECT: PNG valid and non-blank; report `RESULT: PASS`.

## CHECK 2: session cards grid geometry reusing OUTER_MARGIN and SECTION_GAP
- CHECK: `HOME32_CANVAS_W` is 1200.0px; available width `home32_content_avail_width(1200.0)` is 1176.0px (canvas minus two 12.0px `GEO_OUTER_MARGIN` margins).
- CHECK: margins reuse `crate::theme::GEO_OUTER_MARGIN` (12.0px = `SPACE_MD`) for horizontal and vertical content boundaries.
- CHECK: section gap reuses `crate::theme::GEO_SECTION_GAP` (8.0px = `SPACE_SM`).
- CHECK: card padding reuses `crate::theme::GEO_SECTION_GAP` (8.0px), card grid gap reuses `crate::theme::GEO_CARD_GAP` (24.0px = `SPACE_XL`).
- CHECK: card min width is 240.0px (`10 * SPACE_XL`), 3 columns at 1200px fit cleanly (`3 * 240 + 2 * 24 = 768 <= 1176`).
- CHECK: flexed card width at 1200px is 376.0px (`(1176 - 48) / 3`), inner width is 360.0px, card CTA width is 360.0px (min 224.0px).
- EXPECT: pure, total, and headless-tested functions.

## CHECK 3: search field height 28px reusing SPACE_*
- CHECK: `HOME32_SEARCH_H` is 28.0px, derived strictly from `crate::theme::SPACE_XL + crate::theme::SPACE_XS` (24.0 + 4.0 = 28.0px), matching `CHROME_HEADER_BAR_H`.
- CHECK: search field horizontal pad reuses `crate::theme::GEO_SECTION_GAP` (8.0px = `SPACE_SM`).
- CHECK: search field icon slot reuses `crate::theme::SPACE_LG` (16.0px).
- CHECK: `home32_search_height()` returns exactly 28.0px.
- EXPECT: zero magic constants; strict SPACE token derivation.

## CHECK 4: workspace rows alignment
- CHECK: workspace sidebar width `HOME32_WORKSPACE_WIDTH` matches `HOME_CARD_MIN_W` (240.0px).
- CHECK: workspace row height `HOME32_WORKSPACE_ROW_H` is 24.0px (`SPACE_XL` = `SESSION_TREE_ROW_H`).
- CHECK: workspace indent step `HOME32_WORKSPACE_INDENT_STEP` reuses `crate::theme::GEO_TREE_INDENT_STEP` (12.0px = `SPACE_MD`).
- CHECK: workspace row padding `HOME32_WORKSPACE_PAD_X` reuses `crate::theme::GEO_SECTION_GAP` (8.0px).
- CHECK: workspace rows and cards grid share vertical origin margin `HOME32_CONTENT_MARGIN_Y` (12.0px).
- CHECK: full combined layout (sidebar 240px + gap 24px + 3-column cards 768px = 1032px) fits within 1176px available width.
- EXPECT: workspace hierarchy rows align precisely with the cards grid.

## CHECK 5: 1200px no horizontal scroll
- CHECK: `home32_needs_hscroll(1200.0)` returns `false`.
- CHECK: `home32_tab_strip_needs_hscroll(&eight_tabs, 1200.0)` returns `false`.
- CHECK: `home32_workspace_layout_needs_hscroll(1200.0)` returns `false`.
- EXPECT: canvas width 1200px accommodates cards, workspace rows, and 8 tabs without horizontal overflow.

## CHECK 6: edge 8tabs fixture + headless test
- CHECK: fixture `checks/gui-exq-home32/comparison/home32-8tabs-1200.json` and round-1 copy specify 8 tabs, 1200px canvas, 28px search height, workspace alignment, and `needs_hscroll: false`.
- CHECK: headless test in `crates/gui/src/home.rs` loads fixture via `include_str!` and verifies all assertions and geometry tokens.
- EXPECT: test runs and passes cleanly in `cargo test -p autocompare-gui`.

## CHECK 7: MS Rust + touch/scope + fmt + banned patterns
- CHECK: `#![forbid(unsafe_code)]` holds; zero `unwrap/expect/panic/todo` in non-test code.
- CHECK: touch ONLY `crates/gui/src/home.rs` and `checks/gui-exq-home32/**`.
- CHECK: `cargo fmt --check` passes cleanly; zero banned tokens.
- EXPECT: pristine code quality, zero warnings in owned code.
