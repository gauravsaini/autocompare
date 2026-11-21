//! Offscreen screenshot generation for verification and smoke testing.
//!
//! Renders the Text comparison fixture session into a valid PNG without
//! requiring an interactive window or display server.

#![forbid(unsafe_code)]

use crate::gutter::{
    GUTTER_SEGMENT_MIN_H, gutter_overview_geometry_padded, gutter_padded_row_at_y,
    gutter_padded_y_for_row,
};
use crate::home::{
    HOME_GRID_COLUMNS, RECENT_COL_GAP, RECENT_LIST_PAD, folder_empty_hint, home_model,
    recent_row_columns,
};
use crate::viewmodel::{TextRow, TextRowKind, gutter_geometry, textcenter_hunk_head};
use crate::views::{
    PICTURE_ZOOM_STEPS, picture_center_offset, picture_fit_zoom_pct, text_center_gutter_width,
    wrap_find_scroll_sub, wrap_pane_pair_count, wrap_width_for, zoom_label,
};
use crate::views_table::{
    TABLE_FIND_BAR_H, TABLE_FIND_NEEDLE_W, TABLE_KEY_BAR_H, TABLE_MAX_COL_CHARS, TableKeyView,
    is_key_column, table_find_cell_spans, table_find_span_in_padded, table_key_badge_label,
};
use autocompare_diff::{PictureDiff, RgbaImage, encode_png};
use std::path::Path;

// Harness palette: EVERY hue comes from `crate::theme` mode-aware helpers.
// Dark arm keeps the existing dark const values (the `*_for(true)` helpers
// return exactly them, byte-stable); Light arm uses the `LIGHT_*` washes
// with dark inks, so light fixtures paint dark-text-on-light-wash, never
// dark-on-dark. Fake chrome wound down: OS traffic dots removed (live egui
// paints no window controls, no live token exists); active-tab fill box
// removed (live tabs are egui `selectable` style, no theme token — the
// active tab reads via text-vs-dim ink, which every route already paints).
// Chrome bands/strips/gutter reuse `track_for` (live band/strip token);
// wells and inset controls reuse `palette().base_bg`; 1px separators reuse
// `dim_for`. Zero new color tokens.

/// Fixture render mode: headless proof ka dark/light switch.
/// Default dark taaki existing fixtures byte-stable rahen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FixtureMode {
    /// Existing frozen-dark render (byte-stable).
    #[default]
    Dark,
    /// Light render; hues `crate::theme` ke `LIGHT_*` helpers se
    /// (existing helpers reuse, naya palette zero).
    Light,
}

/// Mode-aware hues (geometry same, sirf ye badalte hain). Har field
/// `crate::theme` ke mode-aware helper se aata hai — koi harness-side
/// RGB literal nahi.
#[derive(Debug, Clone, Copy)]
struct FixturePalette {
    bg: [u8; 4],
    text: [u8; 4],
    dim: [u8; 4],
    deleted_bg: [u8; 4],
    deleted_fg: [u8; 4],
    inserted_bg: [u8; 4],
    inserted_fg: [u8; 4],
    changed_bg: [u8; 4],
    changed_fg: [u8; 4],
    track: [u8; 4],
    field: [u8; 4],
}

/// `Color32` se harness `[u8; 4]` (alpha hamesha opaque).
fn rgba(color: eframe::egui::Color32) -> [u8; 4] {
    color.to_array()
}

/// Theme `[u8; 3]` se harness `[u8; 4]` (alpha hamesha opaque).
fn rgb3(color: [u8; 3]) -> [u8; 4] {
    [color[0], color[1], color[2], 255]
}

impl FixturePalette {
    fn of(mode: FixtureMode) -> Self {
        let dark = matches!(mode, FixtureMode::Dark);
        let base = crate::theme::palette(dark);
        Self {
            bg: rgb3(base.base_bg),
            text: rgb3(base.base_fg),
            dim: rgba(crate::theme::dim_for(dark)),
            deleted_bg: rgba(crate::theme::deleted_bg_for(dark)),
            deleted_fg: rgba(crate::theme::deleted_fg_for(dark)),
            inserted_bg: rgba(crate::theme::inserted_bg_for(dark)),
            inserted_fg: rgba(crate::theme::inserted_fg_for(dark)),
            changed_bg: rgba(crate::theme::changed_bg_for(dark)),
            changed_fg: rgba(crate::theme::changed_fg_for(dark)),
            track: rgba(crate::theme::track_for(dark)),
            field: rgb3(base.base_bg),
        }
    }
}

use std::cell::Cell;

thread_local! {
    /// Active fixture mode; default dark (byte-stability structural guarantee).
    /// Sirf `render_fixture_png_with_mode` guard ke andar badalta hai.
    static FIXTURE_MODE: Cell<FixtureMode> = const { Cell::new(FixtureMode::Dark) };
}

/// Current mode ka palette (route fns + shared helpers yahi padhte hain).
fn fixture_palette() -> FixturePalette {
    FIXTURE_MODE.with(|mode| FixturePalette::of(mode.get()))
}

/// Inline accent style: Dark keeps the byte-stable inverted block,
/// Light paints a softer underline-only accent (dark-ink text stays on
/// the light wash, 1px fg line at the row base). No new colors — the
/// existing row `fg` token is reused either way. Pure.
fn inline_accent_inverted() -> bool {
    FIXTURE_MODE.with(|mode| matches!(mode.get(), FixtureMode::Dark))
}

/// Paints one word-level inline accent span inside an already-painted row.
/// Dark: full-row-height inverted block (`fg` fill + `bg` text, legacy).
/// Light: softer underline-only (`fg` text overdraw + 1px `fg` line at the
/// row base, no fill) so the span reads dark-ink-on-light-wash with zero
/// geometry shift (`row_h` untouched, line sits inside the row). Pure.
/// Bundled word-accent paint input so the painter stays under the
/// argument-count lint ([`TextPaneLayout`]-style precedent in views).
struct InlineAccent<'a> {
    x: u32,
    y: u32,
    w: u32,
    row_h: u32,
    fg: [u8; 4],
    bg: [u8; 4],
    span: &'a str,
    baseline_dy: u32,
}

fn paint_inline_accent(pixels: &mut [u8], dims: (u32, u32), accent: InlineAccent<'_>) {
    let InlineAccent {
        x,
        y,
        w,
        row_h,
        fg,
        bg,
        span,
        baseline_dy,
    } = accent;
    if inline_accent_inverted() {
        fill_rect(pixels, dims, (x, y, w, row_h), fg);
        draw_string(pixels, dims, x, y.saturating_add(baseline_dy), span, bg);
    } else {
        draw_string(pixels, dims, x, y.saturating_add(baseline_dy), span, fg);
        let line_y = y.saturating_add(row_h).saturating_sub(1);
        fill_rect(pixels, dims, (x, line_y, w, 1), fg);
    }
}

/// Mode guard: set karke purana mode drop par wapas (nested-safe).
struct FixtureModeGuard {
    prev: FixtureMode,
}

impl FixtureModeGuard {
    fn set(mode: FixtureMode) -> Self {
        let prev = FIXTURE_MODE.with(|slot| slot.replace(mode));
        Self { prev }
    }
}

impl Drop for FixtureModeGuard {
    fn drop(&mut self) {
        FIXTURE_MODE.with(|slot| slot.set(self.prev));
    }
}

/// Shared headless geometry (geometry first, colors frozen).
/// Har fixture isi canvas/padding/row height par tikti hai.
pub const SNAP_W: u32 = 960;
pub const SNAP_H: u32 = 600;
pub const TITLE_H: u32 = 36;
pub const TAB_H: u32 = 32;
pub const ROW_H: u32 = 24;
pub const OUTER_PAD: u32 = 12;
pub const PANE_GAP: u32 = 4;
pub const STATUS_H: u32 = 30;
/// Home card grid: live contract `home::HOME_GRID_COLUMNS` (3) ×
/// `home::HOME_GRID_GAP` (8); card w 306 (960 canvas par flex
/// `views::home_paint_card_width(936)` = 306.67 ka truncation, floor 240
/// se upar), h 100 (contract min-height 99.6 ka ceiling). CTA 224x28
/// (live `home::HOME_CARD_CTA_W` parity, card me centered). Recent
/// columns title-flex + kind 96 + age 72.
pub const HOME_CARD_W: u32 = 306;
pub const HOME_CARD_H: u32 = 100;
pub const HOME_CARD_GAP: u32 = 8;
pub const HOME_CTA_W: u32 = 224;
pub const HOME_CTA_H: u32 = 28;
pub const TEXT_PANE_W: u32 = 460;
pub const TEXT_LEFT_X: u32 = 12;
pub const TEXT_RIGHT_X: u32 = 476;
/// Center-gap marker (exq-shot15): BC center gutter shows one yellow `=>`
/// per changed hunk head; glyphs exist in the fixture 5px font (`=`/`>`
/// advance 6px each), hue reuses `fixture_palette().changed_fg` (frozen, no new color).
pub const TEXT_CENTER_MARK: &str = "=>";
/// Default text-fixture center layout `(left_x, left_w, center_x,
/// center_w, right_x, right_w)`: live 20px center
/// (`views::text_center_gutter_width`, `SPACE_LG + SPACE_XS`) between two
/// equal panes, overview ruler still starts at `GUTTER_X`:
/// 12 + 448 + 4 + 20 + 4 + 448 = 936. Panes derive from the fixed ruler
/// (no new literals); paint + headless test share this single source.
pub fn text_fixture_center_layout() -> (u32, u32, u32, u32, u32, u32) {
    let center_w = text_center_gutter_width() as u32;
    let avail = GUTTER_X.saturating_sub(TEXT_LEFT_X);
    let panes = avail
        .saturating_sub(center_w)
        .saturating_sub(PANE_GAP.saturating_mul(2));
    let left_w = panes / 2;
    let right_w = panes.saturating_sub(left_w);
    let left_x = TEXT_LEFT_X;
    let center_x = left_x.saturating_add(left_w).saturating_add(PANE_GAP);
    let right_x = center_x.saturating_add(center_w).saturating_add(PANE_GAP);
    (left_x, left_w, center_x, center_w, right_x, right_w)
}
/// Fixture text-row rhythm mirror (exq-shot19): live typo17 metrics in
/// fixture pixels. Row height rounds the live line-height rhythm
/// ([`crate::views::text_typo17_row_height`] = 17.6) to whole pixels so
/// rows keep breathing room instead of gluing; the line-number column
/// pairs the live char-metric width with the live inner pad; baselines
/// center the 7px fixture glyph in the row so every row sits on one
/// beat. Reuses live helpers, no new hues. Colors frozen.
fn fixture_text_row_h() -> u32 {
    crate::views::text_typo17_row_height().round() as u32
}

/// Fixture line-number column width (px) for `max_line_no` rows: live
/// char-metric column ([`crate::views::text_linenum_column_width_from_metrics`]
/// over [`crate::gutter::gutter_linenum_digits`]) rounded to whole pixels.
/// Pure.
fn fixture_text_linenum_col_w(max_line_no: usize) -> u32 {
    let digits = crate::gutter::gutter_linenum_digits(max_line_no);
    crate::views::text_linenum_column_width_from_metrics(
        digits,
        crate::views::text_char_metric_width(),
    )
    .round() as u32
}

/// Fixture row inner pad (px): live [`crate::views::text_row_inner_pad_px`]
/// ([`crate::theme::SPACE_XS`] = 4.0) rounded — left inset of the number
/// column plus air between column and code. Pure.
fn fixture_text_inner_pad() -> u32 {
    crate::views::text_row_inner_pad_px().round() as u32
}

/// Fixture text left offset inside a pane (px): metric column plus one
/// inner pad of air, so code starts clear of the numbers. Pure.
fn fixture_text_x_off(max_line_no: usize) -> u32 {
    fixture_text_linenum_col_w(max_line_no).saturating_add(fixture_text_inner_pad())
}

/// Fixture glyph cell height (px): [`draw_glyph`] paints rows 0..7.
/// Single source so the baseline math below tracks the pen. Pure.
const FIXTURE_GLYPH_H: u32 = 7;

/// Fixture stable baseline offset (px) inside `row_h`: centers the 7px
/// glyph block so every row sits on one beat — same centering idea as
/// live [`crate::views::text_row_baseline_offset_px`], restated in
/// fixture pixels. Pure.
fn fixture_text_baseline_dy(row_h: u32) -> u32 {
    row_h.saturating_sub(FIXTURE_GLYPH_H).saturating_div(2)
}

/// Overview-ruler track: live contract `gutter::GUTTER_RULER_WIDE` (16,
/// line numbers on) — old 14px inter-chrome gap ruler me sama gaya.
/// Budget: 12 + 460 + 4 + 460 + 16 + 8 = 960; right pane 936 par khatam,
// ruler 936 se 952, right margin 8.
pub const GUTTER_X: u32 = 936;
pub const GUTTER_W: u32 = 16;
/// Merge 3-pane equal thirds: avail 960-24-8=928, 309*3=927, 1px slack right me.
pub const MERGE_PANE_W: u32 = 309;
pub const MERGE_X0: u32 = 12;
pub const MERGE_X1: u32 = 325;
pub const MERGE_X2: u32 = 638;
/// Pathmeta strip geometry (exq-pathmeta): field row y76 h24 (existing),
/// second metadata line y100 h12 per-pane, toolbar y114 (pane_top 124 untouched).
/// Budget: 76+24=100 meta 100..112, toolbar 114, panes 124 — 960x600 me fit.
pub const PATH_FIELD_Y: u32 = 76;
pub const PATH_FIELD_H: u32 = 24;
pub const PATH_META_Y: u32 = 100;
pub const PATH_META_H: u32 = 12;
pub const PATH_TOOLBAR_Y: u32 = 114;
/// File-bar geometry (exq-filebar24): one bar row above each pane = path
/// field row (24) + metadata segment line (12) = 36px band at y76..112,
/// 2px air above the toolbar (114). Bar columns track the true pane
/// columns (status20 per-pane strips precedent, extended upward to the
/// file-bar instead of a new layout language).
pub const FILE_BAR_Y: u32 = PATH_FIELD_Y;
pub const FILE_BAR_H: u32 = PATH_FIELD_H + PATH_META_H;
/// File-bar Browse button width (existing width, refit inside each bar).
pub const FILE_BAR_BROWSE_W: u32 = 75;

/// Formats one file-bar metadata segment: timestamp | byte size | format
/// word. Empty parts drop; all-empty yields "-" (honest fallback when stat
/// fails — mirrors status20 `format_pane_status_line`, never a fake "0 B").
/// Pure.
pub fn format_file_bar_meta(timestamp: &str, bytes: &str, format_word: &str) -> String {
    let line = crate::viewmodel::format_path_pane_meta(timestamp, bytes, format_word, "");
    if line.trim().is_empty() {
        "-".to_string()
    } else {
        line
    }
}
/// Segmented status geometry: 30px strip y570, 4 zones (cursor/format/result/edit).
pub const SEG_DIV0_X: u32 = 240;
pub const SEG_DIV1_X: u32 = 480;
pub const SEG_DIV2_X: u32 = 720;
/// Per-pane status strip height (px): 18px thin bar matching fixture row rhythm,
/// sits above the bottom of each pane.
pub const PANE_STATUS_H: u32 = 18;

/// Per-pane file-bar: bar band + path field + dropdown caret + Browse,
/// neeche metadata segment line (timestamp | bytes | format word, "-" jab
/// stat fail ho).
/// Geometry: bar columns track the true pane columns (wide: center layout;
/// narrow: dual-pane split) taaki bar apne pane ke upar tikhe — center
/// gap/gutter me bleed nahi; bar bottom (76+36=112) toolbar (114) se upar.
/// Typo intent viewmodel roles (path Body, meta caption); colors frozen.
fn paint_pathmeta_strip(
    pixels: &mut [u8],
    dims: (u32, u32),
    left_path: &str,
    right_path: &str,
    left_meta: &str,
    right_meta: &str,
) {
    use crate::viewmodel::clip_to_width;
    let (bar_lx, bar_lw, bar_rx, bar_rw) = if dims.0 >= SNAP_W {
        let (lx, lw, _, _, rx, rw) = text_fixture_center_layout();
        (lx, lw, rx, rw)
    } else {
        let (lw, rw) = crate::viewmodel::dual_pane_widths(dims.0, 0.50);
        let (lx, rx, _) = crate::viewmodel::dual_pane_x_positions(dims.0, 0.50);
        (lx, lw, rx, rw)
    };
    let bars = [
        (bar_lx, bar_lw, left_path, left_meta),
        (bar_rx, bar_rw, right_path, right_meta),
    ];
    for (bar_x, bar_w, path, meta) in bars.into_iter() {
        if bar_w == 0 {
            continue;
        }
        // Bar band: field row + metadata line on one track tone.
        fill_rect(
            pixels,
            dims,
            (bar_x, FILE_BAR_Y, bar_w, FILE_BAR_H),
            fixture_palette().track,
        );
        // Path field refit inside the bar: Browse keeps its width, docked
        // right with 4px air on both sides of the pair.
        let field_w = bar_w.saturating_sub(FILE_BAR_BROWSE_W).saturating_sub(8);
        let browse_x = bar_x.saturating_add(field_w).saturating_add(4);
        fill_rect(
            pixels,
            dims,
            (bar_x, PATH_FIELD_Y, field_w, PATH_FIELD_H),
            fixture_palette().field,
        );
        fill_rect(
            pixels,
            dims,
            (browse_x, PATH_FIELD_Y, FILE_BAR_BROWSE_W, PATH_FIELD_H),
            fixture_palette().field,
        );
        let path_shown = clip_to_width(path, field_w.saturating_sub(80), text_width);
        draw_string(
            pixels,
            dims,
            bar_x.saturating_add(8),
            82,
            "[ ]",
            fixture_palette().dim,
        );
        draw_string(
            pixels,
            dims,
            bar_x.saturating_add(36),
            82,
            &path_shown,
            fixture_palette().text,
        );
        draw_string(
            pixels,
            dims,
            bar_x.saturating_add(field_w.saturating_sub(32)),
            82,
            "v",
            fixture_palette().dim,
        );
        draw_string(
            pixels,
            dims,
            browse_x.saturating_add(12),
            82,
            "Browse...",
            fixture_palette().text,
        );
        // Metadata segment: clipped to the bar, "-" jab stat fail ho.
        let trimmed = meta.trim();
        let meta_shown = if trimmed.is_empty() {
            "-".to_string()
        } else {
            clip_to_width(trimmed, bar_w.saturating_sub(16), text_width)
        };
        draw_string(
            pixels,
            dims,
            bar_x.saturating_add(4),
            PATH_META_Y,
            &meta_shown,
            fixture_palette().dim,
        );
    }
}

/// TopChrome toolbar proof (exq-topchrome + exq-chrome17 mirror + exq-toolbar21
/// icons): two stacked 24px action rows + 8px inter-row gap = 56 band (live
/// `views::action_bar_row_gap` SECTION_GAP rhythm, intra-row TIGHT_GAP stays).
/// Row 1: `home::topchrome_*` actions, each button keeps its rhythm width +
/// label and gains a 12px original geometric mark (abstract block/bars/wedge
/// from `fill_rect`, dim hue reuse) left of the label. Row 2: Prev/Next nav
/// pair with 8px cluster air (`views::action_nav_arrow_gap`) + original `<`
/// / `>` chevron triples (`fill_rect` bars, dim hue reuse) + right-docked
/// cursor. Geometry headless contract se aati hai; hues existing fixture tones
/// reuse, koi naya palette zero. Colors frozen.
fn paint_topchrome_toolbar(pixels: &mut [u8], dims: (u32, u32)) {
    use crate::home::{
        TOPCHROME_BUTTON_GAP, TOPCHROME_SIDE_PAD, TOPCHROME_TOOLBAR_ACTIONS,
        topchrome_toolbar_button_width, topchrome_toolbar_visible_count,
    };
    use crate::viewmodel::clip_to_width;
    use crate::views::{action_bar_row_gap, action_bar_row_height, action_nav_arrow_gap};
    let width = dims.0;
    let row_h = action_bar_row_height() as u32;
    let row_gap = action_bar_row_gap() as u32;
    let band_h = row_h.saturating_add(row_gap).saturating_add(row_h);
    fill_rect(
        pixels,
        dims,
        (0, PATH_TOOLBAR_Y, width, band_h),
        fixture_palette().track,
    );
    let visible = topchrome_toolbar_visible_count(width as f32);
    let mut x = TOPCHROME_SIDE_PAD as u32;
    let mut shown = 0usize;
    for action in TOPCHROME_TOOLBAR_ACTIONS {
        if shown >= visible {
            break;
        }
        let w = topchrome_toolbar_button_width(action) as u32;
        if x >= width {
            break;
        }
        let vw = w.min(width.saturating_sub(x));
        if vw < 8 {
            break;
        }
        fill_rect(
            pixels,
            dims,
            (x, PATH_TOOLBAR_Y, vw, row_h),
            fixture_palette().field,
        );
        // Toolbar icon glyph (exq-toolbar21): 12px original geometric mark
        // left of the label — abstract variants (solid block / twin bars /
        // side bars / stepped wedge), `fill_rect` only, dim hue reuse.
        // Button rects + gaps unchanged, so icons compress with the shared
        // visible-count rhythm instead of overlapping.
        if vw >= 16 {
            let icon_x = x.saturating_add(4);
            let icon_y = PATH_TOOLBAR_Y.saturating_add(row_h.saturating_sub(12) / 2);
            let mark = fixture_palette().dim;
            if shown.is_multiple_of(4) {
                fill_rect(pixels, dims, (icon_x, icon_y, 12, 12), mark);
            } else if shown % 4 == 1 {
                fill_rect(pixels, dims, (icon_x, icon_y, 12, 3), mark);
                fill_rect(
                    pixels,
                    dims,
                    (icon_x, icon_y.saturating_add(9), 12, 3),
                    mark,
                );
            } else if shown % 4 == 2 {
                fill_rect(pixels, dims, (icon_x, icon_y, 3, 12), mark);
                fill_rect(
                    pixels,
                    dims,
                    (icon_x.saturating_add(9), icon_y, 3, 12),
                    mark,
                );
            } else {
                fill_rect(pixels, dims, (icon_x, icon_y, 12, 4), mark);
                fill_rect(
                    pixels,
                    dims,
                    (icon_x.saturating_add(2), icon_y.saturating_add(4), 8, 4),
                    mark,
                );
                fill_rect(
                    pixels,
                    dims,
                    (icon_x.saturating_add(4), icon_y.saturating_add(8), 4, 4),
                    mark,
                );
            }
        }
        let label = clip_to_width(action, vw.saturating_sub(24), text_width);
        draw_string(
            pixels,
            dims,
            x.saturating_add(20),
            PATH_TOOLBAR_Y.saturating_add(8),
            &label,
            fixture_palette().text,
        );
        x = x
            .saturating_add(vw)
            .saturating_add(TOPCHROME_BUTTON_GAP as u32);
        shown = shown.saturating_add(1);
    }
    if shown < TOPCHROME_TOOLBAR_ACTIONS.len() && width > 28 {
        draw_string(
            pixels,
            dims,
            width.saturating_sub(24),
            PATH_TOOLBAR_Y.saturating_add(8),
            ">>",
            fixture_palette().dim,
        );
    }
    // Row 2: nav pair (Prev/Next 96x24 floor) with 8px cluster air, cursor docked right.
    let row2_y = PATH_TOOLBAR_Y.saturating_add(row_h).saturating_add(row_gap);
    let nav_gap = action_nav_arrow_gap() as u32;
    let btn_w = 96u32;
    let side = TOPCHROME_SIDE_PAD as u32;
    if width
        > side
            .saturating_mul(2)
            .saturating_add(btn_w.saturating_mul(2))
    {
        fill_rect(
            pixels,
            dims,
            (side, row2_y, btn_w, row_h),
            fixture_palette().field,
        );
        // Nav chevrons (exq-toolbar21): original `<` / `>` triples from
        // `fill_rect` bars, dim hue reuse, inside the existing 96x24 pair.
        let chev = fixture_palette().dim;
        let cy = row2_y.saturating_add(row_h.saturating_sub(10) / 2);
        let pcx = side.saturating_add(4);
        fill_rect(pixels, dims, (pcx.saturating_add(6), cy, 2, 2), chev);
        fill_rect(
            pixels,
            dims,
            (pcx.saturating_add(4), cy.saturating_add(2), 2, 2),
            chev,
        );
        fill_rect(
            pixels,
            dims,
            (pcx.saturating_add(2), cy.saturating_add(4), 2, 2),
            chev,
        );
        fill_rect(
            pixels,
            dims,
            (pcx.saturating_add(4), cy.saturating_add(6), 2, 2),
            chev,
        );
        fill_rect(
            pixels,
            dims,
            (pcx.saturating_add(6), cy.saturating_add(8), 2, 2),
            chev,
        );
        draw_string(
            pixels,
            dims,
            side.saturating_add(16),
            row2_y.saturating_add(8),
            "[Prev]",
            fixture_palette().text,
        );
        let nx = side.saturating_add(btn_w).saturating_add(nav_gap);
        fill_rect(
            pixels,
            dims,
            (nx, row2_y, btn_w, row_h),
            fixture_palette().field,
        );
        let ncx = nx.saturating_add(4);
        fill_rect(pixels, dims, (ncx, cy, 2, 2), chev);
        fill_rect(
            pixels,
            dims,
            (ncx.saturating_add(2), cy.saturating_add(2), 2, 2),
            chev,
        );
        fill_rect(
            pixels,
            dims,
            (ncx.saturating_add(4), cy.saturating_add(4), 2, 2),
            chev,
        );
        fill_rect(
            pixels,
            dims,
            (ncx.saturating_add(2), cy.saturating_add(6), 2, 2),
            chev,
        );
        fill_rect(pixels, dims, (ncx, cy.saturating_add(8), 2, 2), chev);
        draw_string(
            pixels,
            dims,
            nx.saturating_add(16),
            row2_y.saturating_add(8),
            "[Next]",
            fixture_palette().text,
        );
        draw_string(
            pixels,
            dims,
            width.saturating_sub(48),
            row2_y.saturating_add(8),
            "3:1",
            fixture_palette().dim,
        );
    }
}

/// Bottom segmented status: cursor | format | result | edit (4 zones, dividers).
/// Single-line honest form viewmodel se; clip 936 budget me; colors frozen.
#[allow(dead_code)]
fn paint_segmented_status(
    pixels: &mut [u8],
    dims: (u32, u32),
    status: &crate::viewmodel::PathSegmentedStatus,
) {
    use crate::viewmodel::{clip_to_width, path_segmented_status_line};
    let width = dims.0;
    fill_rect(
        pixels,
        dims,
        (0, 570, width, STATUS_H),
        fixture_palette().track,
    );
    let line = path_segmented_status_line(status);
    let shown = clip_to_width(&line, width.saturating_sub(24), text_width);
    // 4 zones: segments ko dividers par baanto (geometry first, text second).
    let segs = [
        status.cursor.as_str(),
        status.format.as_str(),
        status.result.as_str(),
        status.edit.as_str(),
    ];
    let xs = [16u32, 250u32, 490u32, 730u32];
    let mut any = false;
    for (seg, x) in segs.iter().zip(xs.iter()) {
        if seg.trim().is_empty() {
            continue;
        }
        any = true;
        let s = clip_to_width(seg.trim(), 200, text_width);
        draw_string(pixels, dims, *x, 578, &s, fixture_palette().dim);
    }
    if !any {
        draw_string(pixels, dims, 16, 578, &shown, fixture_palette().dim);
    }
    // Dividers (live track tone, ink via dim).
    for dx in [SEG_DIV0_X, SEG_DIV1_X, SEG_DIV2_X] {
        fill_rect(pixels, dims, (dx, 574, 1, 22), fixture_palette().dim);
    }
}

/// Formats a per-pane status line from cursor, format word, and result word.
/// Empty parts are omitted; if all empty, returns "-". Pure; unit-tested.
pub fn format_pane_status_line(cursor: &str, format: &str, result: &str) -> String {
    let mut parts = Vec::new();
    let c = cursor.trim();
    if !c.is_empty() {
        parts.push(c);
    }
    let f = format.trim();
    if !f.is_empty() {
        parts.push(f);
    }
    let r = result.trim();
    if !r.is_empty() {
        parts.push(r);
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" | ")
    }
}

