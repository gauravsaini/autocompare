# GATES home45 — Live home cards geometry lock: cards grid geometry reuse OUTER_MARGIN/SECTION_GAP + card stack & CTA reuse SPACE_* + exact bounding boxes + single-source delegation + 1200px no h-scroll (written BEFORE implementing per leaf prompt)

OWNS: crates/gui/src/home.rs, checks/gui-exq-home45/**
NO-TOUCH: views.rs, screenshot.rs, viewmodel.rs, tree.rs, views_table.rs, and all other crates. NEVER push.

Scope: Live Home cards geometry lock wave. Home view owns `home.rs`;
this wave locks the live home cards geometry across 1200px desktop canvas:
multi-column responsive grid reusing OUTER_MARGIN (12px) and SECTION_GAP (8px),
generous 24px card grid gap (GEO_CARD_GAP), 8px card outer padding,
4px inner vertical spacing (GEO_TIGHT_GAP), 28px CTA button hit height (SPACE_XL + SPACE_XS),
card minimum height locked to 99.6px, deterministic card cell (row, col) and bounding box
calculations, CTA bounding box calculations within cards, delegation to existing single-source
helpers so views cannot drift, and zero horizontal scrolling on a 1200px desktop canvas.
Geometry FIRST, typography SECOND, colors FROZEN.

Clean-room (own words): In the reference comparative launcher, session cards provide primary
launch triggers for comparison modalities (Text, Folder, Hex, Merge, Table, Sync, Picture, FolderMerge, Audio).
Each card is a structured vertical tile comprising a strong title, a two-line description, and a full-width
call-to-action button. On a 1200px desktop viewport, the cards arrange in 3 uniform columns separated by 24px
generous grid gaps and bounded by 12px outer margins, flexing card widths to 376px with 360px inner content
width. The card stack maintains an exact 99.6px minimum height floor, ensuring harmonious visual cadence across
all 9 session cards with zero clipping or horizontal scrolling.

## CHECK 1: baseline harness runs on round-1 dir
- CHECK: `tools/exquisite/run.sh checks/gui-exq-home45/round-1` exits 0 and produces `after.png` (>= 8192 bytes, valid PNG magic, non-blank) + `exquisite-report.md` with `RESULT: PASS`.
- EXPECT: PNG valid and non-blank; report `RESULT: PASS`.

## CHECK 2: live home cards grid geometry reusing OUTER_MARGIN and SECTION_GAP
- CHECK: `HOME45_CANVAS_W` is 1200.0px; available width `home45_content_avail_width(1200.0)` is 1176.0px (canvas minus two 12.0px `GEO_OUTER_MARGIN` margins).
- CHECK: margins reuse `crate::theme::GEO_OUTER_MARGIN` (12.0px = `SPACE_MD`) for horizontal and vertical content boundaries.
- CHECK: section gap reuses `crate::theme::GEO_SECTION_GAP` (8.0px = `SPACE_SM`).
- CHECK: card padding reuses `crate::theme::GEO_SECTION_GAP` (8.0px), card grid gap reuses `crate::theme::GEO_CARD_GAP` (24.0px = `SPACE_XL`).
- CHECK: card min width is 240.0px (`10 * SPACE_XL`), 3 columns at 1200px fit cleanly (`3 * 240 + 2 * 24 = 768 <= 1176`).
- CHECK: flexed card width at 1200px is 376.0px (`(1176 - 48) / 3`), inner width is 360.0px, card CTA width is 360.0px (min 224.0px).
- EXPECT: pure, total, and headless-tested functions.

## CHECK 3: card stack height and CTA geometry reusing SPACE_*
- CHECK: `HOME45_CARD_BUTTON_MIN_H` is 28.0px, derived strictly from `crate::theme::SPACE_XL + crate::theme::SPACE_XS` (24.0 + 4.0 = 28.0px), matching `CHROME_HEADER_BAR_H`.
- CHECK: card inner gap reuses `crate::theme::GEO_TIGHT_GAP` (4.0px = `SPACE_XS`).
- CHECK: `home45_card_min_height()` returns exactly 99.6px, delegating to `home_card_min_height()`.
- CHECK: `home45_card_cta_hit_area(240.0)` is at least 6272.0 px² (224.0 * 28.0), and 10080.0 px² at 1200px (360.0 * 28.0).
- EXPECT: zero magic constants; strict SPACE token derivation.

## CHECK 4: card cell and bounding box geometry in grid
- CHECK: `home45_card_cell(index, 1200.0)` maps all 9 items to (row, col) with 3 columns (indices 0..2 in row 0, 3..5 in row 1, 6..8 in row 2).
- CHECK: `home45_card_rect(index, 1200.0)` returns valid non-overlapping `Some((x0, y0, x1, y1))` bounding boxes for all 9 cards.
- CHECK: `home45_card_cta_rect(card_rect)` returns valid bottom-aligned CTA bounding box within the card bounds.
- CHECK: all cards fit within `home45_content_avail_width(1200.0)` with content height matching `home45_grid_content_height(9, 1200.0)` (346.8px).
- EXPECT: exact geometric containment and zero card overlap.

## CHECK 5: 1200px no horizontal scroll & single-source delegation
- CHECK: `home45_grid_needs_hscroll(1200.0)` returns `false`.
- CHECK: `home45_content_avail_width` delegates to `home32_content_avail_width`.
- CHECK: `home45_grid_columns` delegates to `home32_grid_columns`.
- CHECK: `home45_grid_rows` delegates to `home32_grid_rows`.
- CHECK: `home45_grid_card_width` delegates to `home32_grid_card_width`.
- CHECK: `home45_card_inner_width` delegates to `home32_card_inner_width`.
- CHECK: `home45_card_cta_width` delegates to `home32_card_cta_width`.
- CHECK: `home45_grid_content_height` delegates to `home32_grid_content_height`.
- EXPECT: zero drift across views; single-source delegation verified.

## CHECK 6: edge live cards fixture + headless test
- CHECK: fixture `checks/gui-exq-home45/comparison/home45-live-cards-1200.json` and round-1 copy specify 9 cards, 1200px canvas, 3 columns, 99.6px card min height, 28px CTA button height, card bounding boxes, and `grid_needs_hscroll_1200: false`.
- CHECK: headless test `home_exq_home45_live_home_cards` in `crates/gui/src/home.rs` loads fixture via `include_str!` and verifies all assertions and geometry tokens.
- EXPECT: test runs and passes cleanly in `cargo test -p autocompare-gui home`.

## CHECK 7: MS Rust + touch/scope + fmt + banned patterns
- CHECK: `#![forbid(unsafe_code)]` holds; zero `unwrap/expect/panic/todo` in non-test code.
- CHECK: touch ONLY `crates/gui/src/home.rs` and `checks/gui-exq-home45/**`.
- CHECK: `cargo fmt --check` passes cleanly; zero banned tokens.
- EXPECT: pristine code quality, zero warnings in owned code.