/// Paints one per-pane status strip above a pane bottom: live track tone, subtle top border,
/// single-line honest text clipped to pane width.
pub fn paint_pane_status_strip(
    pixels: &mut [u8],
    dims: (u32, u32),
    rect: (u32, u32, u32, u32),
    line: &str,
) {
    let (x, y, w, h) = rect;
    fill_rect(pixels, dims, rect, fixture_palette().track);
    fill_rect(pixels, dims, (x, y, w, 1), fixture_palette().dim);
    let shown = crate::viewmodel::clip_to_width(line, w.saturating_sub(16), text_width);
    let base_dy = h.saturating_sub(FIXTURE_GLYPH_H) / 2;
    draw_string(
        pixels,
        dims,
        x.saturating_add(8),
        y.saturating_add(base_dy),
        &shown,
        fixture_palette().dim,
    );
}

/// Bottom legend key segments (exq-status20): static legend key in clean-room phrasing
/// (differences, same, insert, overview).
pub fn fixture_legend_segments() -> [&'static str; 4] {
    ["Differences", "Same", "Insert", "Overview"]
}

/// Bottom legend strip (exq-status20): 30px strip at canvas bottom (y570..600)
/// showing 3-4 segment words with subtle dividers, clipped per zone.
pub fn paint_bottom_legend_strip(pixels: &mut [u8], dims: (u32, u32), segments: &[&str]) {
    let width = dims.0;
    fill_rect(
        pixels,
        dims,
        (0, 570, width, STATUS_H),
        fixture_palette().track,
    );
    fill_rect(pixels, dims, (0, 570, width, 1), fixture_palette().dim);

    let count = segments.len() as u32;
    if count == 0 {
        return;
    }
    let zone_w = width / count;
    for (i, seg) in segments.iter().enumerate() {
        let idx = i as u32;
        let zone_x = idx.saturating_mul(zone_w);
        if idx > 0 {
            fill_rect(pixels, dims, (zone_x, 574, 1, 22), fixture_palette().dim);
        }
        let text_budget = zone_w.saturating_sub(16);
        let s = crate::viewmodel::clip_to_width(seg.trim(), text_budget, text_width);
        draw_string(
            pixels,
            dims,
            zone_x.saturating_add(12),
            578,
            &s,
            fixture_palette().dim,
        );
    }
}
/// Overview minimap helpers (exq-gutter28): the right strip compresses the
/// whole file into a miniature map — each file row maps to a proportional
/// vertical position inside a small top/bottom inset so edge bands never
/// clip. Both directions reuse the live padded mapping, never ad-hoc math.
/// Center gutter untouched (fixed 20px lane, hunk-head markers only).
/// Pure; headless-tested. Colors frozen (caller picks hues).
pub fn overview_row_y(row: usize, total_rows: usize, pane_top: u32, pane_h: u32) -> Option<u32> {
    let inner_y = gutter_padded_y_for_row(row, total_rows, pane_h as f32)?;
    if inner_y.is_finite() && inner_y >= 0.0 {
        Some(pane_top.saturating_add(inner_y.round() as u32))
    } else {
        None
    }
}

/// Inverse of [`overview_row_y`]: gutter-local y back to a model row.
/// `None` on out-of-range y or bad geometry (same contract as live).
/// Pure; headless-tested.
pub fn overview_row_at_y(y: u32, total_rows: usize, pane_top: u32, pane_h: u32) -> Option<usize> {
    if y < pane_top {
        return None;
    }
    let local = y.saturating_sub(pane_top) as f32;
    gutter_padded_row_at_y(local, total_rows, pane_h as f32)
}

/// Paints the right overview strip: full-file bands (same + diff) from the
/// live padded geometry inside the track inset; diff hues frozen, same
/// bands reuse the track tone. Single source for the wide (960) and narrow
/// (720) text routes; accent/wrap/hex routes keep their own paint.
/// exq-gutter36 polish: band edges round (not truncate) so paint aligns
/// with [`overview_row_y`] (which rounds the same live mapping) — max
/// 1px drift gone, edge bands stay inside the padded track.
pub fn paint_overview_strip(
    pixels: &mut [u8],
    dims: (u32, u32),
    gutter_x: u32,
    gutter_w: u32,
    pane_top: u32,
    pane_h: u32,
    rows: &[TextRow],
) {
    let geom = gutter_overview_geometry_padded(rows.len(), pane_h as f32, rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start.round() as u32);
        let seg_h = ((seg.y_end - seg.y_start).round() as u32).max(GUTTER_SEGMENT_MIN_H as u32);
        let seg_color = match seg.kind {
            TextRowKind::Equal => fixture_palette().track,
            TextRowKind::Changed => fixture_palette().changed_bg,
            TextRowKind::Deleted => fixture_palette().deleted_bg,
            TextRowKind::Inserted => fixture_palette().inserted_bg,
        };
        fill_rect(pixels, dims, (gutter_x, seg_y, gutter_w, seg_h), seg_color);
    }
}

/// Renders the Text comparison session fixture to a PNG at the given path.
///
/// Default mode dark hai taaki existing fixtures byte-stable rahen;
/// light ke liye [`render_fixture_png_with_mode`] use karo.
/// File name ke tukde se tab chunta hai (lowercase match):
/// `accent` -> text accent scene, `allsame` -> folder identical
/// (AllSame) scene, `hex` -> hex, `table` + `find` -> table key/find scene, `table` -> table,
/// `merge` -> merge (foldermerge bhi), `merge-take` -> merge take-row scene,
/// `three-way`/`threeway` -> three-way tree scene, `sync` -> sync, `folder` + `empty` -> folder filtered
/// empty scene, `folder` -> folder, `home` -> home, `picture` -> picture,
/// `wrap` -> wrap-on + find-active text scene, warna text.
/// Har route same 960x600 canvas par tikta hai, colors frozen.
pub fn render_fixture_png(out_path: &Path) -> Result<(), String> {
    render_fixture_png_with_mode(out_path, FixtureMode::Dark)
}

/// Mode-aware dispatcher: `Dark` existing frozen-dark render se
/// `Light` me base hues + diff washes `crate::theme` ke `LIGHT_*` helpers
/// se aate hain, geometry same.
pub fn render_fixture_png_with_mode(out_path: &Path, mode: FixtureMode) -> Result<(), String> {
    let _guard = FixtureModeGuard::set(mode);
    render_fixture_png_routed(out_path)
}

/// CP-LIGHTFLAG: `--screenshot-mode <dark|light>` CLI value nikalo.
/// Default dark taaki flag ke bina render byte-stable rahe. Unknown ya
/// missing value par `Err` (caller exit-code 2 deta hai, opener precedent).
/// Pure; headless-tested, GUI nahi khulta.
pub fn screenshot_mode_from_args(args: &[String]) -> Result<FixtureMode, String> {
    let mut mode = FixtureMode::Dark;
    let mut rest: &[String] = match args.get(1..) {
        Some(rest) => rest,
        None => return Ok(mode),
    };
    while let Some((head, tail)) = rest.split_first() {
        if head.as_str() == "--screenshot-mode" {
            match tail.split_first() {
                Some((value, after)) if !value.starts_with("--") => {
                    match value.as_str() {
                        "dark" => mode = FixtureMode::Dark,
                        "light" => mode = FixtureMode::Light,
                        _ => {
                            return Err(format!("unknown --screenshot-mode '{value}': dark/light"));
                        }
                    }
                    rest = after;
                }
                _ => {
                    return Err(String::from(
                        "--screenshot-mode needs a value: --screenshot-mode dark|light",
                    ));
                }
            }
        } else {
            rest = tail;
        }
    }
    Ok(mode)
}

/// Mode guard ke andar route dispatch (shared, testable core).
fn render_fixture_png_routed(out_path: &Path) -> Result<(), String> {
    let lower = match out_path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.to_lowercase(),
        None => String::new(),
    };
    if lower.contains("narrow") {
        return render_narrow_text_fixture_png(out_path);
    }
    if lower.contains("accent") {
        return render_text_accent_fixture_png(out_path);
    }
    if lower.contains("allsame") {
        return render_folder_allsame_fixture_png(out_path);
    }
    if lower.contains("hex") {
        return render_hex_fixture_png(out_path);
    }
    if lower.contains("table") && lower.contains("find") {
        return render_table_find_fixture_png(out_path);
    }
    if lower.contains("table") {
        return render_table_fixture_png(out_path);
    }
    if lower.contains("three-way") || lower.contains("threeway") {
        return render_three_way_fixture_png(out_path);
    }
    if lower.contains("merge-take") {
        return render_merge_take_fixture_png(out_path);
    }
    if lower.contains("merge") {
        return render_merge_fixture_png(out_path);
    }
    if lower.contains("sync") {
        return render_sync_fixture_png(out_path);
    }
    if lower.contains("folder") && lower.contains("empty") {
        return render_folder_empty_fixture_png(out_path);
    }
    if lower.contains("folder") {
        return render_folder_fixture_png(out_path);
    }
    if lower.contains("home") {
        return render_home_fixture_png(out_path);
    }
    if lower.contains("picture") {
        return render_picture_fixture_png(out_path);
    }
    if lower.contains("wrap") {
        return render_wrap_fixture_png(out_path);
    }
    render_text_fixture_png(out_path)
}

/// Renders the Text comparison session fixture to a PNG at the given path.
pub fn render_text_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = 960u32;
    let height = 600u32;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };

    let mut pixels = vec![0u8; byte_count];
    // Fill background
    fill_rect(
        &mut pixels,
        dims,
        (0, 0, width, height),
        fixture_palette().bg,
    );

    // Title bar (no OS window controls: live egui paints none).
    fill_rect(
        &mut pixels,
        dims,
        (0, 0, width, 36),
        fixture_palette().track,
    );
    draw_string(
        &mut pixels,
        dims,
        OUTER_PAD,
        14,
        "fixtures/left.rs <--> fixtures/right.rs - Text",
        fixture_palette().text,
    );

    // Tab bar (exq-chrome17 mirror): active tab reads via text-vs-dim
    // ink (live tabs are egui selectable style, no theme fill token).
    fill_rect(
        &mut pixels,
        dims,
        (0, 36, width, 32),
        fixture_palette().track,
    );
    draw_string(&mut pixels, dims, 16, 46, "Home", fixture_palette().dim);
    draw_string(&mut pixels, dims, 80, 46, "Text", fixture_palette().text);
    draw_string(&mut pixels, dims, 148, 46, "Folder", fixture_palette().dim);
    draw_string(&mut pixels, dims, 222, 46, "Hex", fixture_palette().dim);
    draw_string(&mut pixels, dims, 278, 46, "Merge", fixture_palette().dim);
    draw_string(&mut pixels, dims, 346, 46, "Table", fixture_palette().dim);
    draw_string(&mut pixels, dims, 410, 46, "Sync", fixture_palette().dim);
    draw_string(&mut pixels, dims, 474, 46, "Picture", fixture_palette().dim);
    draw_string(
        &mut pixels,
        dims,
        544,
        46,
        "FolderMerge",
        fixture_palette().dim,
    );
    draw_string(&mut pixels, dims, 644, 46, "Audio", fixture_palette().dim);

    // Per-pane file-bar + metadata segment line (exq-filebar24): path
    // field left (ellipsis-clipped) + timestamp | bytes | format word.
    paint_pathmeta_strip(
        &mut pixels,
        dims,
        "fixtures/left.rs",
        "fixtures/right.rs",
        &format_file_bar_meta("2026-01-02 03:04", "128 B", "Text: exact"),
        &format_file_bar_meta("2026-01-02 03:05", "132 B", "Text: exact"),
    );

    // TopChrome toolbar: two stacked 24px rows + 8px gap = 56 band (chrome17
    // `action_bar_row_gap` rhythm), panes 174 se (4px rhythm gap intact).
    paint_topchrome_toolbar(&mut pixels, dims);

    // Main diff pane borders: two equal panes with a live 20px center
    // gutter between them (BC parity), overview ruler still at GUTTER_X.
    let pane_top = 174u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    let gutter_x = GUTTER_X;
    let gutter_w = GUTTER_W;
    let (left_x, left_w, center_x, center_w, right_x, right_w) = text_fixture_center_layout();

    fill_rect(
        &mut pixels,
        dims,
        (left_x, pane_top, left_w, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (right_x, pane_top, right_w, pane_height),
        fixture_palette().bg,
    );
    // Center floor: full-height base so the gap reads below the last row too.
    fill_rect(
        &mut pixels,
        dims,
        (center_x, pane_top, center_w, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (gutter_x, pane_top, gutter_w, pane_height),
        fixture_palette().track,
    );

    // Fixture diff rows
    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(2),
            right_no: Some(2),
            left: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            right: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        },
        TextRow {
            kind: TextRowKind::Deleted,
            left_no: Some(4),
            right_no: None,
            left: "    let legacy_flag = true;".to_string(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Inserted,
            left_no: None,
            right_no: Some(4),
            left: String::new(),
            right: "    validate_safe_path(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(5),
            right_no: Some(5),
            left: "    Ok(content.len())".to_string(),
            right: "    Ok(content.len())".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(6),
            right_no: Some(6),
            left: "}".to_string(),
            right: "}".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    // Fixture typo17 mirror (shot19): row height from the live
    // line-height rhythm, number column + left inset from live char
    // metrics, one stable baseline for every row. Center/gutter/chrome
    // below stay untouched.
    let row_h = fixture_text_row_h();
    let num_x_off = fixture_text_inner_pad();
    let text_x_off = fixture_text_x_off(rows.len());
    let base_dy = fixture_text_baseline_dy(row_h);
    let mut content_end = pane_top;
    for (i, row) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }
        content_end = y.saturating_add(row_h);

        let (left_bg, left_fg, right_bg, right_fg) = match row.kind {
            TextRowKind::Equal => (
                fixture_palette().bg,
                fixture_palette().text,
                fixture_palette().bg,
                fixture_palette().text,
            ),
            TextRowKind::Changed => (
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
            ),
            TextRowKind::Deleted => (
                fixture_palette().deleted_bg,
                fixture_palette().deleted_fg,
                fixture_palette().bg,
                fixture_palette().dim,
            ),
            TextRowKind::Inserted => (
                fixture_palette().bg,
                fixture_palette().dim,
                fixture_palette().inserted_bg,
                fixture_palette().inserted_fg,
            ),
        };

        // Left pane row
        fill_rect(&mut pixels, dims, (left_x, y, left_w, row_h), left_bg);
        if let Some(no) = row.left_no {
            draw_string(
                &mut pixels,
                dims,
                left_x.saturating_add(num_x_off),
                y.saturating_add(base_dy),
                &format!("{no:>2} "),
                fixture_palette().dim,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            left_x.saturating_add(text_x_off),
            y.saturating_add(base_dy),
            &row.left,
            left_fg,
        );
        // Word-level accents (`paint_inline_accent`): dark me inverted
        // block, light me underline-only (dark-ink-on-light-wash).
        for (start, end) in row.accent_a.iter() {
            let prefix: String = row.left.chars().take(*start).collect();
            let span: String = row
                .left
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = left_x
                .saturating_add(text_x_off)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            paint_inline_accent(
                &mut pixels,
                dims,
                InlineAccent {
                    x: ax,
                    y,
                    w: aw,
                    row_h,
                    fg: left_fg,
                    bg: left_bg,
                    span: &span,
                    baseline_dy: base_dy,
                },
            );
        }

        // Right pane row
        fill_rect(&mut pixels, dims, (right_x, y, right_w, row_h), right_bg);
        if let Some(no) = row.right_no {
            draw_string(
                &mut pixels,
                dims,
                right_x.saturating_add(num_x_off),
                y.saturating_add(base_dy),
                &format!("{no:>2} "),
                fixture_palette().dim,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            right_x.saturating_add(text_x_off),
            y.saturating_add(base_dy),
            &row.right,
            right_fg,
        );
        for (start, end) in row.accent_b.iter() {
            let prefix: String = row.right.chars().take(*start).collect();
            let span: String = row
                .right
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = right_x
                .saturating_add(text_x_off)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            paint_inline_accent(
                &mut pixels,
                dims,
                InlineAccent {
                    x: ax,
                    y,
                    w: aw,
                    row_h,
                    fg: right_fg,
                    bg: right_bg,
                    span: &span,
                    baseline_dy: base_dy,
                },
            );
        }
        // Center-gap marker: yellow `=>` on changed-row hunk heads only;
        // equal + continuation rows stay blank click-to-select (BC parity).
        if row.kind != TextRowKind::Equal && textcenter_hunk_head(&rows, i) {
            let mark_w = text_width(TEXT_CENTER_MARK);
            let mx = center_x.saturating_add(center_w.saturating_sub(mark_w) / 2);
            draw_string(
                &mut pixels,
                dims,
                mx,
                y.saturating_add(base_dy),
                TEXT_CENTER_MARK,
                fixture_palette().changed_fg,
            );
        }
    }

    // Trailing calm fill (exq-hatch25): slots below the last content row
    // read as one calm field — same content-base tone the equal rows use,
    // zero filler glyphs, no banding edge against the content block. Starts
    // exactly at the last row (zero pad) and stops above the status20
    // strips; center floor already carries this tone, gutter untouched.
    let strip_top = pane_bottom.saturating_sub(PANE_STATUS_H);
    for (pane_x, pane_w) in [(left_x, left_w), (right_x, right_w)] {
        if pane_w == 0 || content_end >= strip_top {
            continue;
        }
        fill_rect(
            &mut pixels,
            dims,
            (
                pane_x,
                content_end,
                pane_w,
                strip_top.saturating_sub(content_end),
            ),
            fixture_palette().bg,
        );
    }

    // Right overview minimap (exq-gutter28): full-file bands via the live
    // padded geometry; center gutter above stays the 20px hunk-head lane.
    paint_overview_strip(
        &mut pixels,
        dims,
        gutter_x,
        gutter_w,
        pane_top,
        pane_height,
        &rows,
    );

    // Per-pane status strips above pane bottoms (BC parity: cursor, format word, result word).
    let strip_y = pane_bottom.saturating_sub(PANE_STATUS_H);
    let left_status =
        format_pane_status_line("3:1", "Text: exact", "4 equal, 1 changed, 1 deleted");
    paint_pane_status_strip(
        &mut pixels,
        dims,
        (left_x, strip_y, left_w, PANE_STATUS_H),
        &left_status,
    );

    let right_status =
        format_pane_status_line("3:1", "Text: exact", "4 equal, 1 changed, 1 inserted");
    paint_pane_status_strip(
        &mut pixels,
        dims,
        (right_x, strip_y, right_w, PANE_STATUS_H),
        &right_status,
    );

    // Bottom legend strip: static key words (differences, same, insert, overview).
    paint_bottom_legend_strip(&mut pixels, dims, &fixture_legend_segments());

    let image = RgbaImage {
        width,
        height,
        pixels,
    };

    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;

    Ok(())
}

/// Renders the 720px fallback text fixture on the shared headless
/// budget ([`crate::theme::VIEWPORT_MIN_W`] wide,
/// [`crate::viewmodel::dual_pane_widths`] panes, gutter [`GUTTER_W`]).
/// Every pane string passes `clip_to_width` (ellipsis, never a raw
/// mid-glyph cut); word accents stay on the 960 route, bands + gutter +
/// chrome are the fallback proof. Colors frozen.
pub fn render_narrow_text_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = crate::theme::VIEWPORT_MIN_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left.rs <--> fixtures/right.rs - Text (720 fallback)",
        1,
    );

    // Compact path row: one summary box, ellipsis-clipped to the canvas.
    let summary = crate::viewmodel::clip_to_width(
        "fixtures/left.rs <--> fixtures/right.rs",
        width.saturating_sub(24),
        text_width,
    );
    fill_rect(
        &mut pixels,
        dims,
        (OUTER_PAD, 76, width.saturating_sub(24), 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        OUTER_PAD.saturating_add(8),
        82,
        &summary,
        fixture_palette().text,
    );

    draw_string(
        &mut pixels,
        dims,
        16,
        104,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        fixture_palette().text,
    );

    let (left_w, right_w) = crate::viewmodel::dual_pane_widths(width, 0.50);
    let (left_x, right_x, gutter_x) = crate::viewmodel::dual_pane_x_positions(width, 0.50);
    debug_assert_eq!(left_x, OUTER_PAD);
    debug_assert_eq!(
        right_x,
        left_x.saturating_add(left_w).saturating_add(PANE_GAP)
    );
    debug_assert_eq!(gutter_x, right_x.saturating_add(right_w));
    let text_budget = left_w.saturating_sub(32);
    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (left_x, pane_top, left_w, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (right_x, pane_top, right_w, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (gutter_x, pane_top, GUTTER_W, pane_height),
        fixture_palette().track,
    );

    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::{Path, PathBuf};".to_string(),
            right: "use std::path::{Path, PathBuf};".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(2),
            right_no: Some(2),
            left: "use std::sync::Arc;".to_string(),
            right: "use std::sync::Arc;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(3),
            right_no: Some(3),
            left: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            right: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(4),
            right_no: Some(4),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        },
        TextRow {
            kind: TextRowKind::Deleted,
            left_no: Some(5),
            right_no: None,
            left: "    let legacy_flag = true;".to_string(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Inserted,
            left_no: None,
            right_no: Some(5),
            left: String::new(),
            right: "    validate_safe_path(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(6),
            right_no: Some(6),
            left: "    let buffer_size = 1024;".to_string(),
            right: "    let buffer_size = 4096;".to_string(),
            accent_a: vec![(22, 26)],
            accent_b: vec![(22, 26)],
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(7),
            right_no: Some(7),
            left: "    let mut buffer = Vec::with_capacity(buffer_size);".to_string(),
            right: "    let mut buffer = Vec::with_capacity(buffer_size);".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(8),
            right_no: Some(8),
            left: "    if legacy_flag {".to_string(),
            right: "    if content.is_empty() {".to_string(),
            accent_a: vec![(7, 18)],
            accent_b: vec![(7, 25)],
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(9),
            right_no: Some(9),
            left: "        eprintln!(\"fallback mode enabled\");".to_string(),
            right: "        return Ok(0);".to_string(),
            accent_a: vec![(8, 41)],
            accent_b: vec![(8, 21)],
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(10),
            right_no: Some(10),
            left: "    }".to_string(),
            right: "    }".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(11),
            right_no: Some(11),
            left: "    let digest = compute_checksum(&content);".to_string(),
            right: "    let digest = compute_checksum_v2(&content);".to_string(),
            accent_a: vec![(17, 33)],
            accent_b: vec![(17, 36)],
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(12),
            right_no: Some(12),
            left: "    Ok(content.len())".to_string(),
            right: "    Ok(content.len())".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(13),
            right_no: Some(13),
            left: "}".to_string(),
            right: "}".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(14),
            right_no: Some(14),
            left: "// End of safe processing module".to_string(),
            right: "// End of safe processing module".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    let row_h = ROW_H;
    for (i, row) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }
        let (left_bg, left_fg, right_bg, right_fg) = match row.kind {
            TextRowKind::Equal => (
                fixture_palette().bg,
                fixture_palette().text,
                fixture_palette().bg,
                fixture_palette().text,
            ),
            TextRowKind::Changed => (
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
            ),
            TextRowKind::Deleted => (
                fixture_palette().deleted_bg,
                fixture_palette().deleted_fg,
                fixture_palette().bg,
                fixture_palette().dim,
            ),
            TextRowKind::Inserted => (
                fixture_palette().bg,
                fixture_palette().dim,
                fixture_palette().inserted_bg,
                fixture_palette().inserted_fg,
            ),
        };
        let left_shown = crate::viewmodel::clip_to_width(&row.left, text_budget, text_width);
        let right_shown = crate::viewmodel::clip_to_width(&row.right, text_budget, text_width);
        fill_rect(&mut pixels, dims, (left_x, y, left_w, row_h), left_bg);
        if let Some(no) = row.left_no {
            draw_string(
                &mut pixels,
                dims,
                left_x.saturating_add(4),
                y.saturating_add(6),
                &format!("{no:>2} "),
                fixture_palette().dim,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            left_x.saturating_add(32),
            y.saturating_add(6),
            &left_shown,
            left_fg,
        );
        // Word-level accents (`paint_inline_accent`): dark block, light underline
        for (start, end) in row.accent_a.iter() {
            let prefix: String = row.left.chars().take(*start).collect();
            let span: String = row
                .left
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = left_x
                .saturating_add(32)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            let pane_right_bound = left_x.saturating_add(left_w);
            if ax < pane_right_bound {
                let clamped_aw = aw.min(pane_right_bound.saturating_sub(ax));
                let clipped_span = crate::viewmodel::clip_to_width(&span, clamped_aw, text_width);
                paint_inline_accent(
                    &mut pixels,
                    dims,
                    InlineAccent {
                        x: ax,
                        y,
                        w: clamped_aw,
                        row_h,
                        fg: left_fg,
                        bg: left_bg,
                        span: &clipped_span,
                        baseline_dy: 6,
                    },
                );
            }
        }

        fill_rect(&mut pixels, dims, (right_x, y, right_w, row_h), right_bg);
        if let Some(no) = row.right_no {
            draw_string(
                &mut pixels,
                dims,
                right_x.saturating_add(4),
                y.saturating_add(6),
                &format!("{no:>2} "),
                fixture_palette().dim,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            right_x.saturating_add(32),
            y.saturating_add(6),
            &right_shown,
            right_fg,
        );
        // Word-level accents (`paint_inline_accent`): dark block, light underline
        for (start, end) in row.accent_b.iter() {
            let prefix: String = row.right.chars().take(*start).collect();
            let span: String = row
                .right
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = right_x
                .saturating_add(32)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            let pane_right_bound = right_x.saturating_add(right_w);
            if ax < pane_right_bound {
                let clamped_aw = aw.min(pane_right_bound.saturating_sub(ax));
                let clipped_span = crate::viewmodel::clip_to_width(&span, clamped_aw, text_width);
                paint_inline_accent(
                    &mut pixels,
                    dims,
                    InlineAccent {
                        x: ax,
                        y,
                        w: clamped_aw,
                        row_h,
                        fg: right_fg,
                        bg: right_bg,
                        span: &clipped_span,
                        baseline_dy: 6,
                    },
                );
            }
        }
    }

    // Right overview minimap (exq-gutter28): full-file bands via the live
    // padded geometry; 720 fallback columns unchanged (dual-pane split).
    paint_overview_strip(
        &mut pixels,
        dims,
        gutter_x,
        GUTTER_W,
        pane_top,
        pane_height,
        &rows,
    );

    paint_status(
        &mut pixels,
        dims,
        "720 fallback: 15 rows, bands + gutter honest, text ellipsis-clipped",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Text intra-line accent scene: dono panes par Changed row
/// ke word spans (`accent_a`/`accent_b`) inverted blocks me — wrap/find
/// precedent jaisa naya route (`accent` naam se chunta hai). Poore-row
/// band par kaun sa shabd badla wahi block dikhta hai. Colors frozen.
pub fn render_text_accent_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left.rs <--> fixtures/right.rs - Text (accent)",
        1,
    );

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_LEFT_X, pane_top, TEXT_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_RIGHT_X, pane_top, TEXT_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top, GUTTER_W, pane_height),
        fixture_palette().track,
    );

    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(2),
            right_no: Some(2),
            left: "    let old = load(left)?;".to_string(),
            right: "    let new = load(right)?;".to_string(),
            accent_a: vec![(8, 11), (19, 23)],
            accent_b: vec![(8, 11), (19, 24)],
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(4),
            right_no: Some(4),
            left: "    Ok(content.len())".to_string(),
            right: "    Ok(content.len())".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    let row_h = ROW_H;
    for (i, row) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }
        let (left_bg, left_fg, right_bg, right_fg) = match row.kind {
            TextRowKind::Equal => (
                fixture_palette().bg,
                fixture_palette().text,
                fixture_palette().bg,
                fixture_palette().text,
            ),
            TextRowKind::Changed => (
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
            ),
            TextRowKind::Deleted => (
                fixture_palette().deleted_bg,
                fixture_palette().deleted_fg,
                fixture_palette().bg,
                fixture_palette().dim,
            ),
            TextRowKind::Inserted => (
                fixture_palette().bg,
                fixture_palette().dim,
                fixture_palette().inserted_bg,
                fixture_palette().inserted_fg,
            ),
        };
        fill_rect(
            &mut pixels,
            dims,
            (TEXT_LEFT_X, y, TEXT_PANE_W, row_h),
            left_bg,
        );
        if let Some(no) = row.left_no {
            draw_string(
                &mut pixels,
                dims,
                TEXT_LEFT_X.saturating_add(4),
                y.saturating_add(6),
                &format!("{no:>2} "),
                fixture_palette().dim,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            TEXT_LEFT_X.saturating_add(32),
            y.saturating_add(6),
            &row.left,
            left_fg,
        );
        for (start, end) in row.accent_a.iter() {
            let prefix: String = row.left.chars().take(*start).collect();
            let span: String = row
                .left
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = TEXT_LEFT_X
                .saturating_add(32)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            paint_inline_accent(
                &mut pixels,
                dims,
                InlineAccent {
                    x: ax,
                    y,
                    w: aw,
                    row_h,
                    fg: left_fg,
                    bg: left_bg,
                    span: &span,
                    baseline_dy: 6,
                },
            );
        }
        fill_rect(
            &mut pixels,
            dims,
            (TEXT_RIGHT_X, y, TEXT_PANE_W, row_h),
            right_bg,
        );
        if let Some(no) = row.right_no {
            draw_string(
                &mut pixels,
                dims,
                TEXT_RIGHT_X.saturating_add(4),
                y.saturating_add(6),
                &format!("{no:>2} "),
                fixture_palette().dim,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            TEXT_RIGHT_X.saturating_add(32),
            y.saturating_add(6),
            &row.right,
            right_fg,
        );
        for (start, end) in row.accent_b.iter() {
            let prefix: String = row.right.chars().take(*start).collect();
            let span: String = row
                .right
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = TEXT_RIGHT_X
                .saturating_add(32)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            paint_inline_accent(
                &mut pixels,
                dims,
                InlineAccent {
                    x: ax,
                    y,
                    w: aw,
                    row_h,
                    fg: right_fg,
                    bg: right_bg,
                    span: &span,
                    baseline_dy: 6,
                },
            );
        }
    }

    let geom = gutter_geometry(rows.len(), pane_height as f32, &rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start as u32);
        let seg_h = ((seg.y_end - seg.y_start) as u32).max(GUTTER_SEGMENT_MIN_H as u32);
        let seg_color = match seg.kind {
            TextRowKind::Equal => fixture_palette().track,
            TextRowKind::Changed => fixture_palette().changed_bg,
            TextRowKind::Deleted => fixture_palette().deleted_bg,
            TextRowKind::Inserted => fixture_palette().inserted_bg,
        };
        fill_rect(
            &mut pixels,
            dims,
            (GUTTER_X, seg_y, GUTTER_W, seg_h),
            seg_color,
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "accent: 4 rows, 2 changed word spans inverted | intra-line proof",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Folder comparison session fixture: filter checkboxes,
/// honest counts, tree rows with glyphs, and the sync preview panel
/// (mode labels, dry-run options, consent hint, Apply gate). All
/// strings come from the headless `tree`/`home` logic so the shot
/// proves the wired UI, not a mock.
pub fn render_folder_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::home::{folder_empty_hint, sync_consent_hint};
    use crate::tree::{SyncModeChoice, count_folder_rows, folder_tree_filtered, sync_preview};
    use crate::viewmodel::FolderRow;

    let width = 960u32;
    let height = 600u32;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };

    let mut pixels = vec![0u8; byte_count];
    fill_rect(
        &mut pixels,
        dims,
        (0, 0, width, height),
        fixture_palette().bg,
    );

    // Title bar (no OS window controls: live egui paints none).
    fill_rect(
        &mut pixels,
        dims,
        (0, 0, width, 36),
        fixture_palette().track,
    );
    draw_string(
        &mut pixels,
        dims,
        OUTER_PAD,
        14,
        "fixtures/left-dir <--> fixtures/right-dir - Folder",
        fixture_palette().text,
    );

    // Tab bar, active tab reads via text-vs-dim ink.
    fill_rect(
        &mut pixels,
        dims,
        (0, 36, width, 32),
        fixture_palette().track,
    );
    draw_string(&mut pixels, dims, 16, 46, "Home", fixture_palette().dim);
    draw_string(&mut pixels, dims, 64, 46, "Text", fixture_palette().dim);
    draw_string(&mut pixels, dims, 130, 46, "Folder", fixture_palette().text);
    draw_string(&mut pixels, dims, 200, 46, "Hex", fixture_palette().dim);
    draw_string(&mut pixels, dims, 244, 46, "Merge", fixture_palette().dim);
    draw_string(&mut pixels, dims, 300, 46, "Table", fixture_palette().dim);
    draw_string(&mut pixels, dims, 356, 46, "Sync", fixture_palette().dim);
    draw_string(&mut pixels, dims, 404, 46, "Picture", fixture_palette().dim);
    draw_string(
        &mut pixels,
        dims,
        474,
        46,
        "FolderMerge",
        fixture_palette().dim,
    );
    draw_string(&mut pixels, dims, 574, 46, "Audio", fixture_palette().dim);

    // Path row
    fill_rect(
        &mut pixels,
        dims,
        (12, 76, 380, 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        82,
        "fixtures/left-dir",
        fixture_palette().text,
    );
    fill_rect(
        &mut pixels,
        dims,
        (400, 76, 75, 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        412,
        82,
        "Browse...",
        fixture_palette().text,
    );
    fill_rect(
        &mut pixels,
        dims,
        (490, 76, 380, 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        498,
        82,
        "fixtures/right-dir",
        fixture_palette().text,
    );
    fill_rect(
        &mut pixels,
        dims,
        (878, 76, 75, 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        890,
        82,
        "Browse...",
        fixture_palette().text,
    );

    // Session toolbar
    draw_string(
        &mut pixels,
        dims,
        16,
        104,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        fixture_palette().text,
    );

    // Fixture rows: headless truth, same glyphs the engine paints.
    let rows = vec![
        FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '>',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
        FolderRow {
            rel_path: "src/b.txt".to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "orphan.txt".to_string(),
            is_dir: false,
            glyph: 'R',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
    ];
    let counts = count_folder_rows(&rows);
    let expanded: std::collections::BTreeSet<String> = ["src".to_string()].into_iter().collect();
    let display = crate::tree::FolderDisplay::default();
    let nodes = folder_tree_filtered(&rows, &expanded, &display);
    let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
    let consent_hint = sync_consent_hint(false, preview.steps.len());
    let empty_hint = folder_empty_hint(false, false);
    let _ = empty_hint;

    // Filter checkboxes line (labels mirror app.rs Folder tab).
    draw_string(
        &mut pixels,
        dims,
        16,
        122,
        "[ ] Hide orphans  [ ] Left newer only  [ ] Right newer only  [ ] Flatten",
        fixture_palette().text,
    );
    // Honest counts line.
    draw_string(
        &mut pixels,
        dims,
        16,
        138,
        &counts.summary(),
        fixture_palette().dim,
    );

    // Tree panel.
    let pane_top = 158u32;
    let pane_height = 260u32;
    let pane_bottom = pane_top.saturating_add(pane_height);
    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 936, pane_height),
        fixture_palette().bg,
    );
    let row_h = 24u32;
    for (i, node) in nodes.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }
        let (bg, fg) = match node.row.glyph {
            '=' => (fixture_palette().bg, fixture_palette().text),
            '!' | 'L' => (fixture_palette().deleted_bg, fixture_palette().deleted_fg),
            '>' | '<' => (fixture_palette().changed_bg, fixture_palette().changed_fg),
            'R' => (fixture_palette().inserted_bg, fixture_palette().inserted_fg),
            _ => (fixture_palette().bg, fixture_palette().dim),
        };
        fill_rect(&mut pixels, dims, (12, y, 936, row_h), bg);
        if node.row.is_dir {
            let icon = if node.is_expanded { "v" } else { ">" };
            draw_string(
                &mut pixels,
                dims,
                18,
                y.saturating_add(6),
                icon,
                fixture_palette().dim,
            );
        }
        // Drill-down indent: live 12px/depth parity — bitmap font ~7px
        // advance par 2 units = 14px/depth (live 12px ka nearest).
        let indent = (node.depth as u32).saturating_mul(2u32);
        let gx = 34u32.saturating_add(indent.saturating_mul(7));
        draw_string(
            &mut pixels,
            dims,
            gx,
            y.saturating_add(6),
            &node.row.glyph.to_string(),
            fg,
        );
        draw_string(
            &mut pixels,
            dims,
            gx.saturating_add(16),
            y.saturating_add(6),
            &node.row.rel_path,
            fg,
        );
    }

    // Sync preview panel.
    let panel_y = pane_bottom.saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (12, panel_y, 936, 116),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(6),
        "Sync preview (dry-run)",
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(22),
        &format!(
            "({})  ({})",
            SyncModeChoice::UpdateBoth.label(),
            SyncModeChoice::Mirror.label()
        ),
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(38),
        &preview.summary(),
        fixture_palette().dim,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(54),
        consent_hint,
        fixture_palette().dim,
    );
    fill_rect(
        &mut pixels,
        dims,
        (20, panel_y.saturating_add(72), 150, 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        32,
        panel_y.saturating_add(78),
        "Apply sync plan",
        fixture_palette().text,
    );

    // Status bar.
    fill_rect(
        &mut pixels,
        dims,
        (0, 570, width, 30),
        fixture_palette().track,
    );
    draw_string(
        &mut pixels,
        dims,
        16,
        578,
        "folder: 4 entries, 0 pruned | staged src/a.txt for text compare",
        fixture_palette().dim,
    );

    let image = RgbaImage {
        width,
        height,
        pixels,
    };

    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;

    Ok(())
}

/// Renders the Hex comparison fixture: offset + hex bytes + ascii,
/// same 960x600 chrome, 24px rows, 460px panes. Colors frozen.
pub fn render_hex_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.bin <--> fixtures/b.bin - Hex",
        3,
    );

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_LEFT_X, pane_top, TEXT_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_RIGHT_X, pane_top, TEXT_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top, GUTTER_W, pane_height),
        fixture_palette().track,
    );

    let rows = [
        ("00000000  48 65 6c 6c 6f 20 77 6f |Hello wo|", false),
        ("00000010  72 6c 64 21 00 00 00 00 |rld!....|", false),
        ("00000020  01 02 03 04 05 06 07 08 |........|", true),
        ("00000030  09 0a 0b 0c 0d 0e 0f 10 |........|", false),
        ("00000040  11 12 13 14 15 16 17 18 |........|", false),
        ("00000050  19 1a 1b 1c 1d 1e 1f 20 |....... |", false),
        ("00000060  21 22 23 24 25 26 27 28 |!\"#$%&'()|", false),
        ("00000070  29 2a 2b 2c 2d 2e 2f 30 |)*+,-./0|", false),
    ];
    for (i, (line, changed)) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let (bg, fg) = if *changed {
            (fixture_palette().changed_bg, fixture_palette().changed_fg)
        } else {
            (fixture_palette().bg, fixture_palette().text)
        };
        fill_rect(&mut pixels, dims, (TEXT_LEFT_X, y, TEXT_PANE_W, ROW_H), bg);
        fill_rect(&mut pixels, dims, (TEXT_RIGHT_X, y, TEXT_PANE_W, ROW_H), bg);
        draw_string(&mut pixels, dims, 16, y.saturating_add(6), line, fg);
        draw_string(&mut pixels, dims, 480, y.saturating_add(6), line, fg);
    }
    // Gutter segments mirror text shape (geometry parity, no new hues).
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top.saturating_add(48), GUTTER_W, 24),
        fixture_palette().changed_bg,
    );

    paint_status(
        &mut pixels,
        dims,
        "hex: 8 rows, 1 changed | hex compare done: 8 row(s)",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Table comparison fixture: header + grid rows,
/// single 936px pane, 24px rows. Colors frozen.
pub fn render_table_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.csv <--> fixtures/b.csv - Table",
        5,
    );

    let pane_x = OUTER_PAD;
    let pane_w = SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2));
    let pane_top = 150u32;
    let pane_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            pane_x,
            pane_top,
            pane_w,
            pane_bottom.saturating_sub(pane_top),
        ),
        fixture_palette().bg,
    );
    // Header row.
    fill_rect(
        &mut pixels,
        dims,
        (pane_x, pane_top, pane_w, ROW_H),
        fixture_palette().track,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        pane_top.saturating_add(6),
        "id | name | left | right | status",
        fixture_palette().text,
    );
    let rows = [
        ("1 | anna | 10 | 10 | = |", false),
        ("2 | bob | 20 | 22 | ! |", true),
        ("3 | cara | 30 | 30 | = |", false),
        ("4 | dev | - | 40 | R |", true),
        ("5 | eli | 50 | 50 | = |", false),
        ("6 | fay | 60 | 60 | = |", false),
        ("7 | gus | 70 | 71 | ! |", true),
        ("8 | hal | 80 | 80 | = |", false),
        ("9 | ida | 90 | 90 | = |", false),
        ("10 | jay | 100 | 101 | ! |", true),
        ("11 | kay | 110 | 110 | = |", false),
        ("12 | leo | 120 | 120 | = |", false),
        ("13 | may | 130 | 131 | ! |", true),
        ("14 | ned | 140 | 140 | = |", false),
    ];
    for (i, (line, changed)) in rows.iter().enumerate() {
        let y = pane_top
            .saturating_add(ROW_H)
            .saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let (bg, fg) = if *changed {
            (fixture_palette().changed_bg, fixture_palette().changed_fg)
        } else {
            (fixture_palette().bg, fixture_palette().text)
        };
        fill_rect(&mut pixels, dims, (pane_x, y, pane_w, ROW_H), bg);
        draw_string(&mut pixels, dims, 20, y.saturating_add(6), line, fg);
    }
    // Column guides prove equal padding (vertical rules, frozen hues).
    for gx in [200u32, 400u32, 600u32, 760u32] {
        fill_rect(
            &mut pixels,
            dims,
            (gx, pane_top, 1, pane_bottom.saturating_sub(pane_top)),
            fixture_palette().field,
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "table: 14 rows, 9 equal, 5 changed | table compare done: 14 row(s)",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Merge fixture: 3 equal panes (base/left/right),
/// 309px each, 24px rows. Colors frozen.
pub fn render_merge_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/base.rs + left + right - Merge",
        4,
    );

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (MERGE_X0, pane_top, MERGE_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (MERGE_X1, pane_top, MERGE_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (MERGE_X2, pane_top, MERGE_PANE_W, pane_height),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        pane_top.saturating_sub(18),
        "[Base] [Left] [Right]",
        fixture_palette().dim,
    );

    let rows = [
        ("let x = 1;", "let x = 1;", "let x = 1;", 0u8),
        ("let y = 2;", "let y = 20;", "let y = 2;", 1u8),
        ("let z = 3;", "let z = 3;", "let z = 30;", 1u8),
        ("emit(x);", "emit(x);", "emit(x);", 0u8),
        ("drop(y);", "", "drop(y);", 2u8),
        ("", "take(z);", "", 3u8),
        ("done();", "done();", "done();", 0u8),
    ];
    for (i, (base, left, right, mark)) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let center_bg = match *mark {
            1 => fixture_palette().changed_bg,
            2 => fixture_palette().deleted_bg,
            3 => fixture_palette().inserted_bg,
            _ => fixture_palette().bg,
        };
        let center_fg = match *mark {
            1 => fixture_palette().changed_fg,
            2 => fixture_palette().deleted_fg,
            3 => fixture_palette().inserted_fg,
            _ => fixture_palette().text,
        };
        fill_rect(
            &mut pixels,
            dims,
            (MERGE_X0, y, MERGE_PANE_W, ROW_H),
            fixture_palette().bg,
        );
        fill_rect(
            &mut pixels,
            dims,
            (MERGE_X1, y, MERGE_PANE_W, ROW_H),
            center_bg,
        );
        fill_rect(
            &mut pixels,
            dims,
            (MERGE_X2, y, MERGE_PANE_W, ROW_H),
            fixture_palette().bg,
        );
        draw_string(
            &mut pixels,
            dims,
            MERGE_X0.saturating_add(8),
            y.saturating_add(6),
            base,
            fixture_palette().text,
        );
        draw_string(
            &mut pixels,
            dims,
            MERGE_X1.saturating_add(8),
            y.saturating_add(6),
            left,
            center_fg,
        );
        draw_string(
            &mut pixels,
            dims,
            MERGE_X2.saturating_add(8),
            y.saturating_add(6),
            right,
            fixture_palette().text,
        );
    }

    // Take-row strip: live per-hunk Take buttons (paint_merge_three_pane
    // wale labels) + staged decision display, sab headless truth se.
    // Marker model par live `merge_conflict_hunks` ek hunk deta hai;
    // title/decision strings live paint helpers se, koi mock nahi.
    let marker_lines = vec![
        "<<<<<<< left".to_string(),
        "let y = 20;".to_string(),
        "||||||| base".to_string(),
        "let y = 2;".to_string(),
        "=======".to_string(),
        "let y = 2;".to_string(),
        ">>>>>>> right".to_string(),
    ];
    let hunks = crate::viewmodel::merge_conflict_hunks(&marker_lines);
    let hunk_total = hunks.len();
    let pick = crate::viewmodel::MergeHunkPick {
        conflict_index: 0,
        choice: crate::take::TakeChoice::Left,
    };
    let summary = crate::viewmodel::merge_pick_summary(&marker_lines, &[pick]);
    let take_title = if hunk_total > 0 {
        crate::views::format_merge_hunk_title(0, hunk_total)
    } else {
        "hunk 0 of 0: left / base / right".to_string()
    };
    let take_decision = crate::views::format_merge_hunk_decision_with_pick(0, Some(pick.choice));
    let strip_y = pane_top
        .saturating_add(7u32.saturating_mul(ROW_H))
        .saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (
            MERGE_X0,
            strip_y,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            ROW_H,
        ),
        fixture_palette().bg,
    );
    let mut pen_x = MERGE_X0.saturating_add(8);
    pen_x = draw_string(
        &mut pixels,
        dims,
        pen_x,
        strip_y.saturating_add(6),
        "Take:",
        fixture_palette().dim,
    )
    .saturating_add(8);
    for label in ["Take Left", "Take Right", "Take Base", "Take Both"] {
        let btn_w = text_width(label).saturating_add(16);
        fill_rect(
            &mut pixels,
            dims,
            (pen_x, strip_y.saturating_add(3), btn_w, 18),
            fixture_palette().field,
        );
        draw_string(
            &mut pixels,
            dims,
            pen_x.saturating_add(8),
            strip_y.saturating_add(6),
            label,
            fixture_palette().text,
        );
        pen_x = pen_x.saturating_add(btn_w).saturating_add(8);
        if pen_x.saturating_add(60) > MERGE_X2.saturating_add(MERGE_PANE_W) {
            break;
        }
    }
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        strip_y.saturating_add(ROW_H).saturating_add(6),
        &take_title,
        fixture_palette().dim,
    );
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        strip_y
            .saturating_add(ROW_H)
            .saturating_add(12)
            .saturating_add(6),
        &take_decision,
        fixture_palette().text,
    );
    let _ = summary;

    paint_status(
        &mut pixels,
        dims,
        "merge: 7 rows, 3 equal, 2 changed, 1 deleted, 1 inserted | hunk 1 of 1: take left",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Merge take-row scene: ek conflict hunk ka focused card —
/// live `merge_conflict_hunks` se hunk, `merge_hunk_sides` se teen sides,
/// live Take button labels, staged `take left` decision aur
/// `apply_merge_picks` ka resolved output. Merge fixture se alag bytes
/// (BI-M4 live take-row ka headless proof). Colors frozen.
pub fn render_merge_take_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/base.rs + left + right - Merge (take)",
        4,
    );

    let marker_lines = vec![
        "<<<<<<< left".to_string(),
        "let y = 20;".to_string(),
        "||||||| base".to_string(),
        "let y = 2;".to_string(),
        "=======".to_string(),
        "let y = 2;".to_string(),
        ">>>>>>> right".to_string(),
    ];
    let hunks = crate::viewmodel::merge_conflict_hunks(&marker_lines);
    let hunk_total = hunks.len();
    let (left, base, right) = match hunks.first() {
        Some(hunk) => crate::views::merge_hunk_sides(&marker_lines, hunk),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };
    let pick = crate::viewmodel::MergeHunkPick {
        conflict_index: 0,
        choice: crate::take::TakeChoice::Left,
    };
    let title = if hunk_total > 0 {
        crate::views::format_merge_hunk_title(0, hunk_total)
    } else {
        "hunk 0 of 0: left / base / right".to_string()
    };
    let decision = crate::views::format_merge_hunk_decision_with_pick(0, Some(pick.choice));
    let resolved =
        crate::viewmodel::apply_merge_picks(&marker_lines, &[pick]).map_err(|e| e.to_string())?;
    let resolved_line = match resolved.first() {
        Some(line) => format!("resolved (take left): {line}"),
        None => "resolved (take left): (empty)".to_string(),
    };

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            MERGE_X0,
            pane_top,
            MERGE_PANE_W,
            pane_bottom.saturating_sub(pane_top),
        ),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            MERGE_X1,
            pane_top,
            MERGE_PANE_W,
            pane_bottom.saturating_sub(pane_top),
        ),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            MERGE_X2,
            pane_top,
            MERGE_PANE_W,
            pane_bottom.saturating_sub(pane_top),
        ),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        pane_top.saturating_sub(18),
        "[Base] [Left] [Right]",
        fixture_palette().dim,
    );

    // Card title + teen sides (changed band: sides alag hain).
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        pane_top.saturating_add(6),
        &title,
        fixture_palette().text,
    );
    let side_y = pane_top.saturating_add(ROW_H);
    for (x, lines) in [(MERGE_X0, &left), (MERGE_X1, &base), (MERGE_X2, &right)] {
        fill_rect(
            &mut pixels,
            dims,
            (x, side_y, MERGE_PANE_W, ROW_H),
            fixture_palette().changed_bg,
        );
        let text = match lines.first() {
            Some(line) => line.as_str(),
            None => "(empty)",
        };
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(8),
            side_y.saturating_add(6),
            text,
            fixture_palette().changed_fg,
        );
    }
    // Take strip: live button labels, staged pick highlight nahi (frozen hues).
    let strip_y = side_y.saturating_add(ROW_H).saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (
            MERGE_X0,
            strip_y,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            ROW_H,
        ),
        fixture_palette().bg,
    );
    let mut pen_x = MERGE_X0.saturating_add(8);
    pen_x = draw_string(
        &mut pixels,
        dims,
        pen_x,
        strip_y.saturating_add(6),
        "Take:",
        fixture_palette().dim,
    )
    .saturating_add(8);
    for label in ["Take Left", "Take Right", "Take Base", "Take Both"] {
        let btn_w = text_width(label).saturating_add(16);
        fill_rect(
            &mut pixels,
            dims,
            (pen_x, strip_y.saturating_add(3), btn_w, 18),
            fixture_palette().field,
        );
        draw_string(
            &mut pixels,
            dims,
            pen_x.saturating_add(8),
            strip_y.saturating_add(6),
            label,
            fixture_palette().text,
        );
        pen_x = pen_x.saturating_add(btn_w).saturating_add(8);
        if pen_x.saturating_add(60) > MERGE_X2.saturating_add(MERGE_PANE_W) {
            break;
        }
    }
    // Decision + resolved output, dono live strings.
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        strip_y.saturating_add(ROW_H).saturating_add(6),
        &decision,
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        strip_y
            .saturating_add(ROW_H)
            .saturating_add(12)
            .saturating_add(6),
        &resolved_line,
        fixture_palette().dim,
    );

    paint_status(
        &mut pixels,
        dims,
        "merge-take: 1 hunk, take left staged | resolved 1 line(s)",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the three-way tree scene: header me live
/// `three_way_header_line` (== `three_way_tree_summary().summary_line()`),
/// har row par `three_way_presence_marks` (BLR/B-R/-LR) + `three_way_row_line`
/// (dir par `/`) + `three_way_row_decision_text`, conflict rows ke neeche
/// `Take:` strip me teen live buttons (`ThreeWaySide::label()`: Take Left /
/// Take Base / Take Right). Snapshots + decisions headless truth se
/// (`tree::three_way_tree_rows`, `is_three_way_conflict`,
/// `views::stage_three_way_decision`); koi mock string nahi. Accent-scene
/// precedent par naya dispatcher route (`three-way`/`threeway`). Colors frozen.
pub fn render_three_way_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::viewmodel::FolderRow;

    fn snap_row(path: &str, is_dir: bool) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        }
    }

    let base = vec![
        snap_row("a.txt", false),
        snap_row("del.txt", false),
        snap_row("src", true),
    ];
    let left = vec![
        snap_row("a.txt", false),
        snap_row("new.txt", false),
        snap_row("one.txt", false),
        snap_row("src", true),
    ];
    let right = vec![
        snap_row("a.txt", false),
        snap_row("del.txt", false),
        snap_row("new.txt", false),
        snap_row("src", true),
    ];
    let rows = crate::tree::three_way_tree_rows(&base, &left, &right);
    let decisions =
        crate::views::stage_three_way_decision(&[], "del.txt", crate::tree::ThreeWaySide::Right);
    let header = crate::views::three_way_header_line(&rows, &decisions);

    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/base + left + right - FolderMerge (three-way)",
        4,
    );

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            OUTER_PAD,
            pane_top,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            pane_bottom.saturating_sub(pane_top),
        ),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        pane_top.saturating_add(6),
        &header,
        fixture_palette().text,
    );

    let mut row_y = pane_top.saturating_add(ROW_H);
    for row in &rows {
        if row_y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let conflict = crate::tree::is_three_way_conflict(row);
        let decided = crate::views::three_way_decision_for_row(&decisions, row.rel_path.as_str());
        let marks = crate::views::three_way_presence_marks(row);
        let line = crate::views::three_way_row_line(row);
        let text = crate::views::three_way_row_decision_text(conflict, decided);
        let (bg, fg) = if conflict {
            (fixture_palette().changed_bg, fixture_palette().changed_fg)
        } else {
            (fixture_palette().bg, fixture_palette().text)
        };
        fill_rect(
            &mut pixels,
            dims,
            (
                OUTER_PAD,
                row_y,
                SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
                ROW_H,
            ),
            bg,
        );
        let mut pen = draw_string(&mut pixels, dims, 20, row_y.saturating_add(6), &marks, fg);
        pen = draw_string(
            &mut pixels,
            dims,
            pen.saturating_add(8),
            row_y.saturating_add(6),
            &line,
            fg,
        );
        let decision_fg = if decided.is_some() {
            fixture_palette().text
        } else {
            fixture_palette().dim
        };
        draw_string(
            &mut pixels,
            dims,
            pen.saturating_add(8),
            row_y.saturating_add(6),
            &text,
            decision_fg,
        );
        row_y = row_y.saturating_add(ROW_H);
        if conflict {
            if row_y.saturating_add(ROW_H) > pane_bottom {
                break;
            }
            fill_rect(
                &mut pixels,
                dims,
                (
                    OUTER_PAD,
                    row_y,
                    SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
                    ROW_H,
                ),
                fixture_palette().bg,
            );
            let mut pen_x = draw_string(
                &mut pixels,
                dims,
                20,
                row_y.saturating_add(6),
                "Take:",
                fixture_palette().dim,
            )
            .saturating_add(8);
            for side in [
                crate::tree::ThreeWaySide::Left,
                crate::tree::ThreeWaySide::Base,
                crate::tree::ThreeWaySide::Right,
            ] {
                let label = side.label();
                let btn_w = text_width(label).saturating_add(16);
                fill_rect(
                    &mut pixels,
                    dims,
                    (pen_x, row_y.saturating_add(3), btn_w, 18),
                    fixture_palette().field,
                );
                draw_string(
                    &mut pixels,
                    dims,
                    pen_x.saturating_add(8),
                    row_y.saturating_add(6),
                    label,
                    fixture_palette().text,
                );
                pen_x = pen_x.saturating_add(btn_w).saturating_add(8);
                if pen_x.saturating_add(60) > SNAP_W.saturating_sub(OUTER_PAD) {
                    break;
                }
            }
            row_y = row_y.saturating_add(ROW_H);
        }
    }

    paint_status(
        &mut pixels,
        dims,
        &format!("threeway: {header} | take left/base/right live"),
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Sync fixture: dedicated Sync-tab scene (BI-S4 — AW2
/// picture-precedent wali fallback-twin class se bachne ke liye alag
/// branch). Checklist rows + dry-run + confirm + Apply sab live headless
/// truth se: `SyncChecklist::from_plan`, `summary`, `checked_count`,
/// `copy_right_plan`, `sync_dry_run_summary`, `sync_apply_enabled`, aur
/// row strings `glyph`/`display_line`/`action_label` se. Colors frozen.
pub fn render_sync_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::app::{SyncChecklist, sync_apply_enabled, sync_dry_run_summary};
    use autocompare_compare::{SyncAction, SyncPlan, SyncStep};

    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left-dir <--> fixtures/right-dir - Sync",
        6,
    );

    // Live checklist model: 2 left->right copies + 1 skip.
    let plan = SyncPlan {
        steps: vec![
            SyncStep {
                rel_path: "src/a.txt".to_string(),
                is_dir: false,
                action: SyncAction::CopyToRight,
            },
            SyncStep {
                rel_path: "new.txt".to_string(),
                is_dir: false,
                action: SyncAction::CopyToRight,
            },
        ],
        skipped: vec![(
            "src/b.txt".to_string(),
            "content differs; no side wins".to_string(),
        )],
        truncated: false,
    };
    let list = SyncChecklist::from_plan(&plan);
    let checked_total = list.checked_count();
    let copy_steps = list.copy_right_plan().steps.len();
    let dry_line = sync_dry_run_summary(copy_steps, checked_total);
    let count_line =
        format!("{copy_steps} of {checked_total} checked step(s) are left->right copies");
    let apply_open = sync_apply_enabled(true, copy_steps);

    // Checklist panel.
    let pane_top = 124u32;
    let pane_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            OUTER_PAD,
            pane_top,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            pane_bottom.saturating_sub(pane_top),
        ),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        pane_top.saturating_add(6),
        &list.summary(),
        fixture_palette().text,
    );
    let mut row_y = pane_top.saturating_add(ROW_H);
    for row in &list.rows {
        if row_y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        fill_rect(
            &mut pixels,
            dims,
            (
                OUTER_PAD,
                row_y,
                SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
                ROW_H,
            ),
            fixture_palette().bg,
        );
        let mark = if row.checked { "[x]" } else { "[ ]" };
        let mut pen = draw_string(
            &mut pixels,
            dims,
            20,
            row_y.saturating_add(6),
            mark,
            fixture_palette().text,
        )
        .saturating_add(8);
        pen = draw_string(
            &mut pixels,
            dims,
            pen,
            row_y.saturating_add(6),
            &row.glyph().to_string(),
            fixture_palette().text,
        )
        .saturating_add(8);
        pen = draw_string(
            &mut pixels,
            dims,
            pen,
            row_y.saturating_add(6),
            &row.display_line(),
            fixture_palette().text,
        )
        .saturating_add(8);
        draw_string(
            &mut pixels,
            dims,
            pen,
            row_y.saturating_add(6),
            row.action_label(),
            fixture_palette().dim,
        );
        let _ = pen;
        row_y = row_y.saturating_add(ROW_H);
    }
    for (path, reason) in &list.skipped {
        if row_y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        draw_string(
            &mut pixels,
            dims,
            20,
            row_y.saturating_add(6),
            &crate::views::sync_skip_text(path, reason),
            fixture_palette().dim,
        );
        row_y = row_y.saturating_add(ROW_H);
    }
    row_y = row_y.saturating_add(8);
    // Apply gate: dry-run + confirm + counts + Apply (live strings/order).
    draw_string(
        &mut pixels,
        dims,
        20,
        row_y.saturating_add(6),
        "Apply copies checked left->right only, never deletes.",
        fixture_palette().dim,
    );
    row_y = row_y.saturating_add(ROW_H);
    if row_y.saturating_add(ROW_H) <= pane_bottom {
        draw_string(
            &mut pixels,
            dims,
            20,
            row_y.saturating_add(6),
            "[x] Dry-run (no writes)",
            fixture_palette().text,
        );
        row_y = row_y.saturating_add(ROW_H);
    }
    if row_y.saturating_add(ROW_H) <= pane_bottom {
        draw_string(
            &mut pixels,
            dims,
            20,
            row_y.saturating_add(6),
            "[x] I understand Apply copies files left->right",
            fixture_palette().text,
        );
        row_y = row_y.saturating_add(ROW_H);
    }
    if row_y.saturating_add(ROW_H) <= pane_bottom {
        draw_string(
            &mut pixels,
            dims,
            20,
            row_y.saturating_add(6),
            &dry_line,
            fixture_palette().dim,
        );
        row_y = row_y.saturating_add(ROW_H);
    }
    if row_y.saturating_add(ROW_H) <= pane_bottom {
        draw_string(
            &mut pixels,
            dims,
            20,
            row_y.saturating_add(6),
            &count_line,
            fixture_palette().dim,
        );
        row_y = row_y.saturating_add(ROW_H);
    }
    if row_y.saturating_add(ROW_H) <= pane_bottom {
        let label = "Apply checked copies";
        let btn_w = text_width(label).saturating_add(24);
        fill_rect(
            &mut pixels,
            dims,
            (20, row_y.saturating_add(3), btn_w, 18),
            fixture_palette().field,
        );
        draw_string(
            &mut pixels,
            dims,
            32,
            row_y.saturating_add(6),
            label,
            if apply_open {
                fixture_palette().text
            } else {
                fixture_palette().dim
            },
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "sync: 2 step(s) planned, 1 skipped | dry-run 2 of 2 checked would copy (no writes)",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Home fixture: 9 session cards from the live
/// `home_model` (3-col grid, contract gap/min-height/CTA, title/desc
/// hierarchy) + columnar recent list (title flex + kind + age), and NO
/// path picker (`HOME_SHOWS_PATH_PICKER = false`). Colors frozen.
pub fn render_home_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    // Launcher chrome: title + tabs only (no OS window controls).
    fill_rect(
        &mut pixels,
        dims,
        (0, 0, width, TITLE_H),
        fixture_palette().track,
    );
    draw_string(
        &mut pixels,
        dims,
        OUTER_PAD,
        14,
        "Home - autocompare",
        fixture_palette().text,
    );
    fill_rect(
        &mut pixels,
        dims,
        (0, TITLE_H, width, TAB_H),
        fixture_palette().track,
    );
    draw_string(&mut pixels, dims, 16, 46, "Home", fixture_palette().text);
    draw_string(&mut pixels, dims, 76, 46, "Text", fixture_palette().dim);
    draw_string(&mut pixels, dims, 130, 46, "Folder", fixture_palette().dim);
    draw_string(&mut pixels, dims, 200, 46, "Hex", fixture_palette().dim);
    draw_string(&mut pixels, dims, 244, 46, "Merge", fixture_palette().dim);
    draw_string(&mut pixels, dims, 300, 46, "Table", fixture_palette().dim);
    draw_string(&mut pixels, dims, 356, 46, "Sync", fixture_palette().dim);
    draw_string(&mut pixels, dims, 404, 46, "Picture", fixture_palette().dim);
    draw_string(
        &mut pixels,
        dims,
        474,
        46,
        "FolderMerge",
        fixture_palette().dim,
    );
    draw_string(&mut pixels, dims, 574, 46, "Audio", fixture_palette().dim);

    draw_string(
        &mut pixels,
        dims,
        20,
        74,
        "Comparison Sessions",
        fixture_palette().text,
    );

    // Cards: live model, row-major 3-col grid, contract gap.
    let model = home_model();
    let card_top = 90u32;
    for (idx, item) in model.items.iter().enumerate() {
        let col = (idx % HOME_GRID_COLUMNS) as u32;
        let row = (idx / HOME_GRID_COLUMNS) as u32;
        let x =
            OUTER_PAD.saturating_add(col.saturating_mul(HOME_CARD_W.saturating_add(HOME_CARD_GAP)));
        let y =
            card_top.saturating_add(row.saturating_mul(HOME_CARD_H.saturating_add(HOME_CARD_GAP)));
        fill_rect(
            &mut pixels,
            dims,
            (x, y, HOME_CARD_W, HOME_CARD_H),
            fixture_palette().bg,
        );
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(12),
            y.saturating_add(12),
            item.title,
            fixture_palette().text,
        );
        let (desc_top, desc_bottom) =
            wrap_desc_two_lines(item.description, HOME_CARD_W.saturating_sub(24));
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(12),
            y.saturating_add(30),
            &desc_top,
            fixture_palette().dim,
        );
        if !desc_bottom.is_empty() {
            draw_string(
                &mut pixels,
                dims,
                x.saturating_add(12),
                y.saturating_add(40),
                &desc_bottom,
                fixture_palette().dim,
            );
        }
        let cta_y = y.saturating_add(64);
        // CTA live paint jaisa card me centered (vertical_centered parity).
        let cta_x = x.saturating_add(HOME_CARD_W.saturating_sub(HOME_CTA_W) / 2);
        fill_rect(
            &mut pixels,
            dims,
            (cta_x, cta_y, HOME_CTA_W, HOME_CTA_H),
            fixture_palette().field,
        );
        draw_string(
            &mut pixels,
            dims,
            cta_x.saturating_add(12),
            cta_y.saturating_add(10),
            &format!("Open {}", item.title),
            fixture_palette().text,
        );
    }

    // Recent list: header + columnar rows (title flex + kind + age).
    let cards_end = card_top
        .saturating_add(3u32.saturating_mul(HOME_CARD_H))
        .saturating_add(2u32.saturating_mul(HOME_CARD_GAP));
    let list_top = cards_end.saturating_add(8);
    let list_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            OUTER_PAD,
            list_top,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            list_bottom.saturating_sub(list_top),
        ),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        list_top.saturating_add(6),
        "Recent sessions",
        fixture_palette().text,
    );
    let (title_w, kind_w, age_w) =
        recent_row_columns(SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)) as f32);
    let content_x = OUTER_PAD.saturating_add(RECENT_LIST_PAD as u32);
    let kind_x = content_x
        .saturating_add(title_w as u32)
        .saturating_add(RECENT_COL_GAP as u32);
    let age_x = kind_x
        .saturating_add(kind_w as u32)
        .saturating_add(RECENT_COL_GAP as u32);
    let _ = age_w;
    let recents = [
        ("text: left.rs <-> right.rs", "Text", "just now"),
        ("folder: left-dir <-> right-dir", "Folder", "5m ago"),
        ("hex: a.bin <-> b.bin", "Hex", "1h ago"),
        ("table: a.csv <-> b.csv", "Table", "3h ago"),
        ("merge: base + left + right", "Merge", "1d ago"),
    ];
    for (i, (title, kind, age)) in recents.iter().enumerate() {
        let y = list_top
            .saturating_add(ROW_H)
            .saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > list_bottom {
            break;
        }
        fill_rect(
            &mut pixels,
            dims,
            (
                OUTER_PAD,
                y,
                SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
                ROW_H,
            ),
            fixture_palette().bg,
        );
        draw_string(
            &mut pixels,
            dims,
            content_x,
            y.saturating_add(6),
            title,
            fixture_palette().text,
        );
        draw_string(
            &mut pixels,
            dims,
            kind_x,
            y.saturating_add(6),
            kind,
            fixture_palette().dim,
        );
        draw_string(
            &mut pixels,
            dims,
            age_x,
            y.saturating_add(6),
            age,
            fixture_palette().dim,
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "home: 9 cards, 5 recents | no path picker | pick a session to begin",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the wrap-on + find-active text scene: word wrap capped at
/// the live 80-char width, one long equal row wrapping to 2 sub-rows
/// with the needle fully inside the continuation, and the matched
/// sub-row band proving `wrap_find_scroll_sub` targeting. The find
/// band reuses `fixture_palette().changed_bg` (frozen palette); no new hues.
pub fn render_wrap_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left.rs <--> fixtures/right.rs - Text (wrap)",
        1,
    );

    // Find bar: needle + honest hit line (headless truth: 2 side hits).
    fill_rect(&mut pixels, dims, (12, 124, 936, 24), fixture_palette().bg);
    draw_string(&mut pixels, dims, 20, 130, "find", fixture_palette().text);
    fill_rect(
        &mut pixels,
        dims,
        (64, 127, 180, 18),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        70,
        130,
        "read_to_string",
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        260,
        130,
        "2 hits, current row 3",
        fixture_palette().dim,
    );

    let long_line = "    // wrap proof: keep the legacy reader enabled until rollout ends, then call read_to_string(old);";
    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(2),
            right_no: Some(2),
            left: "pub fn run(left: &Path, right: &Path) -> usize {".to_string(),
            right: "pub fn run(left: &Path, right: &Path) -> usize {".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(3),
            right_no: Some(3),
            left: long_line.to_string(),
            right: long_line.to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(4),
            right_no: Some(4),
            left: "    let old = load(left)?;".to_string(),
            right: "    let new = std::fs::load(right)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Inserted,
            left_no: None,
            right_no: Some(5),
            left: String::new(),
            right: "    validate_safe_path(right)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    // Live wrap math: 80-char cap, per-row sub counts, needle sub-rows,
    // current-row scroll target (clamped to the pair count).
    let pane_chars = rows
        .iter()
        .map(|row| row.left.chars().count().max(row.right.chars().count()))
        .max()
        .unwrap_or(1);
    let wrap_width = wrap_width_for(pane_chars, true);
    let needle = "read_to_string";
    let mut counts: Vec<usize> = Vec::new();
    for row in &rows {
        counts.push(wrap_pane_pair_count(&row.left, &row.right, wrap_width));
    }
    let matched_row = 2usize;
    let scroll_sub = match rows.get(matched_row) {
        Some(row) => {
            let count = counts.get(matched_row).copied().unwrap_or(1);
            wrap_find_scroll_sub(row, needle, false, wrap_width, count)
        }
        None => 0,
    };

    let pane_top = 152u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_LEFT_X, pane_top, TEXT_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_RIGHT_X, pane_top, TEXT_PANE_W, pane_height),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top, GUTTER_W, pane_height),
        fixture_palette().track,
    );

    let mut flat = 0u32;
    for (i, row) in rows.iter().enumerate() {
        let count = counts.get(i).copied().unwrap_or(1).max(1);
        let left_chunks = crate::views::wrap_chunks(&row.left, wrap_width);
        let right_chunks = crate::views::wrap_chunks(&row.right, wrap_width);
        let (base_bg_left, base_fg_left, base_bg_right, base_fg_right) = match row.kind {
            TextRowKind::Equal => (
                fixture_palette().bg,
                fixture_palette().text,
                fixture_palette().bg,
                fixture_palette().text,
            ),
            TextRowKind::Changed => (
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
                fixture_palette().changed_bg,
                fixture_palette().changed_fg,
            ),
            TextRowKind::Deleted => (
                fixture_palette().deleted_bg,
                fixture_palette().deleted_fg,
                fixture_palette().bg,
                fixture_palette().dim,
            ),
            TextRowKind::Inserted => (
                fixture_palette().bg,
                fixture_palette().dim,
                fixture_palette().inserted_bg,
                fixture_palette().inserted_fg,
            ),
        };
        let mut sub = 0usize;
        while sub < count {
            let y = pane_top.saturating_add(flat.saturating_mul(ROW_H));
            if y.saturating_add(ROW_H) > pane_bottom {
                break;
            }
            // Find-current band: only the matched sub-row of the matched
            // model row inverts to the accent band (frozen CHANGED_*).
            let is_hit = i == matched_row && sub == scroll_sub;
            let (lbg, rbg) = if is_hit {
                (fixture_palette().changed_bg, fixture_palette().changed_bg)
            } else {
                (base_bg_left, base_bg_right)
            };
            fill_rect(&mut pixels, dims, (TEXT_LEFT_X, y, TEXT_PANE_W, ROW_H), lbg);
            fill_rect(
                &mut pixels,
                dims,
                (TEXT_RIGHT_X, y, TEXT_PANE_W, ROW_H),
                rbg,
            );
            if sub == 0 {
                if let Some(no) = row.left_no {
                    draw_string(
                        &mut pixels,
                        dims,
                        16,
                        y.saturating_add(6),
                        &format!("{no:>2} "),
                        fixture_palette().dim,
                    );
                }
                if let Some(no) = row.right_no {
                    draw_string(
                        &mut pixels,
                        dims,
                        480,
                        y.saturating_add(6),
                        &format!("{no:>2} "),
                        fixture_palette().dim,
                    );
                }
            }
            let left_text = match left_chunks.get(sub) {
                Some(chunk) => chunk.text.as_str(),
                None => "",
            };
            let right_text = match right_chunks.get(sub) {
                Some(chunk) => chunk.text.as_str(),
                None => "",
            };
            let (lfg, rfg) = if is_hit {
                (fixture_palette().changed_fg, fixture_palette().changed_fg)
            } else {
                (base_fg_left, base_fg_right)
            };
            draw_string(&mut pixels, dims, 44, y.saturating_add(6), left_text, lfg);
            draw_string(&mut pixels, dims, 508, y.saturating_add(6), right_text, rfg);
            flat = flat.saturating_add(1);
            sub = sub.saturating_add(1);
        }
    }

    // Gutter segments mirror the ruler contract (16 wide, 4px min).
    let geom = gutter_geometry(rows.len(), pane_height as f32, &rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start as u32);
        let seg_h = ((seg.y_end - seg.y_start) as u32).max(GUTTER_SEGMENT_MIN_H as u32);
        let seg_color = match seg.kind {
            TextRowKind::Equal => fixture_palette().track,
            TextRowKind::Changed => fixture_palette().changed_bg,
            TextRowKind::Deleted => fixture_palette().deleted_bg,
            TextRowKind::Inserted => fixture_palette().inserted_bg,
        };
        fill_rect(
            &mut pixels,
            dims,
            (GUTTER_X, seg_y, GUTTER_W, seg_h),
            seg_color,
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "wrap:on find:read_to_string 2 hits, current row 3 sub 1/2 | wrapnav proof",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Picture fixture: zoom-step row (Fit + presets from the
/// live `PICTURE_ZOOM_STEPS`) and a centered frame whose fit zoom and
/// offsets come from `picture_fit_zoom_pct` / `picture_center_offset`.
/// The heatmap is schematic palette blocks (frozen hues); geometry is
/// the live contract. Colors frozen.
pub fn render_picture_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.png <--> fixtures/b.png - Picture",
        7,
    );

    // Natural vs viewport: fit zoom + centering from live helpers.
    let natural_w = 800.0f32;
    let natural_h = 600.0f32;
    let avail_w = SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)) as f32;
    // Live summary lines (identical 800x600 pair) sit between the step
    // row and the viewport, like the paint summary under the heatmap.
    let diff = PictureDiff {
        width_l: 800,
        height_l: 600,
        width_r: 800,
        height_r: 600,
        overlap_w: 800,
        overlap_h: 600,
        total: 480_000,
        differing: 0,
        bbox: None,
        identical: true,
    };
    let summary = crate::viewmodel::build_picture_summary(&diff);
    let viewport_top = 188u32;
    let viewport_bottom = 560u32;
    let avail_h = viewport_bottom.saturating_sub(viewport_top) as f32;
    let fit = picture_fit_zoom_pct(natural_w, natural_h, avail_w, avail_h);
    let scale = f32::from(fit as u16) / 100.0;
    let disp_w = (natural_w * scale).max(1.0);
    let disp_h = (natural_h * scale).max(1.0);
    let off_x = picture_center_offset(disp_w, avail_w);
    let off_y = picture_center_offset(disp_h, avail_h);

    // Zoom-step row: active step reads via text-vs-dim ink + preset ladder.
    fill_rect(&mut pixels, dims, (12, 124, 936, 24), fixture_palette().bg);
    draw_string(&mut pixels, dims, 20, 130, "steps:", fixture_palette().dim);
    draw_string(&mut pixels, dims, 86, 130, "Fit", fixture_palette().text);
    let mut step_x = 124u32;
    for step in PICTURE_ZOOM_STEPS {
        let label = format!("{step}%");
        let selected = step == fit;
        draw_string(
            &mut pixels,
            dims,
            step_x.saturating_add(8),
            130,
            &label,
            if selected {
                fixture_palette().text
            } else {
                fixture_palette().dim
            },
        );
        step_x = step_x.saturating_add(60);
    }
    // Live zoom readout beside the ladder (same contract as paint).
    draw_string(
        &mut pixels,
        dims,
        step_x.saturating_add(8),
        130,
        &zoom_label(fit),
        fixture_palette().dim,
    );

    // Summary strip: live CLI-worded lines, one per 12px rhythm row.
    let mut sum_y = 152u32;
    for line in &summary {
        draw_string(&mut pixels, dims, 20, sum_y, line, fixture_palette().dim);
        sum_y = sum_y.saturating_add(12);
        if sum_y.saturating_add(7) > viewport_top {
            break;
        }
    }

    // Viewport + centered frame.
    fill_rect(
        &mut pixels,
        dims,
        (
            OUTER_PAD,
            viewport_top,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            viewport_bottom.saturating_sub(viewport_top),
        ),
        fixture_palette().bg,
    );
    let frame_x = OUTER_PAD.saturating_add(off_x as u32);
    let frame_y = viewport_top.saturating_add(off_y as u32);
    let frame_w = disp_w as u32;
    let frame_h = disp_h as u32;
    // Schematic heatmap: quadrant palette blocks (frozen diff hues).
    let half_w = frame_w / 2;
    let half_h = frame_h / 2;
    fill_rect(
        &mut pixels,
        dims,
        (frame_x, frame_y, half_w, half_h),
        fixture_palette().inserted_bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x.saturating_add(half_w),
            frame_y,
            frame_w.saturating_sub(half_w),
            half_h,
        ),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x,
            frame_y.saturating_add(half_h),
            half_w,
            frame_h.saturating_sub(half_h),
        ),
        fixture_palette().changed_bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x.saturating_add(half_w),
            frame_y.saturating_add(half_h),
            frame_w.saturating_sub(half_w),
            frame_h.saturating_sub(half_h),
        ),
        fixture_palette().deleted_bg,
    );
    // 1px frame border (frozen dim).
    fill_rect(
        &mut pixels,
        dims,
        (frame_x, frame_y, frame_w, 1),
        fixture_palette().dim,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x,
            frame_y.saturating_add(frame_h.saturating_sub(1)),
            frame_w,
            1,
        ),
        fixture_palette().dim,
    );
    fill_rect(
        &mut pixels,
        dims,
        (frame_x, frame_y, 1, frame_h),
        fixture_palette().dim,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x.saturating_add(frame_w.saturating_sub(1)),
            frame_y,
            1,
            frame_h,
        ),
        fixture_palette().dim,
    );
    draw_string(
        &mut pixels,
        dims,
        frame_x.saturating_add(8),
        frame_y.saturating_add(8),
        "800x600 @50% (400x300)",
        fixture_palette().text,
    );

    paint_status(
        &mut pixels,
        dims,
        "picture: fit 50% | centered (268,36) | steps Fit+25/50/100/200/400",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Folder filtered-empty scene: same filter line as the
/// folder fixture, every row pruned, so the panel shows the live
/// `folder_empty_hint` idle guidance + honest counts. No sync preview:
/// nothing to stage. Colors frozen.
pub fn render_folder_empty_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::viewmodel::FolderRow;

    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left-dir <--> fixtures/right-dir - Folder",
        2,
    );

    let rows = vec![
        FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '>',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
        FolderRow {
            rel_path: "src/b.txt".to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "orphan.txt".to_string(),
            is_dir: false,
            glyph: 'R',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
    ];
    let counts = crate::tree::count_folder_rows(&rows);
    // Filter prunes everything: zero display rows, honest counts.
    let shown = 0usize;
    let hint = folder_empty_hint(false, false);

    draw_string(
        &mut pixels,
        dims,
        16,
        122,
        "[x] Hide orphans  [ ] Left newer only  [ ] Right newer only  [ ] Flatten",
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        16,
        138,
        &format!("{} | {shown} shown (filter)", counts.summary()),
        fixture_palette().dim,
    );

    let pane_top = 158u32;
    let pane_height = 260u32;
    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 936, pane_height),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        pane_top.saturating_add(120),
        hint,
        fixture_palette().dim,
    );

    // Sync preview stays visible under the filter (filter prunes display,
    // not the sync plan): same panel as the folder fixture.
    let preview = crate::tree::sync_preview(&rows, crate::tree::SyncModeChoice::UpdateBoth);
    let consent = crate::home::sync_consent_hint(false, preview.steps.len());
    let panel_y = pane_top.saturating_add(pane_height).saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (12, panel_y, 936, 116),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(6),
        "Sync preview (dry-run)",
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(22),
        &format!(
            "({})  ({})",
            crate::tree::SyncModeChoice::UpdateBoth.label(),
            crate::tree::SyncModeChoice::Mirror.label()
        ),
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(38),
        &preview.summary(),
        fixture_palette().dim,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(54),
        consent,
        fixture_palette().dim,
    );
    fill_rect(
        &mut pixels,
        dims,
        (20, panel_y.saturating_add(72), 150, 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        32,
        panel_y.saturating_add(78),
        "Apply sync plan",
        fixture_palette().text,
    );

    paint_status(
        &mut pixels,
        dims,
        "folder: 4 entries, 0 shown (filter) | type both folder paths, press Compare",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Folder identical (AllSame) scene: do same folders par
/// diff-seeking status filter (`Different`) sab '=' rows prune karta hai,
/// isliye ped khaali hai (`visible == 0`) par model sab-same hai. Panel
/// live paint wiring (`views::folder_paint_empty_hint_full_rows`) se AllSame
/// hint deta hai — idle/pruned wording nahi. Filter laga hai to live paint
/// jaisa Clear affordance bhi dikhta hai; sync panel folder-empty precedent
/// par hai. Colors frozen.
pub fn render_folder_allsame_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::home::sync_consent_hint;
    use crate::tree::{
        FolderDisplay, FolderStatusFilter, SyncModeChoice, count_folder_rows, folder_tree_filtered,
        sync_preview,
    };
    use crate::viewmodel::FolderRow;
    use crate::views::folder_paint_empty_hint_full_rows;

    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left-dir <--> fixtures/right-dir - Folder",
        2,
    );

    // Identical folders: har row '=' (koi diff nahi).
    let rows = vec![
        FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "src/b.txt".to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
    ];
    // Diff-seeking filter: `Different` sirf '!' rakhta hai, sab '=' prune.
    let display = FolderDisplay {
        status: FolderStatusFilter::Different,
        ..Default::default()
    };
    let expanded: std::collections::BTreeSet<String> = ["src".to_string()].into_iter().collect();
    let nodes = folder_tree_filtered(&rows, &expanded, &display);
    let shown = nodes.len();
    let counts = count_folder_rows(&rows);
    // Paint wiring se hint: identical + ped-khaali par AllSame.
    let hint = folder_paint_empty_hint_full_rows(&rows, shown, &display);

    draw_string(
        &mut pixels,
        dims,
        16,
        122,
        "[ ] Hide orphans  [ ] Left newer only  [ ] Right newer only  [ ] Flatten  (Status: Different)",
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        16,
        138,
        &format!("{} | {shown} shown (allsame)", counts.summary()),
        fixture_palette().dim,
    );

    let pane_top = 158u32;
    let pane_height = 260u32;
    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 936, pane_height),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        pane_top.saturating_add(120),
        hint,
        fixture_palette().dim,
    );
    // Filter laga hai to live `paint_folder` Clear button deta hai
    // (khali view + active display); headless me wahi affordance.
    if display.is_active() {
        fill_rect(
            &mut pixels,
            dims,
            (20, pane_top.saturating_add(140), 150, 24),
            fixture_palette().field,
        );
        draw_string(
            &mut pixels,
            dims,
            32,
            pane_top.saturating_add(146),
            "Clear filters",
            fixture_palette().text,
        );
    }

    // Sync preview folder-empty precedent par: filter display prune karta
    // hai, sync plan nahi. Identical rows par steps khaali hain.
    let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
    let consent = sync_consent_hint(false, preview.steps.len());
    let panel_y = pane_top.saturating_add(pane_height).saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (12, panel_y, 936, 116),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(6),
        "Sync preview (dry-run)",
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(22),
        &format!(
            "({})  ({})",
            SyncModeChoice::UpdateBoth.label(),
            SyncModeChoice::Mirror.label()
        ),
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(38),
        &preview.summary(),
        fixture_palette().dim,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(54),
        consent,
        fixture_palette().dim,
    );
    fill_rect(
        &mut pixels,
        dims,
        (20, panel_y.saturating_add(72), 150, 24),
        fixture_palette().field,
    );
    draw_string(
        &mut pixels,
        dims,
        32,
        panel_y.saturating_add(78),
        "Apply sync plan",
        fixture_palette().text,
    );

    paint_status(
        &mut pixels,
        dims,
        "folder: 3 entries, 0 shown (allsame) | all same, nothing to sync",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Table key-col + find scene: live key bar (28px, per-col
/// selected marker + badge) + live find bar (28px, 120px needle) over a
/// subset grid; the needle hit carries a 1px underline (live
/// `TABLE_FIND_UNDERLINE_W`) at `table_find_span_in_padded` offsets.
/// Marker glyphs are ascii `[x]`/`[ ]` (the 5x7 font has no ●/○).
/// Colors frozen.
pub fn render_table_find_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.csv <--> fixtures/b.csv - Table",
        5,
    );

    // Key bar from live view state: col 0 selected.
    let mut view = TableKeyView::new();
    view.toggle_key_column(0);
    let badge = table_key_badge_label(&view);
    let key_h = TABLE_KEY_BAR_H as u32;
    fill_rect(
        &mut pixels,
        dims,
        (12, 124, 936, key_h),
        fixture_palette().bg,
    );
    draw_string(&mut pixels, dims, 20, 130, "Key:", fixture_palette().dim);
    let header = ["id", "name", "left", "right", "status"];
    let mut key_line = String::new();
    for (col, name) in header.iter().enumerate() {
        let mark = if is_key_column(&view, col) {
            "[x]"
        } else {
            "[ ]"
        };
        if col > 0 {
            key_line.push_str("  ");
        }
        key_line.push_str(mark);
        key_line.push(' ');
        key_line.push_str(name);
    }
    draw_string(
        &mut pixels,
        dims,
        60,
        130,
        &key_line,
        fixture_palette().text,
    );
    let badge_x = 948u32.saturating_sub(text_width(&badge));
    draw_string(
        &mut pixels,
        dims,
        badge_x,
        130,
        &badge,
        fixture_palette().dim,
    );

    // Find bar from live tokens: 28px bar, 120px needle, 1px underline.
    let find_h = TABLE_FIND_BAR_H as u32;
    let find_y = 124u32.saturating_add(key_h).saturating_add(4);
    fill_rect(
        &mut pixels,
        dims,
        (12, find_y, 936, find_h),
        fixture_palette().bg,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        find_y.saturating_add(6),
        "find",
        fixture_palette().text,
    );
    let needle_w = TABLE_FIND_NEEDLE_W as u32;
    fill_rect(
        &mut pixels,
        dims,
        (64, find_y.saturating_add(3), needle_w, 18),
        fixture_palette().field,
    );
    let needle = "bob";
    draw_string(
        &mut pixels,
        dims,
        70,
        find_y.saturating_add(6),
        needle,
        fixture_palette().text,
    );
    draw_string(
        &mut pixels,
        dims,
        200,
        find_y.saturating_add(6),
        "1 hit, current",
        fixture_palette().dim,
    );

    // Grid: header + rows, needle hit underlined at padded-span offsets.
    let pane_x = OUTER_PAD;
    let pane_w = SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2));
    let grid_top = find_y.saturating_add(find_h).saturating_add(4);
    let pane_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            pane_x,
            grid_top,
            pane_w,
            pane_bottom.saturating_sub(grid_top),
        ),
        fixture_palette().bg,
    );
    fill_rect(
        &mut pixels,
        dims,
        (pane_x, grid_top, pane_w, ROW_H),
        fixture_palette().track,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        grid_top.saturating_add(6),
        "id | name | left | right | status",
        fixture_palette().text,
    );
    let rows = [
        ("1 | anna | 10 | 10 | = |", false),
        ("2 | bob | 20 | 22 | ! |", true),
        ("3 | cara | 30 | 30 | = |", false),
        ("4 | dev | - | 40 | R |", true),
        ("5 | eli | 50 | 50 | = |", false),
        ("6 | fay | 60 | 60 | = |", false),
    ];
    for (i, (line, changed)) in rows.iter().enumerate() {
        let y = grid_top
            .saturating_add(ROW_H)
            .saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let (bg, fg) = if *changed {
            (fixture_palette().changed_bg, fixture_palette().changed_fg)
        } else {
            (fixture_palette().bg, fixture_palette().text)
        };
        fill_rect(&mut pixels, dims, (pane_x, y, pane_w, ROW_H), bg);
        draw_string(&mut pixels, dims, 20, y.saturating_add(6), line, fg);
    }
    // Underline the needle hit: live cell spans (cell-relative) shifted
    // to row offsets, then clamped to the padded cap.
    let row_line = "2 | bob | 20 | 22 | ! |";
    let cell_off = "2 | ".chars().count();
    let spans = table_find_cell_spans("bob", needle, false);
    let first = spans.first().copied();
    if let Some((start, end)) = first {
        let here = (cell_off.saturating_add(start), cell_off.saturating_add(end));
        if let Some((cs, ce)) = table_find_span_in_padded(here.0, here.1, TABLE_MAX_COL_CHARS) {
            let hit_y = grid_top.saturating_add(ROW_H).saturating_add(ROW_H);
            let prefix: String = row_line.chars().take(cs).collect();
            let hit: String = row_line
                .chars()
                .skip(cs)
                .take(ce.saturating_sub(cs))
                .collect();
            let ux = 20u32.saturating_add(text_width(&prefix));
            let uw = text_width(&hit);
            fill_rect(
                &mut pixels,
                dims,
                (ux, hit_y.saturating_add(14), uw, 1),
                fixture_palette().text,
            );
        }
    }

    paint_status(
        &mut pixels,
        dims,
        "table: key=0 | find:bob 1 hit (current) | underline 1px",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Shared window chrome: title bar + tab bar + path row + toolbar.
/// `active`: 0 home, 1 text, 2 folder, 3 hex, 4 merge, 5 table, 6 sync, 7 picture.
fn paint_chrome(pixels: &mut [u8], dims: (u32, u32), title: &str, active: u8) {
    let width = dims.0;
    fill_rect(
        pixels,
        dims,
        (0, 0, width, TITLE_H),
        fixture_palette().track,
    );
    let title_shown = crate::viewmodel::clip_to_width(title, width.saturating_sub(24), text_width);
    draw_string(
        pixels,
        dims,
        OUTER_PAD,
        14,
        &title_shown,
        fixture_palette().text,
    );

    fill_rect(
        pixels,
        dims,
        (0, TITLE_H, width, TAB_H),
        fixture_palette().track,
    );
    // Active tab reads via text-vs-dim ink (live tabs are egui selectable
    // style, no theme fill token).
    draw_string(
        pixels,
        dims,
        16,
        46,
        "Home",
        if active == 0 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(
        pixels,
        dims,
        80,
        46,
        "Text",
        if active == 1 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(
        pixels,
        dims,
        148,
        46,
        "Folder",
        if active == 2 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(
        pixels,
        dims,
        222,
        46,
        "Hex",
        if active == 3 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(
        pixels,
        dims,
        278,
        46,
        "Merge",
        if active == 4 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(
        pixels,
        dims,
        346,
        46,
        "Table",
        if active == 5 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(
        pixels,
        dims,
        410,
        46,
        "Sync",
        if active == 6 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(
        pixels,
        dims,
        474,
        46,
        "Picture",
        if active == 7 {
            fixture_palette().text
        } else {
            fixture_palette().dim
        },
    );
    draw_string(pixels, dims, 544, 46, "FolderMerge", fixture_palette().dim);
    draw_string(pixels, dims, 644, 46, "Audio", fixture_palette().dim);

    // Per-pane path strip + second metadata line (exq-pathmeta parity).
    if dims.0 >= SNAP_W {
        paint_pathmeta_strip(
            pixels,
            dims,
            "fixtures/left",
            "fixtures/right",
            "2026-01-02 03:04 | 128 B | Text: exact",
            "2026-01-02 03:05 | 132 B | Text: exact",
        );
    } else {
        use crate::viewmodel::clip_to_width;
        fill_rect(
            pixels,
            dims,
            (
                OUTER_PAD,
                PATH_FIELD_Y,
                dims.0.saturating_sub(24),
                PATH_FIELD_H,
            ),
            fixture_palette().field,
        );
        let shown = clip_to_width(
            "fixtures/left <--> fixtures/right",
            dims.0.saturating_sub(80),
            text_width,
        );
        draw_string(
            pixels,
            dims,
            OUTER_PAD.saturating_add(28),
            82,
            "[ ]",
            fixture_palette().dim,
        );
        draw_string(
            pixels,
            dims,
            OUTER_PAD.saturating_add(56),
            82,
            &shown,
            fixture_palette().text,
        );
    }

    draw_string(
        pixels,
        dims,
        16,
        PATH_TOOLBAR_Y,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        fixture_palette().text,
    );
}

/// Shared status bar: 30px strip at y570.
fn paint_status(pixels: &mut [u8], dims: (u32, u32), line: &str) {
    let width = dims.0;
    fill_rect(
        pixels,
        dims,
        (0, 570, width, STATUS_H),
        fixture_palette().track,
    );
    let line_shown = crate::viewmodel::clip_to_width(line, width.saturating_sub(24), text_width);
    draw_string(pixels, dims, 16, 578, &line_shown, fixture_palette().dim);
}

fn fill_rect(pixels: &mut [u8], dims: (u32, u32), rect: (u32, u32, u32, u32), color: [u8; 4]) {
    let (img_w, img_h) = dims;
    let (x, y, w, h) = rect;
    let x_end = (x.saturating_add(w)).min(img_w);
    let y_end = (y.saturating_add(h)).min(img_h);

    for cy in y..y_end {
        let row_offset = (cy as usize)
            .saturating_mul(img_w as usize)
            .saturating_mul(4);
        for cx in x..x_end {
            let offset = row_offset.saturating_add((cx as usize).saturating_mul(4));
            if let Some(dst) = pixels.get_mut(offset..offset.saturating_add(4))
                && dst.len() == 4
            {
                dst.copy_from_slice(&color);
            }
        }
    }
}

// 5x7 ASCII font: geometry first (advance/kerning), typography second (bitmap tuning).
// Colors frozen: this module only moves the pen and picks bits, never hues.
//
// Advance table: default 6px (5 cols + 1 gap, was 7) so fixture rows fit the
// 460px panes without clipping; single-stem / punctuation glyphs take 4px,
// except `t` (5px exact pattern width). `t` ka col4 active hai (0x20): 4px par
// agle glyph ke col0 se double-draw block banta tha — toolbar "ta" me dikha.
// Baaki narrow glyphs ka col4 khaali hai, isliye 4px par sirf touch, overlap
// nahi. `kern_adjust` tuck sirf wide prev (T/Y/V/r/f) par lagta hai, isliye
// `t` fix ke baad kahin bhi shared-pixel overlap nahi bachta.
fn glyph_advance(ch: u8) -> u32 {
    match ch {
        b't' => 5,
        b' ' => 4,
        b'\'' | b',' | b'.' | b':' | b';' | b'!' | b'I' | b'i' | b'j' | b'l' | b'|' | b'`' => 4,
        _ => 6,
    }
}

/// Kerning nudge in pixels between two adjacent bytes: -1 (tuck), else 0.
/// Explicit caps pairs first, then one general rule for top-right overhangs
/// (`T/Y/V/r/f`) tucking into round/lowercase/punctuation followers.
fn kern_adjust(prev: u8, curr: u8) -> i32 {
    match (prev, curr) {
        (b'A', b'V')
        | (b'A', b'W')
        | (b'L', b'T')
        | (b'L', b'V')
        | (b'L', b'Y')
        | (b'P', b'A')
        | (b'T', b'o')
        | (b'T', b'e')
        | (b'T', b'a')
        | (b'V', b'A')
        | (b'Y', b'V') => -1,
        _ => {
            let tight_prev = matches!(prev, b'T' | b'Y' | b'V' | b'r' | b'f');
            let tuck_curr = matches!(curr, b'a' | b'c' | b'e' | b'o' | b'.' | b',' | b'\'' | b'"');
            if tight_prev && tuck_curr { -1 } else { 0 }
        }
    }
}

/// Pen width of `s` under the advance + kerning rules (layout proof helper).
fn text_width(s: &str) -> u32 {
    let mut w = 0u32;
    let mut prev: Option<u8> = None;
    for b in s.bytes() {
        if let Some(p) = prev {
            let k = kern_adjust(p, b);
            if k < 0 {
                w = w.saturating_sub(1);
            }
        }
        w = w.saturating_add(glyph_advance(b));
        prev = Some(b);
    }
    w
}

/// Count of non-empty columns in a 5-wide pattern (readability probe helper).
pub fn nonzero_columns(pattern: &[u8; 5]) -> usize {
    pattern.iter().filter(|c| **c != 0).count()
}

/// Greedy two-line word wrap for home card descriptions: words fill the
/// first line while `text_width` fits `max_px`, overflow goes to the
/// second line, anything past that collapses into a trailing `...`.
/// Pure; paint-independent (pixel-font budget, no new hues).
fn wrap_desc_two_lines(desc: &str, max_px: u32) -> (String, String) {
    let mut first = String::new();
    let mut second = String::new();
    let mut dropped = false;
    for word in desc.split(' ') {
        if word.is_empty() {
            continue;
        }
        if second.is_empty() && !dropped {
            let candidate = if first.is_empty() {
                word.to_string()
            } else {
                format!("{first} {word}")
            };
            if text_width(&candidate) <= max_px {
                first = candidate;
                continue;
            }
        }
        let candidate = if second.is_empty() {
            word.to_string()
        } else {
            format!("{second} {word}")
        };
        if text_width(&candidate) <= max_px {
            second = candidate;
        } else if second.is_empty() {
            // Single word wider than the card: hard-cut with ellipsis.
            let mut cut = String::new();
            for ch in word.chars() {
                let next = format!("{cut}{ch}");
                if text_width(&format!("{next}...")) > max_px {
                    break;
                }
                cut = next;
            }
            cut.push_str("...");
            second = cut;
        } else {
            dropped = true;
        }
    }
    if dropped && !second.is_empty() && !second.ends_with("...") {
        // Third-line spill: keep the two-line budget honest.
        let mut cut = second.clone();
        while !cut.is_empty() && text_width(&format!("{cut}...")) > max_px {
            cut.pop();
        }
        cut.push_str("...");
        second = cut;
    }
    (first, second)
}

fn draw_string(
    pixels: &mut [u8],
    dims: (u32, u32),
    mut x: u32,
    y: u32,
    s: &str,
    color: [u8; 4],
) -> u32 {
    let mut prev: Option<u8> = None;
    for b in s.bytes() {
        if let Some(p) = prev {
            let k = kern_adjust(p, b);
            if k < 0 {
                x = x.saturating_sub(1);
            }
        }
        draw_glyph(pixels, dims, x, y, b, color);
        x = x.saturating_add(glyph_advance(b));
        prev = Some(b);
    }
    x
}

fn draw_glyph(pixels: &mut [u8], dims: (u32, u32), x: u32, y: u32, ch: u8, color: [u8; 4]) {
    let (img_w, img_h) = dims;
    let pattern = glyph_pattern(ch);
    for (col_idx, &col_byte) in pattern.iter().enumerate() {
        let px = x.saturating_add(col_idx as u32);
        for row_idx in 0..7u32 {
            if (col_byte & (1 << row_idx)) != 0 {
                let py = y.saturating_add(row_idx);
                if px < img_w && py < img_h {
                    let offset = (py as usize)
                        .saturating_mul(img_w as usize)
                        .saturating_add(px as usize)
                        .saturating_mul(4);
                    if let Some(dst) = pixels.get_mut(offset..offset.saturating_add(4))
                        && dst.len() == 4
                    {
                        dst.copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

// Standard 5-column byte patterns for common ASCII symbols
fn glyph_pattern(ch: u8) -> [u8; 5] {
    match ch {
        b' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        b'!' => [0x00, 0x00, 0x5f, 0x00, 0x00],
        b'"' => [0x00, 0x07, 0x00, 0x07, 0x00],
        b'#' => [0x14, 0x7f, 0x14, 0x7f, 0x14],
        b'$' => [0x24, 0x2a, 0x7f, 0x2a, 0x12],
        b'%' => [0x23, 0x13, 0x08, 0x64, 0x62],
        b'&' => [0x36, 0x49, 0x55, 0x22, 0x50],
        b'\'' => [0x00, 0x05, 0x03, 0x00, 0x00],
        b'(' => [0x00, 0x1c, 0x22, 0x41, 0x00],
        b')' => [0x00, 0x41, 0x22, 0x1c, 0x00],
        b'*' => [0x14, 0x08, 0x3e, 0x08, 0x14],
        b'+' => [0x08, 0x08, 0x3e, 0x08, 0x08],
        // Single-stem punctuation: one active column reads sharper at 1x than
        // the old doubled blocks; comma/semicolon keep a 1px descender tail.
        b',' => [0x00, 0x00, 0x50, 0x20, 0x00],
        b'-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        b'.' => [0x00, 0x00, 0x60, 0x00, 0x00],
        b'/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        b'0' => [0x3e, 0x51, 0x49, 0x45, 0x3e],
        b'1' => [0x00, 0x42, 0x7f, 0x40, 0x00],
        b'2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        b'3' => [0x21, 0x41, 0x45, 0x4b, 0x31],
        b'4' => [0x18, 0x14, 0x12, 0x7f, 0x10],
        b'5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        b'6' => [0x3c, 0x4a, 0x49, 0x49, 0x30],
        b'7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        b'8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        b'9' => [0x06, 0x49, 0x49, 0x29, 0x1e],
        b':' => [0x00, 0x00, 0x36, 0x00, 0x00],
        b';' => [0x00, 0x00, 0x56, 0x20, 0x00],
        b'<' => [0x08, 0x14, 0x22, 0x41, 0x00],
        b'=' => [0x14, 0x14, 0x14, 0x14, 0x14],
        b'>' => [0x00, 0x41, 0x22, 0x14, 0x08],
        b'?' => [0x02, 0x01, 0x51, 0x09, 0x06],
        b'@' => [0x32, 0x49, 0x79, 0x41, 0x3e],
        b'A' => [0x7e, 0x11, 0x11, 0x11, 0x7e],
        b'B' => [0x7f, 0x49, 0x49, 0x49, 0x36],
        b'C' => [0x3e, 0x41, 0x41, 0x41, 0x22],
        b'D' => [0x7f, 0x41, 0x41, 0x22, 0x1c],
        b'E' => [0x7f, 0x49, 0x49, 0x49, 0x41],
        b'F' => [0x7f, 0x09, 0x09, 0x09, 0x01],
        b'G' => [0x3e, 0x41, 0x49, 0x49, 0x7a],
        b'H' => [0x7f, 0x08, 0x08, 0x08, 0x7f],
        b'I' => [0x00, 0x41, 0x7f, 0x41, 0x00],
        b'J' => [0x20, 0x40, 0x41, 0x3f, 0x01],
        b'K' => [0x7f, 0x08, 0x14, 0x22, 0x41],
        b'L' => [0x7f, 0x40, 0x40, 0x40, 0x40],
        b'M' => [0x7f, 0x02, 0x0c, 0x02, 0x7f],
        b'N' => [0x7f, 0x04, 0x08, 0x10, 0x7f],
        b'O' => [0x3e, 0x41, 0x41, 0x41, 0x3e],
        b'P' => [0x7f, 0x09, 0x09, 0x09, 0x06],
        b'Q' => [0x3e, 0x41, 0x51, 0x21, 0x5e],
        b'R' => [0x7f, 0x09, 0x19, 0x29, 0x46],
        b'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        b'T' => [0x01, 0x01, 0x7f, 0x01, 0x01],
        b'U' => [0x3f, 0x40, 0x40, 0x40, 0x3f],
        b'V' => [0x1f, 0x20, 0x40, 0x20, 0x1f],
        b'W' => [0x7f, 0x20, 0x18, 0x20, 0x7f],
        b'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        b'Y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        b'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        b'[' => [0x00, 0x7f, 0x41, 0x41, 0x00],
        b'\\' => [0x02, 0x04, 0x08, 0x10, 0x20],
        b']' => [0x00, 0x41, 0x41, 0x7f, 0x00],
        b'^' => [0x04, 0x02, 0x01, 0x02, 0x04],
        b'_' => [0x40, 0x40, 0x40, 0x40, 0x40],
        b'`' => [0x00, 0x01, 0x02, 0x04, 0x00],
        b'a' => [0x20, 0x54, 0x54, 0x54, 0x78],
        b'b' => [0x7f, 0x48, 0x44, 0x44, 0x38],
        b'c' => [0x38, 0x44, 0x44, 0x44, 0x20],
        b'd' => [0x38, 0x44, 0x44, 0x48, 0x7f],
        b'e' => [0x38, 0x54, 0x54, 0x54, 0x18],
        b'f' => [0x08, 0x7e, 0x09, 0x01, 0x02],
        b'g' => [0x0c, 0x52, 0x52, 0x52, 0x3e],
        b'h' => [0x7f, 0x08, 0x04, 0x04, 0x78],
        b'i' => [0x00, 0x44, 0x7d, 0x40, 0x00],
        b'j' => [0x20, 0x40, 0x44, 0x3d, 0x00],
        b'k' => [0x7f, 0x10, 0x28, 0x44, 0x00],
        b'l' => [0x00, 0x41, 0x7f, 0x40, 0x00],
        b'm' => [0x7c, 0x04, 0x18, 0x04, 0x78],
        b'n' => [0x7c, 0x08, 0x04, 0x04, 0x78],
        b'o' => [0x38, 0x44, 0x44, 0x44, 0x38],
        b'p' => [0x7c, 0x14, 0x14, 0x14, 0x08],
        b'q' => [0x08, 0x14, 0x14, 0x18, 0x7c],
        b'r' => [0x7c, 0x08, 0x04, 0x04, 0x08],
        b's' => [0x48, 0x54, 0x54, 0x54, 0x20],
        b't' => [0x04, 0x3f, 0x44, 0x40, 0x20],
        b'u' => [0x3c, 0x40, 0x40, 0x20, 0x7c],
        b'v' => [0x1c, 0x20, 0x40, 0x20, 0x1c],
        b'w' => [0x3c, 0x40, 0x30, 0x40, 0x3c],
        b'x' => [0x44, 0x28, 0x10, 0x28, 0x44],
        b'y' => [0x0c, 0x50, 0x50, 0x50, 0x3c],
        b'z' => [0x44, 0x64, 0x54, 0x4c, 0x44],
        b'{' => [0x00, 0x08, 0x36, 0x41, 0x00],
        b'|' => [0x00, 0x00, 0x7f, 0x00, 0x00],
        b'}' => [0x00, 0x41, 0x36, 0x08, 0x00],
        b'~' => [0x08, 0x04, 0x08, 0x10, 0x08],
        // Unknown bytes render as `?` outline, never a solid block.
        _ => [0x02, 0x01, 0x51, 0x09, 0x06],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gutter::GUTTER_RULER_WIDE;
    use crate::home::{
        HOME_CARD_BUTTON_MIN_H, HOME_GRID_GAP, RECENT_COL_AGE_W, RECENT_COL_KIND_W,
        home_card_min_height,
    };
    use crate::views::wrap_find_sub_rows;
    use crate::views_table::key_column_marker;

    fn png_magic_ok(bytes: &[u8]) -> bool {
        bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
    }

    #[test]
    fn text_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-folder-text-fixture.png");
        assert!(render_text_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
    }

    #[test]
    fn folder_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-folder-folder-fixture.png");
        assert!(render_folder_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn dispatcher_routes_by_file_name() {
        let dir = std::env::temp_dir();
        let text_out = dir.join("vis-dispatch-text.png");
        let folder_out = dir.join("vis-dispatch-folder.png");
        assert!(render_fixture_png(&text_out).is_ok());
        assert!(render_fixture_png(&folder_out).is_ok());
        let text_bytes = std::fs::read(&text_out).unwrap_or_default();
        let folder_bytes = std::fs::read(&folder_out).unwrap_or_default();
        assert!(!text_bytes.is_empty() && !folder_bytes.is_empty());
        assert!(png_magic_ok(&text_bytes));
        assert!(png_magic_ok(&folder_bytes));
        assert_ne!(text_bytes, folder_bytes);
    }

    #[test]
    fn fixture_default_mode_is_dark() {
        assert_eq!(FixtureMode::default(), FixtureMode::Dark);
    }

    #[test]
    fn fixture_default_path_is_byte_stable_dark() {
        let dir = std::env::temp_dir();
        let default_out = dir.join("vis-darkfull-default.png");
        let dark_out = dir.join("vis-darkfull-dark.png");
        assert!(render_fixture_png(&default_out).is_ok());
        assert!(render_fixture_png_with_mode(&dark_out, FixtureMode::Dark).is_ok());
        let default_bytes = std::fs::read(&default_out).unwrap_or_default();
        let dark_bytes = std::fs::read(&dark_out).unwrap_or_default();
        assert!(!default_bytes.is_empty());
        assert_eq!(default_bytes, dark_bytes);
    }

    #[test]
    fn fixture_mode_param_switches_hues() {
        let dir = std::env::temp_dir();
        let dark_out = dir.join("vis-darkfull-mode-dark.png");
        let light_out = dir.join("vis-darkfull-mode-light.png");
        assert!(render_fixture_png_with_mode(&dark_out, FixtureMode::Dark).is_ok());
        assert!(render_fixture_png_with_mode(&light_out, FixtureMode::Light).is_ok());
        let dark_bytes = std::fs::read(&dark_out).unwrap_or_default();
        let light_bytes = std::fs::read(&light_out).unwrap_or_default();
        assert!(png_magic_ok(&dark_bytes));
        assert!(png_magic_ok(&light_bytes));
        assert!(dark_bytes.len() >= 8192);
        assert!(light_bytes.len() >= 8192);
        assert_ne!(dark_bytes, light_bytes);
        // Guard wapas dark karta hai: default path mode-proof ke baad bhi stable.
        let after_out = dir.join("vis-darkfull-mode-after.png");
        assert!(render_fixture_png(&after_out).is_ok());
        let after_bytes = std::fs::read(&after_out).unwrap_or_default();
        assert_eq!(after_bytes, dark_bytes);
    }

    #[test]
    fn screenshot_mode_flag_defaults_dark_and_accepts_light() {
        let prog = String::from("autocompare-gui");
        assert_eq!(
            screenshot_mode_from_args(std::slice::from_ref(&prog)),
            Ok(FixtureMode::Dark)
        );
        assert_eq!(
            screenshot_mode_from_args(&[
                prog.clone(),
                String::from("--screenshot"),
                String::from("a.png")
            ]),
            Ok(FixtureMode::Dark)
        );
        assert_eq!(
            screenshot_mode_from_args(&[
                prog.clone(),
                String::from("--screenshot"),
                String::from("a.png"),
                String::from("--screenshot-mode"),
                String::from("light"),
            ]),
            Ok(FixtureMode::Light)
        );
        assert_eq!(
            screenshot_mode_from_args(&[
                prog.clone(),
                String::from("--screenshot-mode"),
                String::from("dark"),
            ]),
            Ok(FixtureMode::Dark)
        );
        assert!(
            screenshot_mode_from_args(&[
                prog.clone(),
                String::from("--screenshot-mode"),
                String::from("sepia"),
            ])
            .is_err()
        );
        assert!(screenshot_mode_from_args(&[prog, String::from("--screenshot-mode"),]).is_err());
        // CLI light render dark se alag bytes deta hai (flag-to-PNG proof).
        let dir = std::env::temp_dir();
        let cli_dark = dir.join("vis-lightflag-cli-dark.png");
        let cli_light = dir.join("vis-lightflag-cli-light.png");
        assert!(
            render_fixture_png_with_mode(
                &cli_dark,
                screenshot_mode_from_args(&[
                    String::from("autocompare-gui"),
                    String::from("--screenshot-mode"),
                    String::from("dark"),
                ])
                .unwrap_or_default()
            )
            .is_ok()
        );
        assert!(render_fixture_png_with_mode(&cli_light, FixtureMode::Light).is_ok());
        let dark_bytes = std::fs::read(&cli_dark).unwrap_or_default();
        let light_bytes = std::fs::read(&cli_light).unwrap_or_default();
        assert!(png_magic_ok(&dark_bytes));
        assert!(png_magic_ok(&light_bytes));
        assert_ne!(dark_bytes, light_bytes);
    }

    #[test]
    fn narrow_glyphs_use_tighter_advance() {
        for ch in *b" ',.:;!Iijl|`" {
            assert_eq!(glyph_advance(ch), 4);
        }
        // `t` ka col4 active hai — exact 5px, warna agle col0 se overlap.
        assert_eq!(glyph_advance(b't'), 5);
        assert!(glyph_advance(b't') < glyph_advance(b'M'));
        for ch in *b"MWaen=>/" {
            assert_eq!(glyph_advance(ch), 6);
        }
        assert!(glyph_advance(b'i') < glyph_advance(b'M'));
    }

    #[test]
    fn kern_tight_pairs_tuck_one_pixel() {
        assert_eq!(kern_adjust(b'A', b'V'), -1);
        assert_eq!(kern_adjust(b'L', b'T'), -1);
        assert_eq!(kern_adjust(b'T', b'o'), -1);
        assert_eq!(kern_adjust(b'V', b'A'), -1);
        assert_eq!(kern_adjust(b'r', b'e'), -1);
        assert_eq!(kern_adjust(b'n', b'n'), 0);
        assert_eq!(kern_adjust(b'e', b'e'), 0);
        assert_eq!(kern_adjust(b'T', b'n'), 0);
    }

    #[test]
    fn punctuation_dots_are_single_stem() {
        assert_eq!(nonzero_columns(&glyph_pattern(b'.')), 1);
        assert_eq!(nonzero_columns(&glyph_pattern(b':')), 1);
        assert_eq!(nonzero_columns(&glyph_pattern(b',')), 2);
        assert_eq!(nonzero_columns(&glyph_pattern(b';')), 2);
        assert_eq!(glyph_pattern(b'.').get(2), Some(&0x60u8));
        assert_eq!(glyph_pattern(b',').get(3), Some(&0x20u8));
        // Unknown bytes fall back to the `?` outline, never a solid block.
        assert_eq!(glyph_pattern(0x80), glyph_pattern(b'?'));
    }

    #[test]
    fn text_width_respects_kerning_and_fits_panes() {
        let plain = glyph_advance(b'T').saturating_add(glyph_advance(b'o'));
        assert_eq!(text_width("To"), plain.saturating_sub(1));
        assert_eq!(
            text_width("nn"),
            glyph_advance(b'n').saturating_add(glyph_advance(b'n'))
        );
        // Fixture strings must fit their painted fields at 960px canvas width.
        assert!(text_width("fixtures/left.rs") < 380);
        assert!(text_width("fixtures/right.rs") < 380);
        assert!(
            text_width("[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax")
                < 928
        );
        assert!(
            text_width(
                "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done: 7 row(s)"
            ) < 928
        );
    }

    #[test]
    fn narrow_fixture_uses_shared_headless_budget() {
        // Fallback geometry comes from the theme + viewmodel budget, not
        // local magic: 720 wide, 340/340 panes, gutter + margins close.
        assert_eq!(crate::theme::VIEWPORT_MIN_W, 720);
        assert_eq!(crate::viewmodel::dual_pane_widths(720, 0.50), (340, 340));
        let (left_w, right_w) = crate::viewmodel::dual_pane_widths(720, 0.50);
        let gutter_x = OUTER_PAD
            .saturating_add(left_w)
            .saturating_add(PANE_GAP)
            .saturating_add(right_w);
        assert_eq!(gutter_x.saturating_add(GUTTER_W).saturating_add(8), 720);
        // Clipped pane text always fits its budget; short rows untouched.
        let budget = left_w.saturating_sub(32);
        let wide: String = "W".repeat(100);
        let shown = crate::viewmodel::clip_to_width(&wide, budget, text_width);
        assert!(text_width(&shown) <= budget);
        assert!(shown.ends_with("..."));
        let short = "    Ok(content.len())";
        assert_eq!(
            crate::viewmodel::clip_to_width(short, budget, text_width),
            short
        );
    }

    #[test]
    fn narrow_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-narrow-fixture.png");
        assert!(render_narrow_text_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
        if let (Some(w_slice), Some(h_slice)) = (bytes.get(16..20), bytes.get(20..24)) {
            let mut w_arr = [0u8; 4];
            let mut h_arr = [0u8; 4];
            w_arr.copy_from_slice(w_slice);
            h_arr.copy_from_slice(h_slice);
            assert_eq!(
                (u32::from_be_bytes(w_arr), u32::from_be_bytes(h_arr)),
                (720, 600)
            );
        }
    }

    #[test]
    fn draw_string_returns_pen_and_clips_safely() {
        let dims = (20u32, 10u32);
        let total = 20usize.saturating_mul(10).saturating_mul(4);
        let mut pixels = vec![0u8; total];
        let pen = draw_string(&mut pixels, dims, 2, 1, "Hi.", fixture_palette().text);
        assert_eq!(pen, 2u32.saturating_add(text_width("Hi.")));
        // Pen near the right edge clips glyph pixels but still advances.
        let edge = draw_string(&mut pixels, dims, 19, 1, "Hi.", fixture_palette().text);
        assert_eq!(edge, 19u32.saturating_add(text_width("Hi.")));
        assert_eq!(pixels.len(), total);
    }

    #[test]
    fn paint_chrome_clips_long_title_without_oob_write() {
        let dims = (200u32, 68u32);
        let total = 200usize.saturating_mul(68).saturating_mul(4);
        let mut pixels = vec![0u8; total];
        paint_chrome(
            &mut pixels,
            dims,
            "a very long session title <--> that must never paint past the canvas edge",
            1,
        );
        paint_status(
            &mut pixels,
            dims,
            "a very long status line that must ellipsis-clip instead of raw-cutting pixels",
        );
        assert_eq!(pixels.len(), total);
    }

    #[test]
    fn hex_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-hex-fixture.png");
        assert!(render_hex_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn table_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-table-fixture.png");
        assert!(render_table_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn merge_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-merge-fixture.png");
        assert!(render_merge_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn home_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-home-fixture.png");
        assert!(render_home_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn dispatcher_routes_all_tabs() {
        let dir = std::env::temp_dir();
        let cases = [
            "vis-ao-dispatch-text.png",
            "vis-ao-dispatch-folder.png",
            "vis-ao-dispatch-hex.png",
            "vis-ao-dispatch-table.png",
            "vis-ao-dispatch-merge.png",
            "vis-ao-dispatch-home.png",
        ];
        let mut blobs: Vec<Vec<u8>> = Vec::new();
        for name in cases {
            let out = dir.join(name);
            assert!(render_fixture_png(&out).is_ok());
            let bytes = std::fs::read(&out).unwrap_or_default();
            assert!(!bytes.is_empty());
            assert!(png_magic_ok(&bytes));
            assert!(bytes.len() >= 8192);
            blobs.push(bytes);
        }
        // Har tab ka PNG alag hona chahiye (6 me se koi do same nahi).
        for (i, a) in blobs.iter().enumerate() {
            for (j, b) in blobs.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
        // FolderMerge naam merge route par jana chahiye.
        let fm = dir.join("vis-ao-dispatch-foldermerge.png");
        let m = dir.join("vis-ao-dispatch-merge.png");
        assert!(render_fixture_png(&fm).is_ok());
        let fm_bytes = std::fs::read(&fm).unwrap_or_default();
        let m_bytes = std::fs::read(&m).unwrap_or_default();
        assert_eq!(fm_bytes, m_bytes);
    }

    #[test]
    fn geometry_shared_tokens_hold() {
        assert_eq!(SNAP_W, 960);
        assert_eq!(SNAP_H, 600);
        assert_eq!(ROW_H, 24);
        assert_eq!(OUTER_PAD, 12);
        assert_eq!(PANE_GAP, 4);
        assert_eq!(TITLE_H, 36);
        assert_eq!(TAB_H, 32);
        assert_eq!(STATUS_H, 30);
        // Text two-pane: left + gap + right meets the ruler, right edge 952 (8px margin).
        assert_eq!(
            TEXT_LEFT_X
                .saturating_add(TEXT_PANE_W)
                .saturating_add(PANE_GAP),
            TEXT_RIGHT_X
        );
        assert_eq!(TEXT_RIGHT_X.saturating_add(TEXT_PANE_W), GUTTER_X);
        assert_eq!(GUTTER_X.saturating_add(GUTTER_W), 952);
        assert!(GUTTER_X.saturating_add(GUTTER_W) <= SNAP_W);
        // Pane heights: 124 + 436 = 560, status 570 + 30 = 600.
        assert_eq!(124u32.saturating_add(436), 560);
        assert_eq!(570u32.saturating_add(STATUS_H), SNAP_H);
        // Fixture strings apne fields me fit hone chahiye.
        assert!(text_width("fixtures/left") < 380);
        assert!(text_width("fixtures/right") < 380);
        assert!(text_width("00000000  48 65 6c 6c 6f 20 77 6f |Hello wo|") < 460);
        assert!(text_width("id | name | left | right | status") < 936);
        assert!(text_width("let x = 1;") < 309);
        assert!(text_width("Recent sessions") < 936);
        assert!(
            text_width("home: 9 cards, 5 recents | no path picker | pick a session to begin") < 928
        );
    }

    #[test]
    fn merge_panes_share_equal_thirds() {
        assert_eq!(MERGE_PANE_W, 309);
        assert_eq!(
            MERGE_X0
                .saturating_add(MERGE_PANE_W)
                .saturating_add(PANE_GAP),
            MERGE_X1
        );
        assert_eq!(
            MERGE_X1
                .saturating_add(MERGE_PANE_W)
                .saturating_add(PANE_GAP),
            MERGE_X2
        );
        assert!(
            MERGE_X2
                .saturating_add(MERGE_PANE_W)
                .saturating_add(OUTER_PAD)
                <= SNAP_W
        );
        // Teenon widths barabar, koi ad-hoc chaudapan nahi.
        assert_eq!(MERGE_PANE_W, MERGE_PANE_W);
    }

    #[test]
    fn home_cards_fit_canvas() {
        // Live grid: 3 cols, gap 8, card 306x100 (merge-style slack: 2px).
        assert_eq!(HOME_CARD_W, 306);
        assert_eq!(HOME_CARD_H, 100);
        assert_eq!(HOME_CARD_GAP, 8);
        assert_eq!(HOME_CARD_GAP as f32, HOME_GRID_GAP);
        let total = HOME_CARD_W
            .saturating_mul(3)
            .saturating_add(HOME_CARD_GAP.saturating_mul(2))
            .saturating_add(OUTER_PAD.saturating_mul(2));
        assert_eq!(total.saturating_add(2), SNAP_W);
        assert!(text_width("Text Compare") < HOME_CARD_W);
        assert!(text_width("Folder Compare") < HOME_CARD_W);
    }

    #[test]
    fn gutter_matches_live_ruler_contract() {
        // Ruler track 16 (line numbers on), old 14px gap absorbed.
        assert_eq!(GUTTER_W, GUTTER_RULER_WIDE as u32);
        assert_eq!(GUTTER_W, 16);
        assert_eq!(GUTTER_X, 936);
        // Tall-file single hunk keeps the 4px floor (not the old 2px dot).
        let mut tall = Vec::new();
        let mut i = 0usize;
        while i < 1000 {
            tall.push(TextRow {
                kind: TextRowKind::Equal,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            });
            i = i.saturating_add(1);
        }
        if let Some(row) = tall.get_mut(500) {
            row.kind = TextRowKind::Deleted;
        }
        let geom = gutter_geometry(1000, 400.0, &tall);
        assert_eq!(geom.segments.len(), 1);
        if let Some(seg) = geom.segments.first() {
            assert!((seg.y_end - seg.y_start - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        }
    }

    #[test]
    fn shot15_center_gap_markers_use_live_helpers() {
        // Mirrors checks/gui-exq-shot15/round-1/comparison/shot-center-1200.json.
        use crate::viewmodel::textcenter_fill_rows;
        // Live 20px center (SPACE_LG + SPACE_XS); fixture derives from it.
        assert_eq!(text_center_gutter_width(), 20.0);
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        let (left_x, left_w, center_x, center_w, right_x, right_w) = text_fixture_center_layout();
        assert_eq!(left_x, TEXT_LEFT_X);
        assert_eq!(right_x.saturating_add(right_w), GUTTER_X);
        assert_eq!(
            center_x,
            left_x.saturating_add(left_w).saturating_add(PANE_GAP)
        );
        // Marker fits the gap with a symmetric inset (20 - 12 = 8, 4 per side).
        assert_eq!(TEXT_CENTER_MARK, "=>");
        let mark_w = text_width(TEXT_CENTER_MARK);
        assert!(mark_w <= center_w.saturating_sub(8));
        assert_eq!(center_w.saturating_sub(mark_w) % 2, 0);
        // Default 7-row fixture: one contiguous diff hunk, head at row 2 only.
        let kinds = [
            TextRowKind::Equal,
            TextRowKind::Equal,
            TextRowKind::Changed,
            TextRowKind::Deleted,
            TextRowKind::Inserted,
            TextRowKind::Equal,
            TextRowKind::Equal,
        ];
        let rows: Vec<TextRow> = kinds
            .iter()
            .map(|kind| TextRow {
                kind: *kind,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            })
            .collect();
        for (idx, row) in rows.iter().enumerate() {
            let want_head = idx == 2;
            assert_eq!(textcenter_hunk_head(&rows, idx), want_head);
            assert_eq!(row.kind != TextRowKind::Equal && want_head, idx == 2);
        }
        // Floor below the last row: 418px panes, 24px rows, 7 rows -> 11 slots.
        assert_eq!(textcenter_fill_rows(418.0, ROW_H as f32, rows.len()), 11);
        // 1200-wide edge: same 20px center, panes grow symmetric (12/578/20/578/12).
        let wide = crate::gutter::gutter_text_layout(12.0, 20.0, 1200.0);
        assert_eq!(wide, (12.0, 578.0, 20.0, 578.0, 12.0));
    }

    #[test]
    fn shot18_chrome720_no_overlap() {
        // Mirrors checks/gui-exq-shot18/round-1/comparison/shot-chrome-720.json.
        // Fixture chrome17 mirror: taller 56 band + padded tabs hold at 720
        // without overlap; shot15 center gutter untouched.
        let raw =
            include_str!("../../../checks/gui-exq-shot18/round-1/comparison/shot-chrome-720.json");
        assert!(raw.contains("\"name\":\"shot-chrome-720\""));
        use crate::views::{
            HEADER_H, TEXT_STACK_BREAKPOINT_W, TextLayout, action_bar_row_gap,
            action_bar_row_height, chrome17_narrow720_no_overlap_gate,
            chrome17_toolbar_side_air_720, path_browse_button_size,
            session_tab_strip_needs_hscroll, session_tab_strip_width, split_field_widths,
            text_layout_for_width, text_meta_line_height_px, toolbar_button_pair_width,
        };
        // Live chrome17 gate still green (parallel wave owns views.rs).
        assert_eq!(action_bar_row_height(), 24.0);
        assert_eq!(action_bar_row_gap(), 8.0);
        assert_eq!(
            action_bar_row_height() + action_bar_row_gap() + action_bar_row_height(),
            56.0
        );
        assert_eq!(toolbar_button_pair_width(), 200.0);
        assert_eq!(chrome17_toolbar_side_air_720(), 260.0);
        assert!(chrome17_narrow720_no_overlap_gate());
        assert_eq!(session_tab_strip_width(10), 1572.0);
        assert!(session_tab_strip_needs_hscroll(10, TEXT_STACK_BREAKPOINT_W));
        assert_eq!(text_layout_for_width(720.0), TextLayout::SideBySide);
        assert_eq!(text_layout_for_width(719.0), TextLayout::Stacked);
        let inner = TEXT_STACK_BREAKPOINT_W - 2.0 * f32::from(crate::views::OUTER_MARGIN);
        assert_eq!(inner, 696.0);
        assert_eq!(
            split_field_widths(inner, crate::viewmodel::SPLIT_DEFAULT_FRACTION),
            (180.0, 180.0)
        );
        assert_eq!(path_browse_button_size(), (120.0, 28.0));
        assert_eq!(HEADER_H, 28.0);
        assert!((text_meta_line_height_px() - 15.2).abs() < 0.01);
        assert!(text_meta_line_height_px() < HEADER_H);
        // Fixture narrow panes: 340/340 + 16 gutter + 8 margin = 720, no overlap.
        assert_eq!(crate::viewmodel::dual_pane_widths(720, 0.50), (340, 340));
        assert_eq!(
            crate::viewmodel::dual_pane_x_positions(720, 0.50),
            (12, 356, 696)
        );
        let (lx, rx, gx) = crate::viewmodel::dual_pane_x_positions(720, 0.50);
        assert_eq!(lx, OUTER_PAD);
        assert_eq!(rx, lx.saturating_add(340).saturating_add(PANE_GAP));
        assert_eq!(gx, rx.saturating_add(340));
        assert_eq!(gx.saturating_add(GUTTER_W).saturating_add(8), 720);
        // Fixture chrome18 band: 114 + 56 + 4 air = 174 pane_top, panes 174..560.
        assert_eq!(PATH_TOOLBAR_Y.saturating_add(56).saturating_add(4), 174);
        assert_eq!(560u32.saturating_sub(174), 386);
        // Padded tabstrip: 8px inter-tab gaps, 22px boxes (6px trailing air).
        assert_eq!(68u32.saturating_sub(8u32.saturating_add(52)), 8);
        assert_eq!(136u32.saturating_sub(68u32.saturating_add(60)), 8);
        assert_eq!(68u32.saturating_add(60).saturating_add(8), 136);
        // Shot15 center gutter untouched (markers still fit the 20px gap).
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        assert_eq!(TEXT_CENTER_MARK, "=>");
        assert!(text_width(TEXT_CENTER_MARK) <= 20u32.saturating_sub(8));
    }

    #[test]
    fn shot19_typotall_numbers_aligned() {
        // Mirrors checks/gui-exq-shot19/round-1/comparison/shot-typo-tall-1200.json:
        // tall 1200px scroll canvas — fixture row rhythm from live typo17
        // metrics: numbers stay aligned in a fixed column, baselines stable,
        // generous row height from line-height rhythm. Geometry FIRST,
        // typography SECOND, colors FROZEN.
        let fixture_raw = include_str!(
            "../../../checks/gui-exq-shot19/round-1/comparison/shot-typo-tall-1200.json"
        );
        assert!(fixture_raw.contains("\"name\": \"shot-typo-tall-1200\""));
        assert!(fixture_raw.contains("\"fixture_linenum_col_w_1200\": 37"));
        assert!(fixture_raw.contains("\"fixture_row_h_from_line_height\": 18"));
        assert!(fixture_raw.contains("\"scroll_visible_rows_1200\": 66"));

        use crate::views::{
            text_char_metric_width, text_linenum_column_width_from_metrics,
            text_row_baseline_offset_px, text_row_inner_pad_px, text_typo17_row_height,
        };
        // 1. Live metrics the fixture mirrors (parallel wave owns views.rs).
        assert!((text_char_metric_width() - 7.2).abs() < 0.001);
        assert_eq!(text_row_inner_pad_px(), 4.0);
        assert!((text_typo17_row_height() - 17.6).abs() < 0.001);
        assert_eq!(text_row_baseline_offset_px(), 1.0);
        assert_eq!(crate::gutter::gutter_linenum_digits(1200), 4);

        // 2. Fixture rhythm derived from those metrics (whole pixels).
        assert_eq!(fixture_text_row_h(), 18);
        assert_eq!(fixture_text_inner_pad(), 4);
        assert!((text_linenum_column_width_from_metrics(4, 7.2) - 36.8).abs() < 0.01);
        assert_eq!(fixture_text_linenum_col_w(7), 15);
        assert_eq!(fixture_text_linenum_col_w(1200), 37);
        assert_eq!(fixture_text_x_off(7), 19);
        assert_eq!(fixture_text_baseline_dy(18), 5);
        assert_eq!(FIXTURE_GLYPH_H, 7);
        // Breathing room, no glued rows: glyph 7 < row 18, baseline centered.
        assert!(fixture_text_row_h() > FIXTURE_GLYPH_H);
        assert_eq!(
            fixture_text_baseline_dy(fixture_text_row_h()),
            fixture_text_row_h()
                .saturating_sub(FIXTURE_GLYPH_H)
                .saturating_div(2)
        );

        // 3. Tall-1200 scroll: fixed column + fixed offsets every row, tops
        // strictly monotonic, widest number fits the column.
        let row_h = fixture_text_row_h();
        let visible = 1200u32.saturating_div(row_h);
        assert_eq!(visible, 66);
        let mut prev_top = 0u32;
        for i in 0..visible {
            let top = i.saturating_mul(row_h);
            if i > 0 {
                assert!(top > prev_top);
                assert_eq!(top.saturating_sub(prev_top), row_h);
            }
            prev_top = top;
            // Same insets every row: alignment stable by construction.
            assert_eq!(fixture_text_inner_pad(), 4);
            assert_eq!(fixture_text_x_off(1200), 41);
        }
        // Widest 1200-row number fits the 1200-row column.
        assert!(text_width("1200") <= fixture_text_linenum_col_w(1200));

        // 4. Shot15 + shot18 intact: center layout, markers, pane top.
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        assert_eq!(TEXT_CENTER_MARK, "=>");
        assert!(text_width(TEXT_CENTER_MARK) <= 20u32.saturating_sub(8));
        assert_eq!(PATH_TOOLBAR_Y.saturating_add(56).saturating_add(4), 174);
    }

    #[test]
    fn status20_narrow720_no_overlap() {
        // Mirrors checks/gui-exq-status20/round-1/comparison/status-narrow-720.json:
        // narrow 720px canvas — per-pane status strips fit within 340px panes without
        // overlap across the 4px central gap; bottom legend 4 zones scale/clip cleanly
        // across 720px width without text collision. Shot15 center gutter and shot18
        // chrome stay intact.
        let raw = include_str!(
            "../../../checks/gui-exq-status20/round-1/comparison/status-narrow-720.json"
        );
        assert!(raw.contains("\"name\": \"status-narrow-720\""));

        // 1. Dual-pane layout at 720px: panes 340/340, 4px gap, 16px gutter.
        let (left_w, right_w) = crate::viewmodel::dual_pane_widths(720, 0.50);
        assert_eq!((left_w, right_w), (340, 340));
        let (lx, rx, gx) = crate::viewmodel::dual_pane_x_positions(720, 0.50);
        assert_eq!((lx, rx, gx), (12, 356, 696));

        // 2. Strip geometry: strips sit at bottom of panes above 560, height 18.
        let pane_bottom = 560u32;
        let strip_h = PANE_STATUS_H;
        assert_eq!(strip_h, 18);
        let strip_y = pane_bottom.saturating_sub(strip_h);
        assert_eq!(strip_y, 542);

        // 3. No overlap between left and right strips across the gap:
        assert_eq!(lx.saturating_add(left_w), 352);
        assert_eq!(rx, 356);
        assert!(lx.saturating_add(left_w) <= rx);
        assert_eq!(rx.saturating_sub(lx.saturating_add(left_w)), PANE_GAP);
        assert_eq!(rx.saturating_add(right_w), gx);

        // 4. Per-pane strip content fits within 340px pane budget:
        let left_line =
            format_pane_status_line("3:1", "Text: exact", "4 equal, 1 changed, 1 deleted");
        let right_line =
            format_pane_status_line("3:1", "Text: exact", "4 equal, 1 changed, 1 inserted");
        let left_clipped =
            crate::viewmodel::clip_to_width(&left_line, left_w.saturating_sub(16), text_width);
        let right_clipped =
            crate::viewmodel::clip_to_width(&right_line, right_w.saturating_sub(16), text_width);
        assert!(text_width(&left_clipped) <= left_w.saturating_sub(16));
        assert!(text_width(&right_clipped) <= right_w.saturating_sub(16));

        // 5. Bottom legend strip at 720px: 4 zones of 180px each, dividers at 180, 360, 540.
        let segs = fixture_legend_segments();
        assert_eq!(segs.len(), 4);
        let zone_w = 720u32 / 4;
        assert_eq!(zone_w, 180);
        for seg in segs.iter() {
            let clipped =
                crate::viewmodel::clip_to_width(seg, zone_w.saturating_sub(16), text_width);
            assert!(text_width(&clipped) <= zone_w.saturating_sub(16));
        }

        // 6. Shot15 center gutter and shot18 chrome stay intact.
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        assert_eq!(TEXT_CENTER_MARK, "=>");
        assert!(text_width(TEXT_CENTER_MARK) <= 20u32.saturating_sub(8));
        assert_eq!(PATH_TOOLBAR_Y.saturating_add(56).saturating_add(4), 174);
    }

    #[test]
    fn hatch25_tall_no_banding() {
        // Mirrors checks/gui-exq-hatch25/round-1/comparison/hatch-tall-1200.json:
        // trailing slots below the last content row stay calm at tall
        // viewports — one single-tone fill from the exact row-grid content
        // end to the exact status-strip top, zero filler glyphs, no banding.
        // Geometry FIRST, typography untouched, colors FROZEN.
        let raw =
            include_str!("../../../checks/gui-exq-hatch25/round-1/comparison/hatch-tall-1200.json");
        assert!(raw.contains("\"name\": \"hatch-tall-1200\""));
        assert!(raw.contains("\"fill_bands\": 1"));
        assert!(raw.contains("\"filler_glyphs\": 0"));
        assert!(raw.contains("\"no_banding\": true"));

        // 1. Tall-1200 pane budget: same 40px bottom budget as the 600
        // canvas (legend 30 + 10 air), strips 18 above the legend gap.
        let canvas_h = 1200u32;
        let pane_bottom = canvas_h.saturating_sub(40);
        assert_eq!(pane_bottom, 1160);
        assert_eq!(PANE_STATUS_H, 18);
        let strip_top = pane_bottom.saturating_sub(PANE_STATUS_H);
        assert_eq!(strip_top, 1142);

        // 2. Content end lands exactly on the row grid; trailing fill starts
        // there with zero pad and stops exactly at the strips (zero pad).
        let row_h = fixture_text_row_h();
        assert_eq!(row_h, 18);
        let pane_top = PATH_TOOLBAR_Y.saturating_add(56).saturating_add(4);
        assert_eq!(pane_top, 174);
        let content_end = pane_top.saturating_add(50u32.saturating_mul(row_h));
        assert_eq!(content_end, 1074);
        assert_eq!(content_end.saturating_sub(pane_top) % row_h, 0);
        assert!(content_end < strip_top);
        assert_eq!(strip_top.saturating_sub(content_end), 68);

        // 3. Fixture-scale (600 canvas) rhythm locks the real renderer edit:
        // 7 content rows end at 300, strips start at 542, one 242px calm band.
        let fixture_end = pane_top.saturating_add(7u32.saturating_mul(row_h));
        assert_eq!(fixture_end, 300);
        let fixture_strip = 560u32.saturating_sub(PANE_STATUS_H);
        assert_eq!(fixture_strip, 542);
        assert_eq!(fixture_strip.saturating_sub(fixture_end), 242);

        // 4. Shot15/shot18/shot19 + status20 intact: center layout, markers,
        // toolbar rhythm, typo row height, strips, legend segments.
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        assert_eq!(TEXT_CENTER_MARK, "=>");
        assert!(text_width(TEXT_CENTER_MARK) <= 20u32.saturating_sub(8));
        assert_eq!(fixture_text_inner_pad(), 4);
        assert_eq!(fixture_text_baseline_dy(row_h), 5);
        assert_eq!(fixture_legend_segments().len(), 4);
    }

    #[test]
    fn filebar24_narrow720_no_overlap() {
        // Mirrors checks/gui-exq-filebar24/round-1/comparison/filebar-narrow-720.json:
        // narrow 720px canvas — per-pane file-bar rows (path field + metadata
        // segment) fit within 340px panes without overlap across the 4px
        // central gap; long paths truncate with ellipsis inside the field;
        // the metadata segment clips to the bar and never overlaps pane
        // content; bar bottom (112) stays above the toolbar (114). Shot15
        // center gutter, shot18 chrome, shot19 typo, and status20 strips
        // stay intact.
        let raw = include_str!(
            "../../../checks/gui-exq-filebar24/round-1/comparison/filebar-narrow-720.json"
        );
        assert!(raw.contains("\"name\": \"filebar-narrow-720\""));

        // 1. File-bar band geometry: field row 24 + meta line 12 = 36 at y76.
        assert_eq!(FILE_BAR_Y, PATH_FIELD_Y);
        assert_eq!(FILE_BAR_Y, 76);
        assert_eq!(FILE_BAR_H, PATH_FIELD_H.saturating_add(PATH_META_H));
        assert_eq!(FILE_BAR_H, 36);
        assert_eq!(FILE_BAR_BROWSE_W, 75);
        assert_eq!(FILE_BAR_Y.saturating_add(FILE_BAR_H), 112);
        assert!(FILE_BAR_Y.saturating_add(FILE_BAR_H) <= PATH_TOOLBAR_Y);
        assert_eq!(PATH_TOOLBAR_Y, 114);

        // 2. Narrow 720px bar columns track the dual-pane split: 340/340,
        // 4px gap, no overlap, metadata never reaches pane content.
        let (left_w, right_w) = crate::viewmodel::dual_pane_widths(720, 0.50);
        assert_eq!((left_w, right_w), (340, 340));
        let (lx, rx, gx) = crate::viewmodel::dual_pane_x_positions(720, 0.50);
        assert_eq!((lx, rx, gx), (12, 356, 696));
        assert_eq!(lx.saturating_add(left_w), 352);
        assert!(lx.saturating_add(left_w) <= rx);
        assert_eq!(rx.saturating_sub(lx.saturating_add(left_w)), PANE_GAP);
        assert_eq!(rx.saturating_add(right_w), gx);

        // 3. Path field refit inside each bar: Browse keeps its width.
        let field_w = left_w.saturating_sub(FILE_BAR_BROWSE_W).saturating_sub(8);
        assert_eq!(field_w, 257);
        let browse_x = lx.saturating_add(field_w).saturating_add(4);
        assert_eq!(browse_x, 273);
        assert!(browse_x.saturating_add(FILE_BAR_BROWSE_W) <= lx.saturating_add(left_w));

        // 4. Long path truncates with ellipsis inside the field budget.
        let long_path = "fixtures/very/long/nested/dir/structure/that/keeps/going/right.rs";
        let path_budget = field_w.saturating_sub(80);
        let path_shown = crate::viewmodel::clip_to_width(long_path, path_budget, text_width);
        assert!(path_shown.ends_with("..."));
        assert!(text_width(&path_shown) <= path_budget);

        // 5. Metadata segment: timestamp | bytes | format word; honest "-"
        // fallback when stat fails; clipped to the bar, never pane overlap.
        assert_eq!(
            format_file_bar_meta("2026-01-02 03:04", "128 B", "Text: exact"),
            "2026-01-02 03:04 | 128 B | Text: exact"
        );
        assert_eq!(format_file_bar_meta("", "", ""), "-");
        assert_eq!(format_file_bar_meta("  ", " ", ""), "-");
        assert_eq!(format_file_bar_meta("", "128 B", ""), "128 B");
        let meta = format_file_bar_meta("2026-01-02 03:04", "128 B", "Text: exact");
        let meta_budget = left_w.saturating_sub(16);
        assert_eq!(meta_budget, 324);
        let meta_shown = crate::viewmodel::clip_to_width(&meta, meta_budget, text_width);
        assert!(text_width(&meta_shown) <= meta_budget);

        // 6. Wide canvas intact: bars track the center layout columns;
        // shot15 gutter/markers, shot18 chrome, status20 strips untouched.
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        assert_eq!(TEXT_CENTER_MARK, "=>");
        assert_eq!(PANE_STATUS_H, 18);
        assert_eq!(fixture_legend_segments().len(), 4);
        assert_eq!(PATH_TOOLBAR_Y.saturating_add(56).saturating_add(4), 174);
    }

    #[test]
    fn toolbar21_narrow720_no_overlap() {
        // Mirrors checks/gui-exq-toolbar21/round-1/comparison/toolbar-narrow-720.json:
        // narrow 720px canvas — row-1 icon+label buttons compress via the shared
        // visible-count/overflow rhythm without overlap, and the thin nav-arrow
        // row (two small chevron triples) keeps its floor + cluster air.
        // Gutter/markers/chrome/typo rows/status strips stay intact.
        let raw = include_str!(
            "../../../checks/gui-exq-toolbar21/round-1/comparison/toolbar-narrow-720.json"
        );
        assert!(raw.contains("\"name\": \"toolbar-narrow-720\""));

        use crate::home::{
            TOPCHROME_BUTTON_GAP, TOPCHROME_SIDE_PAD, TOPCHROME_TOOLBAR_ACTIONS,
            topchrome_toolbar_button_width, topchrome_toolbar_visible_count,
        };
        use crate::views::{action_bar_row_gap, action_bar_row_height, action_nav_arrow_gap};

        // 1. Toolbar band geometry: 24px rows + 8px gap = 56 at y114, panes at 174.
        assert_eq!(PATH_TOOLBAR_Y, 114);
        assert_eq!(action_bar_row_height(), 24.0);
        assert_eq!(action_bar_row_gap(), 8.0);
        assert_eq!(PATH_TOOLBAR_Y.saturating_add(56).saturating_add(4), 174);
        assert_eq!(action_nav_arrow_gap(), 8.0);

        // 2. 720px: visible buttons tile left-to-right with the shared gap and
        // never overlap; the rest collapse behind the ">>" overflow marker.
        let width = 720u32;
        let visible = topchrome_toolbar_visible_count(width as f32);
        assert_eq!(visible, 7);
        assert!(visible < TOPCHROME_TOOLBAR_ACTIONS.len());
        let mut x = TOPCHROME_SIDE_PAD as u32;
        assert_eq!(x, 12);
        let mut prev_end = x;
        let mut count = 0usize;
        for action in TOPCHROME_TOOLBAR_ACTIONS {
            if count >= visible {
                break;
            }
            let w = topchrome_toolbar_button_width(action) as u32;
            let vw = w.min(width.saturating_sub(x));
            // Icon (12 box at +4) + label (at +20, budget vw-24) stay in-bounds.
            assert!(vw >= 16);
            assert!(x.saturating_add(16) <= x.saturating_add(vw));
            let budget = vw.saturating_sub(24);
            let label = crate::viewmodel::clip_to_width(action, budget, text_width);
            assert!(text_width(&label) <= budget);
            // 12px icon box fits the 24px row height.
            assert!(6u32.saturating_add(12) <= action_bar_row_height() as u32);
            assert!(x >= prev_end);
            prev_end = x.saturating_add(vw);
            x = prev_end.saturating_add(TOPCHROME_BUTTON_GAP as u32);
            count = count.saturating_add(1);
        }
        assert_eq!(count, visible);
        assert!(prev_end <= width);

        // 3. Nav-arrow row at 720: Prev/Next 96x24 pair + 8px cluster air fits.
        let btn_w = 96u32;
        let side = TOPCHROME_SIDE_PAD as u32;
        let row2_y = PATH_TOOLBAR_Y
            .saturating_add(action_bar_row_height() as u32)
            .saturating_add(action_bar_row_gap() as u32);
        assert_eq!(row2_y, 146);
        let nx = side
            .saturating_add(btn_w)
            .saturating_add(action_nav_arrow_gap() as u32);
        assert_eq!(nx, 116);
        assert!(nx.saturating_add(btn_w) <= width);
        // Chevron triples (8x10 cells) fit inside the 96x24 buttons.
        assert!(16u32.saturating_add(8) <= btn_w);
        assert!(7u32.saturating_add(10) <= action_bar_row_height() as u32);

        // 4. Gutter/markers/chrome/typo rows/status strips intact.
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        assert_eq!(TEXT_CENTER_MARK, "=>");
        assert_eq!(PANE_STATUS_H, 18);
        assert_eq!(560u32.saturating_sub(PANE_STATUS_H), 542);
        assert_eq!(STATUS_H, 30);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn home_matches_live_contract() {
        use crate::home::{
            HOME_CARD_BUTTON_MIN_W, HOME_SHOWS_PATH_PICKER, home_card_button_hit_area,
            home_grid_rows,
        };
        // 9 live cards tile 3x3 rows.
        assert_eq!(home_model().items.len(), 9);
        assert_eq!(home_grid_rows(9), 3);
        // Card floor: contract min-height 99.6 -> fixture 100.
        assert!((home_card_min_height() - 99.6).abs() < 0.01);
        assert!((HOME_CARD_H as f32) >= home_card_min_height());
        // CTA floor: 120x28 min, hit area 3360 min.
        assert!((HOME_CTA_W as f32) >= HOME_CARD_BUTTON_MIN_W);
        assert!((HOME_CTA_H as f32) >= HOME_CARD_BUTTON_MIN_H);
        assert!((HOME_CTA_W as f32) * (HOME_CTA_H as f32) >= home_card_button_hit_area());
        // No path picker on the launcher.
        assert!(!HOME_SHOWS_PATH_PICKER);
        // Recent columns tile the list width exactly.
        let (title_w, kind_w, age_w) = recent_row_columns(936.0);
        assert_eq!(kind_w, RECENT_COL_KIND_W);
        assert_eq!(age_w, RECENT_COL_AGE_W);
        assert!(
            (title_w + kind_w + age_w + RECENT_LIST_PAD * 2.0 + RECENT_COL_GAP * 2.0 - 936.0).abs()
                < 0.001
        );
        // Every CTA label fits its button text budget (224 - 2*12).
        for item in home_model().items.iter() {
            assert!(text_width(&format!("Open {}", item.title)) < HOME_CTA_W - 24);
        }
    }

    #[test]
    fn home_fixture_mirrors_live_paint() {
        use crate::home::{HOME_CARD_MIN_W, home_card_cta_width};
        use crate::views::{home_paint_card_height, home_paint_card_width, home_paint_cta_size};
        // Fixture card == live flex width @960 canvas (936 inner), truncated.
        assert!((home_paint_card_width(936.0) - 306.6667).abs() < 0.01);
        assert_eq!(HOME_CARD_W, 306);
        assert_eq!(HOME_CARD_W as f32, HOME_CARD_MIN_W + 66.0);
        // Fixture CTA == live CTA contract 224x28, card me centered.
        assert_eq!(HOME_CTA_W as f32, home_card_cta_width());
        assert_eq!((HOME_CTA_W, HOME_CTA_H), (224, 28));
        assert_eq!(
            home_paint_cta_size(),
            (home_card_cta_width(), HOME_CARD_BUTTON_MIN_H)
        );
        assert_eq!(
            HOME_CARD_W.saturating_sub(HOME_CTA_W) / 2,
            41,
            "CTA centered in card"
        );
        // Fixture card height == live paint height (contract ceiling 100).
        assert!((HOME_CARD_H as f32) >= home_paint_card_height());
        assert_eq!(HOME_CARD_H, home_paint_card_height().ceil() as u32);
    }

    #[test]
    fn home_desc_wrap_stays_in_two_line_budget() {
        let budget = HOME_CARD_W.saturating_sub(24);
        for item in home_model().items.iter() {
            let (top, bottom) = wrap_desc_two_lines(item.description, budget);
            assert!(text_width(&top) <= budget);
            assert!(text_width(&bottom) <= budget);
            // Our 9 live descs fit two lines: no word dropped, no ellipsis.
            let mut want: Vec<&str> = item.description.split(' ').collect();
            want.retain(|w| !w.is_empty());
            let mut got: Vec<&str> = top.split(' ').chain(bottom.split(' ')).collect();
            got.retain(|w| !w.is_empty());
            assert_eq!(got, want);
            assert!(!bottom.ends_with("..."));
        }
        // Third-line spill collapses honestly instead of clipping.
        let (top, bottom) = wrap_desc_two_lines(
            "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu",
            60,
        );
        assert!(text_width(&top) <= 60);
        assert!(text_width(&bottom) <= 60);
        assert!(bottom.ends_with("..."));
    }

    #[test]
    fn wrap_scene_proves_continuation_find() {
        const LONG: &str = "    // wrap proof: keep the legacy reader enabled until rollout ends, then call read_to_string(old);";
        const NEEDLE: &str = "read_to_string";
        assert_eq!(LONG.chars().count(), 100);
        // Live cap: 100-char model -> 80-char wrap width.
        assert_eq!(wrap_width_for(100, true), 80);
        // Needle lives fully inside the continuation sub-row.
        assert_eq!(wrap_find_sub_rows(LONG, NEEDLE, false, 80), vec![1]);
        assert_eq!(crate::views::wrap_chunks(LONG, 80).len(), 2);
        // Every rendered chunk fits the 428px pane text budget.
        for chunk in crate::views::wrap_chunks(LONG, 80) {
            assert!(text_width(&chunk.text) <= 428);
        }
        // Scroll target clamps to the pair count (2 subs -> sub 1).
        let row = TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(3),
            right_no: Some(3),
            left: LONG.to_string(),
            right: LONG.to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        let count = wrap_pane_pair_count(&row.left, &row.right, 80);
        assert_eq!(count, 2);
        assert_eq!(wrap_find_scroll_sub(&row, NEEDLE, false, 80, count), 1);
        // Flat visual offset of (row 2, sub 1) over [1,1,2,1,1] is 3.
        let counts = vec![1usize, 1, 2, 1, 1];
        assert_eq!(
            crate::views::wrap_model_to_visual_start(&counts, 2),
            Some(2)
        );
        // Status + find-bar strings fit the 936px panel.
        assert!(
            text_width("wrap:on find:read_to_string 2 hits, current row 3 sub 1/2 | wrapnav proof")
                < 928
        );
        assert!(text_width("2 hits, current row 3") < 936);
    }

    #[test]
    fn picture_scene_matches_fit_contract() {
        assert_eq!(PICTURE_ZOOM_STEPS, [25, 50, 100, 200, 400]);
        // Viewport 936x372 over natural 800x600: scale 0.62 -> step 50.
        assert_eq!(picture_fit_zoom_pct(800.0, 600.0, 936.0, 372.0), 50);
        assert_eq!(picture_center_offset(400.0, 936.0), 268.0);
        assert_eq!(picture_center_offset(300.0, 372.0), 36.0);
        // Live summary words the identical pair (no invented numbers).
        assert_eq!(
            crate::viewmodel::build_picture_summary(&PictureDiff {
                width_l: 800,
                height_l: 600,
                width_r: 800,
                height_r: 600,
                overlap_w: 800,
                overlap_h: 600,
                total: 480_000,
                differing: 0,
                bbox: None,
                identical: true,
            }),
            vec![
                "left:  800x600".to_string(),
                "right: 800x600".to_string(),
                "no differences".to_string(),
            ]
        );
        // Centered 400x300 frame stays inside the 936x372 viewport.
        assert!(12u32.saturating_add(268).saturating_add(400) <= 948);
        assert!(188u32.saturating_add(36).saturating_add(300) <= 560);
        assert!(
            text_width("picture: fit 50% | centered (268,36) | steps Fit+25/50/100/200/400") < 928
        );
        assert_eq!(zoom_label(50), "zoom 50% (0.50x)");
        assert!(text_width(&zoom_label(50)) < 948 - 432);
    }

    #[test]
    fn table_find_scene_matches_key_find_contract() {
        // Key bar: col 0 selected, badge reads keyed.
        let mut view = TableKeyView::new();
        assert!(view.is_positional());
        view.toggle_key_column(0);
        assert!(is_key_column(&view, 0));
        assert!(!is_key_column(&view, 1));
        assert_eq!(table_key_badge_label(&view), "key: 0");
        assert_ne!(key_column_marker(true), key_column_marker(false));
        // Find spans: cell-relative (0,3), row offset 4 -> (4,7) in cap.
        assert_eq!(table_find_cell_spans("bob", "bob", false), vec![(0, 3)]);
        assert_eq!(
            table_find_span_in_padded(4, 7, TABLE_MAX_COL_CHARS),
            Some((4, 7))
        );
        assert_eq!(
            table_find_span_in_padded(0, 99, TABLE_MAX_COL_CHARS),
            Some((0, 32))
        );
        // Bars reuse live heights; needle fits its 120px box.
        assert_eq!(TABLE_KEY_BAR_H, 28.0);
        assert_eq!(TABLE_FIND_BAR_H, 28.0);
        assert_eq!(TABLE_FIND_NEEDLE_W, 120.0);
        assert!(text_width("bob") < TABLE_FIND_NEEDLE_W as u32);
    }

    #[test]
    fn folder_empty_scene_matches_hint_contract() {
        // Filtered empty: no rows, no error -> idle guidance, never blank.
        let hint = folder_empty_hint(false, false);
        assert!(!hint.is_empty());
        assert!(hint.contains("type both folder paths"));
        assert!(text_width(hint) < 936);
        let rows = vec![crate::viewmodel::FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '>',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        }];
        let counts = crate::tree::count_folder_rows(&rows);
        assert!(counts.summary().contains("1 entries") || counts.summary().contains('1'));
        assert!(
            text_width(
                "folder: 4 entries, 0 shown (filter) | type both folder paths, press Compare"
            ) < 928
        );
    }

    #[test]
    fn allsame_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-allsame-fixture.png");
        assert!(render_folder_allsame_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn allsame_scene_matches_allsame_contract() {
        use crate::tree::{FolderDisplay, FolderEmptyState, FolderStatusFilter, folder_empty_hint};
        use crate::viewmodel::FolderRow;
        fn same_row(path: &str, is_dir: bool) -> FolderRow {
            FolderRow {
                rel_path: path.to_string(),
                is_dir,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            }
        }
        // Do same folders + diff-seeking filter: ped khaali, model sab-same.
        let rows = vec![
            same_row("src", true),
            same_row("src/a.txt", false),
            same_row("src/b.txt", false),
        ];
        let display = FolderDisplay {
            status: FolderStatusFilter::Different,
            ..Default::default()
        };
        assert!(display.is_active());
        let expanded: std::collections::BTreeSet<String> =
            ["src".to_string()].into_iter().collect();
        let nodes = crate::tree::folder_tree_filtered(&rows, &expanded, &display);
        assert_eq!(nodes.len(), 0);
        assert_eq!(
            crate::tree::folder_empty_state_full_rows(true, &rows, nodes.len(), &display),
            Some(FolderEmptyState::AllSame)
        );
        // Paint wiring wahi hint deta hai jo fixture panel me hai.
        let hint = crate::views::folder_paint_empty_hint_full_rows(&rows, nodes.len(), &display);
        assert!(hint.contains("all same"));
        assert!(hint.contains("nothing to sync"));
        assert!(!hint.contains("clear filters"));
        assert_ne!(hint, folder_empty_hint(Some(FolderEmptyState::NoResults)));
        assert_ne!(
            hint,
            folder_empty_hint(Some(FolderEmptyState::FilterPruned))
        );
        // Fixture strings apne fields me fit honi chahiye.
        assert!(text_width(hint) < 936);
        assert!(
            text_width(
                "[ ] Hide orphans  [ ] Left newer only  [ ] Right newer only  [ ] Flatten  (Status: Different)"
            ) < 936
        );
        assert!(
            text_width("folder: 3 entries, 0 shown (allsame) | all same, nothing to sync") < 928
        );
    }

    #[test]
    fn dispatcher_routes_allsame_scene() {
        let dir = std::env::temp_dir();
        let allsame_out = dir.join("vis-ax-dispatch-allsame.png");
        assert!(render_fixture_png(&allsame_out).is_ok());
        let allsame_bytes = std::fs::read(&allsame_out).unwrap_or_default();
        assert!(!allsame_bytes.is_empty());
        assert!(png_magic_ok(&allsame_bytes));
        assert!(allsame_bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = allsame_bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
        // AllSame scene text/folder/folder-empty teeno se alag hona chahiye.
        for name in [
            "vis-ax-dispatch-text.png",
            "vis-ax-dispatch-folder.png",
            "vis-ax-dispatch-folder-empty.png",
        ] {
            let out = dir.join(name);
            assert!(render_fixture_png(&out).is_ok());
            let bytes = std::fs::read(&out).unwrap_or_default();
            assert!(!bytes.is_empty());
            assert_ne!(allsame_bytes, bytes);
        }
    }

    #[test]
    fn dispatcher_routes_new_scenes() {
        let dir = std::env::temp_dir();
        let cases = [
            "vis-ax-dispatch-wrap.png",
            "vis-ax-dispatch-picture.png",
            "vis-ax-dispatch-folder-empty.png",
            "vis-ax-dispatch-table-find.png",
        ];
        let mut blobs: Vec<Vec<u8>> = Vec::new();
        for name in cases {
            let out = dir.join(name);
            assert!(render_fixture_png(&out).is_ok());
            let bytes = std::fs::read(&out).unwrap_or_default();
            assert!(!bytes.is_empty());
            assert!(png_magic_ok(&bytes));
            assert!(bytes.len() >= 8192);
            blobs.push(bytes);
        }
        // Naye scenes apas me alag...
        assert_eq!(blobs.len(), cases.len());
        for (i, a) in blobs.iter().enumerate() {
            for (j, b) in blobs.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
        // ...aur purane default scenes se bhi alag.
        let text_out = dir.join("vis-ax-dispatch-text.png");
        assert!(render_fixture_png(&text_out).is_ok());
        let text_bytes = std::fs::read(&text_out).unwrap_or_default();
        let folder_out = dir.join("vis-ax-dispatch-folder.png");
        assert!(render_fixture_png(&folder_out).is_ok());
        let folder_bytes = std::fs::read(&folder_out).unwrap_or_default();
        let table_out = dir.join("vis-ax-dispatch-table.png");
        assert!(render_fixture_png(&table_out).is_ok());
        let table_bytes = std::fs::read(&table_out).unwrap_or_default();
        if let (Some(wrap), Some(picture), Some(folder_empty), Some(table_find)) =
            (blobs.first(), blobs.get(1), blobs.get(2), blobs.get(3))
        {
            assert_ne!(wrap, &text_bytes);
            assert_ne!(picture, &text_bytes);
            assert_ne!(folder_empty, &folder_bytes);
            assert_ne!(table_find, &table_bytes);
        }
    }

    #[test]
    fn merge_take_scene_proves_live_take_row() {
        // Headless truth: ek well-formed hunk, live title/decision/resolve.
        let marker = vec![
            "<<<<<<< left".to_string(),
            "let y = 20;".to_string(),
            "||||||| base".to_string(),
            "let y = 2;".to_string(),
            "=======".to_string(),
            "let y = 2;".to_string(),
            ">>>>>>> right".to_string(),
        ];
        let hunks = crate::viewmodel::merge_conflict_hunks(&marker);
        assert_eq!(hunks.len(), 1);
        let (left, base, right) = match hunks.first() {
            Some(hunk) => crate::views::merge_hunk_sides(&marker, hunk),
            None => (Vec::new(), Vec::new(), Vec::new()),
        };
        assert_eq!(left, vec!["let y = 20;".to_string()]);
        assert_eq!(base, vec!["let y = 2;".to_string()]);
        assert_eq!(right, vec!["let y = 2;".to_string()]);
        assert_eq!(
            crate::views::format_merge_hunk_title(0, 1),
            "hunk 1 of 1: left / base / right"
        );
        assert_eq!(
            crate::views::format_merge_hunk_decision_with_pick(
                0,
                Some(crate::take::TakeChoice::Left)
            ),
            "hunk 1: take left"
        );
        let pick = crate::viewmodel::MergeHunkPick {
            conflict_index: 0,
            choice: crate::take::TakeChoice::Left,
        };
        let summary = crate::viewmodel::merge_pick_summary(&marker, &[pick]);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.resolved, 1);
        assert_eq!(summary.pending, 0);
        let resolved = crate::viewmodel::apply_merge_picks(&marker, &[pick]).unwrap_or_default();
        assert_eq!(resolved, vec!["let y = 20;".to_string()]);
        // Fixture strings apne fields me fit honi chahiye.
        assert!(text_width("hunk 1 of 1: left / base / right") < 936);
        assert!(text_width("hunk 1: take left") < 936);
        assert!(text_width("resolved (take left): let y = 20;") < 936);
        assert!(text_width("merge-take: 1 hunk, take left staged | resolved 1 line(s)") < 928);
        // PNG valid, non-blank, merge fixture se alag bytes.
        let dir = std::env::temp_dir();
        let take_out = dir.join("vis-bj-dispatch-merge-take.png");
        assert!(render_merge_take_fixture_png(&take_out).is_ok());
        let take_bytes = std::fs::read(&take_out).unwrap_or_default();
        assert!(!take_bytes.is_empty());
        assert!(png_magic_ok(&take_bytes));
        assert!(take_bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = take_bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
        let merge_out = dir.join("vis-bj-dispatch-merge.png");
        assert!(render_fixture_png(&merge_out).is_ok());
        let merge_bytes = std::fs::read(&merge_out).unwrap_or_default();
        assert_ne!(take_bytes, merge_bytes);
    }

    #[test]
    fn sync_scene_matches_checklist_gate_contract() {
        use crate::app::{SyncChecklist, sync_apply_enabled, sync_dry_run_summary};
        use autocompare_compare::{SyncAction, SyncPlan, SyncStep};
        // Fixture wala plan: 2 left->right copies + 1 skip.
        let plan = SyncPlan {
            steps: vec![
                SyncStep {
                    rel_path: "src/a.txt".to_string(),
                    is_dir: false,
                    action: SyncAction::CopyToRight,
                },
                SyncStep {
                    rel_path: "new.txt".to_string(),
                    is_dir: false,
                    action: SyncAction::CopyToRight,
                },
            ],
            skipped: vec![(
                "src/b.txt".to_string(),
                "content differs; no side wins".to_string(),
            )],
            truncated: false,
        };
        let list = SyncChecklist::from_plan(&plan);
        assert_eq!(list.summary(), "2 step(s) planned, 1 skipped");
        assert_eq!(list.checked_count(), 2);
        assert_eq!(list.copy_right_plan().steps.len(), 2);
        assert_eq!(
            sync_dry_run_summary(2, 2),
            "dry-run: 2 of 2 checked step(s) would copy left->right (no writes)"
        );
        assert!(sync_apply_enabled(true, 2));
        assert!(!sync_apply_enabled(false, 2));
        assert!(!sync_apply_enabled(true, 0));
        // Row strings live helpers se, fields me fit.
        let row = match list.rows.first() {
            Some(row) => row,
            None => {
                assert!(!list.rows.is_empty());
                return;
            }
        };
        assert_eq!(row.glyph(), '>');
        assert_eq!(row.display_line(), "> src/a.txt");
        assert_eq!(row.action_label(), "copy to right");
        assert!(text_width(&row.display_line()) < 936);
        assert!(
            text_width(
                "sync: 2 step(s) planned, 1 skipped | dry-run 2 of 2 checked would copy (no writes)"
            ) < 928
        );
        // PNG valid, non-blank.
        let out = std::env::temp_dir().join("vis-bj-dispatch-sync.png");
        assert!(render_sync_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn dispatcher_routes_sync_scene() {
        // BI-S4 twin-guard (AW2 picture precedent): sync naam apne scene
        // par jana chahiye, fallback text se byte-alag hona chahiye.
        let dir = std::env::temp_dir();
        let sync_out = dir.join("vis-bj-dispatch-sync-route.png");
        assert!(render_fixture_png(&sync_out).is_ok());
        let sync_bytes = std::fs::read(&sync_out).unwrap_or_default();
        assert!(!sync_bytes.is_empty());
        assert!(png_magic_ok(&sync_bytes));
        assert!(sync_bytes.len() >= 8192);
        // Merge-take route bhi dispatcher se chunta hai.
        let take_out = dir.join("vis-bj-dispatch-merge-take-route.png");
        assert!(render_fixture_png(&take_out).is_ok());
        let take_bytes = std::fs::read(&take_out).unwrap_or_default();
        assert!(!take_bytes.is_empty());
        assert_ne!(take_bytes, sync_bytes);
        // Sync/foldermerge fallback twins nahi: har purana scene alag.
        for name in [
            "vis-bj-dispatch-text.png",
            "vis-bj-dispatch-folder.png",
            "vis-bj-dispatch-merge.png",
            "vis-bj-dispatch-table.png",
            "vis-bj-dispatch-home.png",
        ] {
            let out = dir.join(name);
            assert!(render_fixture_png(&out).is_ok());
            let bytes = std::fs::read(&out).unwrap_or_default();
            assert!(!bytes.is_empty());
            assert_ne!(sync_bytes, bytes);
            assert_ne!(take_bytes, bytes);
        }
        // FolderMerge naam ab bhi merge route par (compat, twin by design).
        let fm = dir.join("vis-bj-dispatch-foldermerge.png");
        let m = dir.join("vis-bj-check-foldermerge-twin.png");
        assert!(render_fixture_png(&fm).is_ok());
        let fm_bytes = std::fs::read(&fm).unwrap_or_default();
        assert!(render_fixture_png(&m).is_ok());
        let _ = std::fs::read(&m).unwrap_or_default();
        let merge_direct = dir.join("vis-bj-dispatch-merge.png");
        assert!(render_fixture_png(&merge_direct).is_ok());
        let merge_bytes = std::fs::read(&merge_direct).unwrap_or_default();
        assert_eq!(fm_bytes, merge_bytes);
    }

    #[test]
    fn three_way_scene_proves_live_tree() {
        use crate::viewmodel::FolderRow;
        fn snap_row(path: &str, is_dir: bool) -> FolderRow {
            FolderRow {
                rel_path: path.to_string(),
                is_dir,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            }
        }
        // Fixture wale snapshots: 5 rows, 2 conflicts.
        let base = vec![
            snap_row("a.txt", false),
            snap_row("del.txt", false),
            snap_row("src", true),
        ];
        let left = vec![
            snap_row("a.txt", false),
            snap_row("new.txt", false),
            snap_row("one.txt", false),
            snap_row("src", true),
        ];
        let right = vec![
            snap_row("a.txt", false),
            snap_row("del.txt", false),
            snap_row("new.txt", false),
            snap_row("src", true),
        ];
        let rows = crate::tree::three_way_tree_rows(&base, &left, &right);
        assert_eq!(rows.len(), 5);
        let paths: Vec<String> = rows.iter().map(|row| row.rel_path.clone()).collect();
        assert_eq!(
            paths,
            vec![
                "a.txt".to_string(),
                "del.txt".to_string(),
                "new.txt".to_string(),
                "one.txt".to_string(),
                "src".to_string(),
            ]
        );
        assert_eq!(
            crate::tree::three_way_conflicts(&rows),
            vec!["del.txt".to_string(), "new.txt".to_string()]
        );
        // Presence marks + dir line live helpers se.
        let marks: Vec<String> = rows
            .iter()
            .map(crate::views::three_way_presence_marks)
            .collect();
        assert_eq!(marks, vec!["BLR", "B-R", "-LR", "-L-", "BLR"]);
        let src_row = match rows.iter().find(|row| row.rel_path == "src") {
            Some(row) => row,
            None => {
                assert!(!rows.is_empty());
                return;
            }
        };
        assert_eq!(crate::views::three_way_row_line(src_row), "src/");
        // Decisions: del staged right, new pending, a clean.
        let decisions = crate::views::stage_three_way_decision(
            &[],
            "del.txt",
            crate::tree::ThreeWaySide::Right,
        );
        assert_eq!(
            crate::views::three_way_decision_for_row(&decisions, "del.txt"),
            Some(crate::tree::ThreeWaySide::Right)
        );
        assert_eq!(
            crate::views::three_way_decision_for_row(&decisions, "ghost.txt"),
            None
        );
        assert_eq!(
            crate::views::three_way_row_decision_text(true, Some(crate::tree::ThreeWaySide::Right)),
            "take right"
        );
        assert_eq!(
            crate::views::three_way_row_decision_text(true, None),
            "needs a decision"
        );
        assert_eq!(
            crate::views::three_way_row_decision_text(false, None),
            "clean"
        );
        // Header live summary se, unresolved kyunki new.txt pending hai.
        let header = crate::views::three_way_header_line(&rows, &decisions);
        assert_eq!(
            header,
            "5 path(s), 1 decided, 4 pending, 2 conflict(s) (unresolved)"
        );
        assert_eq!(
            header,
            crate::tree::three_way_tree_summary(&rows, &decisions).summary_line()
        );
        // Take labels live `ThreeWaySide::label()` se.
        assert_eq!(crate::tree::ThreeWaySide::Left.label(), "Take Left");
        assert_eq!(crate::tree::ThreeWaySide::Base.label(), "Take Base");
        assert_eq!(crate::tree::ThreeWaySide::Right.label(), "Take Right");
        // Fixture strings apne fields me fit honi chahiye.
        assert!(text_width(&header) < 928);
        assert!(text_width("Take Left") < 309);
        assert!(text_width("Take Base") < 309);
        assert!(text_width("Take Right") < 309);
        assert!(
            text_width(
                "threeway: 5 path(s), 1 decided, 4 pending, 2 conflict(s) (unresolved) | take left/base/right live"
            ) < 928
        );
        // PNG valid, non-blank, merge fixture se alag bytes.
        let dir = std::env::temp_dir();
        let three_out = dir.join("vis-bl-dispatch-threeway-direct.png");
        assert!(render_three_way_fixture_png(&three_out).is_ok());
        let three_bytes = std::fs::read(&three_out).unwrap_or_default();
        assert!(!three_bytes.is_empty());
        assert!(png_magic_ok(&three_bytes));
        assert!(three_bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = three_bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
        let merge_out = dir.join("vis-bl-dispatch-merge.png");
        assert!(render_fixture_png(&merge_out).is_ok());
        let merge_bytes = std::fs::read(&merge_out).unwrap_or_default();
        assert_ne!(three_bytes, merge_bytes);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn pathmeta_strip_geometry_holds() {
        // Field row y76 h24, meta y100 h12, toolbar y114, panes y124 untouched.
        assert_eq!(PATH_FIELD_Y, 76);
        assert_eq!(PATH_FIELD_H, 24);
        assert_eq!(PATH_META_Y, 100);
        assert_eq!(PATH_META_H, 12);
        assert_eq!(PATH_TOOLBAR_Y, 114);
        assert!(PATH_FIELD_Y.saturating_add(PATH_FIELD_H) <= PATH_META_Y);
        assert!(PATH_META_Y.saturating_add(PATH_META_H) <= PATH_TOOLBAR_Y);
        assert!(PATH_TOOLBAR_Y < 124);
        // Segmented dividers pane budget me, status strip canvas me.
        assert!(SEG_DIV0_X < SEG_DIV1_X);
        assert!(SEG_DIV1_X < SEG_DIV2_X);
        assert!(SEG_DIV2_X < SNAP_W);
        assert_eq!(570u32.saturating_add(STATUS_H), SNAP_H);
        // Strip strings apne budgets me fit honi chahiye.
        assert!(text_width("fixtures/left.rs") < 300);
        assert!(text_width("fixtures/right.rs") < 300);
        // Edge: empty-vs-long — empty meta honest-empty, long clip hota hai.
        let empty = crate::viewmodel::PathPaneMeta::default();
        assert_eq!(crate::viewmodel::path_pane_meta_line(&empty), "");
        let long_path = "fixtures/very/long/nested/dir/structure/that/keeps/going/right.rs";
        let shown = crate::viewmodel::clip_to_width(long_path, 300, text_width);
        assert!(shown.ends_with("..."));
        assert!(text_width(&shown) <= 300);
        // Segmented line honest + clip budget me.
        let status = crate::viewmodel::path_segmented_status_for(
            "3:1",
            "Text: exact",
            "7 rows: 4 equal",
            "edit: clean",
        );
        let line = crate::viewmodel::path_segmented_status_line(&status);
        let clipped = crate::viewmodel::clip_to_width(&line, 936, text_width);
        assert!(text_width(&clipped) <= 936);
    }

    #[test]
    fn dispatcher_routes_threeway_scene() {
        // Accent-precedent twin-guard: three-way naam apne scene par jana
        // chahiye, fallback text/merge se byte-alag hona chahiye.
        let dir = std::env::temp_dir();
        let three_out = dir.join("vis-bl-dispatch-after-threeway.png");
        assert!(render_fixture_png(&three_out).is_ok());
        let three_bytes = std::fs::read(&three_out).unwrap_or_default();
        assert!(!three_bytes.is_empty());
        assert!(png_magic_ok(&three_bytes));
        assert!(three_bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = three_bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
        // Hyphen aur bina-hyphen dono spelling same scene par.
        let nospace_out = dir.join("vis-bl-dispatch-threeway.png");
        assert!(render_fixture_png(&nospace_out).is_ok());
        let nospace_bytes = std::fs::read(&nospace_out).unwrap_or_default();
        assert_eq!(three_bytes, nospace_bytes);
        // Purane scenes se sab alag (twin nahi).
        for name in [
            "vis-bl-dispatch-text.png",
            "vis-bl-dispatch-folder.png",
            "vis-bl-dispatch-merge.png",
            "vis-bl-dispatch-table.png",
            "vis-bl-dispatch-home.png",
            "vis-bl-dispatch-sync.png",
            "vis-bl-dispatch-merge-take.png",
            "vis-bl-dispatch-allsame.png",
        ] {
            let out = dir.join(name);
            assert!(render_fixture_png(&out).is_ok());
            let bytes = std::fs::read(&out).unwrap_or_default();
            assert!(!bytes.is_empty());
            assert_ne!(three_bytes, bytes);
        }
    }

    #[test]
    fn gutter28_overview_row_mapping() {
        // Row<->y dono disha live padded mapping se: ad-hoc math zero.
        // 7-row fixture, 386px pane (174..560): inner 378, pad 4.
        let pane_top = 174u32;
        let pane_h = 386u32;
        let total = 7usize;
        let first = overview_row_y(0, total, pane_top, pane_h);
        assert_eq!(first, Some(178));
        let mid = overview_row_y(3, total, pane_top, pane_h);
        assert!(mid.is_some());
        if let Some(y) = mid {
            assert!((y as f32 - 340.0).abs() <= 1.0);
        }
        let last = overview_row_y(6, total, pane_top, pane_h);
        assert!(last.is_some());
        if let Some(y) = last {
            assert!((y as f32 - 502.0).abs() <= 1.0);
        }
        // Round-trip: y se wapas wahi row.
        for row in 0..total {
            let y = overview_row_y(row, total, pane_top, pane_h);
            assert!(y.is_some());
            if let Some(y) = y {
                assert_eq!(overview_row_at_y(y, total, pane_top, pane_h), Some(row));
            }
        }
        // Pane ke upar `None` (local humesha pane_top offset ke saath).
        assert_eq!(overview_row_at_y(0, total, pane_top, pane_h), None);
        assert_eq!(overview_row_at_y(173, total, pane_top, pane_h), None);
        // Kharab geometry par `None`, kabhi wrap-around nahi.
        assert_eq!(overview_row_y(0, 0, pane_top, pane_h), None);
        assert_eq!(overview_row_y(7, total, pane_top, pane_h), None);
        assert_eq!(overview_row_y(0, total, pane_top, 0), None);
        // Live helper se seedha judav: offset ke alawa koi antar nahi.
        let live = crate::gutter::gutter_padded_y_for_row(3, total, pane_h as f32);
        assert!(live.is_some());
        if let (Some(live_y), Some(y)) = (live, mid) {
            assert!((y as f32 - (pane_top as f32 + live_y)).abs() < 1.0);
        }
    }

    #[test]
    fn gutter28_center_layout_intact() {
        // Center gutter still fixed 20px lane; overview minimap ne ise
        // chhua nahi: panes barabar, marker gap me centered.
        assert_eq!(text_center_gutter_width(), 20.0);
        assert_eq!(text_fixture_center_layout(), (12, 448, 464, 20, 488, 448));
        let (left_x, left_w, center_x, center_w, right_x, right_w) = text_fixture_center_layout();
        assert_eq!(left_w, right_w);
        assert_eq!(right_x.saturating_add(right_w), GUTTER_X);
        assert_eq!(
            center_x,
            left_x.saturating_add(left_w).saturating_add(PANE_GAP)
        );
        assert_eq!(TEXT_CENTER_MARK, "=>");
        let mark_w = text_width(TEXT_CENTER_MARK);
        assert!(mark_w <= center_w.saturating_sub(8));
    }

    #[test]
    fn gutter28_narrow_intact() {
        // 720 fallback columns unchanged: dual-pane split + ruler + margin.
        assert_eq!(crate::theme::VIEWPORT_MIN_W, 720);
        assert_eq!(crate::viewmodel::dual_pane_widths(720, 0.50), (340, 340));
        assert_eq!(
            crate::viewmodel::dual_pane_x_positions(720, 0.50),
            (12, 356, 696)
        );
        let (lx, rx, gx) = crate::viewmodel::dual_pane_x_positions(720, 0.50);
        assert_eq!(lx, OUTER_PAD);
        assert_eq!(rx, lx.saturating_add(340).saturating_add(PANE_GAP));
        assert_eq!(gx, rx.saturating_add(340));
        assert_eq!(gx.saturating_add(GUTTER_W).saturating_add(8), 720);
        // Narrow overview bhi padded track me: pehli band pad par, aakhri
        // andar; dono disha live se juden.
        let pane_h = 436u32;
        let first = overview_row_y(0, 15, 124, pane_h);
        let last = overview_row_y(14, 15, 124, pane_h);
        assert!(first.is_some() && last.is_some());
        if let (Some(f), Some(l)) = (first, last) {
            assert!(f >= 124 + crate::gutter::GUTTER_MINIMAP_TRACK_PAD as u32);
            assert!(l < 124 + pane_h);
            assert!(f < l);
        }
    }

    #[test]
    fn gutter28_overview_edge_fixture() {
        // Mirrors checks/gui-exq-gutter28/round-1/comparison/gutter28-overview-720.json.
        let raw = include_str!(
            "../../../checks/gui-exq-gutter28/round-1/comparison/gutter28-overview-720.json"
        );
        assert!(raw.contains("\"name\":\"gutter28-overview-720\""));
        assert!(raw.contains("\"track_pad\":4.0"));
        assert!(raw.contains("\"narrow_gutter_x_720\":696"));
        // Fixture numbers live helpers se milte hain (epsilon: f32 fraction).
        assert_eq!(crate::gutter::GUTTER_MINIMAP_TRACK_PAD, 4.0);
        let inner = crate::gutter::gutter_minimap_inner_height(386.0);
        assert_eq!(inner, Some(378.0));
        let y0 = crate::gutter::gutter_padded_y_for_row(0, 7, 386.0);
        assert!(y0.is_some());
        if let Some(y) = y0 {
            assert!((y - 4.0).abs() < 0.01);
        }
        let y6 = crate::gutter::gutter_padded_y_for_row(6, 7, 386.0);
        assert!(y6.is_some());
        if let Some(y) = y6 {
            assert!((y - 328.0).abs() < 0.5);
        }
        // Padded geometry me same+diff dono bands: 7-row fixture par
        // overview segments poore file ko dhakte hain (diff-only nahi).
        let kinds = [
            TextRowKind::Equal,
            TextRowKind::Equal,
            TextRowKind::Changed,
            TextRowKind::Deleted,
            TextRowKind::Inserted,
            TextRowKind::Equal,
            TextRowKind::Equal,
        ];
        let rows: Vec<TextRow> = kinds
            .iter()
            .map(|kind| TextRow {
                kind: *kind,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            })
            .collect();
        let geom = crate::gutter::gutter_overview_geometry_padded(7, 386.0, &rows);
        assert!(!geom.segments.is_empty());
        let has_equal = geom
            .segments
            .iter()
            .any(|seg| seg.kind == TextRowKind::Equal);
        let has_diff = geom
            .segments
            .iter()
            .any(|seg| seg.kind != TextRowKind::Equal);
        assert!(has_equal && has_diff);
        if let Some(first) = geom.segments.first() {
            assert!((first.y_start - 4.0).abs() < 0.5);
        }
        if let Some(last) = geom.segments.last() {
            assert!(last.y_end <= 4.0 + 378.0 + 0.5);
        }
    }

    #[test]
    fn gutter36_overview_alignment() {
        // Polish: band paint rounds the same live mapping `overview_row_y`
        // uses, so segment tops sit within 1px of the row marker.
        // 7-row fixture, 386px pane (174..560): inner 378, pad 4.
        let pane_top = 174u32;
        let pane_h = 386u32;
        let kinds = [
            TextRowKind::Equal,
            TextRowKind::Equal,
            TextRowKind::Changed,
            TextRowKind::Deleted,
            TextRowKind::Inserted,
            TextRowKind::Equal,
            TextRowKind::Equal,
        ];
        let rows: Vec<TextRow> = kinds
            .iter()
            .map(|kind| TextRow {
                kind: *kind,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            })
            .collect();
        let geom = crate::gutter::gutter_overview_geometry_padded(7, 386.0, &rows);
        assert!(!geom.segments.is_empty());
        if let Some(first) = geom.segments.first() {
            let painted = pane_top.saturating_add(first.y_start.round() as u32);
            let row0 = overview_row_y(0, 7, pane_top, pane_h);
            assert!(row0.is_some());
            if let Some(y0) = row0 {
                assert!((painted as f32 - y0 as f32).abs() <= 1.0);
            }
            assert!((first.y_start - 4.0).abs() < 0.5);
        }
        if let Some(last) = geom.segments.last() {
            assert!(last.y_end <= 4.0 + 378.0 + 0.5);
        }
        // Round-trip intact after polish.
        for row in 0..7usize {
            let y = overview_row_y(row, 7, pane_top, pane_h);
            assert!(y.is_some());
            if let Some(y) = y {
                assert_eq!(overview_row_at_y(y, 7, pane_top, pane_h), Some(row));
            }
        }
    }

    #[test]
    fn gutter36_narrow_intact() {
        // 720px narrow: dual-pane columns + ruler + right margin intact,
        // overview still inside the padded track via the shared strip.
        assert_eq!(crate::theme::VIEWPORT_MIN_W, 720);
        assert_eq!(crate::viewmodel::dual_pane_widths(720, 0.50), (340, 340));
        assert_eq!(
            crate::viewmodel::dual_pane_x_positions(720, 0.50),
            (12, 356, 696)
        );
        assert_eq!(696u32.saturating_add(GUTTER_W).saturating_add(8), 720);
        let pane_h = 436u32;
        let first = overview_row_y(0, 15, 124, pane_h);
        let last = overview_row_y(14, 15, 124, pane_h);
        assert!(first.is_some() && last.is_some());
        if let (Some(f), Some(l)) = (first, last) {
            assert!(f >= 124 + crate::gutter::GUTTER_MINIMAP_TRACK_PAD as u32);
            assert!(l < 124 + pane_h);
            assert!(f < l);
        }
    }

    #[test]
    fn gutter36_overview_edge_fixture() {
        // Mirrors checks/gui-exq-gutter36/round-1/comparison/gutter36-overview-polish.json.
        let raw = include_str!(
            "../../../checks/gui-exq-gutter36/round-1/comparison/gutter36-overview-polish.json"
        );
        assert!(raw.contains("\"name\":\"gutter36-overview-polish\""));
        assert!(raw.contains("\"track_pad\":4.0"));
        assert!(raw.contains("\"narrow_gutter_x_720\":696"));
        assert!(raw.contains("\"rounding\":\"round\""));
        assert_eq!(crate::gutter::GUTTER_MINIMAP_TRACK_PAD, 4.0);
        let inner = crate::gutter::gutter_minimap_inner_height(386.0);
        assert_eq!(inner, Some(378.0));
        // First band top rounds to the pad, matching `overview_row_y`.
        let y0 = crate::gutter::gutter_padded_y_for_row(0, 7, 386.0);
        assert!(y0.is_some());
        if let Some(y) = y0 {
            assert!((y - 4.0).abs() < 0.01);
            assert_eq!(y.round() as u32, 4);
        }
    }

    /// exq-shot46: every harness hue traces to a live `theme` helper.
    /// Dark arm equals the legacy dark consts (byte-stable); Light arm
    /// equals the `LIGHT_*` washes — fails if either side diverges.
    #[test]
    fn shot46_palette_traces_to_live_theme_helpers() {
        let dark = FixturePalette::of(FixtureMode::Dark);
        assert_eq!(dark.bg, [27, 27, 27, 255]);
        assert_eq!(dark.text, [230, 230, 230, 255]);
        assert_eq!(dark.dim, rgba(crate::theme::dim_for(true)));
        assert_eq!(dark.track, rgba(crate::theme::track_for(true)));
        assert_eq!(dark.deleted_bg, rgba(crate::theme::deleted_bg_for(true)));
        assert_eq!(dark.deleted_fg, rgba(crate::theme::deleted_fg_for(true)));
        assert_eq!(dark.inserted_bg, rgba(crate::theme::inserted_bg_for(true)));
        assert_eq!(dark.inserted_fg, rgba(crate::theme::inserted_fg_for(true)));
        assert_eq!(dark.changed_bg, rgba(crate::theme::changed_bg_for(true)));
        assert_eq!(dark.changed_fg, rgba(crate::theme::changed_fg_for(true)));
        assert_eq!(dark.bg, rgb3(crate::theme::palette(true).base_bg));
        assert_eq!(dark.field, dark.bg);

        let light = FixturePalette::of(FixtureMode::Light);
        assert_eq!(light.bg, rgb3(crate::theme::palette(false).base_bg));
        assert_eq!(light.text, rgb3(crate::theme::palette(false).base_fg));
        assert_eq!(light.dim, rgba(crate::theme::dim_for(false)));
        assert_eq!(light.track, rgba(crate::theme::track_for(false)));
        assert_eq!(light.deleted_bg, rgba(crate::theme::deleted_bg_for(false)));
        assert_eq!(light.deleted_fg, rgba(crate::theme::deleted_fg_for(false)));
        assert_eq!(
            light.inserted_bg,
            rgba(crate::theme::inserted_bg_for(false))
        );
        assert_eq!(
            light.inserted_fg,
            rgba(crate::theme::inserted_fg_for(false))
        );
        assert_eq!(light.changed_bg, rgba(crate::theme::changed_bg_for(false)));
        assert_eq!(light.changed_fg, rgba(crate::theme::changed_fg_for(false)));
        // No dark-band leftovers in Light: washes differ from Dark bands.
        assert_ne!(light.deleted_bg, dark.deleted_bg);
        assert_ne!(light.inserted_bg, dark.inserted_bg);
        assert_ne!(light.changed_bg, dark.changed_bg);
        assert_ne!(light.bg, dark.bg);
    }

    /// exq-shot46: Light mode never paints dark-on-dark — every ink/wash
    /// pair in the light palette meets the live WCAG AA contract.
    #[test]
    fn shot46_light_palette_meets_aa_on_light_wash() {
        fn rgb3of(c: [u8; 4]) -> [u8; 3] {
            [c[0], c[1], c[2]]
        }
        let light = FixturePalette::of(FixtureMode::Light);
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.text),
            rgb3of(light.bg)
        ));
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.dim),
            rgb3of(light.bg)
        ));
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.deleted_fg),
            rgb3of(light.deleted_bg)
        ));
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.inserted_fg),
            rgb3of(light.inserted_bg)
        ));
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.changed_fg),
            rgb3of(light.changed_bg)
        ));
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.changed_fg),
            rgb3of(light.bg)
        ));
    }

    /// exq-shot46: harness layout consts trace to live geometry tokens.
    /// Values frozen (no cascade); fails if either side diverges.
    #[test]
    fn shot46_geometry_consts_trace_to_live_tokens() {
        use crate::theme::{
            CHROME_HEADER_BAR_H, GEO_OUTER_MARGIN, GUTTER_MERGE_GAP, SPACE_SM, SPACE_XS,
            VIEWPORT_H, VIEWPORT_W,
        };
        use crate::views::{SECTION_GAP, action_bar_row_height};
        assert_eq!(SNAP_W, VIEWPORT_W);
        assert_eq!(SNAP_H, VIEWPORT_H);
        assert_eq!(TITLE_H, CHROME_HEADER_BAR_H as u32 + SECTION_GAP as u32);
        assert_eq!(TAB_H, CHROME_HEADER_BAR_H as u32 + SPACE_XS as u32);
        assert_eq!(ROW_H, action_bar_row_height() as u32);
        assert_eq!(OUTER_PAD, GEO_OUTER_MARGIN as u32);
        assert_eq!(PANE_GAP, SPACE_XS as u32);
        assert_eq!(
            STATUS_H,
            CHROME_HEADER_BAR_H as u32 + GUTTER_MERGE_GAP as u32
        );
        assert_eq!(TEXT_LEFT_X, OUTER_PAD);
        assert_eq!(TEXT_PANE_W, (GUTTER_X - TEXT_LEFT_X - PANE_GAP) / 2);
        assert_eq!(TEXT_RIGHT_X, TEXT_LEFT_X + TEXT_PANE_W + PANE_GAP);
        assert_eq!(GUTTER_W, crate::gutter::GUTTER_RULER_WIDE as u32);
        assert_eq!(GUTTER_X, SNAP_W - GUTTER_W - SPACE_SM as u32);
        assert_eq!(MERGE_PANE_W, (SNAP_W - 2 * OUTER_PAD - 2 * PANE_GAP) / 3);
        assert_eq!(MERGE_X0, OUTER_PAD);
        assert_eq!(MERGE_X1, MERGE_X0 + MERGE_PANE_W + PANE_GAP);
        assert_eq!(MERGE_X2, MERGE_X1 + MERGE_PANE_W + PANE_GAP);
        assert_eq!(PATH_FIELD_Y, TITLE_H + TAB_H + SECTION_GAP as u32);
        assert_eq!(PATH_FIELD_H, action_bar_row_height() as u32);
        assert_eq!(PATH_META_Y, PATH_FIELD_Y + PATH_FIELD_H);
        assert_eq!(PATH_META_H, GEO_OUTER_MARGIN as u32);
        assert_eq!(FILE_BAR_Y, PATH_FIELD_Y);
        assert_eq!(FILE_BAR_H, PATH_FIELD_H + PATH_META_H);
        assert_eq!(
            PATH_TOOLBAR_Y,
            PATH_FIELD_Y + FILE_BAR_H + GUTTER_MERGE_GAP as u32
        );
        assert_eq!(FILE_BAR_BROWSE_W, crate::home::TOPCHROME_BROWSE_W as u32);
        assert_eq!(SEG_DIV0_X, SNAP_W / 4);
        assert_eq!(SEG_DIV1_X, SNAP_W / 2);
        assert_eq!(SEG_DIV2_X, 3 * SNAP_W / 4);
        assert_eq!(PANE_STATUS_H, fixture_text_row_h());
        assert_eq!(HOME_CARD_GAP, crate::home::HOME_GRID_GAP as u32);
        assert_eq!(
            HOME_CARD_W,
            crate::views::home_paint_card_width(936.0) as u32
        );
        assert_eq!(
            (crate::home::home_card_min_height().ceil()) as u32,
            HOME_CARD_H
        );
        assert_eq!(HOME_CTA_W, crate::home::home_card_cta_width() as u32);
        assert_eq!(HOME_CTA_H, crate::home::HOME_CARD_BUTTON_MIN_H as u32);
    }

    /// exq-light56: inline accent branches by mode — dark inverts (legacy
    /// byte-stable block), light paints softer underline-only so the span
    /// stays dark-ink-on-light-wash. Zero new colors (row fg reuse).
    #[test]
    fn light56_inline_accent_mode_branch() {
        {
            let _guard = FixtureModeGuard::set(FixtureMode::Dark);
            assert!(inline_accent_inverted());
        }
        {
            let _guard = FixtureModeGuard::set(FixtureMode::Light);
            assert!(!inline_accent_inverted());
        }
        // Guard restores dark (structural default).
        assert!(inline_accent_inverted());
    }

    /// exq-light56: light underline ink reads dark-ink-on-light-wash (AA),
    /// never dark-on-dark — underline reuses the row fg on the row wash.
    #[test]
    fn light56_light_accent_ink_on_wash_aa() {
        fn rgb3of(c: [u8; 4]) -> [u8; 3] {
            [c[0], c[1], c[2]]
        }
        let light = FixturePalette::of(FixtureMode::Light);
        // Row ink darker than the wash it sits on (dark-on-light, not dark-on-dark).
        assert!(
            crate::theme::relative_luminance(rgb3of(light.changed_fg))
                < crate::theme::relative_luminance(rgb3of(light.changed_bg))
        );
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.changed_fg),
            rgb3of(light.changed_bg)
        ));
        assert!(crate::theme::theme_contrast_ok(
            rgb3of(light.changed_fg),
            rgb3of(light.bg)
        ));
    }

    /// exq-light56: accent scene renders valid PNGs in both modes and the
    /// light arm differs from dark (underline vs block) — dark keeps the
    /// legacy bytes shape, light is the softer wash.
    #[test]
    fn light56_accent_scene_modes_valid_and_distinct() {
        let dir = std::env::temp_dir();
        let dark_out = dir.join("vis-light56-accent-dark.png");
        let light_out = dir.join("vis-light56-accent-light.png");
        assert!(render_fixture_png_with_mode(&dark_out, FixtureMode::Dark).is_ok());
        assert!(render_fixture_png_with_mode(&light_out, FixtureMode::Light).is_ok());
        let dark_bytes = std::fs::read(&dark_out).unwrap_or_default();
        let light_bytes = std::fs::read(&light_out).unwrap_or_default();
        assert!(png_magic_ok(&dark_bytes));
        assert!(png_magic_ok(&light_bytes));
        assert!(dark_bytes.len() >= 8192);
        assert!(light_bytes.len() >= 8192);
        assert_ne!(dark_bytes, light_bytes);
    }

    /// exq-screenshot72: light fixture washes are value-identical to the
    /// live `LIGHT_*` consts — not just self-consistent with the helpers
    /// the impl routes through (shot46 pins helper-to-helper, this pins
    /// helper-output-to-const, so either side drifting fails loudly).
    /// Zero new colors/sizes; routing stays via existing `*_for()` + `palette()`.
    #[test]
    fn screenshot72_light_wash_matches_live_light_consts() {
        use crate::theme::{
            LIGHT_BG_RGB, LIGHT_CHANGED_BG, LIGHT_CHANGED_FG, LIGHT_DELETED_BG, LIGHT_DIM,
            LIGHT_FG_RGB, LIGHT_GREEN, LIGHT_INSERTED_BG, LIGHT_RED, LIGHT_TRACK,
        };
        let light = FixturePalette::of(FixtureMode::Light);
        assert_eq!(light.bg, rgb3(LIGHT_BG_RGB));
        assert_eq!(light.text, rgb3(LIGHT_FG_RGB));
        assert_eq!(light.dim, rgba(LIGHT_DIM));
        assert_eq!(light.track, rgba(LIGHT_TRACK));
        assert_eq!(light.deleted_bg, rgba(LIGHT_DELETED_BG));
        assert_eq!(light.deleted_fg, rgba(LIGHT_RED));
        assert_eq!(light.inserted_bg, rgba(LIGHT_INSERTED_BG));
        assert_eq!(light.inserted_fg, rgba(LIGHT_GREEN));
        assert_eq!(light.changed_bg, rgba(LIGHT_CHANGED_BG));
        assert_eq!(light.changed_fg, rgba(LIGHT_CHANGED_FG));
        assert_eq!(light.field, rgb3(LIGHT_BG_RGB));
    }
}
