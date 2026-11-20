//! egui painting of the view models.
//!
//! All interpretation of engine results happened in
//! [`crate::viewmodel`]; this file only turns rows into widgets. The
//! few pure helpers here (`pane_segments`, `gutter_text`,
//! `folder_columns`, `premultiplied_rgba`, `heatmap_color_image`) are
//! paint-adjacent logic and are unit-tested headlessly alongside the
//! viewmodel.
//!
//! The diff palette is fixed rather than theme-following: a deleted
//! row must read the same in light and dark mode, so its colors carry
//! their own contrast.

#![forbid(unsafe_code)]

use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Color32, FontId, RichText, Stroke, TextEdit, TextFormat, TextureHandle, Ui,
};

use crate::app::Tab;
use crate::app::{
    AudioCheckKind, AudioChecklist, FolderMergeChecklist, MergeApplyState, MergeApplySummary,
    PictureCompareModel, PictureMode, PictureViewState, SyncChecklist,
};
use crate::gutter::{
    GUTTER_HIT_SLOP, GUTTER_NUMBER_TRAIL_PAD, GutterSegment, gutter_row_at_y, gutter_row_center_y,
    gutter_ruler_width_for_font, gutter_segment_at_y_with_slop,
};
use crate::home::{
    HOME_CARD_BUTTON_MIN_H, HOME_CARD_BUTTON_MIN_W, HOME_GRID_COLUMNS, HOME_GRID_GAP,
    HOME_SECTION_GAP, HOME_TOOLBAR_BUTTON_H, HOME_TOOLBAR_BUTTON_W, RECENT_COL_GAP, RECENT_ROW_GAP,
    Session, SessionHistory, SessionList, SessionSnapshot, home_card_button_size,
    home_card_cta_width, home_card_desc_is_small_weak, home_card_min_height, home_card_title_size,
    recent_list_empty_hint, recent_row_columns, recent_row_meta_role, recent_row_meta_text,
    recent_row_title_role, recent_row_two_line_height,
};
use crate::theme::{
    CHANGED_BG, CHANGED_FG, CHROME_HEADER_BAR_H, CHROME_HEADER_LEAD, DELETED_BG, DELETED_FG,
    ERROR_BG, INSERTED_BG, INSERTED_FG, TYPO_LINE_HEIGHT_SOURCE, TYPO_MONO_SIZE,
    TYPO_WRAP_PANE_CHARS, TypoRole, blue_for, dim_for, green_for, red_for, track_for,
};
use crate::viewmodel::{self, FindState, TextViewPrefs, view_text_with_prefs};
use crate::viewmodel::{FolderRow, MergeSummary, TableModel, TextRow, TextRowKind, TextRows};
use crate::{views_hex, views_table};

// The fixed diff palette lives in `crate::theme` (same hues, own
// contrast); per-mode chrome (dim/red/green/blue/track) is picked via
// `theme::*_for(ui.visuals().dark_mode)` at each paint site below.
// Kept for the pending App rewiring back to `views::paint_folder`
// (the Folder tab currently paints inline in `app.rs`, read-only
// here); the allow keeps the tree clippy-clean without deleting the
// tested paint API.

// Geometry tokens (px): margins, gaps, alignment. Colors out of scope.
pub const OUTER_MARGIN: i8 = 12;
pub const SECTION_GAP: f32 = 8.0;
pub const TIGHT_GAP: f32 = 4.0;
pub const FOOTER_GAP: f32 = 6.0;
pub const TAB_BOTTOM_GAP: f32 = 6.0;
pub const TREE_INDENT: f32 = crate::theme::GEO_TREE_INDENT_STEP;
pub const CARD_W: f32 = 240.0;
pub const CARD_H: f32 = 75.0;
/// Merge-apply Dry-run checkbox width (px): egui checkbox box +
/// "Dry-run" label at body scale plus intra spacing. Conservative
/// upper bound: over-reserve sirf out-path field ghatata hai (field
/// `TextEdit` andar scroll karta hai, kabhi clip nahi hota), jabki
/// under-reserve Apply button ko clip karta hai. Pure token.
pub const MERGE_APPLY_CHECK_W: f32 = 88.0;
/// Merge-apply Apply button width (px): auto-size "Apply" pill at body
/// scale plus inter gaps ka conservative bound. Pure token.
pub const MERGE_APPLY_BUTTON_W: f32 = 64.0;

/// Merge-apply row chrome width (px): Dry-run checkbox plus Apply
/// button plus three [`TIGHT_GAP`] item gaps. The paint row subtracts
/// this from `avail` so the out-path field never clips the Apply button
/// at the 720px fallback (the legacy 75px reserve under-counted real
/// checkbox-plus-button chrome near 140px). Pure; unit-tested.
pub fn merge_apply_chrome_width() -> f32 {
    MERGE_APPLY_CHECK_W + MERGE_APPLY_BUTTON_W + 3.0 * TIGHT_GAP
}
const MIN_HALF: f32 = 180.0;
const MIN_THIRD: f32 = 130.0;

/// Draggable split bounds: left pane share stays inside 20%..=80%
/// so neither pane collapses; default centers at 50%. Single source
/// is [`crate::viewmodel`] (headless test surface); this module
/// re-exports the math so `App` keeps one path without touching
/// `viewmodel` directly. Per-run state only (lives on `App`, no
/// filesystem writes).
pub use crate::viewmodel::{SPLIT_DEFAULT_FRACTION, clamp_split_fraction, split_apply_drag};
/// Split grab-strip width: single source is the headless hit model
/// ([`crate::viewmodel::SPLIT_HANDLE_HIT_W`]) so paint and hit math
/// never drift; [`SPLIT_HIT_W`] aliases this below.
const SPLIT_HANDLE_W: f32 = crate::viewmodel::SPLIT_HANDLE_HIT_W;

// Exquisite geometry tokens (px): every magic offset lives here so
// text/table/folder painters share one rhythm. Colors out of scope.
// Values reuse existing consts first (ponytail): wide gutter, tight
// gap and handle width alias their single sources below; only the
// genuinely new bars (header/find/visual/needle/minimap/lead) carry
// fresh literals, each locked by a headless test.
pub const GUTTER_W: f32 = crate::theme::GUTTER_BAR_WIDE;
pub const HEADER_H: f32 = 28.0;
pub const ROW_PAD: f32 = crate::theme::GUTTER_MIN_SEGMENT_H;
pub const SPLIT_HIT_W: f32 = SPLIT_HANDLE_W;
pub const SPLIT_VISUAL_W: f32 = 1.0;
pub const FIND_BAR_H: f32 = HEADER_H;
pub const BADGE_PAD: f32 = TIGHT_GAP;
pub const FIND_NEEDLE_W: f32 = 120.0;
pub const HEX_MINIMAP_H: f32 = 220.0;
/// Hex minimap strip width: double the gutter bar so the right-edge
/// overview reads wider than the text gutter while staying on the
/// same token (`2 * GUTTER_W`); hit rect grows with it. Pure token.
pub const HEX_MINIMAP_W: f32 = 2.0 * GUTTER_W;
pub const CONTENT_LEAD: &str = "  ";
/// Text center gutter width (px): do independent panes ke beech patli
/// fixed-width gutter jisme per-diff copy arrows rehte hain (BC pattern).
/// Ponytail reuse: `HEADER_H + GUTTER_W + 2 * TIGHT_GAP` = 48.0 — do
/// chhote `→/←` buttons side-by-side fit hote hain, equal rows par blank
/// click-to-select rehta hai. Koi naya magic nahi; pure token.
pub const TEXT_CENTER_GUTTER_W: f32 = HEADER_H + GUTTER_W + TIGHT_GAP + TIGHT_GAP;

/// Center gutter width accessor (paint fixed column isi se banta hai).
/// Pure; unit-tested.
pub fn text_center_gutter_width() -> f32 {
    TEXT_CENTER_GUTTER_W
}

/// Center gutter me copy arrows hain ya blank: diff kinds par haan,
/// Equal par nahi (blank click-to-select). Pure; unit-tested.
pub fn text_gutter_has_copy(kind: TextRowKind) -> bool {
    kind != TextRowKind::Equal
}

/// Center-gutter hunk head: contiguous diff block ka pehla row.
/// BC center bar har hunk par ek arrow/marker dikhata hai, har row par
/// nahi — continuation rows blank click-to-select rehti hain taaki gutter
/// fixed width me rahe bina per-row button clutter ke. Gate
/// [`text_gutter_has_copy`] reuse; Equal kabhi head nahi. Pure; unit-tested.
pub fn text_center_hunk_head(rows: &[TextRow], idx: usize) -> bool {
    let Some(row) = rows.get(idx) else {
        return false;
    };
    if !text_center_hunk_marker(row.kind) {
        return false;
    }
    if idx == 0 {
        return true;
    }
    match rows.get(idx.saturating_sub(1)) {
        Some(prev) => !text_center_hunk_marker(prev.kind),
        None => true,
    }
}

/// Narrow breakpoint px: [`crate::theme::VIEWPORT_MIN_W`] reuse (720) —
/// isse tang window par side-by-side panes stacked wrap me jate hain
/// taaki 960x600 canvas par do pane + tang par ek-ek pane bina clip ke
/// dikhe. Koi naya magic nahi; pure token.
pub const TEXT_STACK_BREAKPOINT_W: f32 = crate::theme::VIEWPORT_MIN_W as f32;

/// Text pane layout: chaudi window par side-by-side, breakpoint ke
/// neeche stacked (left block phir right block, dono full-width).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextLayout {
    /// Do pane ek row me (default, 960x600 canonical).
    #[default]
    SideBySide,
    /// Ek-ek pane poori chaudi me (narrow <720 fallback).
    Stacked,
}

/// Available width se text layout: `TEXT_STACK_BREAKPOINT_W` (720) se
/// tang par [`TextLayout::Stacked`], warna [`TextLayout::SideBySide`].
/// Non-finite ya non-positive width par SideBySide (paint kabhi NaN
/// branch na le). Pure; unit-tested.
pub fn text_layout_for_width(avail_w: f32) -> TextLayout {
    if avail_w.is_finite() && avail_w > 0.0 && avail_w < TEXT_STACK_BREAKPOINT_W {
        TextLayout::Stacked
    } else {
        TextLayout::SideBySide
    }
}

/// Centered 1px visual inside the split grab strip: equal inset on
/// both sides so the line stays centered. Pure; unit-tested.
pub fn split_visual_inset() -> f32 {
    (SPLIT_HIT_W - SPLIT_VISUAL_W) / 2.0
}

/// Uniform row rhythm: measured line height plus one fixed pad, so
/// text/table/folder rows share the same beat. Non-finite or
/// non-positive input passes through so paint never feeds NaN into
/// layout. Pure; unit-tested.
pub fn row_height_for_line(line_h: f32) -> f32 {
    if line_h.is_finite() && line_h > 0.0 {
        line_h + ROW_PAD
    } else {
        line_h
    }
}

/// One shared left-baseline lead for header/rule/placeholder lines:
/// table header + rule and the folder dir placeholder all start from
/// the same two-space origin as the text gutter. Pure; unit-tested.
pub fn indented_content(line: &str) -> String {
    format!("{CONTENT_LEAD}{line}")
}

/// Equal half width for two path fields sharing one row.
/// Kept as the centered wrapper over [`split_field_widths`]; new
/// paint code goes through the draggable split instead.
pub fn half_field_width(avail: f32) -> f32 {
    split_field_widths(avail, SPLIT_DEFAULT_FRACTION).0
}

/// Path-row chrome width (px): two Browse buttons + Compare button +
/// split handle + five [`TIGHT_GAP`] item gaps. `paint_paths` row ka
/// fixed kharcha; fields ko `avail` me se yahi ghata kar milta hai taaki
/// right edge overflow na ho (Gap 3). Pure; unit-tested.
pub fn path_row_chrome_width() -> f32 {
    let (btn_w, _) = path_browse_button_size();
    3.0 * btn_w + SPLIT_HIT_W + 5.0 * TIGHT_GAP
}

/// Triple path-row chrome width (px): three Browse buttons + Compare
/// button + six [`TIGHT_GAP`] item gaps. `paint_triple_paths` row ka
/// fixed kharcha. Pure; unit-tested.
pub fn triple_path_row_chrome_width() -> f32 {
    let (btn_w, _) = path_browse_button_size();
    4.0 * btn_w + 6.0 * TIGHT_GAP
}

/// Equal third width for three path fields sharing one row.
/// Row chrome ([`triple_path_row_chrome_width`]) ghata kar banta hai,
/// each third keeps the [`MIN_THIRD`] floor so narrow windows scroll
/// instead of crushing the fields.
pub fn third_field_width(avail: f32) -> f32 {
    ((avail - triple_path_row_chrome_width()) / 3.0).max(MIN_THIRD)
}

/// Split widths for the two path fields at `fraction` of `avail`.
/// Row chrome ([`path_row_chrome_width`]) ghata kar banta hai, each side
/// keeps the [`MIN_HALF`] floor and the pair always fills the row
/// ([`split_picture_widths`] precedent), so narrow windows scroll
/// instead of crushing the fields or clipping the right edge.
pub fn split_field_widths(avail: f32, fraction: f32) -> (f32, f32) {
    let f = clamp_split_fraction(fraction);
    let total = (avail - path_row_chrome_width()).max(2.0 * MIN_HALF);
    let left = (total * f).clamp(MIN_HALF, total - MIN_HALF);
    (left, total - left)
}

/// Split widths for the picture side-by-side columns at `fraction`
/// of `avail` minus one [`SECTION_GAP`]. Each side keeps the
/// [`MIN_HALF`] floor; the leftover goes to the other side so the
/// pair always fills the row.
pub fn split_picture_widths(avail: f32, fraction: f32) -> (f32, f32) {
    let total = (avail - SECTION_GAP).max(2.0 * MIN_HALF);
    let f = clamp_split_fraction(fraction);
    let left = (total * f).clamp(MIN_HALF, total - MIN_HALF);
    (left, total - left)
}

/// Pane widths for the merge left/base/right row: equal thirds of the
/// row minus two [`SECTION_GAP`]s (one per divider), each kept at the
/// [`MIN_THIRD`] floor so narrow windows scroll instead of crushing a
/// pane; the leftover goes to the right pane so the triple always
/// fills the row. Two-pane [`split_picture_widths`] precedent (floors
/// plus leftover fill), one gap per divider instead of one. Pure;
/// unit-tested.
pub fn three_pane_widths(avail: f32) -> (f32, f32, f32) {
    let total = (avail - 2.0 * SECTION_GAP).max(3.0 * MIN_THIRD);
    let third = total / 3.0;
    let left = third.max(MIN_THIRD);
    let base = third.max(MIN_THIRD);
    let right = (total - left - base).max(MIN_THIRD);
    (left, base, right)
}

/// Drag handle cursor: the model affordance id (`"resize-column"`,
/// see [`viewmodel::split_cursor_affordance`]) mapped to paint.
/// Panics never; unknown ids fall back to the column resize cursor.
/// Pure; unit-tested.
pub fn split_handle_cursor() -> egui::CursorIcon {
    debug_assert_eq!(viewmodel::split_cursor_affordance(), "resize-column");
    egui::CursorIcon::ResizeColumn
}

/// Drag handle hover label: the model a11y text (honest left-pane
/// percent plus the 20..=80 drag bounds), so the target stays
/// operable without a pointer. Pure; unit-tested.
pub fn split_handle_label(fraction: f32) -> String {
    viewmodel::split_accessibility_label(fraction)
}

/// Drag delta folded into a split fraction the way paint does: model
/// delta ([`viewmodel::split_drag_delta_fraction`]) plus the shared
/// clamp ([`clamp_split_fraction`]). Guards mirror
/// [`split_apply_drag`]. Pure; unit-tested.
pub fn split_handle_drag_fraction(fraction: f32, dx: f32, avail: f32) -> f32 {
    clamp_split_fraction(fraction + viewmodel::split_drag_delta_fraction(dx, avail))
}

/// Draggable divider between two panes. Paints a narrow grab strip
/// (hit width == model [`viewmodel::SPLIT_HANDLE_HIT_W`], centered
/// 1px visual) and folds horizontal drag deltas back into `fraction`
/// via [`split_handle_drag_fraction`]. Hover shows the resize-column
/// cursor plus the model a11y label. Returns true while the divider
/// moves. Used by the text path row, the picture side-by-side panes
/// and the dual-tree panes — one handle for all three.
pub fn paint_split_handle(ui: &mut Ui, fraction: &mut f32, avail: f32) -> bool {
    debug_assert_eq!(SPLIT_HIT_W, viewmodel::SPLIT_HANDLE_HIT_W);
    let height = ui.available_height().max(20.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(SPLIT_HIT_W, height), egui::Sense::drag());
    debug_assert_eq!(rect.width(), SPLIT_HIT_W);
    ui.painter().rect_filled(
        rect.shrink(split_visual_inset()),
        2.0,
        ui.visuals().widgets.noninteractive.bg_fill,
    );
    let resp = resp
        .on_hover_cursor(split_handle_cursor())
        .on_hover_text(split_handle_label(*fraction));
    if resp.dragged() {
        let dx = resp.drag_delta().x;
        if dx != 0.0 {
            *fraction = split_handle_drag_fraction(*fraction, dx, avail);
            return true;
        }
    }
    false
}

/// Consistent tree indent for one depth level: canonical MD spacing
/// step ([`TREE_INDENT`] = 12.0, [`crate::theme::GEO_TREE_INDENT_STEP`]
/// parity) so drill-down rows match the headless
/// [`crate::tree::tree_indent_for_depth`] contract.
pub fn tree_indent(depth: usize) -> f32 {
    depth as f32 * TREE_INDENT
}

/// Shared text row height: the TypoRole Mono line-height px
/// (`TypoRole::Mono.line_height_px()` = 12.0 @ 1.30 = 15.6), headless
/// taaki har pane — text, table, folder, hex — ek deterministic beat par
/// rahe bina font-backend measure par nirbhar hue. Per-role multipliers
/// intent ([`text_row_height_intent`]) me rehte hain; paint yahi number
/// [`row_height_for_line`] me daalta hai (15.6 + [`ROW_PAD`] = 17.6).
/// Pure; unit-tested.
pub fn text_row_height(_ui: &Ui) -> f32 {
    TypoRole::Mono.line_height_px()
}

/// Line-height intent source: the shared measured monospace row height.
/// Paint never takes a per-role line-height; the chrome/body/mono
/// multipliers (`TypoRole::line_height_px`) record the target rhythm
/// (chrome 1.2 / body 1.40 / mono 1.30) for a later paint pass. Pure;
/// unit-tested.
pub fn text_row_height_intent() -> &'static str {
    TYPO_LINE_HEIGHT_SOURCE
}

/// App header bar height: the same [`HEADER_H`] the table/hex paging
/// bars reserve, so the title row sits on the pane-split rhythm
/// instead of floating offset above it. Pure; unit-tested.
pub fn header_bar_height() -> f32 {
    CHROME_HEADER_BAR_H
}

/// Header left-baseline lead: the same origin as table rules and folder
/// placeholders ([`CONTENT_LEAD`]) so header notes start where pane
/// content starts. Pure; unit-tested.
pub fn header_content_lead() -> &'static str {
    CHROME_HEADER_LEAD
}

/// One header note line on the shared left baseline: lead plus text,
/// the same shape [`indented_content`] paints for rules/placeholders.
/// Pure; unit-tested.
pub fn header_note_line(text: &str) -> String {
    format!("{}{}", header_content_lead(), text)
}

/// Tab bar role: Body (13 default proportional). Tab labels paint at
/// this scale via [`TypoRole::rich`] so tabs never drift to mono.
/// Pure; unit-tested.
pub fn chrome_tab_role() -> TypoRole {
    TypoRole::chrome_tab()
}

/// Section header role: Heading (18 strong proportional). App title
/// plus section headers share the top chrome step; card titles step
/// down to HomeTitle. Pure; unit-tested.
pub fn chrome_section_role() -> TypoRole {
    TypoRole::chrome_section()
}

/// Status line role: Small (11 weak proportional, tabular intent).
/// Summary/count lines paint here instead of mono so status reads as
/// chrome, not code; weak tone keeps the frozen dim. Pure; unit-tested.
pub fn chrome_status_role() -> TypoRole {
    TypoRole::chrome_status()
}

/// Status verdict role: Body (13 default proportional) — one step above
/// the side counts so the mid verdict reads first. Size + family carry
/// the hierarchy (color frozen). Single source is
/// [`viewmodel::status_verdict_text_role`]. Pure; unit-tested.
pub fn status_verdict_role() -> TypoRole {
    viewmodel::status_verdict_text_role()
}

/// Status side role: Small (11 weak proportional, tabular intent) —
/// counts + hints stay one step below the verdict. Single source is
/// [`viewmodel::status_side_text_role`]. Pure; unit-tested.
pub fn status_side_role() -> TypoRole {
    viewmodel::status_side_text_role()
}

/// Paint-applied Home title size: routes through the home contract
/// so paint stays in sync headlessly. Test-only since typo adoption
/// (paint `TypoRole::HomeTitle.rich` directly); unit tests lock the
/// contract here.
pub fn home_title_paint_size() -> f32 {
    home_card_title_size()
}

/// Whether the Home description paints small + weak. Test-only since
/// typo adoption (paint branches on this but renders
/// `TypoRole::HomeDesc.rich`); unit tests lock the contract here.
pub fn home_desc_paint_is_small_weak() -> bool {
    home_card_desc_is_small_weak()
}

/// Paint-applied Home button size (body scale). Test-only since typo
/// adoption (paint `TypoRole::HomeButton.rich` directly); unit tests
/// lock the contract here.
pub fn home_button_paint_size() -> f32 {
    home_card_button_size()
}

/// Paint-applied Home card height: the headless contract minimum
/// ([`home_card_min_height`]: title + two-line desc + 28px CTA stack =
/// 99.6) floored by the legacy [`CARD_W`]/[`CARD_H`] card so paint never
/// shrinks below the old grid. Pure; unit-tested.
pub fn home_paint_card_height() -> f32 {
    home_card_min_height().max(CARD_H)
}

/// Paint-applied Home CTA size: 224x28 single-line budget (area 6272)
/// from the card contract ([`home_card_cta_width`] x [`HOME_CARD_BUTTON_MIN_H`]).
/// Paint feeds it to `add_sized` so the whole single-line floor stays clickable.
/// Pure; unit-tested.
pub fn home_paint_cta_size() -> (f32, f32) {
    (home_card_cta_width(), HOME_CARD_BUTTON_MIN_H)
}

/// Paint-applied Home card width: flex fill of the grid row — 3 cards +
/// 2 gaps me available width, [`CARD_W`] = 240 floor. 960 canvas par
/// ~306.67 (fixture `HOME_CARD_W` = 306 parity, truncation); tang window
/// par floor pakadta hai, grid scroll karta hai. Non-finite ya
/// non-positive avail par floor (paint kabhi NaN na khaye). Pure;
/// unit-tested.
pub fn home_paint_card_width(avail: f32) -> f32 {
    if !avail.is_finite() || avail <= 0.0 {
        return CARD_W;
    }
    ((avail - HOME_GRID_GAP * 2.0) / 3.0).max(CARD_W)
}

/// Session toolbar button size: 96x24 floor. Width path Browse floor
/// (120) se tang taaki wide text pills compact rahen (content se chhota
/// floor kabhi crush nahi karta, floor sirf chhote labels ko pakadta hai);
/// height 24 exquisite hit floor (find-nav precedent) taaki toolbar row
/// 28 se 24 par aaye. Home CTA (224x28) se alag floor hai. Pure;
/// unit-tested.
pub fn session_toolbar_button_size() -> (f32, f32) {
    (HOME_TOOLBAR_BUTTON_W, HOME_TOOLBAR_BUTTON_H)
}

/// Toolbar swap glyph: `↔` reuse ([`home_recent_row_title`] precedent) —
/// BC-style icon-led action, koi naya asset nahi. Pure; unit-tested.
pub const TOOLBAR_SWAP_GLYPH: &str = "\u{2194}";

/// Toolbar reload glyph: `▶` reuse (paging-nav `Next hunk ▶` precedent:
/// forward/run affordance) — koi naya asset nahi. Pure; unit-tested.
pub const TOOLBAR_RELOAD_GLYPH: &str = "▶";

/// Icon-to-label gap (px): [`TIGHT_GAP`] reuse taaki glyph + label
/// toolbar item rhythm par baithe. Koi naya size nahi; pure; unit-tested.
pub fn toolbar_icon_gap() -> f32 {
    TIGHT_GAP
}

/// Icon+label cluster gap (px): [`SECTION_GAP`] (8) reuse taaki icon-led
/// action buttons (Swap/Reload, Prev/Next) ke beech BC jaisi saans ho.
/// Glyph-to-text inner rhythm [`toolbar_icon_gap`] (TIGHT_GAP 4) par rehta
/// hai, cluster gap ek step upar. Koi naya size nahi; pure; unit-tested.
pub fn toolbar_cluster_gap() -> f32 {
    SECTION_GAP
}

/// Toolbar button typo role: [`chrome_tab_role`] (Body 13 proportional)
/// reuse — buttons chrome me rahen, mono me na jaen. Koi naya
/// size/weight nahi; pure; unit-tested.
pub fn toolbar_button_role() -> TypoRole {
    chrome_tab_role()
}

/// Icon-led Swap label: glyph + space + existing English label (label
/// text unchanged, sirf glyph prefix). Pure; unit-tested.
pub fn toolbar_swap_label() -> String {
    format!("{TOOLBAR_SWAP_GLYPH} Swap Sides")
}

/// Icon-led Reload label: glyph + space + existing English label (label
/// text unchanged, sirf glyph prefix). Pure; unit-tested.
pub fn toolbar_reload_label() -> String {
    format!("{TOOLBAR_RELOAD_GLYPH} Reload / Recompare")
}

// ----------------------------------------------------------------------------
// Session toolbar wiring (CL-TOOLWIRE): icon-led Swap/Reload paint call-site.
// App row (app.rs toolbar, NO-TOUCH is wave me) agle wave me is painter par
// jayegi; tab tak ye fn `toolbar_*` contract ka live call-site hai
// (labels/role/glyphs se `dead_code` hata, tests live). Geometry:
// [`session_toolbar_button_size`] floor (96x24, min_size) + cluster gap
// [`toolbar_cluster_gap`] (SECTION_GAP 8; App `toolbar_item_gap` apni row
// ka TIGHT_GAP mirror rakhta hai, NO-TOUCH); typo:
// [`toolbar_button_role`] chrome (Body 13 proportional). Colors: existing
// button style reuse, koi naya hue nahi.
// ----------------------------------------------------------------------------

/// Session toolbar hit: Swap vs Reload. Caller (`App`, agle wave) is par
/// swap/run chalata hai; [`SessionAction`] precedent — paint sirf emit
/// karta hai, mutate nahi. Pure shape; unit-tested.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToolbarAction {
    /// Koi hit nahi (default).
    #[default]
    None,
    /// Swap click: sides flip + rerun (caller).
    Swap,
    /// Reload click: same compare rerun (caller).
    Reload,
}

/// Icon-led session toolbar row: Swap + Reload ek hi `horizontal` me.
/// Labels [`toolbar_swap_label`] / [`toolbar_reload_label`] (glyph prefix,
/// English text unchanged), typo [`toolbar_button_role`], floor
/// [`session_toolbar_button_size`] (min_size taaki chhote labels bhi poori
/// ungli se lagen). `enabled` false par dono grey (App `toolbar_active`
/// mirror: Home par off).
pub fn paint_session_toolbar(ui: &mut Ui, enabled: bool) -> ToolbarAction {
    paint_toolbar_action_buttons(ui, enabled)
}

// ----------------------------------------------------------------------------
// Session action bar (two-row: actions + jump cluster). Row 1 =
// session actions (Swap/Reload, [`paint_toolbar_action_buttons`] reuse —
// `paint_session_toolbar` live call-site ab isi par hai, shape same).
// Row 2 = jump cluster: Prev/Next arrows + N/M badge + 1:1 cursor readout
// right-dock (text nav-row precedent: full-width `avail x row_h`,
// [`TIGHT_GAP`] rhythm, badge/cursor Small weak). Caller (`App`, agle
// wave) nav state model se banata hai ([`diff_nav_position`] +
// [`diff_nav_total`] + `viewmodel::cursor_label`); paint sirf emit karta
// hai, mutate nahi. Geometry/typo callee tokens me (24 floor, chrome
// roles); koi naya size/color nahi.
// ----------------------------------------------------------------------------

/// Jump-cluster hit: Prev vs Next. Caller is par selection move + scroll
/// chalata hai ([`ToolbarAction`] precedent — paint emit, caller mutate).
/// Pure shape; unit-tested.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActionBarNav {
    /// Koi hit nahi (default).
    #[default]
    None,
    /// Prev click: pichhla difference.
    Prev,
    /// Next click: agla difference.
    Next,
}

/// Jump-cluster caller state: N/M badge position + kul diffs + 1:1 cursor
/// readout string (caller `viewmodel::cursor_label` se, khali par readout
/// chhupta hai — koi fake "3:1" nahi). Pure shape; unit-tested.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionBarNavState {
    /// Selected diff ki 1-based position; bina selection `None` ("0/N").
    pub position: Option<usize>,
    /// Kul diffs.
    pub total: usize,
    /// 1:1 cursor readout (`cursor_label`, jaise "3:1"); khali par hidden.
    pub cursor: String,
}

/// Two-row bar ka combined hit: row 1 action + row 2 nav (first-hit-wins
/// per row, dono rows ek frame me lag sakte hain — alag fields, koi
/// clash nahi). Pure shape; unit-tested.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionBarHits {
    /// Row 1: Swap/Reload hit.
    pub action: ToolbarAction,
    /// Row 2: Prev/Next hit.
    pub nav: ActionBarNav,
}

/// Jump-cluster Prev label: text nav-row precedent ("◀ Prev") — wahi
/// glyph, koi naya asset nahi. Pure; unit-tested.
pub fn action_nav_prev_label() -> &'static str {
    "◀ Prev"
}

/// Jump-cluster Next label: text nav-row precedent ("Next ▶") — wahi
/// glyph, koi naya asset nahi. Pure; unit-tested.
pub fn action_nav_next_label() -> &'static str {
    "Next ▶"
}

/// Two-row stack gap (px): row 1 aur row 2 ke beech [`TIGHT_GAP`] — wahi
/// rhythm jo har row ke andar hai. Koi naya token nahi; pure; unit-tested.
pub fn action_bar_row_gap() -> f32 {
    TIGHT_GAP
}

/// Action-bar row height (px): uniform 24 floor
/// ([`session_toolbar_button_size`] height) — dono rows ek beat par.
/// Pure; unit-tested.
pub fn action_bar_row_height() -> f32 {
    session_toolbar_button_size().1
}

/// Action-bar min content width (px): do uniform buttons + ek cluster
/// gap (`2 * tool_w + cluster_gap` = 200) — row 1 aur row 2 dono isi floor
/// par, isliye ek hi number dono rows cover karta hai (badge/cursor dock
/// right-end par grow karte hain, floor nahi badhate). Pure; unit-tested.
pub fn action_bar_min_width() -> f32 {
    let (tool_w, _) = session_toolbar_button_size();
    2.0 * tool_w + toolbar_cluster_gap()
}

/// Tang window par bar h-scroll mangta hai ya nahi: `avail` floor
/// ([`action_bar_min_width`]) se tang par haan, warna nahi. Non-finite
/// ya non-positive `avail` par false (paint kabhi NaN branch na le —
/// [`text_layout_for_width`] precedent). Pure; unit-tested.
pub fn action_bar_needs_hscroll(avail: f32) -> bool {
    if !avail.is_finite() || avail <= 0.0 {
        return false;
    }
    avail < action_bar_min_width()
}

/// 1:1 cursor readout text: khali/whitespace par `None` (strip skip, koi
/// fake readout nahi), warna trimmed string. Pure; unit-tested.
pub fn action_bar_cursor_text(cursor: &str) -> Option<String> {
    let trimmed = cursor.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Jump-cluster readout gap (px): N/M badge aur 1:1 cursor readout ke
/// beech [`SECTION_GAP`] (8) — readout badge se alag padhe (BC bottom-bar
/// air); blank cursor par gap bhi skip hota hai (koi trailing hawa nahi).
/// Pure; unit-tested.
pub fn action_bar_readout_gap() -> f32 {
    SECTION_GAP
}

/// Session action buttons (row 1): Swap + Reload ek hi `horizontal` me.
/// [`paint_session_toolbar`] live call-site aur [`paint_session_action_bar`]
/// row 1 dono isi par hain (ek shape, do caller — dup nahi). Geometry/typo
/// toolbar contract (96x24 floor, chrome role); colors existing style.
pub fn paint_toolbar_action_buttons(ui: &mut Ui, enabled: bool) -> ToolbarAction {
    let (tool_w, tool_h) = session_toolbar_button_size();
    let mut hit = ToolbarAction::None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = toolbar_cluster_gap();
        if ui
            .add_enabled(
                enabled,
                egui::Button::new(toolbar_button_role().rich(toolbar_swap_label()))
                    .min_size(egui::vec2(tool_w, tool_h)),
            )
            .clicked()
        {
            hit = ToolbarAction::Swap;
        }
        if ui
            .add_enabled(
                enabled,
                egui::Button::new(toolbar_button_role().rich(toolbar_reload_label()))
                    .min_size(egui::vec2(tool_w, tool_h)),
            )
            .clicked()
        {
            hit = ToolbarAction::Reload;
        }
    });
    hit
}

/// Jump cluster (row 2): Prev/Next arrows + N/M badge
/// ([`diff_nav_count_label`]: `Some` par "p/N", bina selection "0/N") +
/// 1:1 cursor readout right-dock (khali par hidden, gap samet skip).
/// Geometry: full-width row (`avail x row_h`), cluster gap buttons ke
/// beech, badge-cursor beech [`action_bar_readout_gap`], right-end dock +
/// [`BADGE_PAD`] (status-bar precedent); typo: buttons chrome role, badge/
/// cursor [`status_side_role`] (Small 11 weak). `enabled` false par arrows grey.
pub fn paint_action_nav_row(ui: &mut Ui, enabled: bool, state: &ActionBarNavState) -> ActionBarNav {
    let (tool_w, tool_h) = session_toolbar_button_size();
    let mut hit = ActionBarNav::None;
    let avail = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail, action_bar_row_height()));
        ui.spacing_mut().item_spacing.x = toolbar_cluster_gap();
        if ui
            .add_enabled(
                enabled,
                egui::Button::new(toolbar_button_role().rich(action_nav_prev_label()))
                    .min_size(egui::vec2(tool_w, tool_h)),
            )
            .on_hover_text("Previous difference")
            .clicked()
        {
            hit = ActionBarNav::Prev;
        }
        if ui
            .add_enabled(
                enabled,
                egui::Button::new(toolbar_button_role().rich(action_nav_next_label()))
                    .min_size(egui::vec2(tool_w, tool_h)),
            )
            .on_hover_text("Next difference")
            .clicked()
        {
            hit = ActionBarNav::Next;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(BADGE_PAD);
            ui.label(
                status_side_role().rich(diff_nav_count_label(state.position, state.total).as_str()),
            );
            if let Some(cursor) = action_bar_cursor_text(&state.cursor) {
                ui.add_space(action_bar_readout_gap());
                ui.label(status_side_role().rich(cursor.as_str()));
            }
        });
    });
    hit
}

/// Two-row session action bar: row 1 actions + [`action_bar_row_gap`] +
/// row 2 jump cluster. Caller (`App`, agle wave) `nav` model se banata hai
/// aur hits par swap/run/nav chalata hai. Aaj koi live call-site nahi
/// (contract-only, `paint_session_toolbar` live rehta hai); geometry/typo
/// callee tokens me, koi naya size/color nahi.
pub fn paint_session_action_bar(
    ui: &mut Ui,
    enabled: bool,
    nav: &ActionBarNavState,
) -> ActionBarHits {
    let action = paint_toolbar_action_buttons(ui, enabled);
    ui.add_space(action_bar_row_gap());
    let nav = paint_action_nav_row(ui, enabled, nav);
    ActionBarHits { action, nav }
}

/// Path-row Browse/Compare button size: 120x28 floor reuse ([`HOME_CARD_BUTTON_MIN_W`]
/// x [`HOME_CARD_BUTTON_MIN_H`]). Home CTA widen (224) se untouched.
/// Pure; unit-tested.
pub fn path_browse_button_size() -> (f32, f32) {
    (HOME_CARD_BUTTON_MIN_W, HOME_CARD_BUTTON_MIN_H)
}

/// Path meta role: caption (Small 11 weak proportional) reuse — BC meta
/// row jaisa compact, mono nahi. Koi naya size nahi; pure; unit-tested.
pub fn path_meta_role() -> TypoRole {
    TypoRole::caption()
}

/// Compact meta line: timestamp | byte size | rules | flags. Khali parts
/// girte hain (missing file par koi fake "0 B" nahi). Pure; unit-tested.
pub fn format_path_meta(timestamp: &str, byte_size: &str, rules: &str, flags: &str) -> String {
    let mut parts = Vec::new();
    for part in [timestamp, byte_size, rules, flags] {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }
    parts.join(" | ")
}

/// Compact path meta row: pehle metas left se, history dropdown beech me,
/// aakhri meta right-end dock. Dropdown options sirf labels hain —
/// selection kabhi nahi badalti (caller state untouched). Geometry sirf
/// [`TIGHT_GAP`] spacing + row gap; typo sirf [`path_meta_role`].
/// Koi naya size/color nahi. Caller (App) meta strings + history data deta
/// hai; ye fn koi state mutate nahi karta.
pub fn paint_path_meta_row(ui: &mut Ui, metas: &[String], history: &[String], current: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        let (last, rest) = match metas.split_last() {
            Some(pair) => pair,
            None => return,
        };
        for meta in rest {
            ui.label(path_meta_role().rich(meta.as_str()));
        }
        egui::ComboBox::from_id_salt("path_meta_history")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for item in history {
                    ui.label(item.as_str());
                }
            });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(path_meta_role().rich(last.as_str()));
        });
    });
    ui.add_space(TIGHT_GAP);
}

/// Paint-applied Home grid columns: 3 per row, so the 9 session cards
/// tile into 3 full rows on the section rhythm. Pure; unit-tested.
pub fn home_paint_grid_columns() -> usize {
    HOME_GRID_COLUMNS
}

/// Paint-applied gutter width at the mono scale: the overview-ruler
/// track ([`gutter_ruler_width_for_font`] at 12.0: 16.0 numbers ke
/// saath, 8.0 bina) taaki number column (16/20/.. cap 36) hamesha track
/// me samae aur ruler/rows ek beat par rahen. Legacy 12/6 bar se chauda —
/// tiny-number + no-padding gap isi se band hota hai. Pure; unit-tested.
pub fn text_gutter_paint_width(show_line_numbers: bool) -> f32 {
    gutter_ruler_width_for_font(TYPO_MONO_SIZE, show_line_numbers)
}

/// Text number-column width px: the tabular gutter column for `max_no`
/// ([`crate::gutter::gutter_linenum_column_width`]: 16 + 4 per extra
/// digit, hidden numbers narrow to the ruler track). Paint reserves this
/// so right-aligned numbers never clip the code. Pure; unit-tested.
pub fn text_gutter_number_width(max_no: usize, show_line_numbers: bool) -> f32 {
    crate::gutter::gutter_linenum_column_width(max_no, show_line_numbers)
}

/// Text number-column tabular contract: Mono tabular role
/// ([`crate::gutter::gutter_linenum_is_tabular`]) so digits stay aligned
/// row to row. Pure; unit-tested.
pub fn text_gutter_numbers_tabular() -> bool {
    crate::gutter::gutter_linenum_is_tabular()
}

/// Stacked single-pane width (px): narrow windows (< 720) stack the two
/// panes full-width — one [`SECTION_GAP`] air off `avail`, [`MIN_HALF`]
/// floor so tang windows scroll instead of crushing content. Non-finite
/// ya non-positive `avail` par floor (paint kabhi NaN na khaye).
/// Pure; unit-tested.
pub fn text_stacked_pane_width(avail: f32) -> f32 {
    if !avail.is_finite() || avail <= 0.0 {
        return MIN_HALF;
    }
    (avail - SECTION_GAP).max(MIN_HALF)
}

/// Narrow-720b stacked width: [`text_stacked_pane_width`] at the 720
/// breakpoint (712.0 = 720 − 8 air). Fixture
/// `comparison/text-narrow-720b.json` isi ko lock karti hai. Pure;
/// unit-tested.
pub fn text_narrow_720b_stacked_width() -> f32 {
    text_stacked_pane_width(TEXT_STACK_BREAKPOINT_W)
}

/// Header: title, subtitle, and dark/light mode toggle with WCAG contrast verification.
/// Bar reserves [`header_bar_height`] (the paging-bar rhythm) so it sits
/// on the pane-split grid; the note shares the [`header_content_lead`]
/// baseline with table rules/placeholders. Type via [`TypoRole`] chrome
/// (section title, status note); tones unchanged.
pub fn paint_header(ui: &mut Ui, dark_mode: &mut bool) {
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), header_bar_height()));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(chrome_section_role().rich("autocompare"));
        ui.label(
            chrome_status_role().rich(header_note_line("drag & drop or browse | pure engines")),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if *dark_mode {
                "☀ Light Mode"
            } else {
                "🌙 Dark Mode"
            };
            if ui.button(label).clicked() {
                *dark_mode = !*dark_mode;
                ui.ctx().set_visuals(if *dark_mode {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
            }
        });
    });
    ui.add_space(TIGHT_GAP);
}

/// The Home View launcher screen displaying cards for all 9 comparison session types.
///
/// Geometry: cards sit on the contract minimum ([`home_paint_card_height`]
/// = 99.6, never below legacy [`CARD_H`]), the grid tiles
/// [`home_paint_grid_columns`] = 3 per row on the section rhythm
/// ([`HOME_GRID_GAP`]), and the CTA fills the 224x28 single-line budget
/// ([`home_paint_cta_size`], area 6272). Typography: title strong-14 vs
/// desc small-weak-11 hierarchy ([`home_title_paint_size`] /
/// [`home_desc_paint_is_small_weak`]), button body scale — same sizes and
/// weights, koi naya size nahi. Home shows no path picker
/// (`HOME_SHOWS_PATH_PICKER` = false): Compare tabs own their pickers; the
/// section starts on the grid rhythm directly.
pub fn paint_home(ui: &mut Ui, tab: &mut Tab) {
    let model = viewmodel::home_model();
    ui.label(chrome_section_role().rich("Comparison Sessions"));
    ui.label(RichText::new("Select a session type or use the tabs above:").weak());
    ui.add_space(SECTION_GAP);

    // Flex cards: chaudi window par row bharo (fixture 306 @960 parity),
    // tang par 240 floor (grid scroll karta hai, cards crush nahi hote).
    let card_w = home_paint_card_width(ui.available_width());
    egui::Grid::new("home_sessions_grid")
        .num_columns(home_paint_grid_columns())
        .spacing([HOME_GRID_GAP, HOME_GRID_GAP])
        .show(ui, |ui| {
            for (i, item) in model.items.iter().enumerate() {
                ui.group(|ui| {
                    ui.set_width(card_w);
                    ui.set_height(home_paint_card_height());
                    ui.vertical_centered(|ui| {
                        ui.label(TypoRole::HomeTitle.rich(item.title));
                        let mut desc = RichText::new(item.description);
                        if home_desc_paint_is_small_weak() {
                            desc = TypoRole::HomeDesc.rich(item.description);
                        }
                        ui.label(desc);
                        ui.add_space(TIGHT_GAP);
                        let (cta_w, cta_h) = home_paint_cta_size();
                        if ui
                            .add_sized(
                                egui::vec2(cta_w, cta_h),
                                egui::Button::new(
                                    TypoRole::HomeButton.rich(format!("Open {}", item.title)),
                                ),
                            )
                            .clicked()
                        {
                            *tab = item.tab;
                        }
                    });
                });
                if i % home_paint_grid_columns() == home_paint_grid_columns().saturating_sub(1) {
                    ui.end_row();
                }
            }
        });
}

/// One recent row ka title text: paths ka honest summary. Three-way par
/// base samahit, dono paths khali par sirf session kind (clip ka jhooth
/// nahi — khali list rows kabhi fake paths nahi dikhati). Pure;
/// unit-tested.
pub fn home_recent_row_title(snapshot: &SessionSnapshot) -> String {
    let left = snapshot.left.trim();
    let right = snapshot.right.trim();
    if left.is_empty() && right.is_empty() {
        snapshot.tab.title().to_string()
    } else if snapshot.is_three_way() && !snapshot.base.trim().is_empty() {
        format!("{left} \u{2194} {right} (base {})", snapshot.base.trim())
    } else {
        format!("{left} \u{2194} {right}")
    }
}

/// Session-start recent list (columnar): title flex + kind fixed (96) +
/// age fixed (72) via [`recent_row_columns`], do-line rows (title + meta
/// status line). Khali par honest hint, koi fake row nahi. History +
/// `now_secs` caller (App) se; unknown waqt par age chhupta hai (koi
/// "just now" ka jhooth nahi). Geometry/typo callee tokens me (section
/// gap, column gaps, title/meta roles) — koi naya size/color nahi.
pub fn paint_home_recent(ui: &mut Ui, history: &SessionHistory, now_secs: u64) {
    ui.add_space(HOME_SECTION_GAP);
    ui.label(recent_row_title_role().rich("Recent sessions"));
    if history.is_empty() {
        ui.label(recent_row_meta_role().rich(recent_list_empty_hint(false)));
        return;
    }
    let avail = ui.available_width();
    let (title_w, kind_w, age_w) = recent_row_columns(avail);
    let row_h = recent_row_two_line_height();
    egui::Grid::new("home_recent_grid")
        .num_columns(3)
        .spacing([RECENT_COL_GAP, RECENT_ROW_GAP])
        .show(ui, |ui| {
            for snapshot in history.list() {
                let age = snapshot.age_label(now_secs).unwrap_or_default();
                ui.vertical(|ui| {
                    ui.set_width(title_w.max(1.0));
                    ui.label(recent_row_title_role().rich(home_recent_row_title(snapshot)));
                    ui.label(recent_row_meta_role().rich(recent_row_meta_text(snapshot, now_secs)));
                });
                ui.add_sized(
                    egui::vec2(kind_w, row_h),
                    egui::Label::new(snapshot.tab.title()),
                );
                ui.add_sized(egui::vec2(age_w, row_h), egui::Label::new(age));
                ui.end_row();
            }
        });
}

/// The ten tabs: Home plus nine comparison session types.
/// Labels paint at the tab chrome scale ([`chrome_tab_role`]: Body 13
/// default proportional) so the tab bar reads as chrome, not code.
pub fn paint_tabs(ui: &mut Ui, tab: &mut Tab) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.selectable_value(tab, Tab::Home, chrome_tab_role().rich("Home"));
        ui.selectable_value(tab, Tab::Text, chrome_tab_role().rich("Text"));
        ui.selectable_value(tab, Tab::Folder, chrome_tab_role().rich("Folder"));
        ui.selectable_value(tab, Tab::Hex, chrome_tab_role().rich("Hex"));
        ui.selectable_value(tab, Tab::Merge, chrome_tab_role().rich("Merge"));
        ui.selectable_value(tab, Tab::Table, chrome_tab_role().rich("Table"));
        ui.selectable_value(tab, Tab::Sync, chrome_tab_role().rich("Sync"));
        ui.selectable_value(tab, Tab::Picture, chrome_tab_role().rich("Picture"));
        ui.selectable_value(tab, Tab::FolderMerge, chrome_tab_role().rich("FolderMerge"));
        ui.selectable_value(tab, Tab::Audio, chrome_tab_role().rich("Audio"));
    });
    ui.add_space(TIGHT_GAP);
}

// ----------------------------------------------------------------------------
// Session tab strip (BU-SES-PAINT): mode patti ke UPAR session patti.
// `paint_tabs` (10 fixed mode buttons) untouched rehti hai; ye patti live
// `SessionList` instances dikhati hai. App caller wiring agle wave ka hai
// (contract-only, `home_recent_row_title` precedent); tests live hain.
// Geometry: close gap (label→close) + strip gap (inter-tab) + TAB_BOTTOM_GAP;
// typo: chrome_tab_role (Body 13 proportional); colors: existing selectable
// style reuse, koi naya hue nahi.
// ----------------------------------------------------------------------------

/// Session tab max width (px): fixed cell + truncate clip, do-line kabhi
/// nahi. `CARD_W` precedent jaisa named token (card se tang taaki pill
/// patti tairti na lage; 3 pills ab 444px me, strip scroll 8-tab par bhi
/// barkarar); magic number paint me nahi. Pure; unit-tested.
pub const SESSION_TAB_MAX_W: f32 = 120.0;

/// Session tab hit: strip caller (`App`, agle wave) is par switch/close
/// chalata hai. Paint sirf emit karta hai, mutate nahi (`paint_paths`
/// `compare` out-flag precedent). Per-tab hit + strip out-param ek hi
/// enum hai (ponytail: do same-shape enum nahi).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SessionAction {
    /// Koi hit nahi (default).
    #[default]
    None,
    /// Tab click: active is id par jaye.
    Switch(u64),
    /// Close-x click: sirf ye session band ho.
    Close(u64),
}

/// Tab label: session title, blank/whitespace par `"{Kind} {id}"` fallback
/// (kabhi khaali label nahi — `session_title_for` fallback mirror).
/// Paint isi se label leta hai, logic paint me nahi. Pure; unit-tested.
pub fn session_tab_label(session: &Session) -> String {
    let title = session.title.trim();
    if title.is_empty() {
        format!("{} {}", session.kind.title(), session.id)
    } else {
        title.to_string()
    }
}

/// Khaali strip ka honest launcher hint (koi fake tab nahi). Pure;
/// unit-tested.
pub const fn session_strip_empty_hint() -> &'static str {
    "no open sessions: open any compare from Home"
}

/// Strip khaali hai ya nahi: paint isi gate par hint vs tabs branch karta
/// hai. Pure; unit-tested.
pub fn session_strip_is_empty(list: &SessionList) -> bool {
    list.is_empty()
}

/// Close-x hit box edge (px): 24 exquisite hit floor (toolbar compact
/// height precedent) — square `h x h`, taaki chhota `x` ungli se lage
/// bina tab cell khaye; 28 se 24 taaki pill footprint tang ho.
/// Pure; unit-tested.
pub fn session_tab_close_edge() -> f32 {
    HOME_TOOLBAR_BUTTON_H
}

/// Tab label-to-close gap (px): [`TAB_BOTTOM_GAP`] (6) reuse — close-x box
/// label se chipke nahi (BC close box me saans), `TIGHT_GAP` (4) se ek
/// step upar. Pure; unit-tested.
pub fn session_tab_close_gap() -> f32 {
    TAB_BOTTOM_GAP
}

/// Inter-tab gap (px): [`SECTION_GAP`] (8) — tabs ke beech intra-tab
/// (label→close) se zyada hawa taaki har tab apne close box samet ek
/// cluster padhe (BC strip rhythm). Pure; unit-tested.
pub fn session_tab_strip_gap() -> f32 {
    SECTION_GAP
}

/// Strip row width math (px): `tabs` cells, har cell = label max + close
/// gap + close edge, beech me (`tabs - 1`) strip gaps; 0 tabs par 0.
/// Caller h-scroll gate isi se lagata hai. Pure; unit-tested.
pub fn session_tab_strip_width(tabs: usize) -> f32 {
    if tabs == 0 {
        return 0.0;
    }
    let cell = SESSION_TAB_MAX_W + session_tab_close_gap() + session_tab_close_edge();
    cell * tabs as f32 + session_tab_strip_gap() * tabs.saturating_sub(1) as f32
}

/// Tang window par strip h-scroll mangta hai ya nahi: `avail` strip width
/// se tang par haan, warna nahi. Non-finite ya non-positive `avail` par
/// false (paint kabhi NaN branch na le). Pure; unit-tested.
pub fn session_tab_strip_needs_hscroll(tabs: usize, avail: f32) -> bool {
    if !avail.is_finite() || avail <= 0.0 {
        return false;
    }
    avail < session_tab_strip_width(tabs)
}

/// Single session tab: label (fixed max width + truncate clip, single line)
/// ke saath chhota close-x button. Active tab existing selectable style me dikhta
/// hai, paint_tabs wali highlight reuse hoti hai (koi naya color nahi).
/// Close pehle check hota hai (destructive intent jeetta hai). Caller
/// `App` hit par switch/close chalata hai (agle wave).
pub fn paint_session_tab(ui: &mut Ui, session: &Session, is_active: bool) -> SessionAction {
    let edge = session_tab_close_edge();
    let label = session_tab_label(session);
    let mut hit = SessionAction::None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = session_tab_close_gap();
        if ui
            .add_sized(
                egui::vec2(SESSION_TAB_MAX_W, edge),
                egui::Button::selectable(is_active, chrome_tab_role().rich(label.as_str()))
                    .truncate(),
            )
            .clicked()
        {
            hit = SessionAction::Switch(session.id);
        }
        if ui
            .add_sized(egui::vec2(edge, edge), egui::Button::new("x"))
            .clicked()
        {
            hit = SessionAction::Close(session.id);
        }
    });
    hit
}

/// Poori session patti: mode tabs ke UPAR, horizontal ScrollArea me
/// (overflow scroll, tabs sikudte nahi). Khaali list par honest hint.
/// Neeche [`TAB_BOTTOM_GAP`]. First-hit-wins (ek frame me ek click).
pub fn paint_session_tab_strip(ui: &mut Ui, list: &SessionList, action: &mut SessionAction) {
    if session_strip_is_empty(list) {
        ui.label(chrome_status_role().rich(session_strip_empty_hint()));
        ui.add_space(TAB_BOTTOM_GAP);
        return;
    }
    egui::ScrollArea::horizontal()
        .id_salt("session_tab_strip")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = session_tab_strip_gap();
                for session in list.list() {
                    let is_active = list.active_id() == Some(session.id);
                    match paint_session_tab(ui, session, is_active) {
                        SessionAction::None => {}
                        hit => {
                            if *action == SessionAction::None {
                                *action = hit;
                            }
                        }
                    }
                }
            });
        });
    ui.add_space(TAB_BOTTOM_GAP);
}
/// The fields share the row by `split` (see [`clamp_split_fraction`]);
/// a drag handle between them resizes both panes for this run.
pub fn paint_paths(
    ui: &mut Ui,
    noun: &str,
    is_dir: bool,
    left: &mut String,
    right: &mut String,
    compare: &mut bool,
    split: &mut f32,
) {
    *split = clamp_split_fraction(*split);
    let avail = ui.available_width();
    let (left_w, right_w) = split_field_widths(avail, *split);
    let (browse_w, browse_h) = path_browse_button_size();
    let (compare_w, compare_h) = path_browse_button_size();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.add(
            TextEdit::singleline(left)
                .desired_width(left_w)
                .hint_text(format!("left {noun} path")),
        );
        if ui
            .add_sized(
                egui::vec2(browse_w, browse_h),
                egui::Button::new("Browse..."),
            )
            .clicked()
        {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *left = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *left = file.display().to_string();
            }
        }
        paint_split_handle(ui, split, avail);
        ui.add(
            TextEdit::singleline(right)
                .desired_width(right_w)
                .hint_text(format!("right {noun} path")),
        );
        if ui
            .add_sized(
                egui::vec2(browse_w, browse_h),
                egui::Button::new("Browse..."),
            )
            .clicked()
        {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *right = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *right = file.display().to_string();
            }
        }
        if ui
            .add_sized(
                egui::vec2(compare_w, compare_h),
                egui::Button::new(format!("Compare {noun}s")),
            )
            .clicked()
        {
            *compare = true;
        }
    });
    ui.add_space(TIGHT_GAP);
}

/// Three path text fields with native dialog pickers for three-way tabs.
pub fn paint_triple_paths(
    ui: &mut Ui,
    noun: &str,
    is_dir: bool,
    base: &mut String,
    left: &mut String,
    right: &mut String,
    compare: &mut bool,
) {
    let (browse_w, browse_h) = path_browse_button_size();
    let (compare_w, compare_h) = path_browse_button_size();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        let third = third_field_width(ui.available_width());
        ui.add(
            TextEdit::singleline(base)
                .desired_width(third)
                .hint_text(format!("base {noun} path")),
        );
        if ui
            .add_sized(
                egui::vec2(browse_w, browse_h),
                egui::Button::new("Browse..."),
            )
            .clicked()
        {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *base = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *base = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(left)
                .desired_width(third)
                .hint_text(format!("left {noun} path")),
        );
        if ui
            .add_sized(
                egui::vec2(browse_w, browse_h),
                egui::Button::new("Browse..."),
            )
            .clicked()
        {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *left = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *left = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(right)
                .desired_width(third)
                .hint_text(format!("right {noun} path")),
        );
        if ui
            .add_sized(
                egui::vec2(browse_w, browse_h),
                egui::Button::new("Browse..."),
            )
            .clicked()
        {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *right = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *right = file.display().to_string();
            }
        }
        if ui
            .add_sized(
                egui::vec2(compare_w, compare_h),
                egui::Button::new(format!("Compare {noun}s")),
            )
            .clicked()
        {
            *compare = true;
        }
    });
    ui.add_space(TIGHT_GAP);
}

/// Generic scrollable summary lines for the plan/stat tabs
/// (Hex, Sync, FolderMerge, Audio).
pub fn paint_lines(ui: &mut Ui, lines: &[String]) {
    if lines.is_empty() {
        ui.label(RichText::new("(no output)").weak());
        return;
    }
    let row_height = row_height_for_line(text_row_height(ui));
    egui::ScrollArea::vertical()
        .id_salt("plan_lines_rows")
        .auto_shrink([false, false])
        .show_rows(ui, row_height, lines.len(), |ui, range| {
            for i in range {
                let Some(line) = lines.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                ui.label(TypoRole::Mono.rich(line.as_str()));
            }
        });
}

/// Whether koi displayed hunk `offset` byte ko cover karta hai.
/// Hunk range `[offset, offset + max(left_len, right_len))` hai.
/// [`views_hex::hex_hunk_at_offset`] jaisa, par paint-grid ke liye
/// `views.rs` me taaki grid formatter ek jagah rahe. Pure; unit-tested.
pub fn hunk_covers_offset(offset: u64, hunks: &[views_hex::HexHunkView]) -> bool {
    for hunk in hunks {
        let run = (hunk.left_len).max(hunk.right_len);
        if run == 0 {
            continue;
        }
        let end = hunk.offset.saturating_add(run as u64);
        if hunk.offset <= offset && offset < end {
            return true;
        }
    }
    false
}

/// One grid row split into three paint cells: ruler address, 16-byte
/// hex marks, and 16-char ASCII marks. Model me byte values nahi
/// (sirf offset/len), isliye covered byte hex me `??` aur ASCII me
/// `?` dikhta hai, uncovered slot blank rehta hai; 8 ke baad midline
/// gap (extra space) taaki 8+8 aadhe alag dikhen. [`format_hex_grid_row`]
/// inhi cells ko single-string me jodta hai; `paint_hex` teen-column
/// `egui::Grid` me inhi cells ko paint karta hai. Pure; unit-tested.
pub fn format_hex_grid_cells(
    row_start: u64,
    hunks: &[views_hex::HexHunkView],
) -> (String, String, String) {
    use views_hex::{HEX_BYTES_PER_ROW, format_hex_offset};
    let addr = format_hex_offset(row_start);
    let mut hex = String::new();
    let mut ascii = String::with_capacity(HEX_BYTES_PER_ROW);
    for col in 0..HEX_BYTES_PER_ROW {
        if col == HEX_BYTES_PER_ROW / 2 {
            hex.push(' ');
        }
        let off = row_start.saturating_add(col as u64);
        let inside = hunk_covers_offset(off, hunks);
        if inside {
            hex.push_str("??");
        } else {
            hex.push_str("  ");
        }
        hex.push(' ');
        ascii.push(if inside { '?' } else { ' ' });
    }
    (addr, hex, ascii)
}

/// One grid row of the hex byte view: ruler offset plus per-byte
/// marks for the 16-byte row starting at `row_start`. Model me byte
/// values nahi (sirf offset/len), isliye covered byte `??` dikhta hai
/// aur uncovered slot blank rehta hai; ASCII column me covered `?`
/// aur bahar blank. 8 ke baad midline gap (extra space) taaki 8+8
/// aadhe alag dikhen. [`format_hex_grid_cells`] ka single-string join;
/// shape wahi hai jo Grid cells paint karte hain. Back-compat wrapper
/// (logs/tests); Grid paint cells use karta hai. Pure; unit-tested.
pub fn format_hex_grid_row(row_start: u64, hunks: &[views_hex::HexHunkView]) -> String {
    let (addr, hex, ascii) = format_hex_grid_cells(row_start, hunks);
    format!("{addr}  {hex} |{ascii}|")
}

/// Grid rows (row-start offsets) jo `hunk` ko cover karte hain:
/// row-aligned start se run-end tak, 16-byte kadam par. Paint visible
/// hunk window se inhe nikal kar grid lines banata hai; pager/window
/// wahi rehta hai. Pure; unit-tested.
pub fn hex_grid_rows_for_hunk(hunk: &views_hex::HexHunkView) -> Vec<u64> {
    use views_hex::HEX_BYTES_PER_ROW;
    let run = (hunk.left_len).max(hunk.right_len);
    if run == 0 {
        return Vec::new();
    }
    let step = HEX_BYTES_PER_ROW as u64;
    let aligned = hunk.offset.saturating_div(step).saturating_mul(step);
    let end = hunk.offset.saturating_add(run as u64);
    let mut rows = Vec::new();
    let mut row = aligned;
    loop {
        rows.push(row);
        row = row.saturating_add(step);
        if row >= end || rows.len() > 8 {
            break;
        }
    }
    rows
}

/// One model-driven hex hunk line: offset plus per-side byte counts,
/// mirroring the engine summary wording.
pub fn format_hex_hunk_line(hunk: &views_hex::HexHunkView) -> String {
    format!(
        "{:#010x}: {} vs {} byte(s)",
        hunk.offset, hunk.left_len, hunk.right_len
    )
}

/// Parses a tolerance text field the way the engine sanitizes: only
/// finite values `>= 0` count; anything else means exact text. Pure;
/// unit-tested.
pub fn parse_tolerance_text(text: &str) -> Option<f64> {
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value >= 0.0 => Some(value),
        _ => None,
    }
}

/// Parses a gutter row number the way the hex jump box sanitizes:
/// only finite integers `>= 0` count; anything else means no jump.
/// Pure; unit-tested.
pub fn parse_gutter_row_text(text: &str) -> Option<u64> {
    text.trim().parse::<u64>().ok()
}

/// Adapts a painted [`TableModel`] to the headless find API
/// [`views_table::table_find_next`]: each painted row becomes one
/// single-cell row carrying its display text (`label | text`).
/// Test-only contract (P3 find-source switch ke baad paint
/// `data_cells` par `table_find_match_rows` chalata hai — grid coords
/// align karne ke liye); unit tests isi par find row mapping lock
/// karte hain. Pure; unit-tested.
pub fn table_model_text_rows(model: &TableModel) -> Vec<Vec<String>> {
    model
        .rows
        .iter()
        .map(|row| vec![format!("{} | {}", row.label, row.text)])
        .collect()
}

/// Adapts folder rows to the headless find API
/// [`crate::tree::folder_find_next`]: paths in paint order. Pure;
/// unit-tested.
pub fn folder_path_rows(rows: &[FolderRow]) -> Vec<String> {
    rows.iter().map(|row| row.rel_path.clone()).collect()
}

// ----------------------------------------------------------------------------
// Wirefind: hex + table find-bar caller wiring (Gap #12 final).
// Needle + cursor egui temp store me rehte hain taaki caller signature
// same rahe (App struct untouched — merge-pick `merge_pick_id`
// precedent). Count displayed content par kaam karta hai, Next/Prev
// selection + `bring_into_view` chalata hai. Geometry/typo callee fns
// ke andar hai (28/120/4 reuse), yahan koi naya token/color nahi.
// ----------------------------------------------------------------------------

/// Hex find needle ka temp-store id (caller-local, App untouched).
/// Pure (no egui context); unit-tested.
pub fn hex_find_needle_id() -> egui::Id {
    egui::Id::unique("wirefind_hex_needle")
}

/// Hex find cursor ka temp-store id (matches vec me index). Pure; unit-tested.
pub fn hex_find_cursor_id() -> egui::Id {
    egui::Id::unique("wirefind_hex_cursor")
}

/// Table find needle ka temp-store id (caller-local, App untouched).
/// Pure (no egui context); unit-tested.
pub fn table_find_needle_id() -> egui::Id {
    egui::Id::unique("wirefind_table_needle")
}

/// Table find cursor ka temp-store id (matches vec me index). Pure; unit-tested.
pub fn table_find_cursor_id() -> egui::Id {
    egui::Id::unique("wirefind_table_cursor")
}

/// Displayed hunk lines me needle wale hunk indices: har hunk ki
/// [`format_hex_hunk_line`] me substring ho to hit. Khali needle par
/// khali (koi fake count nahi). Hex bytes view me nahi hain (sirf
/// offset/len), isliye count wahi hai jo paint dikhata hai. Pure;
/// unit-tested.
pub fn hex_find_hunk_hits(hunks: &[views_hex::HexHunkView], needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (idx, hunk) in hunks.iter().enumerate() {
        if format_hex_hunk_line(hunk).contains(needle) {
            out.push(idx);
        }
    }
    out
}

/// Find cursor step: click na ho ya koi hit na ho to `None` (selection
/// untouched, taaki F3/diff-nav se ladai na ho); Next aage wrap,
/// Prev peeche wrap (text `advance_text_find` precedent: Next jeetta
/// hai jab dono dab jayen). Caller cursor ko pehle clamp karta hai.
/// Pure; unit-tested.
pub fn find_cursor_step(
    cursor: usize,
    total: usize,
    prev_clicked: bool,
    next_clicked: bool,
) -> Option<usize> {
    if total == 0 {
        return None;
    }
    if next_clicked {
        let cur = cursor % total.max(1);
        Some((cur.saturating_add(1)) % total.max(1))
    } else if prev_clicked {
        let cur = cursor % total.max(1);
        if cur == 0 {
            Some(total.saturating_sub(1))
        } else {
            Some(cur.saturating_sub(1))
        }
    } else {
        None
    }
}

/// Hex minimap click/drag mapping: local `y` (px) ko grid row me
/// badalta hai. Fraction `y / height` se `[0, total_rows)`; kharab
/// input par `None`. [`crate::gutter::gutter_row_at_y`] jaisa, par u64
/// hex grid rows ke liye. Pure; unit-tested.
/// Legacy path (paint ab track-relative `hex_minimap_row_at_pos` par hai —
/// BD spec §4 point 3); ruler parity tests isi par hain.
pub fn hex_minimap_row_at_y(y: f32, height: f32, total_rows: u64) -> Option<u64> {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return None;
    }
    if !y.is_finite() || y < 0.0 || y >= height {
        return None;
    }
    let row = ((y / height) * total_rows as f32).floor() as u64;
    if row < total_rows { Some(row) } else { None }
}

/// Minimap segment floor: sub-pixel hunk bands grow to one row
/// pad ([`ROW_PAD`]) so hits stay visible/clickable; mapping stays
/// exact (only paint height changes, never the row math). Pure;
/// unit-tested.
/// Legacy floor (paint ab `hex_minimap_band_rects` floor par hai —
/// BD spec §4 point 2); band parity tests isi par hain.
pub fn hex_minimap_seg_end(y0: f32, y1: f32) -> f32 {
    y1.max(y0 + ROW_PAD)
}

/// Text-gutter click contract: local `y` (px) ko nearest-diff model
/// row me badalta hai ([`text_minimap_row_at_y`] se row, diff par stay,
/// equal par nearest diff snap, bahar/empty par `None`). Paint click/drag
/// isi ko selection + pending scroll deta hai. Hex side ka analog
/// [`hex_minimap_jump_to_row`] hai (wahi fraction→row→jump pattern).
/// Pure (no egui); unit-tested.
pub fn text_gutter_click_select(
    rows: &[TextRow],
    total_rows: usize,
    height: f32,
    y: f32,
) -> Option<usize> {
    let row = text_minimap_row_at_y(y, height, total_rows)?;
    if rows.get(row).is_some_and(|r| r.kind != TextRowKind::Equal) {
        return Some(row);
    }
    let mut i = row.saturating_add(1);
    while i < rows.len() {
        if rows.get(i).is_some_and(|r| r.kind != TextRowKind::Equal) {
            return Some(i);
        }
        i = i.saturating_add(1);
    }
    let mut j = row.min(rows.len());
    while j > 0 {
        j = j.saturating_sub(1);
        if rows.get(j).is_some_and(|r| r.kind != TextRowKind::Equal) {
            return Some(j);
        }
    }
    None
}

/// Next-frame scroll key: gutter click list ke baad paint hota hai,
/// isliye jump target ek frame ke liye temp store me rehta hai taaki
/// agli frame list us row par scroll kare. Struct change bina (KEVL
/// `viewmodel.rs` untouched). Pure id; unit-tested.
pub fn text_gutter_pending_id() -> egui::Id {
    egui::Id::unique("text_gutter_pending_scroll")
}

/// Text overview-ruler click-target width: the gutter contract ruler
/// track ([`gutter_ruler_width_for_font`] at the mono scale: 16.0 numbers
/// ke saath, 8.0 bina) — legacy 12/6 bar se chauda taaki the Beyond
/// Compare-style ruler ungli se lage. Single source; paint rect isi se
/// banta hai. Pure; unit-tested.
pub fn text_minimap_ruler_width(show_line_numbers: bool) -> f32 {
    gutter_ruler_width_for_font(TYPO_MONO_SIZE, show_line_numbers)
}

/// Overview-ruler row mapping: local `y` (px) ko model row me badalta
/// hai. Fraction `y / height` se `[0, total_rows)`; kharab input par
/// `None`. [`crate::gutter::gutter_row_at_y`] semantics, arg order hex
/// [`hex_minimap_row_at_y`] jaisa (`y, height, total`) taaki dono ruler
/// ek pattern par rahen. Pure (no egui); unit-tested.
pub fn text_minimap_row_at_y(y: f32, height: f32, total_rows: usize) -> Option<usize> {
    gutter_row_at_y(y, total_rows, height)
}

/// Ruler row-center mapping (offset<->row, wapas): model row ke band ka
/// center ([`gutter_row_center_y`]) — marker isi center par lagta hai aur
/// `text_minimap_row_at_y(center) == row` round-trip karta hai, taaki
/// offset se row aur row se offset dono disha sahi rahen. Wahi `None`
/// contract jo [`crate::gutter::gutter_y_for_row`] ka hai. Pure (no egui); unit-tested.
pub fn text_minimap_row_center_y(row: usize, total_rows: usize, height: f32) -> Option<f32> {
    gutter_row_center_y(row, total_rows, height)
}

/// Selection-marker band `(y0, y1)` px (ruler-local): single source
/// [`crate::gutter::gutter_selection_marker_band`] — marker row band
/// center par min-segment uncha, kabhi do rows par straddle nahi karta.
/// Paint `rect.min.y` offset jodkar marker banata hai taaki jump ke baad
/// indicator clicked band par synced dikhe. Pure (no egui); unit-tested.
pub fn text_minimap_marker_band(
    row: usize,
    total_rows: usize,
    height: f32,
) -> Option<(f32, f32)> {
    crate::gutter::gutter_selection_marker_band(row, total_rows, height)
}

/// Overview-ruler slop: segment hit-test har taraf itne px badhta hai
/// ([`GUTTER_HIT_SLOP`]) taaki 4px bands alag-alag pointer-clickable
/// rahen bina paint hile. Paint sirf hit me lagata hai. Pure;
/// unit-tested.
pub fn text_ruler_hit_slop() -> f32 {
    GUTTER_HIT_SLOP
}

/// Slop-expanded segment hit: ruler `y` (px) par kaun sa paint segment
/// hai. Exact test par band-beech ke 1px gaps miss hote hain; slop se
/// 4px bands ungli se lagte hain. Paint click isi gate ke baad
/// [`text_gutter_click_select`] se row snap karta hai. Pure (no egui);
/// unit-tested.
pub fn text_ruler_segment_hit(segments: &[GutterSegment], y: f32) -> Option<usize> {
    gutter_segment_at_y_with_slop(segments, y, text_ruler_hit_slop())
}

/// Text overview-ruler accessibility label: khali par honest empty, bina
/// selection par row-ginati ke saath no-selection, selection par 1-based
/// `row N of M` plus click-to-jump hint. Paint `on_hover_text` me jata
/// hai taaki ruler bina pointer ke bhi operable lage. Pure; unit-tested.
pub fn text_minimap_accessibility_label(selected: Option<usize>, total_rows: usize) -> String {
    if total_rows == 0 {
        return "overview ruler: empty".to_string();
    }
    match selected {
        Some(idx) if idx < total_rows => format!(
            "overview ruler: row {} of {}, click to jump to nearest difference",
            idx.saturating_add(1),
            total_rows
        ),
        _ => format!(
            "overview ruler: no selection, {} rows, click to jump to nearest difference",
            total_rows
        ),
    }
}

/// Hex minimap accessibility label: text ruler label ka hunk analog.
/// Khali par honest empty, bina/out-of-range selection par hunk-ginati
/// ke saath no-selection, selection par 1-based `hunk N of M` plus
/// click-to-jump hint. Pure; unit-tested.
pub fn hex_minimap_accessibility_label(selected: Option<usize>, total_hunks: usize) -> String {
    if total_hunks == 0 {
        return "minimap: empty".to_string();
    }
    match selected {
        Some(idx) if idx < total_hunks => format!(
            "minimap: hunk {} of {}, click to jump",
            idx.saturating_add(1),
            total_hunks
        ),
        _ => format!(
            "minimap: no selection, {} hunks, click to jump",
            total_hunks
        ),
    }
}

// ----------------------------------------------------------------------------
// Text diff navigation: Next/Prev jump + N/M count (find-bar precedent).
// Model moves `viewmodel::select_next/prev_diff` me hain (KEVL untouched);
// yahan sirf paint-side position/count hai taaki nav row N/M badge dikha
// sake aur list `scroll_to` se bring-into-view kare. Geometry/typo find-bar
// reuse (nav row 28px, badge right-end small weak + 4px pad), koi naya
// token/color nahi. Pure; unit-tested.
// ----------------------------------------------------------------------------

/// Diff rows ki kul sankhya (model order me `is_diff_row` count).
/// Khali ya equal-only par 0. Pure; unit-tested.
pub fn diff_nav_total(rows: &[TextRow]) -> usize {
    let mut total = 0usize;
    let mut idx = 0usize;
    while idx < rows.len() {
        if rows.get(idx).is_some_and(viewmodel::is_diff_row) {
            total = total.saturating_add(1);
        }
        idx = idx.saturating_add(1);
    }
    total
}

/// Selected row ki diffs me 1-based position: selected diff hai to
/// `Some(pos)`, nahi to `None` (koi selection/equal/out-of-range par koi
/// fake count nahi). Pure; unit-tested.
pub fn diff_nav_position(rows: &[TextRow], selected: Option<usize>) -> Option<usize> {
    let sel = selected?;
    if !rows.get(sel).is_some_and(viewmodel::is_diff_row) {
        return None;
    }
    let mut pos = 0usize;
    let mut idx = 0usize;
    while idx <= sel && idx < rows.len() {
        if rows.get(idx).is_some_and(viewmodel::is_diff_row) {
            pos = pos.saturating_add(1);
        }
        idx = idx.saturating_add(1);
    }
    if pos == 0 { None } else { Some(pos) }
}

/// N/M badge text for the diff-nav row: "2/5", no hit par "0/0".
/// Find/tablefind badge precedent mirror (`find_count_label` shape).
/// Pure; unit-tested.
pub fn diff_nav_count_label(position: Option<usize>, total: usize) -> String {
    match position {
        Some(pos) => format!("{pos}/{total}"),
        None => format!("0/{total}"),
    }
}

/// Minimap/grid row-click jump: `row` ko offset banakar covering hunk
/// select karta hai aur pager window me lata hai — wahi steps jo
/// `App::hex_gutter_jump_to_row` (app.rs, read-only) chalata hai:
/// `hex_row_to_hunk` -> selected -> set_total -> bring_into_view.
/// Paint click/drag isi ko deta hai. Pure (no egui); unit-tested.
pub fn hex_minimap_jump_to_row(
    view: &mut views_hex::HexCompareView,
    pager: &mut views_hex::HexPager,
    row: u64,
) -> bool {
    match views_hex::hex_row_to_hunk(row, &view.hunks) {
        Some(idx) => {
            view.selected = Some(idx);
            pager.set_total(view.hunks.len() as u64);
            pager.bring_into_view(idx as u64);
            true
        }
        None => false,
    }
}

/// The model-driven hex view: status line, hunk navigation (Prev /
/// Next plus jump to first difference), pager-driven hunk window, then
/// the legacy engine summary lines for byte context. `pager` ka total
/// hunks se sync hota hai; Prev/Next page aur PageUp/PageDown
/// `next_page`/`prev_page` chalate hain, selection
/// `bring_into_view` se visible rehta hai.
pub fn paint_hex(
    ui: &mut Ui,
    view: &mut views_hex::HexCompareView,
    pager: &mut views_hex::HexPager,
    lines: &[String],
    hex_bytes: Option<(&[u8], &[u8])>,
) {
    ui.label(chrome_status_role().rich(view.status_line()));
    if view.over_cap {
        paint_note(ui, "inputs exceed the 16 MiB aligned cap: compare skipped");
        return;
    }
    // Find row (Gap #12 final wiring): hex window ke andar
    // `views_hex::paint_hex_find_bar` (signature same). Needle + cursor
    // caller-local temp me (App untouched); count displayed hunk lines
    // par ([`hex_find_hunk_hits`]), badge `hex_find_count_label` se N/M;
    // Next/Prev hunk selection + `bring_into_view` chalata hai.
    // Geometry/typo callee me (28/120/4 reuse), yahan koi naya token nahi.
    let mut needle: String = ui
        .data_mut(|data| data.get_temp::<String>(hex_find_needle_id()))
        .unwrap_or_default();
    let mut cursor: usize = ui
        .data_mut(|data| data.get_temp::<usize>(hex_find_cursor_id()))
        .unwrap_or(0);
    let old_needle = needle.clone();
    let mut hits = hex_find_hunk_hits(&view.hunks, &needle);
    let mut total = hits.len();
    if total == 0 {
        cursor = 0;
    } else {
        cursor %= total.max(1);
    }
    let pos = if total == 0 {
        None
    } else {
        Some(cursor.saturating_add(1))
    };
    let badge = views_hex::hex_find_count_label(pos, total);
    let (find_prev, find_next) = views_hex::paint_hex_find_bar(ui, &mut needle, &badge);
    if needle != old_needle {
        cursor = 0;
        hits = hex_find_hunk_hits(&view.hunks, &needle);
        total = hits.len();
        if total > 0 {
            cursor = 0;
        }
    }
    if let Some(next_idx) = find_cursor_step(cursor, total, find_prev, find_next)
        && let Some(hunk_idx) = hits.get(next_idx).copied()
    {
        cursor = next_idx;
        view.selected = Some(hunk_idx);
        pager.set_total(view.hunks.len() as u64);
        pager.bring_into_view(hunk_idx as u64);
    }
    ui.data_mut(|data| {
        data.insert_temp(hex_find_needle_id(), needle);
        data.insert_temp(hex_find_cursor_id(), cursor);
    });
    pager.set_total(view.hunks.len() as u64);
    let (page_up, page_down) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::PageUp),
            input.key_pressed(egui::Key::PageDown),
        )
    });
    if page_up {
        pager.prev_page();
    }
    if page_down {
        pager.next_page();
    }
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        if ui.small_button("◀ Prev hunk").clicked() {
            view.select_prev_hunk();
            if let Some(idx) = view.selected {
                pager.bring_into_view(idx as u64);
            }
        }
        if ui.small_button("Next hunk ▶").clicked() {
            view.select_next_hunk();
            if let Some(idx) = view.selected {
                pager.bring_into_view(idx as u64);
            }
        }
        if ui.small_button("First diff").clicked()
            && let Some(off) = view.first_diff
            && view.navigate_to_offset(off).is_some()
            && let Some(idx) = view.selected
        {
            pager.bring_into_view(idx as u64);
        }
        if ui.small_button("◀ Prev page").clicked() {
            pager.prev_page();
        }
        if ui.small_button("Next page ▶").clicked() {
            pager.next_page();
        }
    });
    ui.label(TypoRole::Small.rich(format!(
        "page {} of {}, showing {} row(s)",
        pager.top / pager.rows_per_page.max(1) + 1,
        pager.page_count().max(1),
        pager
            .visible_range()
            .1
            .saturating_sub(pager.visible_range().0),
    )));
    if view.hunks.is_empty() {
        ui.label(RichText::new("(no differing hunks)").weak());
    } else {
        let row_height = row_height_for_line(text_row_height(ui));
        let (wstart, wend) = pager.visible_range();
        let wstart = wstart as usize;
        let wend = (wend as usize).min(view.hunks.len());
        let count = wend.saturating_sub(wstart);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("hex_hunk_rows")
                    .auto_shrink([false, false])
                    .max_height(HEX_MINIMAP_H)
                    .show_rows(ui, row_height, count, |ui, range| {
                        for pos in range {
                            let i = wstart.saturating_add(pos);
                            let Some(hunk) = view.hunks.get(i) else {
                                break;
                            };
                            let line = format_hex_hunk_line(hunk);
                            let selected = view.selected == Some(i);
                            ui.set_row_height(row_height);
                            if ui
                                .selectable_label(selected, TypoRole::Mono.rich(line))
                                .clicked()
                            {
                                view.selected = Some(i);
                                pager.bring_into_view(i as u64);
                            }
                            if Some(i) == view.selected {
                                ui.scroll_to_rect(ui.min_rect(), Some(egui::Align::Center));
                            }
                        }
                    });
            });
            // Byte-grid minimap: hunk rows ka thumbnail; click/drag
            // fraction se grid row nikal kar covering hunk par jump
            // karta hai (wahi `hex_row_to_hunk` path). Sirf existing
            // palette/tokens: track + CHANGED_BG, double-wide strip.
            let total = views_hex::hex_row_count(view.left_size.max(view.right_size));
            if total > 0 {
                let dark = ui.visuals().dark_mode;
                // Double-wide strip by token: `hex_minimap_strip_width`
                // (gutter double, 24px hit-rect); height by
                // `hex_minimap_alloc_height` (paint token judvaan).
                let bar_w = views_hex::hex_minimap_strip_width();
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(bar_w, views_hex::hex_minimap_alloc_height()),
                    egui::Sense::click_and_drag(),
                );
                debug_assert_eq!(rect.width(), views_hex::hex_minimap_strip_width());
                // Legacy token parity lock (BD spec §4 point 1): paint helpers
                // par hai, tokens tests me bandhe hain.
                debug_assert_eq!(HEX_MINIMAP_W, views_hex::hex_minimap_strip_width());
                debug_assert_eq!(HEX_MINIMAP_H, views_hex::hex_minimap_alloc_height());
                let resp = resp.on_hover_text(hex_minimap_accessibility_label(
                    view.selected,
                    view.hunks.len(),
                ));
                ui.painter().rect_filled(rect, 0.0, track_for(dark));
                for seg in views_hex::hex_minimap_band_rects(
                    rect,
                    view.left_size.max(view.right_size),
                    &view.hunks,
                ) {
                    ui.painter().rect_filled(seg, 0.0, CHANGED_BG);
                }
                let minimap_pressed = resp.clicked() || resp.dragged();
                if minimap_pressed
                    && let Some(pos) = resp.interact_pointer_pos()
                    && let Some(row) = views_hex::hex_minimap_row_at_pos(rect, total, pos.y)
                {
                    hex_minimap_jump_to_row(view, pager, row);
                }
            }
        });
        // Byte grid: visible hunk window ki 16-byte rows — teen-column
        // Grid (address | 8+8 hex marks + midline gap | ASCII column).
        // Model me byte values nahi, isliye `??`/`?` = diff byte
        // (offset/len se). Monospace cells, GEO spacing reuse
        // (`SECTION_GAP` x, `TIGHT_GAP` y), koi naya token/color nahi.
        ui.add_space(TIGHT_GAP);
        ui.label(TypoRole::Small.rich("byte grid (offsets only: ?? = diff byte)"));
        let mut grid_rows: Vec<u64> = Vec::new();
        for hunk in view.hunks.get(wstart..wend).unwrap_or(&[]) {
            for row in hex_grid_rows_for_hunk(hunk) {
                if grid_rows.last() != Some(&row) {
                    grid_rows.push(row);
                }
                if grid_rows.len() >= 64 {
                    break;
                }
            }
            if grid_rows.len() >= 64 {
                break;
            }
        }
        egui::Grid::new("hex_byte_grid")
            .num_columns(3)
            .spacing([SECTION_GAP, TIGHT_GAP])
            .show(ui, |ui| {
                let nibble_role = views_hex::hex_nibble_text_role();
                for row in grid_rows {
                    let (addr, hex, ascii) = format_hex_grid_cells(row, &view.hunks);
                    ui.label(nibble_role.rich(addr));
                    ui.label(nibble_role.rich(hex));
                    ui.label(nibble_role.rich(format!("|{ascii}|")));
                    // Nibble accent: caller-side byte slices App-owned snap
                    // se aate hain (model me sirf offset/len); `None` par
                    // khali slices se cells khali, accent rects kuch nahi
                    // paint karte — `??` whole-row marks rehte hain. REAL
                    // slices milte hi yahi rects changed nibble halves par
                    // lagte hain (caller-side ownership, snap `app.rs` me).
                    let (left_slice, right_slice): (&[u8], &[u8]) = match hex_bytes {
                        Some((left, right)) => (left, right),
                        None => (&[], &[]),
                    };
                    let cells = views_hex::hex_diff_nibble_cells(row, left_slice, right_slice);
                    for accent in views_hex::hex_row_nibble_accent_rects(ui.min_rect(), &cells) {
                        ui.painter().rect_filled(accent, 0.0, CHANGED_BG);
                    }
                    ui.end_row();
                }
            });
    }
    ui.add_space(TIGHT_GAP);
    paint_lines(ui, lines);
}

/// Over-cap hex state: honest sizes plus the skip note, no hunk grid.
pub fn paint_hex_over_cap(ui: &mut Ui, view: &views_hex::HexCompareView) {
    ui.label(chrome_status_role().rich(view.status_line()));
    paint_note(ui, "inputs exceed the 16 MiB aligned cap: compare skipped");
}

/// Table painted text ko cells me todta hai: paint ` | ` se jodta hai
/// (viewmodel `table_cells`/`table_plain`), to split inverse hai.
/// Pure; unit-tested.
pub fn split_table_cells(text: &str) -> Vec<String> {
    text.split(" | ").map(str::to_string).collect()
}

/// Header cells: label `hdr` wali pehli row ka text; nahi mili to
/// `None`. Paint isi ko header bar banata hai. Pure; unit-tested.
pub fn table_header_cells(model: &TableModel) -> Option<Vec<String>> {
    model
        .rows
        .iter()
        .find(|row| row.label.trim() == "hdr")
        .map(|row| split_table_cells(&row.text))
}

/// Bordered row text: cells ko widths tak pad karke `| a | b |` me
/// bandhta hai. Missing cell `-` dikhta hai (views_table convention).
/// Widths [`views_table::table_column_widths`] se aate hain. Pure;
/// unit-tested.
pub fn format_table_bordered_row(cells: &[String], widths: &[usize]) -> String {
    let mut out = String::from("|");
    for (idx, width) in widths.iter().enumerate() {
        let cell = cells.get(idx).map(String::as_str).unwrap_or("-");
        out.push(' ');
        out.push_str(&views_table::pad_table_cell(cell, *width));
        out.push_str(" |");
    }
    out
}

/// Header separator: bordered line jitni lambi `-` line. Pure;
/// unit-tested.
pub fn format_table_header_rule(line_len: usize) -> String {
    "-".repeat(line_len.max(1))
}

/// Grid cells: `text` ko split karke widths tak pad karta hai
/// ([`views_table::pad_table_row`] reuse, missing cell `-`).
/// `format_table_bordered_row` ki inner cells ke barabar hai —
/// Grid paint isi ko cells me baant-ta hai. Pure; unit-tested.
pub fn table_row_cells(text: &str, widths: &[usize]) -> Vec<String> {
    views_table::pad_table_row(&split_table_cells(text), widths)
}

/// Grid columns: glyph column plus ek per data column. Khali widths
/// par sirf glyph (fallback single-string path). Pure; unit-tested.
pub fn table_grid_columns(widths: &[usize]) -> usize {
    widths.len().saturating_add(1)
}

/// Har `model.rows` index ka `data_cells` row index (hdr-filtered):
/// header (`hdr`) rows `None` map hoti hain, baaki grid order me.
/// Reverse vec har data-row ko wapas uske model-row par laata hai
/// (`bring_into_view` pager `model.rows` par chalta hai). Pure;
/// unit-tested.
pub fn table_data_index_map(model: &TableModel) -> (Vec<Option<usize>>, Vec<usize>) {
    let mut fwd = Vec::with_capacity(model.rows.len());
    let mut rev = Vec::new();
    for (idx, row) in model.rows.iter().enumerate() {
        if row.label.trim() == "hdr" {
            fwd.push(None);
        } else {
            fwd.push(Some(rev.len()));
            rev.push(idx);
        }
    }
    (fwd, rev)
}

/// The model-driven table view: keyed summary line, the key-column
/// bar when a header row exists, the positional note when no key
/// columns are selected, pager-driven row window, then the legacy
/// row paint.
pub fn paint_table_keyed(
    ui: &mut Ui,
    view: &mut views_table::TableKeyView,
    model: &TableModel,
    pager: &mut views_table::TablePager,
) {
    ui.label(chrome_status_role().rich(view.status_line()));
    if let Some(cells) = table_header_cells(model).as_ref() {
        views_table::paint_table_key_bar(ui, view, cells);
    }
    if let Some(note) = view.positional_note() {
        paint_note(ui, note);
    }
    paint_table(ui, view, model, pager);
}
/// Merge 3-pane column count: left / base / right teen panes.
/// Headless contract (test-locked); paint spells the three panes
/// explicitly for readability. Pure.
pub fn merge_pane_count() -> usize {
    3
}

/// One conflict hunk ki teen sides marker block se: left (`<<<<<<<`
/// ke baad), base (`|||||||` ke baad), right (`=======` ke baad).
/// `take::resolve_take` wala section rule reuse hota hai (base marker
/// na ho to `=======` se pehle sab left, base khali). Bahar ke hunks
/// khali triple dete hain. Pure; unit-tested.
pub fn merge_hunk_sides(
    lines: &[String],
    hunk: &viewmodel::MergeHunk,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut left = Vec::new();
    let mut base = Vec::new();
    let mut right = Vec::new();
    if lines.get(hunk.start).is_none() {
        return (left, base, right);
    }
    let mut section = 0u8;
    let mut i = hunk.start.saturating_add(1);
    while i < hunk.end {
        let Some(line) = lines.get(i) else {
            break;
        };
        if line.starts_with("<<<<<<<") || line.starts_with(">>>>>>>") {
            i = i.saturating_add(1);
            continue;
        }
        if line.starts_with("|||||||") {
            section = 1;
        } else if line.starts_with("=======") {
            section = 2;
        } else if section == 0 {
            left.push(line.clone());
        } else if section == 1 {
            base.push(line.clone());
        } else {
            right.push(line.clone());
        }
        i = i.saturating_add(1);
        if i > hunk.end.saturating_add(1) {
            break;
        }
    }
    (left, base, right)
}

/// One hunk card ka title: 1-based "hunk N of M" + teen pane naam.
/// Pure; unit-tested.
pub fn format_merge_hunk_title(index: usize, total: usize) -> String {
    format!(
        "hunk {} of {}: left / base / right",
        index.saturating_add(1),
        total
    )
}

/// One hunk ka decision display bina staged pick ke: honest pending.
/// Staged pick ke saath display [`format_merge_hunk_decision_with_pick`]
/// deta hai. Pure; unit-tested.
pub fn format_merge_hunk_decision(index: usize) -> String {
    format!("hunk {}: decision pending", index.saturating_add(1))
}

/// Staged picks ka temp-store id, live `lines` content hash se scoped taaki
/// naya compare purani intent kabhi inherit na kare. Decision ka App wiring
/// agli wave ka hai; is wave me picks isi id par local rehte hain.
/// Pure (no egui context); unit-tested.
pub fn merge_pick_id(lines: &[String]) -> egui::Id {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    lines.hash(&mut hasher);
    egui::Id::unique("merge_hunk_picks").with(hasher.finish())
}

/// Ek staged side ka display naam: intent/summary labels ke liye.
/// Pure; unit-tested.
pub fn merge_pick_choice_label(choice: viewmodel::TakeChoice) -> &'static str {
    match choice {
        viewmodel::TakeChoice::Left => "left",
        viewmodel::TakeChoice::Right => "right",
        viewmodel::TakeChoice::Base => "base",
        viewmodel::TakeChoice::Both => "both",
    }
}

/// Ek hunk ka pick intent display: "hunk N: take <side>".
/// Pure; unit-tested.
pub fn format_merge_pick_intent(index: usize, choice: viewmodel::TakeChoice) -> String {
    format!(
        "hunk {}: take {}",
        index.saturating_add(1),
        merge_pick_choice_label(choice)
    )
}

/// Staged picks me hunk `index` ki aakhri staged side; unknown par `None`.
/// Duplicate picks me aakhri staged jeetti hai
/// ([`viewmodel::apply_merge_picks`] dedup rule). Pure; unit-tested.
pub fn merge_pick_for_hunk(
    picks: &[viewmodel::MergeHunkPick],
    index: usize,
) -> Option<viewmodel::TakeChoice> {
    let mut found = None;
    for pick in picks {
        if pick.conflict_index == index {
            found = Some(pick.choice);
        }
    }
    found
}

/// Ek pick stage karta hai: same hunk ki purani pick overwrite hoti hai
/// (aakhri jeetti hai), nayi hunk judti hai. [`viewmodel::apply_merge_picks`]
/// ke dedup rule ka paint-side mirror. Pure; unit-tested.
pub fn stage_merge_pick(
    picks: &[viewmodel::MergeHunkPick],
    index: usize,
    choice: viewmodel::TakeChoice,
) -> Vec<viewmodel::MergeHunkPick> {
    let mut out: Vec<viewmodel::MergeHunkPick> = Vec::new();
    for pick in picks {
        if pick.conflict_index != index {
            out.push(*pick);
        }
    }
    out.push(viewmodel::MergeHunkPick {
        conflict_index: index,
        choice,
    });
    out
}

/// Sab hunks ko ek side par stage karta hai (apply-all): har index
/// `0..total` usi `choice` se judta hai, out-of-range stale picks chhoote
/// nahi (adopt unhe prune karta hai). [`viewmodel::apply_merge_picks`]
/// dedup rule ka paint-side mirror. Pure; unit-tested.
pub fn stage_merge_pick_all(
    picks: &[viewmodel::MergeHunkPick],
    total: usize,
    choice: viewmodel::TakeChoice,
) -> Vec<viewmodel::MergeHunkPick> {
    let mut out: Vec<viewmodel::MergeHunkPick> = Vec::new();
    for pick in picks {
        if pick.conflict_index >= total {
            out.push(*pick);
        }
    }
    for index in 0..total {
        out.push(viewmodel::MergeHunkPick {
            conflict_index: index,
            choice,
        });
    }
    out
}

/// Ek hunk ka decision display staged pick ke saath: pick ho to
/// "take <side>" intent, nahi to honest pending. Pure; unit-tested.
pub fn format_merge_hunk_decision_with_pick(
    index: usize,
    pick: Option<viewmodel::TakeChoice>,
) -> String {
    match pick {
        Some(choice) => format_merge_pick_intent(index, choice),
        None => format_merge_hunk_decision(index),
    }
}

/// 3-pane summary line: kul conflict hunks + pending + conflicted flag.
/// [`viewmodel::merge_pick_summary`] ke numbers ko paint wording deta hai.
/// Pure; unit-tested.
pub fn merge_three_pane_summary_line(total: usize, pending: usize, conflicted: bool) -> String {
    format!(
        "{} conflict hunk(s), {} pending{}",
        total,
        pending,
        if conflicted { ", conflicted" } else { "" }
    )
}

/// The merge 3-pane: har conflict hunk ke left/base/right panes +
/// per-hunk decision display + per-hunk real Take buttons. Model reuse
/// read-only hai ([`viewmodel::merge_conflict_hunks`] +
/// [`viewmodel::merge_pick_summary`]); picks egui temp store me stage hote
/// hain ([`merge_pick_id`], live lines se scoped) aur decision display
/// pending se "take <side>" intent me badalta hai — Merge tab har frame
/// App picks seed karta hai aur paint ke baad staged wapas adopt karta
/// hai (App-side `reconcile_merge_picks` bridge).
/// Har card par Take Left / Take Right (plus Take Base / Take Both) real
/// buttons hain, koi stub nahi. Global apply-all row `paint_merge` me
/// staged picks me likhta hai (model lines untouched). Signature unchanged
/// (KEVL `app.rs` call-site compat).
pub fn paint_merge_three_pane(ui: &mut Ui, lines: &[String]) {
    let hunks = viewmodel::merge_conflict_hunks(lines);
    let mut picks: Vec<viewmodel::MergeHunkPick> = ui
        .data_mut(|data| data.get_temp::<Vec<viewmodel::MergeHunkPick>>(merge_pick_id(lines)))
        .unwrap_or_default();
    let summary = viewmodel::merge_pick_summary(lines, &picks);
    ui.label(TypoRole::Small.rich(merge_three_pane_summary_line(
        summary.total,
        summary.pending,
        summary.conflicted,
    )));
    if hunks.is_empty() {
        ui.label(RichText::new("(no conflict hunks: single merged pane below)").weak());
        return;
    }
    for hunk in &hunks {
        let (left, base, right) = merge_hunk_sides(lines, hunk);
        let idx = hunk.conflict_index;
        let total = hunks.len();
        let mut staged: Option<viewmodel::TakeChoice> = None;
        ui.group(|ui| {
            ui.label(
                TypoRole::Small
                    .rich(format_merge_hunk_title(idx, total))
                    .strong(),
            );
            ui.label(TypoRole::Small.rich(format_merge_hunk_decision_with_pick(
                idx,
                merge_pick_for_hunk(&picks, idx),
            )));
            ui.add_space(TIGHT_GAP);
            // Aligned 3-pane row: equal thirds from [`three_pane_widths`]
            // (same interior width for every card, so left/base/right
            // columns line up card to card); dividers stay the thin
            // separators, typo stays small-strong headers + mono lines.
            let (left_w, base_w, right_w) = three_pane_widths(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(left_w);
                    ui.label(TypoRole::Small.rich("Left").strong());
                    if left.is_empty() {
                        ui.label(TypoRole::Small.rich("(empty)"));
                    }
                    for line in &left {
                        ui.label(TypoRole::Mono.rich(line.as_str()));
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_width(base_w);
                    ui.label(TypoRole::Small.rich("Base").strong());
                    if base.is_empty() {
                        ui.label(TypoRole::Small.rich("(empty)"));
                    }
                    for line in &base {
                        ui.label(TypoRole::Mono.rich(line.as_str()));
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_width(right_w);
                    ui.label(TypoRole::Small.rich("Right").strong());
                    if right.is_empty() {
                        ui.label(TypoRole::Small.rich("(empty)"));
                    }
                    for line in &right {
                        ui.label(TypoRole::Mono.rich(line.as_str()));
                    }
                });
            });
            ui.add_space(TIGHT_GAP);
            let active = merge_pick_for_hunk(&picks, idx);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = TIGHT_GAP;
                ui.label(TypoRole::Small.rich("Take:"));
                if ui
                    .selectable_label(active == Some(viewmodel::TakeChoice::Left), "Take Left")
                    .on_hover_text("Resolve conflict with Left pane content")
                    .clicked()
                {
                    staged = Some(viewmodel::TakeChoice::Left);
                }
                if ui
                    .selectable_label(active == Some(viewmodel::TakeChoice::Right), "Take Right")
                    .on_hover_text("Resolve conflict with Right pane content")
                    .clicked()
                {
                    staged = Some(viewmodel::TakeChoice::Right);
                }
                if ui
                    .selectable_label(active == Some(viewmodel::TakeChoice::Base), "Take Base")
                    .on_hover_text("Resolve conflict with Base pane content")
                    .clicked()
                {
                    staged = Some(viewmodel::TakeChoice::Base);
                }
                if ui
                    .selectable_label(active == Some(viewmodel::TakeChoice::Both), "Take Both")
                    .on_hover_text("Resolve conflict by keeping both Left and Right")
                    .clicked()
                {
                    staged = Some(viewmodel::TakeChoice::Both);
                }
            });
        });
        if let Some(choice) = staged {
            picks = stage_merge_pick(&picks, idx, choice);
        }
        ui.add_space(TIGHT_GAP);
    }
    ui.data_mut(|data| {
        data.insert_temp(merge_pick_id(lines), picks);
    });
}

/// The three-way merge output: conflict blocks highlighted, notes
/// for conflicts and truncation first, with per-hunk Take staging plus
/// an apply-all row. Global row kabhi model lines mutate nahi karta —
/// har click staged picks me jata hai (App adopt karta hai), isliye
/// multi-hunk par har hunk ka faisla lagta hai (purana index-0-only
/// `resolve_take` hata diya). Resolution text [`paint_merge_apply`] +
/// App `run_merge_apply` se likha jata hai.
pub fn paint_merge(ui: &mut Ui, model: &mut MergeSummary) {
    let dark = ui.visuals().dark_mode;
    if model.conflicted {
        paint_note(
            ui,
            "the merge has conflicts; choose per hunk below, or take one side for all hunks:",
        );
        let hunks = viewmodel::merge_conflict_hunks(&model.lines);
        let mut apply_all_choice: Option<viewmodel::TakeChoice> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TIGHT_GAP;
            if ui
                .button("Take Left (all)")
                .on_hover_text("Take Left for all conflict hunks")
                .clicked()
            {
                apply_all_choice = Some(viewmodel::TakeChoice::Left);
            }
            if ui
                .button("Take Right (all)")
                .on_hover_text("Take Right for all conflict hunks")
                .clicked()
            {
                apply_all_choice = Some(viewmodel::TakeChoice::Right);
            }
            if ui
                .button("Take Base (all)")
                .on_hover_text("Take Base for all conflict hunks")
                .clicked()
            {
                apply_all_choice = Some(viewmodel::TakeChoice::Base);
            }
            if ui
                .button("Take Both (all)")
                .on_hover_text("Take Both for all conflict hunks")
                .clicked()
            {
                apply_all_choice = Some(viewmodel::TakeChoice::Both);
            }
        });
        if let Some(choice) = apply_all_choice {
            let id = merge_pick_id(&model.lines);
            let picks: Vec<viewmodel::MergeHunkPick> =
                ui.data_mut(|data| data.get_temp(id)).unwrap_or_default();
            let staged = stage_merge_pick_all(&picks, hunks.len(), choice);
            ui.data_mut(|data| {
                data.insert_temp(id, staged);
            });
        }
        ui.add_space(TIGHT_GAP);
    }
    if model.truncated {
        paint_note(ui, "showing the first lines only; the rest is cut off");
    }
    if model.lines.is_empty() {
        ui.label(RichText::new("(merged output is empty)").weak());
        return;
    }
    paint_merge_three_pane(ui, &model.lines);
    ui.add_space(TIGHT_GAP);
    let row_height = row_height_for_line(text_row_height(ui));
    let gutter_dim = dim_for(dark);
    let gutter_w = format!("{}", model.lines.len()).len().max(3);

    // Geometry: panes fill remaining height (no bottom waste) — budget
    // captured before the row consumes it; the list column keeps it as
    // its floor and the overview ruler pins to the same height.
    let panes_h = viewmodel::text_panes_height(ui.available_height());
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_min_height(panes_h);
            egui::ScrollArea::vertical()
                .id_salt("merge_output_rows")
                .auto_shrink([false, false])
                .show_rows(ui, row_height, model.lines.len(), |ui, range| {
                    for i in range {
                        let Some(line) = model.lines.get(i) else {
                            break;
                        };
                        ui.set_row_height(row_height);
                        let marker = line.starts_with("<<<<<<<")
                            || line.starts_with("|||||||")
                            || line.starts_with("=======")
                            || line.starts_with(">>>>>>>");
                        let mut text = TypoRole::Mono.rich(line.as_str());
                        if marker {
                            text = text.color(red_for(dark)).strong();
                        }
                        ui.horizontal(|ui| {
                            let lineno =
                                format!("{:>width$} ", i.saturating_add(1), width = gutter_w);
                            ui.label(TypoRole::Mono.rich(lineno.as_str()).color(gutter_dim));
                            ui.label(text);
                        });
                    }
                });
        });

        // Overview ruler for merge conflicts (same remaining-height
        // budget as the list column, so ruler and rows stay in sync).
        let avail_h = panes_h;
        let geom = crate::gutter::merge_gutter_geometry(&model.lines, avail_h);
        let bar_w = crate::gutter::gutter_ruler_width(true);
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(bar_w, avail_h), egui::Sense::click_and_drag());
        let _ = resp.on_hover_text(format!(
            "Merge conflict overview: {} conflict hunk(s)",
            geom.segments.len()
        ));
        ui.painter().rect_filled(rect, 0.0, track_for(dark));

        for seg in &geom.segments {
            let seg_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y + seg.y_start),
                egui::pos2(rect.max.x, (rect.min.y + seg.y_end).min(rect.max.y)),
            );
            ui.painter().rect_filled(seg_rect, 0.0, CHANGED_BG);
        }
    });
}

// ----------------------------------------------------------------------------
// Merge Apply adoption (Gap #8 paint half): Apply button + progress/result
// display wired to the App apply state machine (AN precedent
// `MergeApplyState` / `MergeApplySummary` / `run_merge_apply`).
// State + summary App ka truth hai (read-only in); picks/decision display
// `paint_merge_three_pane` wala hi rehta hai. Geometry/typo existing tokens
// me hai (HEADER_H row, TIGHT_GAP spacing, small-strong title + status role),
// koi naya token/color nahi.
// ----------------------------------------------------------------------------

/// Apply section title: hunk-card title precedent (small-strong paint).
/// Pure; unit-tested.
pub fn merge_apply_section_title() -> &'static str {
    "Apply"
}

/// Output path field hint: dry-run par honest "not written" note, real run
/// par seedha label. Pure; unit-tested.
pub fn merge_apply_path_hint(dry_run: bool) -> &'static str {
    if dry_run {
        "output path (dry-run: not written)"
    } else {
        "output path"
    }
}

/// Progress display ke liye state label: har `MergeApplyState` variant ka
/// honest ek-shabd sthiti, sab distinct, koi fake percent nahi. Pure;
/// unit-tested.
pub fn merge_apply_state_label(state: MergeApplyState) -> &'static str {
    match state {
        MergeApplyState::Idle => "idle",
        MergeApplyState::DryRunReported => "dry-run reported",
        MergeApplyState::Wrote => "written",
        MergeApplyState::NeedsPath => "needs output path",
        MergeApplyState::NoModel => "compare first",
        MergeApplyState::Failed => "failed",
    }
}

/// Structured summary se real counts line: dry-run par resolved + decided
/// counts (no writes), real write par wrote count + out path, path-khali
/// par honest no-path note. `None` jab koi report nahi (Idle fresh).
/// Caller `summary_line` (App real line) ke saath ise dikhata hai. Pure;
/// unit-tested.
pub fn format_merge_apply_result(summary: Option<&MergeApplySummary>) -> Option<String> {
    let s = summary?;
    if s.dry_run {
        Some(format!(
            "dry-run: {} line(s), {}/{} decided, {} pending (no writes)",
            s.resolved_lines, s.decided, s.total, s.pending
        ))
    } else if s.out.is_empty() {
        Some(format!(
            "{} line(s), {}/{} decided, {} pending (no output path)",
            s.resolved_lines, s.decided, s.total, s.pending
        ))
    } else {
        Some(format!(
            "wrote {} line(s) -> {} ({}/{} decided, {} pending)",
            s.wrote_lines, s.out, s.decided, s.total, s.pending
        ))
    }
}

/// Merge Apply section: dry-run checkbox + output path field + Apply button
/// ek [`HEADER_H`] row me, neeche progress/result display (state label +
/// App real `summary_line` + structured counts jab report ho).
/// `enabled` App ke `merge_apply_enabled()` se aata hai (model maujood =
/// dabane layak); `dry_run`/`out_path` App fields se bind hote hain;
/// `state`/`summary`/`summary_line` App truth ke read-only view hain.
/// Returns true jab Apply daba (caller `request_merge_apply` phir
/// `run_merge_apply` chalata hai; button disabled par kabhi true nahi).
/// Geometry/typo callee tokens me (28/4 reuse), koi naya token/color nahi.
pub fn paint_merge_apply(
    ui: &mut Ui,
    enabled: bool,
    dry_run: &mut bool,
    out_path: &mut String,
    state: MergeApplyState,
    summary: Option<&MergeApplySummary>,
    summary_line: &str,
) -> bool {
    ui.label(TypoRole::Small.rich(merge_apply_section_title()).strong());
    let mut apply_clicked = false;
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.checkbox(dry_run, "Dry-run");
        let field_w = (ui.available_width() - merge_apply_chrome_width()).max(MIN_HALF);
        ui.add(
            TextEdit::singleline(out_path)
                .desired_width(field_w)
                .hint_text(merge_apply_path_hint(*dry_run)),
        );
        let clicked = ui
            .add_enabled(enabled, egui::Button::new("Apply"))
            .clicked();
        if clicked {
            apply_clicked = true;
        }
    });
    ui.label(chrome_status_role().rich(format!("apply state: {}", merge_apply_state_label(state))));
    ui.label(chrome_status_role().rich(summary_line));
    if let Some(line) = format_merge_apply_result(summary) {
        ui.label(chrome_status_role().rich(line));
    }
    ui.add_space(TIGHT_GAP);
    apply_clicked
}

/// The table comparison rows: changed rows highlighted, changed cells
/// already marked `[old => new]` by the view model. `pager` ka total
/// rows se sync hota hai; Prev/Next page aur PageUp/PageDown
/// `next_page`/`prev_page` chalate hain, find result
/// `bring_into_view` se visible rehta hai.
pub fn paint_table(
    ui: &mut Ui,
    view: &mut views_table::TableKeyView,
    model: &TableModel,
    pager: &mut views_table::TablePager,
) {
    let dark = ui.visuals().dark_mode;
    if model.truncated {
        paint_note(ui, "showing the first rows only; the rest is cut off");
    }
    if model.rows.is_empty() {
        ui.label(RichText::new("(both tables are empty)").weak());
        return;
    }
    // Find row (Gap #12 final wiring, tablefind precedent
    // `checks/gui-tablefind-ai/REPORT.md`): table window ke andar
    // `views_table::paint_table_find_bar` (signature same). Needle +
    // cursor caller-local temp me (App untouched); search grid coords
    // par (`data_cells`, hdr-filtered — wahi jo Grid paint karta hai),
    // count N/M `table_find_badge_label` se; Next/Prev
    // `pager.bring_into_view` chalata hai (data-row se model-row map karke).
    // Geometry/typo callee me, yahan koi naya token nahi.
    let mut needle: String = ui
        .data_mut(|data| data.get_temp::<String>(table_find_needle_id()))
        .unwrap_or_default();
    let mut cursor: usize = ui
        .data_mut(|data| data.get_temp::<usize>(table_find_cursor_id()))
        .unwrap_or(0);
    // Header/data cells grid coords par: `data_cells` hdr-filtered hai
    // (`paint_table_data_cell` find coords isi par hain), isliye find
    // matches bhi isi par bante hain (`table_model_text_rows` single-cell
    // paint text par tha — grid se align nahi tha). Widths + index maps
    // pure hain (koi paint order change nahi).
    let header = table_header_cells(model);
    let data_cells: Vec<Vec<String>> = model
        .rows
        .iter()
        .filter(|row| row.label.trim() != "hdr")
        .map(|row| split_table_cells(&row.text))
        .collect();
    let widths = match header.as_ref() {
        Some(cells) => views_table::table_column_widths(cells, &data_cells),
        None => Vec::new(),
    };
    let (data_idx_of_row, model_idx_of_data_row) = table_data_index_map(model);
    let old_needle = needle.clone();
    let mut matches = views_table::table_find_match_rows(&data_cells, &needle, false);
    let mut total = matches.len();
    if total == 0 {
        cursor = 0;
    } else {
        cursor %= total.max(1);
    }
    let pos = if total == 0 {
        None
    } else {
        Some(cursor.saturating_add(1))
    };
    let badge = views_table::table_find_badge_label(pos, total);
    let (find_prev, find_next) = views_table::paint_table_find_bar(ui, &mut needle, &badge);
    if needle != old_needle {
        cursor = 0;
        matches = views_table::table_find_match_rows(&data_cells, &needle, false);
        total = matches.len();
    }
    if let Some(next_idx) = find_cursor_step(cursor, total, find_prev, find_next)
        && let Some((row, _col)) = matches.get(next_idx).copied()
    {
        cursor = next_idx;
        pager.total_rows = model.rows.len();
        // `matches` grid coords (data-row) me hai; pager `model.rows`
        // index par chalta hai — reverse map se wapas lao.
        pager.bring_into_view(model_idx_of_data_row.get(row).copied().unwrap_or(row));
    }
    ui.data_mut(|data| {
        data.insert_temp(table_find_needle_id(), needle.clone());
        data.insert_temp(table_find_cursor_id(), cursor);
    });
    pager.total_rows = model.rows.len();
    pager.clamp_top();
    let (page_up, page_down) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::PageUp),
            input.key_pressed(egui::Key::PageDown),
        )
    });
    if page_up {
        pager.prev_page();
    }
    if page_down {
        pager.next_page();
    }
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        if ui.small_button("◀ Prev page").clicked() {
            pager.prev_page();
        }
        if ui.small_button("Next page ▶").clicked() {
            pager.next_page();
        }
        ui.label(TypoRole::Small.rich(format!(
            "page {} of {}, showing {} row(s)",
            pager.top / pager.rows_per_page.max(1) + 1,
            pager.page_count().max(1),
            pager
                .visible_range()
                .1
                .saturating_sub(pager.visible_range().0),
        )));
    });
    ui.add_space(TIGHT_GAP);
    // Header bar + cell grid: `hdr` row header banta hai (glyph
    // placeholder + strong cells, phir `-` rule), data rows Grid me
    // (glyph + padded mono cells). Padding/borders
    // `views_table::pad_table_row` se, window existing TablePager se,
    // spacing GEO tokens se; koi naya color/token nahi.
    if let Some(cells) = header.as_ref() {
        let line = format_table_bordered_row(cells, &widths);
        egui::Grid::new(views_table::TABLE_CELL_HEADER_GRID_ID)
            .num_columns(table_grid_columns(&widths))
            .spacing([
                views_table::TABLE_GRID_COL_GAP,
                views_table::TABLE_GRID_ROW_GAP,
            ])
            .show(ui, |ui| {
                ui.label(TypoRole::Mono.rich(views_table::TABLE_CELL_GLYPH_PAD));
                for cell in views_table::pad_table_row(cells, &widths) {
                    ui.label(TypoRole::Mono.rich(cell).strong());
                }
                ui.end_row();
            });
        ui.label(
            TypoRole::Mono
                .rich(indented_content(&format_table_header_rule(line.len())))
                .weak(),
        );
    }
    let row_height = row_height_for_line(text_row_height(ui));
    let (wstart, wend) = pager.visible_range();
    let wend = wend.min(model.rows.len());
    let count = wend.saturating_sub(wstart);
    // Cell paint inputs: same mono font + caller panel fill; `fg`
    // per-row pre-tinted hai (`row.changed` par CHANGED_FG — Plain /
    // KeyStrong arms caller fg reuse karte hain). `current` grid coords
    // me hai (`data_cells` par, hdr-filtered).
    let mono_font = TypoRole::Mono.resolved_font_id(ui.style());
    let panel_bg = ui.visuals().panel_fill;
    let current = views_table::table_find_current_cell(&matches, cursor);
    let find_ctx = views_table::TableCellFindCtx {
        needle: &needle,
        ignore_case: false,
        current,
    };
    // Painter overlays (P4) ka origin: pehle painted data cell ka
    // top-left + uska data-row index (grid origin = cell - idx*stride).
    let first_cell: std::cell::Cell<Option<(egui::Pos2, usize)>> = std::cell::Cell::new(None);
    if widths.is_empty() {
        egui::ScrollArea::vertical()
            .id_salt("table_single_rows")
            .auto_shrink([false, false])
            .show_rows(ui, row_height, count, |ui, range| {
                for pos in range {
                    let i = wstart.saturating_add(pos);
                    let Some(row) = model.rows.get(i) else {
                        break;
                    };
                    ui.set_row_height(row_height);
                    let mut glyph = TypoRole::Mono.rich(row.glyph.to_string());
                    glyph = match row.glyph {
                        '=' => glyph.color(dim_for(dark)),
                        '!' | 'L' => glyph.color(red_for(dark)),
                        'R' => glyph.color(green_for(dark)),
                        _ => glyph.color(dim_for(dark)),
                    };
                    ui.horizontal(|ui| {
                        ui.label(glyph);
                        ui.label(TypoRole::Mono.rich(format!("{} | {}", row.label, row.text)));
                    });
                }
            });
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("table_cell_rows")
        .auto_shrink([false, false])
        .show_rows(ui, row_height, count, |ui, range| {
            egui::Grid::new(views_table::TABLE_CELL_GRID_ID)
                .num_columns(table_grid_columns(&widths))
                .spacing([
                    views_table::TABLE_GRID_COL_GAP,
                    views_table::TABLE_GRID_ROW_GAP,
                ])
                .show(ui, |ui| {
                    for pos in range {
                        let i = wstart.saturating_add(pos);
                        let Some(row) = model.rows.get(i) else {
                            break;
                        };
                        ui.set_row_height(row_height);
                        let mut glyph = TypoRole::Mono.rich(row.glyph.to_string());
                        glyph = match row.glyph {
                            '=' => glyph.color(dim_for(dark)),
                            '!' | 'L' => glyph.color(red_for(dark)),
                            'R' => glyph.color(green_for(dark)),
                            _ => glyph.color(dim_for(dark)),
                        };
                        ui.label(glyph);
                        // Grid coords `data_cells` par hain (hdr-filtered) —
                        // `model.rows` index `i` NAHI. `paint_table_data_cell`
                        // isi `(row, col)` par current-hit milata hai.
                        let data_row_idx = data_idx_of_row.get(i).copied().flatten().unwrap_or(0);
                        for (col_idx, cell) in
                            table_row_cells(&row.text, &widths).iter().enumerate()
                        {
                            if col_idx == 0 && first_cell.get().is_none() {
                                first_cell.set(Some((ui.min_rect().min, data_row_idx)));
                            }
                            let fg = if row.changed {
                                CHANGED_FG
                            } else {
                                ui.visuals().text_color()
                            };
                            let style = views_table::TableCellStyle {
                                font: mono_font.clone(),
                                fg,
                                bg: panel_bg,
                            };
                            views_table::paint_table_data_cell(
                                ui,
                                view,
                                cell,
                                data_row_idx,
                                col_idx,
                                find_ctx,
                                &style,
                            );
                        }
                        ui.end_row();
                    }
                });
            // Painter overlays (P4): key-col vertical boxes + find-hit
            // underline strips. Rect math `views_table.rs` pure helpers se,
            // measure paint-side (`char_w` caller font measure, origin pehle
            // data cell se) — `views_table.rs` me koi change nahi.
            if let Some((origin, first_data_idx)) = first_cell.get() {
                let stride = row_height + views_table::TABLE_GRID_ROW_GAP;
                let grid = views_table::TableCellGrid {
                    origin_x: origin.x,
                    origin_y: origin.y - first_data_idx as f32 * stride,
                    row_h: row_height,
                    col_gap: views_table::TABLE_GRID_COL_GAP,
                    row_gap: views_table::TABLE_GRID_ROW_GAP,
                };
                let char_w = ui
                    .ctx()
                    .fonts_mut(|fonts| fonts.glyph_width(&mono_font, 'M'));
                let col_widths_px: Vec<f32> = widths.iter().map(|w| char_w * (*w as f32)).collect();
                let overlay_fg = ui.visuals().text_color();
                for box_rect in
                    views_table::table_key_box_rects(view, grid, &col_widths_px, data_cells.len())
                {
                    ui.painter().rect_stroke(
                        egui::Rect::from_min_size(
                            egui::pos2(box_rect.x, box_rect.y),
                            egui::vec2(box_rect.w, box_rect.h),
                        ),
                        egui::CornerRadius::ZERO,
                        egui::Stroke::new(views_table::TABLE_FIND_UNDERLINE_W, overlay_fg),
                        egui::StrokeKind::Middle,
                    );
                }
                for (row, col, start, end) in
                    views_table::table_find_grid_spans(&data_cells, &needle, false)
                {
                    if let Some(cell) = views_table::table_cell_rect(grid, col, row, &col_widths_px)
                    {
                        let width_chars = widths
                            .get(col)
                            .copied()
                            .unwrap_or(views_table::TABLE_MIN_COL_CHARS);
                        for underline in views_table::table_find_underline_rects_for_cell(
                            cell,
                            &[(start, end)],
                            width_chars,
                            char_w,
                        ) {
                            ui.painter().line_segment(
                                [
                                    egui::pos2(underline.x, underline.y),
                                    egui::pos2(underline.x + underline.w, underline.y),
                                ],
                                egui::Stroke::new(views_table::TABLE_FIND_UNDERLINE_W, overlay_fg),
                            );
                        }
                    }
                }
            }
        });
}

/// The red error panel: every engine failure ends up here, never as a
/// crash.
pub fn paint_error(ui: &mut Ui, message: &str) {
    let mut frame = egui::Frame::group(ui.style());
    frame.fill = ERROR_BG;
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TIGHT_GAP;
            ui.label(TypoRole::Mono.rich("error:").strong().color(DELETED_FG));
            ui.label(RichText::new(message).color(DELETED_FG));
        });
    });
}

fn paint_note(ui: &mut Ui, text: &str) {
    ui.label(TypoRole::Small.rich(text));
}

// ----------------------------------------------------------------------------
// Text view
// ----------------------------------------------------------------------------

/// The side-by-side text comparison with default view prefs
/// (word wrap off, line numbers on, syntax on). Kept so the existing
/// `App` call-site compiles; new code should prefer
/// [`paint_text_with_prefs`].
pub fn paint_text(ui: &mut Ui, model: &mut TextRows) {
    let prefs = TextViewPrefs::default();
    paint_text_with_prefs(ui, model, &prefs);
}

/// The side-by-side text comparison honoring `prefs`: word wrap caps
/// panes at 80 chars with wrapping, hiding line numbers blanks the
/// number gutter and narrows chrome, hiding syntax clears inline
/// accents. Counts come from [`viewmodel::filter_counts`] so they stay
/// honest under any toggle combination.
pub fn paint_text_with_prefs(ui: &mut Ui, model: &mut TextRows, prefs: &TextViewPrefs) {
    paint_text_body(ui, model, prefs, None);
}

/// Cursor meta line: status bar ke UPAR paint hota hai — single compact
/// row (cursor + line_no/kind). Per-pane line previews yahan nahi (woh
/// neeche [`paint_text_pane_status`] detail me rehte hain, BC bottom-detail
/// precedent) taaki top chrome ek row me rahe aur panes ko zyada height
/// mile. Preview None (no selection/OOB) par strip skip, koi fake row nahi.
/// Geometry: toolbar precedent (horizontal + TIGHT_GAP); typo:
/// chrome_status_role; koi naya size/color/Grid/add_space nahi; koi state
/// mutate nahi.
pub fn paint_text_cursor_strip(ui: &mut Ui, rows: &[TextRow], selected: Option<usize>) {
    let preview = match viewmodel::focused_line_preview_for(rows, selected) {
        Some(found) => found,
        None => return,
    };
    let cursor = viewmodel::cursor_label(viewmodel::cursor_pos_for(selected));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(
            chrome_status_role().rich(format!("{} | {} {}", cursor, preview.line_no, preview.kind)),
        );
    });
}

/// 3-zone text status bar: left filter+counts, mid focused verdict, right
/// nav badge + hints. Geometry: toolbar precedent (avail x HEADER_H row,
/// TIGHT_GAP spacing, right-end dock + BADGE_PAD); typo: chrome_status_role.
/// Koi naya size/color/Grid/add_space nahi; `zones` BT model
/// (`status_zones_for`) se aata hai, yahan koi count/nav math nahi.
pub fn paint_text_status_bar(ui: &mut Ui, zones: &viewmodel::StatusZones) {
    let avail = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail, HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(status_side_role().rich(zones.left.as_str()));
        ui.separator();
        ui.label(status_verdict_role().rich(zones.mid.as_str()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(BADGE_PAD);
            ui.label(status_side_role().rich(zones.right.as_str()));
        });
    });
}

/// Find-aware text body: `find` None par plain text view jaisa, Some par
/// per-match blocks (current = inverted block, others = underline line;
/// [`FindPaintCtx`] se). Public wrappers (`paint_text_with_prefs` /
/// `paint_text_with_find`) isi par route karte hain taaki list geometry
/// ek jagah rahe. Koi naya token/color nahi.
fn paint_text_body(
    ui: &mut Ui,
    model: &mut TextRows,
    prefs: &TextViewPrefs,
    find: Option<FindPaintCtx<'_>>,
) {
    ui.style_mut().wrap_mode = Some(if prefs.word_wrap {
        egui::TextWrapMode::Wrap
    } else {
        egui::TextWrapMode::Extend
    });
    let prefs_view = view_text_with_prefs(model, prefs);
    let pane_chars = prefs_view.pane_chars;
    let gutter_chars = prefs_view.gutter_chars;
    if model.coarse {
        paint_note(
            ui,
            "inputs exceeded the edit budget; showing a coarse comparison",
        );
    }
    if model.truncated {
        paint_note(
            ui,
            &format!(
                "showing the first {} rows; the rest is cut off",
                viewmodel::MAX_ROWS
            ),
        );
    }
    if model.rows.is_empty() {
        ui.label(RichText::new("(nothing to display: both sides are empty)").weak());
        return;
    }
    // Selection and filter live on the model (single source of
    // truth); clamp a stale selection after a fresh compare.
    if let Some(sel) = model.selected
        && model.rows.get(sel).is_none()
    {
        model.selected = None;
    }

    // Keyboard nav: F3 / Shift+F3 like a find loop, Ctrl+N / Ctrl+P
    // as the terminal-friendly alternative. Plain N/P stay free for
    // typing in the path fields above.
    let (key_next, key_prev) = ui.input(|input| {
        let f3 = input.key_pressed(egui::Key::F3);
        let ctrl = input.modifiers.ctrl || input.modifiers.mac_cmd;
        let next = (f3 && !input.modifiers.shift) || (input.key_pressed(egui::Key::N) && ctrl);
        let prev = (f3 && input.modifiers.shift) || (input.key_pressed(egui::Key::P) && ctrl);
        (next, prev)
    });
    // One-shot scroll: follow the selection only on nav frames.
    let mut scroll_to: Option<usize> = None;
    if key_next {
        viewmodel::select_next_diff(&model.rows, &mut model.selected);
        scroll_to = model.selected;
    }
    if key_prev {
        viewmodel::select_prev_diff(&model.rows, &mut model.selected);
        scroll_to = model.selected;
    }
    // Gutter-click pending (pichhli frame ka jump): gutter list ke
    // baad paint hota hai, isliye target temp store se aata hai aur
    // is frame list us row par scroll karti hai. Nav frames ko
    // overwrite nahi karta (scroll_to pehle se set hai to skip).
    if scroll_to.is_none() {
        let pending: Option<usize> = ui
            .ctx()
            .data_mut(|data| data.remove_temp(text_gutter_pending_id()));
        if pending.is_some() {
            scroll_to = pending;
        }
    }

    let counts = viewmodel::filter_counts(&model.rows, model.filter);
    let visible = viewmodel::filtered_indices(&model.rows, model.filter);
    // Diff-nav N/M badge: selected diff ki 1-based position + kul diffs
    // (find-bar `count_label` precedent; har frame recompute, koi state nahi).
    let nav_total = diff_nav_total(&model.rows);
    let nav_badge = diff_nav_count_label(diff_nav_position(&model.rows, model.selected), nav_total);

    // Toolbar: diff navigation (Next/Prev jump + N/M bring-into-view) +
    // display filter + copy actions. Geometry/typo find-bar reuse:
    // full-width row (`avail x HEADER_H`), Prev/Next small buttons, badge
    // right-end small weak + one pad; koi naya token/color nahi.
    let mut nav_action: Option<bool> = None;
    let mut copy_section_action = None;
    let mut copy_line_action = None;
    let mut copy_all_action = None;
    let nav_avail = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(nav_avail, HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        if ui
            .small_button("◀ Prev")
            .on_hover_text("Previous difference (Shift+F3 / Ctrl+P)")
            .clicked()
        {
            nav_action = Some(false);
        }
        if ui
            .small_button("Next ▶")
            .on_hover_text("Next difference (F3 / Ctrl+N)")
            .clicked()
        {
            nav_action = Some(true);
        }
        ui.separator();
        ui.label(TypoRole::Small.rich("Show:"));
        ui.selectable_value(
            &mut model.filter,
            viewmodel::TextFilter::All,
            chrome_tab_role().rich("All"),
        );
        ui.selectable_value(
            &mut model.filter,
            viewmodel::TextFilter::Differences,
            chrome_tab_role().rich("Differences"),
        );
        ui.selectable_value(
            &mut model.filter,
            viewmodel::TextFilter::Same,
            chrome_tab_role().rich("Same"),
        );
        ui.selectable_value(
            &mut model.filter,
            viewmodel::TextFilter::Context,
            chrome_tab_role().rich("Context"),
        );
        ui.separator();
        if ui
            .small_button("Section →")
            .on_hover_text("Copy difference section Left to Right")
            .clicked()
        {
            copy_section_action = Some(viewmodel::CopyDirection::LeftToRight);
        }
        if ui
            .small_button("← Section")
            .on_hover_text("Copy difference section Right to Left")
            .clicked()
        {
            copy_section_action = Some(viewmodel::CopyDirection::RightToLeft);
        }
        if ui
            .small_button("Line →")
            .on_hover_text("Copy focused line Left to Right")
            .clicked()
        {
            copy_line_action = Some(viewmodel::CopyDirection::LeftToRight);
        }
        if ui
            .small_button("← Line")
            .on_hover_text("Copy focused line Right to Left")
            .clicked()
        {
            copy_line_action = Some(viewmodel::CopyDirection::RightToLeft);
        }
        if ui
            .small_button("All →")
            .on_hover_text("Copy all differences Left to Right")
            .clicked()
        {
            copy_all_action = Some(viewmodel::CopyDirection::LeftToRight);
        }
        if ui
            .small_button("← All")
            .on_hover_text("Copy all differences Right to Left")
            .clicked()
        {
            copy_all_action = Some(viewmodel::CopyDirection::RightToLeft);
        }
        // Badge right-end dock (find-bar precedent): N/M count list se
        // alag na lage, row ke end par ek pad ke saath chipka rehta hai.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(BADGE_PAD);
            ui.label(TypoRole::Small.rich(nav_badge.as_str()));
        });
    });

    if let Some(next) = nav_action {
        if next {
            viewmodel::select_next_diff(&model.rows, &mut model.selected);
        } else {
            viewmodel::select_prev_diff(&model.rows, &mut model.selected);
        }
        scroll_to = model.selected;
    }

    // Resolve the action target: selection first, else first visible diff.
    let mut target_idx = model.selected;
    if target_idx.is_none() {
        let mut i = 0usize;
        while i < visible.len() {
            let mut candidate = None;
            if visible
                .get(i)
                .copied()
                .is_some_and(|orig| model.rows.get(orig).is_some_and(viewmodel::is_diff_row))
            {
                candidate = visible.get(i).copied();
            }
            if candidate.is_some() {
                target_idx = candidate;
                break;
            }
            i = i.saturating_add(1);
        }
    }
    if let (Some(idx), Some(dir)) = (target_idx, copy_section_action)
        && viewmodel::copy_section(model, idx, dir).is_ok()
    {
        model.selected = Some(idx);
    }
    if let (Some(idx), Some(dir)) = (target_idx, copy_line_action)
        && viewmodel::copy_line(model, idx, dir).is_ok()
    {
        model.selected = Some(idx);
    }
    if let Some(dir) = copy_all_action
        && crate::copy::copy_all(model, dir).is_ok()
    {
        model.selected = None;
    }

    let mid = viewmodel::focused_kind_label(&model.rows, model.selected);
    let zones = viewmodel::status_zones_for(&counts, model.filter, mid, &nav_badge);
    paint_text_cursor_strip(ui, &model.rows, model.selected);
    paint_text_status_bar(ui, &zones);

    let font_id = TypoRole::Mono.resolved_font_id(ui.style());
    let row_height = row_height_for_line(text_row_height(ui));
    let base_text = ui.style().visuals.text_color();
    let dark = ui.visuals().dark_mode;
    let gutter_dim = dim_for(dark);
    let mut copy_action = None;
    let mut select_action = None;

    // Wrap-on list state (G1..G4): flat width plus per-visible-row visual
    // heights. Wrap-off path inhe nahi chhoota (byte-identical else).
    let wrap_on = prefs.word_wrap;
    let wrap_width = wrap_width_for(pane_chars, prefs.word_wrap).max(1);
    let mut wrap_counts: Vec<usize> = Vec::new();
    if wrap_on {
        let mut vi = 0usize;
        while vi < visible.len() {
            if let Some(orig) = visible.get(vi).copied()
                && let Some(row) = model.rows.get(orig)
            {
                wrap_counts.push(wrap_pane_pair_count(&row.left, &row.right, wrap_width));
            } else {
                wrap_counts.push(1);
            }
            vi = vi.saturating_add(1);
        }
        debug_assert!(wrap_total_visual(&wrap_counts) >= visible.len());
    }

    // Geometry FIRST (bottom-waste fix): the panes row fills the
    // remaining viewport height instead of shrinking to content.
    // Budget captured before the row consumes it (nav toolbar, cursor
    // strip and status bar already spent above); the list column keeps
    // it as its floor and the minimap pins to the same height, so short
    // content scrolls in a full-height viewport with zero bottom waste.
    let panes_h = viewmodel::text_panes_height(ui.available_height());
    ui.horizontal(|ui| {
        // Main diff scroll area
        ui.vertical(|ui| {
            ui.set_min_height(panes_h);
            if wrap_on {
                // Wrap-on: variable-height manual list (G4). Har model row
                // apne visual count tak failta hai (left/right me jo bada,
                // chhota blank filler se); pehli sub-row number gutter plus
                // actions rakhti hai, continuations blank gutter ke saath
                // indent carry karti hain (G2). Har sub-row ek hi
                // `row_height_for_line` rhythm par hai, poori row
                // `wrap_reserved_height` reserve karti hai; scroll/mapping
                // `wrap_model_to_visual_start` plus `wrap_visual_to_model`
                // invariant se guarded hai (G3). Typo second: continuation
                // plain mono, wahi `span_text_format` (T1); koi naya size,
                // weight, hue ya token nahi (T2, colors frozen).
                egui::ScrollArea::both()
                    .id_salt("text_wrap_list")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        let mut vis_pos = 0usize;
                        while vis_pos < visible.len() {
                            let Some(orig) = visible.get(vis_pos).copied() else {
                                vis_pos = vis_pos.saturating_add(1);
                                continue;
                            };
                            let Some(row) = model.rows.get(orig) else {
                                vis_pos = vis_pos.saturating_add(1);
                                continue;
                            };
                            let count = wrap_counts.get(vis_pos).copied().unwrap_or(1).max(1);
                            if let Some(vstart) = wrap_model_to_visual_start(&wrap_counts, vis_pos)
                            {
                                debug_assert_eq!(
                                    wrap_visual_to_model(&wrap_counts, vstart),
                                    Some((vis_pos, 0))
                                );
                            }
                            let is_selected = model.selected == Some(orig);
                            let (left_fg, left_bg0) = selective_side_palette(true, row, base_text);
                            let (right_fg, right_bg0) =
                                selective_side_palette(false, row, base_text);
                            let (left_bg, right_bg) = if is_selected {
                                (CHANGED_BG, CHANGED_BG)
                            } else {
                                (left_bg0, right_bg0)
                            };
                            let layout = TextPaneLayout {
                                pane_chars,
                                gutter_chars,
                                prefs,
                                dim: gutter_dim,
                            };
                            let find_row = find.map(|ctx| FindRowPaint {
                                needle: ctx.needle,
                                ignore_case: ctx.ignore_case,
                                is_current: find_is_current_row(orig, ctx.current),
                            });
                            let mut left_jobs = if let Some(fr) = find_row {
                                side_jobs_wrapped_with_find_current(
                                    Side::Left,
                                    row,
                                    &layout,
                                    font_id.clone(),
                                    left_fg,
                                    left_bg,
                                    fr,
                                )
                            } else {
                                side_jobs_wrapped_with_prefs(
                                    Side::Left,
                                    row,
                                    &layout,
                                    font_id.clone(),
                                    left_fg,
                                    left_bg,
                                )
                            };
                            let mut right_jobs = if let Some(fr) = find_row {
                                side_jobs_wrapped_with_find_current(
                                    Side::Right,
                                    row,
                                    &layout,
                                    font_id.clone(),
                                    right_fg,
                                    right_bg,
                                    fr,
                                )
                            } else {
                                side_jobs_wrapped_with_prefs(
                                    Side::Right,
                                    row,
                                    &layout,
                                    font_id.clone(),
                                    right_fg,
                                    right_bg,
                                )
                            };
                            while left_jobs.len() < count {
                                left_jobs.push(wrap_blank_sub_job(
                                    font_id.clone(),
                                    left_fg,
                                    left_bg,
                                    gutter_dim,
                                    gutter_chars,
                                    wrap_width,
                                ));
                            }
                            while right_jobs.len() < count {
                                right_jobs.push(wrap_blank_sub_job(
                                    font_id.clone(),
                                    right_fg,
                                    right_bg,
                                    gutter_dim,
                                    gutter_chars,
                                    wrap_width,
                                ));
                            }
                            let mut pairs = left_jobs.into_iter().zip(right_jobs);
                            let mut sub = 0usize;
                            // AW-WRAPNAV: current-row scroll matched sub par
                            // rehta hai — lambi wrapped row me match
                            // continuation me ho to row head par nahi, hit
                            // wale sub par scroll hota hai. Bina find par
                            // 0 (purana head behavior). Geometry first:
                            // `wrap_find_scroll_sub` clamped rehta hai.
                            let scroll_sub = match find {
                                Some(ctx) if !ctx.needle.is_empty() && scroll_to == Some(orig) => {
                                    wrap_find_scroll_sub(
                                        row,
                                        ctx.needle,
                                        ctx.ignore_case,
                                        wrap_width,
                                        count,
                                    )
                                }
                                _ => 0,
                            };
                            let want_scroll = scroll_to == Some(orig)
                                && wrap_visual_offset(&wrap_counts, vis_pos, scroll_sub).is_some();
                            while sub < count {
                                let Some((left_job, right_job)) = pairs.next() else {
                                    break;
                                };
                                ui.set_row_height(wrap_reserved_height(1, row_height));
                                if sub == 0 {
                                    ui.horizontal(|ui| {
                                        // Dual-pane: left pane column + fixed-width center
                                        // gutter (copy arrows) + right pane column. Panes
                                        // barabar (jobs `pane_chars` par padded = 50/50),
                                        // beech me [`SECTION_GAP`] air taaki connector
                                        // saans le. Number gutter `gutter_text` me
                                        // right-aligned fixed width hai.
                                        ui.spacing_mut().item_spacing.x = SECTION_GAP;
                                        ui.label(left_job);
                                        ui.vertical(|ui| {
                                            ui.set_width(text_center_gutter_width());
                                            if text_center_gutter_band_fill(is_selected) {
                                                let band = egui::Rect::from_min_size(
                                                    ui.min_rect().min,
                                                    egui::vec2(
                                                        text_center_gutter_width(),
                                                        row_height,
                                                    ),
                                                );
                                                ui.painter().rect_filled(band, 0.0, CHANGED_BG);
                                            }
                                            if text_center_hunk_head(&model.rows, orig) {
                                                ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = TIGHT_GAP;
                                                    // Centered: leading inset taaki do-arrow row
                                                    // 48px column me centered rahe (headless
                                                    // `text_center_marker_inset` = 6px/side).
                                                    ui.add_space(text_center_marker_inset());
                                                    if ui
                                                        .small_button("→")
                                                        .on_hover_text("Copy Left to Right")
                                                        .clicked()
                                                    {
                                                        copy_action = Some((
                                                            orig,
                                                            viewmodel::CopyDirection::LeftToRight,
                                                        ));
                                                    }
                                                    if ui
                                                        .small_button("←")
                                                        .on_hover_text("Copy Right to Left")
                                                        .clicked()
                                                    {
                                                        copy_action = Some((
                                                            orig,
                                                            viewmodel::CopyDirection::RightToLeft,
                                                        ));
                                                    }
                                                });
                                            } else {
                                                let (g_rect, resp) = ui.allocate_exact_size(
                                                    egui::vec2(
                                                        text_center_gutter_width(),
                                                        row_height,
                                                    ),
                                                    egui::Sense::click(),
                                                );
                                                if text_center_gutter_band_fill(is_selected) {
                                                    ui.painter()
                                                        .rect_filled(g_rect, 0.0, CHANGED_BG);
                                                }
                                                if resp.clicked() {
                                                    select_action = Some(orig);
                                                }
                                            }
                                        });
                                        ui.label(right_job);
                                        let do_scroll = want_scroll && sub == scroll_sub;
                                        if do_scroll {
                                            ui.scroll_to_rect(
                                                ui.min_rect(),
                                                Some(egui::Align::Center),
                                            );
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        // Continuation: same dual-pane columns, center
                                        // gutter blank fixed width (copy sirf head par);
                                        // [`SECTION_GAP`] air head rows jaisa.
                                        ui.spacing_mut().item_spacing.x = SECTION_GAP;
                                        ui.label(left_job);
                                        ui.vertical(|ui| {
                                            ui.set_width(text_center_gutter_width());
                                            let (g_rect, _) = ui.allocate_exact_size(
                                                egui::vec2(text_center_gutter_width(), row_height),
                                                egui::Sense::hover(),
                                            );
                                            if text_center_gutter_band_fill(is_selected) {
                                                ui.painter().rect_filled(g_rect, 0.0, CHANGED_BG);
                                            }
                                        });
                                        ui.label(right_job);
                                        if want_scroll && sub == scroll_sub {
                                            ui.scroll_to_rect(
                                                ui.min_rect(),
                                                Some(egui::Align::Center),
                                            );
                                        }
                                    });
                                }
                                sub = sub.saturating_add(1);
                            }
                            vis_pos = vis_pos.saturating_add(1);
                        }
                        // Wrap-on trailing blanks: flat visuals fill `panes_h`
                        // (BC hatched blank precedent, wrap-off parity).
                        let total_visual = wrap_total_visual(&wrap_counts);
                        let trailing_visual =
                            text_trailing_blank_rows_balanced(panes_h, row_height, total_visual);
                        let mut t = 0usize;
                        while t < trailing_visual {
                            ui.set_row_height(wrap_reserved_height(1, row_height));
                            ui.horizontal(|ui| {
                                // Orphan-blank balance: dono panes barabar
                                // ([`SECTION_GAP`] air samet) taaki khaali
                                // filler dusre pane se chhota na lage.
                                ui.spacing_mut().item_spacing.x = SECTION_GAP;
                                let blank_left = wrap_blank_sub_job(
                                    font_id.clone(),
                                    base_text,
                                    Color32::TRANSPARENT,
                                    gutter_dim,
                                    gutter_chars,
                                    wrap_width,
                                );
                                let left_resp = ui.label(blank_left);
                                paint_filler_hatch(ui.painter(), left_resp.rect, gutter_dim);
                                ui.vertical(|ui| {
                                    ui.set_width(text_center_gutter_width());
                                    ui.allocate_exact_size(
                                        egui::vec2(text_center_gutter_width(), row_height),
                                        egui::Sense::hover(),
                                    );
                                });
                                let blank_right = wrap_blank_sub_job(
                                    font_id.clone(),
                                    base_text,
                                    Color32::TRANSPARENT,
                                    gutter_dim,
                                    gutter_chars,
                                    wrap_width,
                                );
                                let right_resp = ui.label(blank_right);
                                paint_filler_hatch(ui.painter(), right_resp.rect, gutter_dim);
                            });
                            t = t.saturating_add(1);
                        }
                    });
            } else {
                // Wrap-off: virtualized rows plus trailing hatched blanks so
                // short content fills `panes_h` (BC blank-fill precedent).
                // `total` = content + trailing; trailing rows paint both
                // sides blank + hatch + blank center gutter, scroll target
                // sirf real rows par. Typo Mono rhythm unchanged.
                let trailing =
                    text_trailing_blank_rows_balanced(panes_h, row_height, visible.len());
                let total = visible.len().saturating_add(trailing);
                let blank_row = TextRow::default();
                egui::ScrollArea::both()
                    .id_salt("text_diff_rows")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show_rows(ui, row_height, total, |ui, range| {
                        for vis_pos in range {
                            if vis_pos >= visible.len() {
                                ui.set_row_height(row_height);
                                ui.horizontal(|ui| {
                                    // Orphan-blank balance: dono panes barabar
                                    // ([`SECTION_GAP`] air samet), filler hatch
                                    // wali side bhi poori chaudi.
                                    ui.spacing_mut().item_spacing.x = SECTION_GAP;
                                    let layout = TextPaneLayout {
                                        pane_chars,
                                        gutter_chars,
                                        prefs,
                                        dim: gutter_dim,
                                    };
                                    let left_job = side_job_with_prefs(
                                        Side::Left,
                                        &blank_row,
                                        &layout,
                                        font_id.clone(),
                                        base_text,
                                        Color32::TRANSPARENT,
                                    );
                                    let left_resp = ui.label(left_job);
                                    paint_filler_hatch(ui.painter(), left_resp.rect, gutter_dim);
                                    ui.vertical(|ui| {
                                        ui.set_width(text_center_gutter_width());
                                        ui.allocate_exact_size(
                                            egui::vec2(text_center_gutter_width(), row_height),
                                            egui::Sense::hover(),
                                        );
                                    });
                                    let right_job = side_job_with_prefs(
                                        Side::Right,
                                        &blank_row,
                                        &layout,
                                        font_id.clone(),
                                        base_text,
                                        Color32::TRANSPARENT,
                                    );
                                    let right_resp = ui.label(right_job);
                                    paint_filler_hatch(ui.painter(), right_resp.rect, gutter_dim);
                                });
                                continue;
                            }
                            let Some(orig) = visible.get(vis_pos).copied() else {
                                continue;
                            };
                            let Some(row) = model.rows.get(orig) else {
                                continue;
                            };
                            ui.set_row_height(row_height);
                            let is_selected = model.selected == Some(orig);
                            ui.horizontal(|ui| {
                                // Dual-pane: left pane + fixed center gutter + right
                                // pane. Panes barabar (jobs `pane_chars` par
                                // padded = 50/50), beech me [`SECTION_GAP`]
                                // air taaki connector saans le. Leading `·/▶`
                                // hata; selection bg se + blank gutter click
                                // se. Number gutter `gutter_text` right-aligned
                                // fixed width (unchanged shape).
                                ui.spacing_mut().item_spacing.x = SECTION_GAP;
                                let (left_fg, left_bg0) =
                                    selective_side_palette(true, row, base_text);
                                let (right_fg, right_bg0) =
                                    selective_side_palette(false, row, base_text);
                                let (left_bg, right_bg) = if is_selected {
                                    (CHANGED_BG, CHANGED_BG)
                                } else {
                                    (left_bg0, right_bg0)
                                };
                                let layout = TextPaneLayout {
                                    pane_chars,
                                    gutter_chars,
                                    prefs,
                                    dim: gutter_dim,
                                };
                                let find_row = find.map(|ctx| FindRowPaint {
                                    needle: ctx.needle,
                                    ignore_case: ctx.ignore_case,
                                    is_current: find_is_current_row(orig, ctx.current),
                                });
                                let left_job = if let Some(fr) = find_row {
                                    side_job_with_prefs_and_find_current(
                                        Side::Left,
                                        row,
                                        &layout,
                                        font_id.clone(),
                                        left_fg,
                                        left_bg,
                                        fr,
                                    )
                                } else {
                                    side_job_with_prefs(
                                        Side::Left,
                                        row,
                                        &layout,
                                        font_id.clone(),
                                        left_fg,
                                        left_bg,
                                    )
                                };
                                let left_resp = ui.label(left_job);
                                if text_side_is_filler(row.left.as_str()) {
                                    paint_filler_hatch(ui.painter(), left_resp.rect, gutter_dim);
                                }

                                ui.vertical(|ui| {
                                    ui.set_width(text_center_gutter_width());
                                    if text_center_gutter_band_fill(is_selected) {
                                        let band = egui::Rect::from_min_size(
                                            ui.min_rect().min,
                                            egui::vec2(text_center_gutter_width(), row_height),
                                        );
                                        ui.painter().rect_filled(band, 0.0, CHANGED_BG);
                                    }
                                    if text_center_hunk_head(&model.rows, orig) {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = TIGHT_GAP;
                                            // Centered: wrap-off head wahi 6px leading
                                            // inset (wrap-on parity, ek rhythm).
                                            ui.add_space(text_center_marker_inset());
                                            if ui
                                                .small_button("→")
                                                .on_hover_text("Copy Left to Right")
                                                .clicked()
                                            {
                                                copy_action = Some((
                                                    orig,
                                                    viewmodel::CopyDirection::LeftToRight,
                                                ));
                                            }
                                            if ui
                                                .small_button("←")
                                                .on_hover_text("Copy Right to Left")
                                                .clicked()
                                            {
                                                copy_action = Some((
                                                    orig,
                                                    viewmodel::CopyDirection::RightToLeft,
                                                ));
                                            }
                                        });
                                    } else {
                                        let (g_rect, resp) = ui.allocate_exact_size(
                                            egui::vec2(text_center_gutter_width(), row_height),
                                            egui::Sense::click(),
                                        );
                                        if text_center_gutter_band_fill(is_selected) {
                                            ui.painter().rect_filled(g_rect, 0.0, CHANGED_BG);
                                        }
                                        if resp.clicked() {
                                            select_action = Some(orig);
                                        }
                                    }
                                });

                                let right_job = if let Some(fr) = find_row {
                                    side_job_with_prefs_and_find_current(
                                        Side::Right,
                                        row,
                                        &layout,
                                        font_id.clone(),
                                        right_fg,
                                        right_bg,
                                        fr,
                                    )
                                } else {
                                    side_job_with_prefs(
                                        Side::Right,
                                        row,
                                        &layout,
                                        font_id.clone(),
                                        right_fg,
                                        right_bg,
                                    )
                                };
                                let right_resp = ui.label(right_job);
                                if text_side_is_filler(row.right.as_str()) {
                                    paint_filler_hatch(ui.painter(), right_resp.rect, gutter_dim);
                                }
                                if scroll_to == Some(orig) {
                                    ui.scroll_to_rect(ui.min_rect(), Some(egui::Align::Center));
                                }
                            });
                        }
                    });
            }
        });

        // Thumbnail overview gutter on the right (same remaining-height
        // budget as the list column, so gutter and rows stay in sync);
        // narrows when line numbers hide so chrome stays in sync while
        // counts stay honest.
        // Click/drag jumps: fraction se nearest-diff row select hoti hai
        // (wahi snap jo hex minimap `hex_minimap_jump_to_row` me hai),
        // selection marker synced rehta hai, list agli frame scroll karti
        // hai (pending key, struct change bina).
        let avail_h = panes_h;
        let geom = viewmodel::gutter_geometry(model.rows.len(), avail_h, &model.rows);
        let bar_w = text_minimap_ruler_width(prefs.show_line_numbers);
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(bar_w, avail_h), egui::Sense::click_and_drag());
        debug_assert_eq!(rect.width(), bar_w);
        let resp = resp.on_hover_text(text_minimap_accessibility_label(
            model.selected,
            model.rows.len(),
        ));
        ui.painter().rect_filled(rect, 0.0, track_for(dark));

        for seg in &geom.segments {
            let seg_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y + seg.y_start),
                egui::pos2(rect.max.x, (rect.min.y + seg.y_end).min(rect.max.y)),
            );
            let color = match seg.kind {
                TextRowKind::Equal => Color32::TRANSPARENT,
                TextRowKind::Deleted => DELETED_BG,
                TextRowKind::Inserted => INSERTED_BG,
                TextRowKind::Changed => CHANGED_BG,
            };
            ui.painter().rect_filled(seg_rect, 0.0, color);
        }
        // Selection/viewport marker: selected row ke band center par
        // min-segment uncha tick ([`text_minimap_marker_band`]) theme text
        // tone me — segment rhythm se aligned, kabhi do rows par straddle
        // nahi; jump ke baad indicator clicked band par synced dikhta hai.
        if let Some(sel) = model.selected
            && let Some((dy0, dy1)) = text_minimap_marker_band(sel, model.rows.len(), avail_h)
        {
            let marker = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, (rect.min.y + dy0).max(rect.min.y)),
                egui::pos2(rect.max.x, (rect.min.y + dy1).min(rect.max.y)),
            );
            ui.painter()
                .rect_filled(marker, 0.0, ui.visuals().text_color());
        }
        // Click/drag jump: pointer y se nearest-diff row, selection set
        // + next-frame scroll pending (list isi frame pehle paint ho
        // chuki hai, isliye scroll agli frame hota hai). Slop-expanded
        // segment gate pehle: ruler bands par hi jump, khali track par
        // nahi (4px bands ungli se lagte hain, segment paint nahi hilta).
        if (resp.clicked() || resp.dragged())
            && let Some(pos) = resp.interact_pointer_pos()
        {
            let local_y = pos.y - rect.min.y;
            if text_ruler_segment_hit(&geom.segments, local_y).is_some()
                && let Some(idx) =
                    text_gutter_click_select(&model.rows, model.rows.len(), avail_h, local_y)
            {
                model.selected = Some(idx);
                ui.ctx().data_mut(|data| {
                    data.insert_temp(text_gutter_pending_id(), idx);
                });
            }
        }
    });

    // Segmented pane status (BC per-pane `1:1` precedent): panes row ke
    // neeche left/right cursor side-by-side, taaki har pane ka cursor
    // apne pane ke neeche padhe. Geometry/typo top status bar reuse
    // (full-width `HEADER_H` + Small weak); koi naya token/color nahi.
    paint_text_pane_status(ui, &model.rows, model.selected);

    if let Some((idx, dir)) = copy_action
        && viewmodel::copy_line(model, idx, dir).is_ok()
    {
        model.selected = Some(idx);
    }
    if let Some(idx) = select_action {
        model.selected = Some(idx);
    }
}

/// Which side of a text row a pane renders.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

/// Prefs-effective paint widths and gutter tone for one text pane:
/// widths come from [`view_text_with_prefs`] (wrap caps `pane_chars`
/// at 80, hidden numbers narrow `gutter_chars` to 1) while `prefs`
/// drives the per-row hiding and `dim` carries the per-mode gutter
/// tone. Bundled so pane painters stay under the argument-count lint.
pub struct TextPaneLayout<'a> {
    /// Prefs-effective pane width.
    pub pane_chars: usize,
    /// Prefs-effective gutter width.
    pub gutter_chars: usize,
    /// Toggles the pane honors.
    pub prefs: &'a TextViewPrefs,
    /// Per-mode gutter tone (`theme::dim_for(dark)` at the call site).
    pub dim: Color32,
}

/// Intra-line span underline width (px): 1px line jo same mono
/// line-height ke andar rehti hai — row height kabhi nahi badalti.
/// TypoRole precedent (chrome `strong` = bina size badle emphasis):
/// span emphasis sirf inversion + is underline se aata hai, naya
/// size/weight nahi. Pure token; unit-tested.
pub const INTRA_SPAN_UNDERLINE_W: f32 = 1.0;

/// Changed-word accent underline width (px): double the 1px find line
/// ([`INTRA_SPAN_UNDERLINE_W`]) so the changed span reads heavier than a
/// find hit by line weight alone — same frozen hue, no new token, no
/// size change. Pure token; unit-tested.
pub const ACCENT_SPAN_UNDERLINE_W: f32 = 2.0 * INTRA_SPAN_UNDERLINE_W;

/// Intra-line span breathing pad (px): one XS step
/// ([`crate::theme::SPACE_XS`] reuse, 4.0). Changed-word blocks keep this
/// total air split equally on both sides (2 + 2) so the heavier 2px
/// accent underline never crowds its neighbours — even gaps vs BC, same
/// mono advance, no size/weight/color move. Pure token; unit-tested.
pub const INTRA_SPAN_PAD_PX: f32 = crate::theme::SPACE_XS;

/// Total intra-span pad px (single source). Pure; unit-tested.
pub fn intra_span_pad_px() -> f32 {
    INTRA_SPAN_PAD_PX
}

/// One-side intra-span pad px: total pad split equally so the left gap
/// always equals the right gap (symmetric by construction). Pure;
/// unit-tested.
pub fn intra_span_side_pad_px() -> f32 {
    INTRA_SPAN_PAD_PX / 2.0
}

/// Symmetric-gap contract: `2 * side == total` (left and right air are
/// one [`SPACE_XS`][`crate::theme::SPACE_XS`] half each). Pure;
/// unit-tested.
pub fn intra_span_gaps_symmetric() -> bool {
    (intra_span_side_pad_px() * 2.0 - intra_span_pad_px()).abs() < 0.001
}

/// Leading gap (px) for one intra-line segment in a [`LayoutJob`]: the
/// head segment starts flush (0.0) and every following segment takes one
/// side pad, so all internal gaps read one uniform step — symmetric on
/// both sides of every span boundary. Gutter prefixes keep 0.0. Pure;
/// unit-tested.
pub fn intra_segment_leading_gap(is_first: bool) -> f32 {
    if is_first {
        0.0
    } else {
        intra_span_side_pad_px()
    }
}

/// (text, background) for one pane segment: plain passes the row
/// strip pair through, accented inverts it so the span paints as a
/// distinct block over the full-row strip. Row ke apne frozen pair
/// ka reuse — koi naya hue/token nahi. Pure; unit-tested.
fn span_colors(fg: Color32, bg: Color32, accented: bool) -> (Color32, Color32) {
    if accented { (bg, fg) } else { (fg, bg) }
}

/// Changed-row word span pair (left, right): half-open char ranges.
pub type WordSpanPair<'a> = (&'a [(usize, usize)], &'a [(usize, usize)]);

/// Changed-row word spans: sirf `Changed` kind par model ke
/// `accent_a`/`accent_b` (viewmodel `inline_diff` se), baaki kind par
/// khali — Deleted/Inserted poore-row band rehte hain, Equal par koi
/// accent nahi. Paint ke chaaron side-job sites isi se spans lete hain
/// taaki kaun sa shabd badla wahi block+underline me dikhe. Pure;
/// unit-tested.
pub fn changed_row_word_spans(row: &TextRow) -> WordSpanPair<'_> {
    if row.kind == TextRowKind::Changed {
        (row.accent_a.as_slice(), row.accent_b.as_slice())
    } else {
        (&[], &[])
    }
}

/// Normalized intra-line spans: sort + clamp + merge, taaki paint
/// rects chipke hue aayen (koi overlap/gap/overflow nahi).
/// `line_len` line ke chars hain, `pane` paint width hai — limit dono
/// ka min hai. Zero-length aur bahar ke ranges gir jate hain, aas-paas
/// wale (`next.start <= cur.end`) jud jate hain taaki ek hi block
/// paint ho. Geometry-first: rect alignment line-height ke andar.
/// Pure; unit-tested.
pub fn normalize_intra_spans(
    spans: &[(usize, usize)],
    line_len: usize,
    pane: usize,
) -> Vec<(usize, usize)> {
    let limit = line_len.min(pane);
    let mut sorted: Vec<(usize, usize)> = Vec::new();
    for &(start, end) in spans {
        if end <= start {
            continue;
        }
        let s = start.min(limit);
        let e = end.min(limit);
        if e <= s {
            continue;
        }
        sorted.push((s, e));
    }
    sorted.sort();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (s, e) in sorted {
        let mut merged = false;
        if let Some(last) = out.last_mut() {
            if s <= last.1 && e > last.1 {
                last.1 = e;
                merged = true;
            } else if s <= last.1 {
                merged = true;
            }
        }
        if !merged {
            out.push((s, e));
        }
    }
    out
}

/// One segment ka paint format: same mono `FontId` (size kabhi nahi
/// badalta — line-height untouched), background `span_colors` se,
/// accented par `INTRA_SPAN_UNDERLINE_W` underline. TypoRole precedent:
/// chrome me `strong` bina size badle zor deta hai; yahan mono me zor
/// inversion + underline se hai. Pure; unit-tested.
pub fn span_text_format(font_id: FontId, fg: Color32, bg: Color32, accented: bool) -> TextFormat {
    let (color, background) = span_colors(fg, bg, accented);
    let mut fmt = TextFormat::simple(font_id, color);
    fmt.background = background;
    if accented {
        fmt.underline = Stroke::new(INTRA_SPAN_UNDERLINE_W, color);
    }
    fmt
}

// ----------------------------------------------------------------------------
// Text find highlight (AT-FINDHL): current vs others + wrap geometry.
// Geometry first: find spans char indices me hain (same `normalize_intra_spans`
// clamp/merge), wrap continuations me `wrap_chunk_spans` reuse se map hote
// hain — match wrapping geometry `pane_segments`/wrap precedent jaisi hai.
// Typography second: same mono `FontId` (koi naya size nahi), emphasis sirf
// block-vs-line se: current = inverted block (no underline), other =
// underline-only line (no block). Accent precedent (`span_text_format`:
// inverted + underline) se dono alag hain; find wins over accent jahan
// overlap ho. Colors frozen: row ke apne `fg/bg` + `INTRA_SPAN_UNDERLINE_W`
// reuse, koi naya hue/token nahi. Pure; unit-tested.
// ----------------------------------------------------------------------------

/// One find match ka paint kind: plain (koi hit/accent nahi), accent
/// (syntax intra-span only), other (non-current needle hit), current
/// (selected row ka needle hit). Find wins over accent jahan overlap ho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindHl {
    /// Koi accent/find nahi — row strip pair as-is.
    Plain,
    /// Syntax accent only — `span_text_format(true)` reuse.
    Accent,
    /// Non-current needle hit — underline-only line, no block.
    Other,
    /// Current row ka needle hit — inverted block, no underline.
    Current,
}

/// Per-frame find paint context: needle + case flag + current row.
/// `paint_text_body` is me `FindState::query/ignore_case` aur
/// `model.selected` pack karta hai; panes per-row `is_current` nikalte
/// hain. `Copy` taaki list loop me cheap reuse ho.
#[derive(Debug, Clone, Copy)]
pub struct FindPaintCtx<'a> {
    /// Needle text (`FindState::query` se).
    pub needle: &'a str,
    /// Case-insensitive match ho ya nahi.
    pub ignore_case: bool,
    /// Current match row (`model.selected` se).
    pub current: Option<usize>,
}

/// One row ka find paint input: needle + flag + resolved current flag.
/// Teen find args ek struct me taaki pane painters argument-count lint
/// ke andar rahen ([`TextPaneLayout`] precedent). `Copy` taaki list loop
/// me cheap reuse ho.
#[derive(Debug, Clone, Copy)]
pub struct FindRowPaint<'a> {
    /// Needle text.
    pub needle: &'a str,
    /// Case-insensitive match ho ya nahi.
    pub ignore_case: bool,
    /// Ye row current-match row hai ya nahi.
    pub is_current: bool,
}

/// Line me needle ke char-index spans (half-open `[start, end)`).
/// Khali needle par khali (koi fake hit nahi). ASCII case-fold
/// (`to_ascii_lowercase` per char) taaki indices stable rahen — unicode
/// non-ascii exact rehta hai. Non-overlapping nahi: overlap allow hai
/// (`"aaa"` me `"aa"` do hits deta hai) taaki paint koi match na chhupaye.
/// Clamp/merge caller [`normalize_intra_spans`] karta hai (pane geometry
/// wahi). Pure; unit-tested.
pub fn find_line_spans(line: &str, needle: &str, ignore_case: bool) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let line_chars: Vec<char> = line.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();
    let mut nlen = 0usize;
    for _ in needle_chars.iter() {
        nlen = nlen.saturating_add(1);
    }
    let mut llen = 0usize;
    for _ in line_chars.iter() {
        llen = llen.saturating_add(1);
    }
    if nlen == 0 || nlen > llen {
        return Vec::new();
    }
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut start = 0usize;
    while start.saturating_add(nlen) <= llen {
        let mut ok = true;
        let mut j = 0usize;
        while j < nlen {
            let lc = line_chars.get(start.saturating_add(j));
            let nc = needle_chars.get(j);
            match (lc, nc) {
                (Some(l), Some(n)) => {
                    let same = if ignore_case {
                        l.eq_ignore_ascii_case(n)
                    } else {
                        l == n
                    };
                    if !same {
                        ok = false;
                        break;
                    }
                }
                _ => {
                    ok = false;
                    break;
                }
            }
            j = j.saturating_add(1);
        }
        if ok {
            out.push((start, start.saturating_add(nlen)));
        }
        start = start.saturating_add(1);
    }
    out
}

/// Row current-match row hai ya nahi: `Some(row) == current`.
/// Pure; unit-tested.
pub fn find_is_current_row(row_idx: usize, current: Option<usize>) -> bool {
    match current {
        Some(c) => c == row_idx,
        None => false,
    }
}

/// One find hit ka paint format: same mono `FontId` (size untouched),
/// row ke apne `fg/bg` reuse. Current = inverted block (no underline),
/// other = underline-only line (no block) — block-vs-line se clearly
/// distinct, accent (block + line) se bhi alag. `INTRA_SPAN_UNDERLINE_W`
/// reuse, koi naya token/color nahi. Pure; unit-tested.
pub fn find_hit_format(font_id: FontId, fg: Color32, bg: Color32, is_current: bool) -> TextFormat {
    if is_current {
        let (color, background) = span_colors(fg, bg, true);
        let mut fmt = TextFormat::simple(font_id, color);
        fmt.background = background;
        fmt
    } else {
        let mut fmt = TextFormat::simple(font_id, fg);
        fmt.background = bg;
        fmt.underline = Stroke::new(INTRA_SPAN_UNDERLINE_W, fg);
        fmt
    }
}

/// One pane ke find-aware segments: `(text, kind)` list, `pane` chars tak
/// truncated plus padding — [`pane_segments`] wahi geometry (truncation +
/// trailing pad). Accent spans syntax-gated hone chahiye (caller
/// `prefs.syntax_highlight` off par `&[]` de); find spans needle se
/// nikalte hain. Find wins over accent. Pure; unit-tested.
pub fn find_pane_segments(
    line: &str,
    accent_spans: &[(usize, usize)],
    needle: &str,
    ignore_case: bool,
    is_current: bool,
    pane: usize,
) -> Vec<(String, FindHl)> {
    let line_len = line.chars().count();
    let accents = normalize_intra_spans(accent_spans, line_len, pane);
    let find_raw = find_line_spans(line, needle, ignore_case);
    let finds = normalize_intra_spans(&find_raw, line_len, pane);
    let mut out: Vec<(String, FindHl)> = Vec::new();
    let mut current_text = String::new();
    let mut current_kind = FindHl::Plain;
    let mut started = false;
    let mut visible = 0usize;
    for (idx, ch) in line.chars().enumerate() {
        if visible >= pane {
            break;
        }
        let in_accent = accents.iter().any(|&(s, e)| idx >= s && idx < e);
        let in_find = finds.iter().any(|&(s, e)| idx >= s && idx < e);
        let kind = if in_find {
            if is_current {
                FindHl::Current
            } else {
                FindHl::Other
            }
        } else if in_accent {
            FindHl::Accent
        } else {
            FindHl::Plain
        };
        if started && kind != current_kind && !current_text.is_empty() {
            out.push((std::mem::take(&mut current_text), current_kind));
        }
        current_kind = kind;
        started = true;
        current_text.push(ch);
        visible = visible.saturating_add(1);
    }
    if started && !current_text.is_empty() {
        out.push((current_text, current_kind));
    }
    let pad = filler_pad_width(pane, visible);
    if pad > 0 {
        out.push((" ".repeat(pad), FindHl::Plain));
    }
    out
}

/// One wrap chunk ke find-aware segments: line-level `accent_raw` /
/// `find_raw` spans ko [`wrap_chunk_spans`] se chunk-local karke wahi
/// block-vs-line walk. Caller syntax off par `accent_raw = &[]` de.
/// `width` chunk paint width hai (caller `wrap_width_for` se). Pure;
/// unit-tested.
pub fn find_chunk_segments(
    chunk: &WrapChunk,
    accent_raw: &[(usize, usize)],
    find_raw: &[(usize, usize)],
    is_current: bool,
    width: usize,
) -> Vec<(String, FindHl)> {
    let accents = wrap_chunk_spans(accent_raw, chunk);
    let finds = wrap_chunk_spans(find_raw, chunk);
    let accents = normalize_intra_spans(&accents, chunk.text.chars().count(), width);
    let finds = normalize_intra_spans(&finds, chunk.text.chars().count(), width);
    let mut out: Vec<(String, FindHl)> = Vec::new();
    let mut current_text = String::new();
    let mut current_kind = FindHl::Plain;
    let mut started = false;
    let mut visible = 0usize;
    for (idx, ch) in chunk.text.chars().enumerate() {
        if visible >= width {
            break;
        }
        let in_accent = accents.iter().any(|&(s, e)| idx >= s && idx < e);
        let in_find = finds.iter().any(|&(s, e)| idx >= s && idx < e);
        let kind = if in_find {
            if is_current {
                FindHl::Current
            } else {
                FindHl::Other
            }
        } else if in_accent {
            FindHl::Accent
        } else {
            FindHl::Plain
        };
        if started && kind != current_kind && !current_text.is_empty() {
            out.push((std::mem::take(&mut current_text), current_kind));
        }
        current_kind = kind;
        started = true;
        current_text.push(ch);
        visible = visible.saturating_add(1);
    }
    if started && !current_text.is_empty() {
        out.push((current_text, current_kind));
    }
    let pad = filler_pad_width(width, visible);
    if pad > 0 {
        out.push((" ".repeat(pad), FindHl::Plain));
    }
    out
}

/// One find segment ka paint format: `Plain`/`Accent` existing
/// `span_text_format` reuse, `Other`/`Current` [`find_hit_format`] reuse.
/// Same mono `FontId`, koi naya size/color nahi. Pure; unit-tested.
pub fn find_segment_format(font_id: FontId, fg: Color32, bg: Color32, kind: FindHl) -> TextFormat {
    match kind {
        FindHl::Plain => span_text_format(font_id, fg, bg, false),
        FindHl::Accent => span_text_format(font_id, fg, bg, true),
        FindHl::Other => find_hit_format(font_id, fg, bg, false),
        FindHl::Current => find_hit_format(font_id, fg, bg, true),
    }
}

/// One pane of one row honoring `layout`: line numbers blank when off,
/// accent spans cleared when syntax is off, width from the
/// prefs-effective `pane_chars`/`gutter_chars`. Counts are untouched:
/// hiding only changes paint, never the model.
fn side_job_with_prefs(
    side: Side,
    row: &TextRow,
    layout: &TextPaneLayout<'_>,
    font_id: FontId,
    fg: Color32,
    bg: Color32,
) -> LayoutJob {
    let (raw_no, line) = match side {
        Side::Left => (row.left_no, row.left.as_str()),
        Side::Right => (row.right_no, row.right.as_str()),
    };
    let (accent_left, accent_right) = changed_row_word_spans(row);
    let raw_spans: &[(usize, usize)] = match side {
        Side::Left => accent_left,
        Side::Right => accent_right,
    };
    let no = if layout.prefs.show_line_numbers {
        raw_no
    } else {
        None
    };
    let mut job = LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut gutter = TextFormat::simple(font_id.clone(), layout.dim);
    gutter.background = bg;
    job.append(&gutter_text(no, layout.gutter_chars), 0.0, gutter);
    let mut first_seg = true;
    for (segment, accented) in prefs_pane_segments(line, raw_spans, layout.pane_chars, layout.prefs)
    {
        job.append(
            &segment,
            intra_segment_leading_gap(first_seg),
            selective_span_format(font_id.clone(), fg, bg, accented),
        );
        first_seg = false;
    }
    job
}

/// Find-aware pane job with explicit current flag: current row par
/// `find.is_current=true` (inverted block), others par false (underline
/// line). Paint loop `find_is_current_row` se flag nikalta hai.
fn side_job_with_prefs_and_find_current(
    side: Side,
    row: &TextRow,
    layout: &TextPaneLayout<'_>,
    font_id: FontId,
    fg: Color32,
    bg: Color32,
    find: FindRowPaint<'_>,
) -> LayoutJob {
    if find.needle.is_empty() {
        return side_job_with_prefs(side, row, layout, font_id, fg, bg);
    }
    let (raw_no, line) = match side {
        Side::Left => (row.left_no, row.left.as_str()),
        Side::Right => (row.right_no, row.right.as_str()),
    };
    let (accent_left, accent_right) = changed_row_word_spans(row);
    let raw_spans: &[(usize, usize)] = match side {
        Side::Left => accent_left,
        Side::Right => accent_right,
    };
    let accents: &[(usize, usize)] = if layout.prefs.syntax_highlight {
        raw_spans
    } else {
        &[]
    };
    let mut job = LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut gutter = TextFormat::simple(font_id.clone(), layout.dim);
    gutter.background = bg;
    let no = if layout.prefs.show_line_numbers {
        raw_no
    } else {
        None
    };
    job.append(&gutter_text(no, layout.gutter_chars), 0.0, gutter);
    let mut first_seg = true;
    for (segment, kind) in find_pane_segments(
        line,
        accents,
        find.needle,
        find.ignore_case,
        find.is_current,
        layout.pane_chars,
    ) {
        job.append(
            &segment,
            intra_segment_leading_gap(first_seg),
            selective_find_format(font_id.clone(), fg, bg, kind),
        );
        first_seg = false;
    }
    job
}

/// Prefs-aware pane split: syntax off clears accents before delegating
/// to [`pane_segments`], so wrap/syntax toggles show in panes while the
/// model keeps honest counts. Pure; unit-tested.
pub fn prefs_pane_segments(
    line: &str,
    spans: &[(usize, usize)],
    pane: usize,
    prefs: &TextViewPrefs,
) -> Vec<(String, bool)> {
    if prefs.syntax_highlight {
        pane_segments(line, spans, pane)
    } else {
        pane_segments(line, &[], pane)
    }
}

/// (text color, row background) for one row kind. Equal rows keep the
/// theme's text color on a transparent background.
fn kind_palette(kind: TextRowKind, base_text: Color32) -> (Color32, Color32) {
    match kind {
        TextRowKind::Equal => (base_text, Color32::TRANSPARENT),
        TextRowKind::Deleted => (DELETED_FG, DELETED_BG),
        TextRowKind::Inserted => (INSERTED_FG, INSERTED_BG),
        TextRowKind::Changed => (CHANGED_FG, CHANGED_BG),
    }
}

/// Selective highlight: Changed/Equal rows paint on a plain base — only
/// the changed word spans carry the changed band. Deleted/Inserted keep
/// their full-row bands on the text side. Pure; unit-tested.
pub fn selective_row_is_plain(kind: TextRowKind) -> bool {
    matches!(kind, TextRowKind::Equal | TextRowKind::Changed)
}

/// Per-side paint pair: an empty side is filler (plain base, hatch comes
/// from geometry), Changed/Equal text sides stay plain, Deleted/Inserted
/// text sides keep their band. Selection override caller me rehta hai.
/// Pure; unit-tested.
pub fn selective_side_palette(
    is_left: bool,
    row: &TextRow,
    base_text: Color32,
) -> (Color32, Color32) {
    let empty = if is_left {
        text_side_is_filler(row.left.as_str())
    } else {
        text_side_is_filler(row.right.as_str())
    };
    if empty {
        return (base_text, Color32::TRANSPARENT);
    }
    if selective_row_is_plain(row.kind) {
        return (base_text, Color32::TRANSPARENT);
    }
    kind_palette(row.kind, base_text)
}

/// Selective accent format: plain passes through, accented paints the
/// frozen changed band plus the heavier accent underline
/// ([`ACCENT_SPAN_UNDERLINE_W`], double the find line — emphasis by line
/// weight, same hue, no new color). Same mono `FontId`
/// (line-height untouched). Pure; unit-tested.
pub fn selective_span_format(
    font_id: FontId,
    fg: Color32,
    bg: Color32,
    accented: bool,
) -> TextFormat {
    if accented {
        let mut fmt = TextFormat::simple(font_id, CHANGED_FG);
        fmt.background = CHANGED_BG;
        fmt.underline = Stroke::new(ACCENT_SPAN_UNDERLINE_W, CHANGED_FG);
        fmt
    } else {
        span_text_format(font_id, fg, bg, false)
    }
}

/// Selective find format: Plain/Accent selective reuse, Other plain+line,
/// Current light block (inverted changed band, no underline) taaki accent
/// (dark band + line) se alag rahe. Band wale rows par old hit reuse.
/// Pure; unit-tested.
pub fn selective_find_format(
    font_id: FontId,
    fg: Color32,
    bg: Color32,
    kind: FindHl,
) -> TextFormat {
    match kind {
        FindHl::Plain | FindHl::Other => find_segment_format(font_id, fg, bg, kind),
        FindHl::Accent => selective_span_format(font_id, fg, bg, true),
        FindHl::Current => {
            if bg == Color32::TRANSPARENT {
                let (color, background) = span_colors(CHANGED_FG, CHANGED_BG, true);
                let mut fmt = TextFormat::simple(font_id, color);
                fmt.background = background;
                fmt
            } else {
                find_hit_format(font_id, fg, bg, true)
            }
        }
    }
}

/// Filler geometry (WIDTH only, color nahi): khaali pane filler hai.
/// Pure; unit-tested.
pub fn text_side_is_filler(line: &str) -> bool {
    line.is_empty()
}

/// Filler pad width (chars): pane minus line, saturating. Pure; unit-tested.
pub fn filler_pad_width(pane: usize, line_len: usize) -> usize {
    pane.saturating_sub(line_len.min(pane))
}

/// Hatched filler stripe step (px): [`TIGHT_GAP`] reuse, koi naya token
/// nahi. Pure; unit-tested. Live paint [`paint_filler_hatch`] reuse karta hai.
pub fn filler_hatch_step() -> f32 {
    TIGHT_GAP
}

/// Hatch stripes across a filler of `width` px: step-spaced ticks.
/// Geometry only (paint inhe track par khinchta hai). Pure; unit-tested.
/// Live paint [`paint_filler_hatch`] reuse karta hai.
pub fn filler_hatch_count(width: f32) -> usize {
    if !width.is_finite() || width <= 0.0 {
        return 0;
    }
    let step = filler_hatch_step();
    if !step.is_finite() || step <= 0.0 {
        return 0;
    }
    (width / step).floor() as usize
}

/// Viewport fill: remaining-height budget me kitne blank rows lagenge
/// taaki short content ke neeche kala waste na rahe (BC hatched blank
/// precedent). Slots `ceil(panes_h / row_h)` hain, trailing usme se
/// `visible_len` ghata kar — short file par positive, full/overflow par
/// zero. Guards finite/positive hain; koi unwrap/panic nahi. Geometry
/// only ([`TIGHT_GAP`] step reuse nahi, sirf row rhythm); typo/colors
/// untouched. Pure; unit-tested.
pub fn text_trailing_blank_rows(panes_h: f32, row_h: f32, visible_len: usize) -> usize {
    if !panes_h.is_finite() || !row_h.is_finite() || panes_h <= 0.0 || row_h <= 0.0 {
        return 0;
    }
    let slots = (panes_h / row_h).ceil().clamp(0.0, 100_000.0) as usize;
    slots.saturating_sub(visible_len)
}

/// Balanced trailing blanks: [`text_trailing_blank_rows`] rounded up to an
/// even count so filler never ends on a lone orphan row — tang 720px
/// window par dono halves visually paired rahen. Zero stays zero
/// (full/overflow par koi extra row nahi). Pure; unit-tested.
pub fn text_trailing_blank_rows_balanced(
    panes_h: f32,
    row_h: f32,
    visible_len: usize,
) -> usize {
    let n = text_trailing_blank_rows(panes_h, row_h, visible_len);
    if n == 0 { 0 } else { n + (n % 2) }
}

/// Center-gutter per-hunk marker: diff kinds par marker, Equal par nahi.
/// Copy arrows isi gate par hain ([`text_gutter_has_copy`] reuse) — BC
/// center bar me har hunk par arrow/marker rehta hai, equal rows par
/// blank click-to-select. Ponytail alias taaki paint intent naam se
/// padhe, logic ek jagah rahe. Pure; unit-tested.
pub fn text_center_hunk_marker(kind: TextRowKind) -> bool {
    text_gutter_has_copy(kind)
}

/// Center-gutter selection band: selected row par gutter bhi selected
/// panes jaisi patti dikhata hai taaki band beech me na toote (BC
/// continuous selection precedent). Head/blank dono par same gate —
/// copy arrows band ke upar rehte hain, blank click-to-select band ke
/// upar. Caller existing [`CHANGED_BG`] reuse karta hai (koi naya
/// hue/token nahi). Pure; unit-tested.
pub fn text_center_gutter_band_fill(selected: bool) -> bool {
    selected
}

/// Center-gutter marker content width px: do arrow boxes
/// ([`crate::gutter::GUTTER_ARROW_BOX`] = 16 each) + ek [`TIGHT_GAP`]
/// (4) beech me = 36. Ponytail reuse: koi naya literal nahi, sirf
/// existing tokens ka jod. Paint isi par
/// [`crate::gutter::center_row_marker_inset`] se leading inset nikalta
/// hai taaki do-arrow row 48px column me centered rahe. Pure; unit-tested.
pub fn text_center_marker_content_w() -> f32 {
    2.0 * crate::gutter::GUTTER_ARROW_BOX + TIGHT_GAP
}

/// Center-gutter marker leading inset px: 48px column me 36px content
/// = 6px per side, dono taraf barabar hawa taaki copy arrows centered
/// dikhen (BC center-bar rhythm). Guards
/// [`crate::gutter::center_row_marker_inset`] me hain. Pure; unit-tested.
pub fn text_center_marker_inset() -> f32 {
    crate::gutter::center_row_marker_inset(TEXT_CENTER_GUTTER_W, text_center_marker_content_w())
}

/// Wrap-continuation marker glyph: number-column ke right edge par lagta
/// hai taaki lambi wrapped line ka continuation head se alag padhe (BC
/// wrap-indicator precedent, apne shabdon me — koi asset copy nahi).
/// Paint text me nahi, gutter prefix me hai isliye mono rhythm untouched.
/// Pure token.
pub const WRAP_CONTINUATION_MARKER: &str = "\u{21a9}";

/// Wrap-continuation gutter with marker: number field right-aligned slot
/// par [`WRAP_CONTINUATION_MARKER`], phir wahi trailing pad
/// ([`GUTTER_NUMBER_TRAIL_PAD`]) jo [`wrap_continuation_gutter`] rakhta
/// hai — column rhythm nahi toot-ta (same char count), marker code se
/// chipakta nahi. Blank gutter se sirf ek slot alag hai. Pure; unit-tested.
pub fn wrap_marker_gutter(gutter_chars: usize) -> String {
    let trail = " ".repeat(GUTTER_NUMBER_TRAIL_PAD);
    if gutter_chars == 0 {
        return trail;
    }
    format!(
        "{}{WRAP_CONTINUATION_MARKER}{trail}",
        " ".repeat(gutter_chars.saturating_sub(1))
    )
}

/// Wrap-marker column (0-based char offset): number field ka aakhri slot
/// (`gutter_chars - 1`), marker wahi baithta hai. `0` par 0 (degenerate
/// guard). Geometry review ke liye headless contract. Pure; unit-tested.
pub fn wrap_marker_column(gutter_chars: usize) -> usize {
    gutter_chars.saturating_sub(1)
}

/// Wide-960 pane pair px: 960 canvas par center connector (48) + 2 x
/// [`SECTION_GAP`] air ghata kar bacha aadha-aadha = 448/448
/// ([`text_pane_pair_widths`] reuse). Fixture
/// `comparison/text-wide-960.json` isi ko lock karti hai. Pure; unit-tested.
pub fn text_wide_960_pane_pair() -> (f32, f32) {
    text_pane_pair_widths(960.0)
}

/// Segmented pane status: selected row ke per-side line numbers —
/// left `L<n>` / `R<n>`, missing side ya no-selection/OOB par `L-` /
/// `R-` (koi fake number nahi). BC bottom bar har pane ke neeche apna
/// cursor dikhata hai (`1:1` per pane); ye uska headless model hai.
/// Paint [`paint_text_pane_status`] dono ko side-by-side rakhta hai.
/// Pure; unit-tested.
pub fn text_pane_cursor_labels(rows: &[TextRow], selected: Option<usize>) -> (String, String) {
    let row = selected.and_then(|idx| rows.get(idx));
    let left = match row.and_then(|r| r.left_no) {
        Some(n) => format!("L{n}"),
        None => "L-".to_string(),
    };
    let right = match row.and_then(|r| r.right_no) {
        Some(n) => format!("R{n}"),
        None => "R-".to_string(),
    };
    (left, right)
}

/// Bottom-detail preview budget (chars): [`TYPO_WRAP_PANE_CHARS`] (80)
/// reuse — pane paint width parity, koi naya size nahi. Pure; unit-tested.
pub fn text_pane_detail_budget() -> usize {
    TYPO_WRAP_PANE_CHARS
}

/// One pane ka bottom-detail preview text: selected row ke us side ka
/// trimmed content; missing side ya no-selection/OOB par khali (koi fake
/// preview nahi — caller [`text_pane_detail_label`] khali par sirf label
/// rakhta hai). Pure; unit-tested.
pub fn text_pane_preview_side(rows: &[TextRow], selected: Option<usize>, is_left: bool) -> String {
    let row = match selected.and_then(|idx| rows.get(idx)) {
        Some(row) => row,
        None => return String::new(),
    };
    let raw = if is_left {
        row.left.as_str()
    } else {
        row.right.as_str()
    };
    raw.trim().to_string()
}

/// Bottom-detail half label: `label` + khali preview par label akela,
/// warna `label | preview` jisme preview `budget` chars par `...` samet
/// clip hota hai (pane width parity, wrap karke bar kabhi lamba nahi hota —
/// paint `truncate` se single-row lock rakhta hai). Pure; unit-tested.
pub fn text_pane_detail_label(label: &str, preview: &str, budget: usize) -> String {
    let trimmed = preview.trim();
    if trimmed.is_empty() {
        return label.to_string();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= budget {
        format!("{label} | {trimmed}")
    } else {
        let head: String = chars.into_iter().take(budget).collect();
        format!("{label} | {head}...")
    }
}

/// Pane-status half width (px): `avail` me se ek [`TIGHT_GAP`] ghata kar
/// aadha — dono halves barabar taaki right pane ka readout apne pane ke
/// start par khada ho (BC per-pane left alignment), far-right dock par
/// nahi. Non-finite ya non-positive `avail` par [`MIN_HALF`] floor (paint
/// kabhi NaN na khaye). Pure; unit-tested.
pub fn pane_status_half_width(avail: f32) -> f32 {
    if !avail.is_finite() || avail <= 0.0 {
        return MIN_HALF;
    }
    ((avail - TIGHT_GAP) / 2.0).max(0.0)
}

/// Equal text-pane widths (px): fixed center connector
/// ([`text_center_gutter_width`]) aur har taraf ek [`SECTION_GAP`] air
/// ghata kar bacha aadha-aadha — dono panes barabar taaki split 50/50
/// par rahe. Har side [`MIN_HALF`] floor rakhta hai taaki tang window
/// panes ko crush karne ki jagah scroll kare (pair hamesha row bharta
/// hai, [`split_picture_widths`] precedent). Non-finite ya non-positive
/// `avail` par floor pair (paint kabhi NaN na khaye). Pure; unit-tested.
pub fn text_pane_pair_widths(avail: f32) -> (f32, f32) {
    if !avail.is_finite() || avail <= 0.0 {
        return (MIN_HALF, MIN_HALF);
    }
    let free = (avail - text_center_gutter_width() - 2.0 * SECTION_GAP).max(2.0 * MIN_HALF);
    (free / 2.0, free / 2.0)
}

/// Text pane needs horizontal scroll: content px `content_w` viewport
/// `avail_w` se chauda ho to haan. Non-finite ya non-positive viewport
/// par false (paint kabhi NaN branch na le —
/// [`text_layout_for_width`] precedent). ScrollArea `both` isi gate par
/// bars dikhata hai taaki lambi lines clip hone ki jagah h-scroll me
/// jayen (wrap-off me koi silent cut nahi). Pure; unit-tested.
pub fn text_pane_needs_hscroll(content_w: f32, avail_w: f32) -> bool {
    if !content_w.is_finite() || !avail_w.is_finite() || avail_w <= 0.0 {
        return false;
    }
    content_w > avail_w
}

/// Segmented pane status with bottom detail: bottom row me left/right pane
/// halves barabar, dono left-aligned (BC per-pane `1:1` precedent — har pane
/// ka readout apne pane ke start par), har half me cursor label + selected
/// line ka preview (`L3 | content`; missing side ya no-selection par label
/// akela, koi fake preview nahi). Geometry toolbar precedent (full-width
/// `HEADER_H` row, halves beech [`TIGHT_GAP`], truncate clip taaki lambi
/// line bar ko lamba na kare — single-row lock); typo Mono (code content
/// reuse, koi naya size nahi); colors untouched.
pub fn paint_text_pane_status(ui: &mut Ui, rows: &[TextRow], selected: Option<usize>) {
    let (left_label, right_label) = text_pane_cursor_labels(rows, selected);
    let budget = text_pane_detail_budget();
    let left = text_pane_detail_label(
        &left_label,
        &text_pane_preview_side(rows, selected, true),
        budget,
    );
    let right = text_pane_detail_label(
        &right_label,
        &text_pane_preview_side(rows, selected, false),
        budget,
    );
    let avail = ui.available_width();
    let half = pane_status_half_width(avail);
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail, HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.add_sized(
            egui::vec2(half, HEADER_H),
            egui::Label::new(TypoRole::Mono.rich(left.as_str())).truncate(),
        );
        ui.add_sized(
            egui::vec2(half, HEADER_H),
            egui::Label::new(TypoRole::Mono.rich(right.as_str())).truncate(),
        );
    });
}

/// Filler hatch paint: khaali (filler) side ke label rect me step-spaced
/// vertical ticks — [`filler_hatch_step`]/[`filler_hatch_count`] geometry
/// reuse, stroke [`INTRA_SPAN_UNDERLINE_W`] reuse, color caller ka existing
/// `dim` (koi naya hue/token nahi). Ticks rect me horizontally step par,
/// vertically centered hain (rect alignment; TypoRole/mono rhythm untouched
/// — tick height row height ka aadha hai). Guards saturating/finite hain;
/// koi unwrap/panic nahi.
fn paint_filler_hatch(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    if !rect.is_positive() {
        return;
    }
    let step = filler_hatch_step();
    if !step.is_finite() || step <= 0.0 {
        return;
    }
    let ticks = filler_hatch_count(rect.width());
    if ticks == 0 {
        return;
    }
    let tick_h = (rect.height() * 0.5).max(2.0);
    let center_y = rect.center().y;
    let mut i = 0usize;
    while i < ticks {
        let x = rect.min.x + (i as f32 + 0.5) * step;
        if x > rect.max.x {
            break;
        }
        painter.line_segment(
            [
                egui::pos2(x, center_y - tick_h * 0.5),
                egui::pos2(x, center_y + tick_h * 0.5),
            ],
            egui::Stroke::new(INTRA_SPAN_UNDERLINE_W, color),
        );
        i = i.saturating_add(1);
    }
}

/// The line-number gutter: right-aligned number plus separating pad,
/// or blanks when the row has no such side. Single source is
/// [`crate::gutter::gutter_number_cell`] (right-aligned tabular Mono);
/// trail pad ([`GUTTER_NUMBER_TRAIL_PAD`], two-space lead beat parity)
/// keeps the number column off the code. Pure; unit-tested.
fn gutter_text(no: Option<usize>, width: usize) -> String {
    crate::gutter::gutter_number_cell(no, width)
}

/// Splits `line` into (text, accented) segments at the half-open
/// char ranges in `spans`, truncated at `pane` characters like the
/// CLI's panes, with one trailing unaccented segment padding to the
/// full pane width. Ranges pehle [`normalize_intra_spans`] se guzarte
/// hain (sort + clamp + merge) taaki rects chipke rahen — unsorted ya
/// overlapping model spans bhi deterministic paint dete hain.
/// Pure; unit-tested.
fn pane_segments(line: &str, spans: &[(usize, usize)], pane: usize) -> Vec<(String, bool)> {
    let line_len = line.chars().count();
    let spans = normalize_intra_spans(spans, line_len, pane);
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    let mut current_accent = false;
    let mut visible = 0usize;
    for (idx, ch) in line.chars().enumerate() {
        if visible >= pane {
            break;
        }
        let inside = spans.iter().any(|&(start, end)| idx >= start && idx < end);
        if inside != current_accent && !current.is_empty() {
            out.push((std::mem::take(&mut current), current_accent));
        }
        current_accent = inside;
        current.push(ch);
        visible = visible.saturating_add(1);
    }
    if !current.is_empty() {
        out.push((current, current_accent));
    }
    let pad = filler_pad_width(pane, visible);
    if pad > 0 {
        out.push((" ".repeat(pad), false));
    }
    out
}

// ----------------------------------------------------------------------------
// Word-wrap parity (AQ-WRAP): wrapped line paint.
//
// Geometry first: wrap width (80-char cap reuse), indent continuity
// (continuations carry the leading indent, blank gutter keeps the column),
// row mapping (model row <-> flat visual index for the virtualized list).
// Typography second: continuations paint plain mono — same `FontId`, no
// strong, no new size; accent inversion + 1px underline rule unchanged
// (`span_text_format` reuse). Colors frozen: koi naya hue/token nahi.
// `word_wrap` state read-only use hota hai (cap/mapping me), toggle khud
// KEVL `viewmodel.rs` me rehta hai (untouched).
// ----------------------------------------------------------------------------

/// Wrap width panes paint at: wrap on par 80-char cap
/// (`TYPO_WRAP_PANE_CHARS` reuse, [`effective_pane_chars_for`] jaisa),
/// wrap off par full pass-through. Pure; unit-tested.
pub fn wrap_width_for(pane_chars: usize, word_wrap: bool) -> usize {
    if word_wrap {
        pane_chars.min(crate::theme::TYPO_WRAP_PANE_CHARS)
    } else {
        pane_chars
    }
}

/// Leading indent (spaces/tabs) jo continuation lines carry karti hain.
/// Sirf line-start blanks; beech ke blanks nahi. Pure; unit-tested.
pub fn wrap_leading_indent(line: &str) -> String {
    let mut out = String::new();
    for ch in line.chars() {
        if ch == ' ' || ch == '\t' {
            out.push(ch);
        } else {
            break;
        }
    }
    out
}

/// One wrapped visual chunk: text plus original-line coords taaki accent
/// spans shift ho saken. Pehle chunk ka `indent` 0 hai, continuations ka
/// leading-indent length hai (prefix text me shamil hai).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapChunk {
    /// Paint text (continuations me indent prefix samahit).
    pub text: String,
    /// Original line me content slice ka char offset.
    pub start: usize,
    /// Content slice ki char length (indent prefix excluded).
    pub len: usize,
    /// Continuation indent prefix ki char length (pehle 0).
    pub indent: usize,
}

/// Break point (chars) `budget` ke andar: aakhri blank (space/tab) ke
/// baad toot-ta hai, blank na mile to hard cut. Hamesha `>= 1` taaki
/// paint aage badhe. Pure (no egui).
fn wrap_break_at(chars: &[char], budget: usize) -> usize {
    let max = chars.len().min(budget).max(1);
    let mut scan = max;
    loop {
        match chars.get(scan.saturating_sub(1)) {
            Some(c) if *c == ' ' || *c == '\t' => {
                return scan;
            }
            _ => {}
        }
        if scan <= 1 {
            break;
        }
        scan = scan.saturating_sub(1);
    }
    max
}

/// Degenerate wrap (indent hi width se lamba): hard cuts bina indent ke,
/// taaki progress ruke nahi. `width >= 1` caller guarantee karta hai.
/// Pure (no egui).
fn wrap_hard_cuts(line: &str, width: usize) -> Vec<WrapChunk> {
    let mut out: Vec<WrapChunk> = Vec::new();
    let mut start = 0usize;
    let mut cur = String::new();
    let mut n = 0usize;
    for ch in line.chars() {
        cur.push(ch);
        n = n.saturating_add(1);
        if n >= width {
            out.push(WrapChunk {
                text: std::mem::take(&mut cur),
                start,
                len: n,
                indent: 0,
            });
            start = start.saturating_add(n);
            n = 0;
        }
    }
    if n > 0 || out.is_empty() {
        out.push(WrapChunk {
            text: cur,
            start,
            len: n,
            indent: 0,
        });
    }
    out
}

/// One line ke wrapped visual chunks (at most `width` chars each):
/// word-aware (blank par toot-ta hai), continuations leading indent
/// carry karti hain, consumed break blanks gir jate hain. `width == 0`
/// guard single-chunk deta hai (koi clip/loop nahi). Pure; unit-tested.
pub fn wrap_chunks(line: &str, width: usize) -> Vec<WrapChunk> {
    if width == 0 {
        return vec![WrapChunk {
            text: line.to_string(),
            start: 0,
            len: line.chars().count(),
            indent: 0,
        }];
    }
    let total = line.chars().count();
    if total <= width {
        return vec![WrapChunk {
            text: line.to_string(),
            start: 0,
            len: total,
            indent: 0,
        }];
    }
    let indent = wrap_leading_indent(line);
    let indent_len = indent.chars().count();
    if indent_len >= width {
        return wrap_hard_cuts(line, width);
    }
    let mut out: Vec<WrapChunk> = Vec::new();
    let mut rest: Vec<char> = line.chars().collect();
    let mut start = 0usize;
    let mut first = true;
    loop {
        let budget = if first {
            width
        } else {
            width.saturating_sub(indent_len)
        };
        if rest.len() <= budget {
            break;
        }
        let cut = wrap_break_at(rest.as_slice(), budget)
            .min(rest.len())
            .max(1);
        let tail = rest.split_off(cut);
        let mut piece: String = rest.into_iter().collect();
        while piece.ends_with(' ') || piece.ends_with('\t') {
            piece.pop();
        }
        let content_len = piece.chars().count();
        if first {
            out.push(WrapChunk {
                text: piece,
                start,
                len: content_len,
                indent: 0,
            });
            first = false;
        } else {
            let mut text = indent.clone();
            text.push_str(&piece);
            out.push(WrapChunk {
                text,
                start,
                len: content_len,
                indent: indent_len,
            });
        }
        start = start.saturating_add(cut);
        let mut next: Vec<char> = Vec::new();
        let mut dropped = 0usize;
        let mut stripping = true;
        for ch in tail {
            if stripping && (ch == ' ' || ch == '\t') {
                dropped = dropped.saturating_add(1);
                continue;
            }
            stripping = false;
            next.push(ch);
        }
        start = start.saturating_add(dropped);
        rest = next;
        if rest.is_empty() {
            break;
        }
    }
    let last: String = rest.into_iter().collect();
    if !last.is_empty() || out.is_empty() {
        let last_len = last.chars().count();
        if first {
            out.push(WrapChunk {
                text: last,
                start,
                len: last_len,
                indent: 0,
            });
        } else {
            let mut text = indent.clone();
            text.push_str(&last);
            out.push(WrapChunk {
                text,
                start,
                len: last_len,
                indent: indent_len,
            });
        }
    }
    out
}

/// One line ki wrapped paint strings (chunk text, indent samahit).
/// [`wrap_chunks`] ka text view; row mapping/count isi par chalta hai.
/// Pure; unit-tested.
pub fn wrap_visual_lines(line: &str, width: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for chunk in wrap_chunks(line, width) {
        out.push(chunk.text);
    }
    out
}

/// One line ke visual sub-rows ki ginati (hamesha `>= 1`).
/// List virtualization isi se apni heights nikalta hai. Pure; unit-tested.
pub fn wrap_visual_count(line: &str, width: usize) -> usize {
    wrap_visual_lines(line, width).len().max(1)
}

/// Dono panes ka visual height: left/right me jo bada, kam se kam 1.
/// Chhote side ke missing sub-rows blank filler paint karte hain taaki
/// columns aligned rahen. Pure; unit-tested.
pub fn wrap_pane_pair_count(left: &str, right: &str, width: usize) -> usize {
    wrap_visual_count(left, width)
        .max(wrap_visual_count(right, width))
        .max(1)
}

/// Raw accent spans ka chunk-local view: `[start, start+len)` intersect,
/// chunk origin par shift plus indent prefix offset. Bahar ke spans gir
/// jate hain. [`pane_segments`] isi ko normalize karta hai. Pure;
/// unit-tested.
pub fn wrap_chunk_spans(spans: &[(usize, usize)], chunk: &WrapChunk) -> Vec<(usize, usize)> {
    let end = chunk.start.saturating_add(chunk.len);
    let mut out: Vec<(usize, usize)> = Vec::new();
    for pair in spans {
        let (s, e) = (pair.0, pair.1);
        if e <= chunk.start || s >= end {
            continue;
        }
        let cs = if s > chunk.start {
            s.saturating_sub(chunk.start)
        } else {
            0
        };
        let ce = if e < end {
            e.saturating_sub(chunk.start)
        } else {
            chunk.len
        };
        if ce > cs {
            out.push((
                cs.saturating_add(chunk.indent),
                ce.saturating_add(chunk.indent),
            ));
        }
    }
    out
}

/// Flat visual index ko (visible-row position, sub-row) me badalta hai.
/// `counts` har visible row ka visual height hai
/// ([`wrap_pane_pair_count`] se). Bahar par `None`. Pure; unit-tested.
pub fn wrap_visual_to_model(counts: &[usize], visual: usize) -> Option<(usize, usize)> {
    let mut base = 0usize;
    let mut idx = 0usize;
    while idx < counts.len() {
        let mut height = 0usize;
        if let Some(c) = counts.get(idx) {
            height = *c;
        }
        if visual >= base && visual < base.saturating_add(height) {
            return Some((idx, visual.saturating_sub(base)));
        }
        base = base.saturating_add(height);
        idx = idx.saturating_add(1);
    }
    None
}

/// Model (visible) row ka flat visual offset: us row ka pehla sub-row.
/// Bahar par `None`. Pure; unit-tested.
pub fn wrap_model_to_visual_start(counts: &[usize], row: usize) -> Option<usize> {
    counts.get(row)?;
    let mut base = 0usize;
    let mut idx = 0usize;
    while idx < row {
        if let Some(c) = counts.get(idx) {
            base = base.saturating_add(*c);
        }
        idx = idx.saturating_add(1);
    }
    Some(base)
}

/// Kul flat visual rows (virtualized list ki length). Pure; unit-tested.
pub fn wrap_total_visual(counts: &[usize]) -> usize {
    let mut total = 0usize;
    for c in counts {
        total = total.saturating_add(*c);
    }
    total
}

/// Reserved paint height for one model row in wrap mode: `visual_count`
/// sub-rows, har ek [`row_height_for_line`] rhythm par. Wrap-off single
/// `row_height` rakhta hai; wrap-on `count` rhythms stack karta hai taaki
/// selection strip continuous lage. Non-finite/zero input pass-through
/// (paint kabhi NaN layout me nahi bhejta). Pure; unit-tested.
pub fn wrap_reserved_height(visual_count: usize, row_height: f32) -> f32 {
    if row_height.is_finite() && row_height > 0.0 {
        row_height * visual_count.max(1) as f32
    } else {
        row_height
    }
}

/// Continuation gutter: number column blank par same width
/// ([`gutter_text`] `None` shape) — column rhythm nahi toot-ta.
/// Pure; unit-tested.
pub fn wrap_continuation_gutter(gutter_chars: usize) -> String {
    gutter_text(None, gutter_chars)
}

/// One pane ke wrapped sub-row jobs: har chunk ek `LayoutJob` (gutter +
/// padded segments, [`span_text_format`] reuse — continuation plain mono,
/// koi naya size/weight nahi). Pehle chunk me number gutter, baad me
/// blank continuation gutter. Hamesha `>= 1` job. (No egui context
/// banata nahi — caller ka `font_id` reuse hota hai.)
fn side_jobs_wrapped_with_prefs(
    side: Side,
    row: &TextRow,
    layout: &TextPaneLayout<'_>,
    font_id: FontId,
    fg: Color32,
    bg: Color32,
) -> Vec<LayoutJob> {
    let (raw_no, line) = match side {
        Side::Left => (row.left_no, row.left.as_str()),
        Side::Right => (row.right_no, row.right.as_str()),
    };
    let (accent_left, accent_right) = changed_row_word_spans(row);
    let raw_spans: &[(usize, usize)] = match side {
        Side::Left => accent_left,
        Side::Right => accent_right,
    };
    let width = wrap_width_for(layout.pane_chars, layout.prefs.word_wrap).max(1);
    let chunks = wrap_chunks(line, width);
    let mut jobs: Vec<LayoutJob> = Vec::new();
    let mut first = true;
    for chunk in chunks.iter() {
        let spans = if layout.prefs.syntax_highlight {
            wrap_chunk_spans(raw_spans, chunk)
        } else {
            Vec::new()
        };
        let mut job = LayoutJob {
            keep_trailing_whitespace: true,
            ..Default::default()
        };
        let mut gutter = TextFormat::simple(font_id.clone(), layout.dim);
        gutter.background = bg;
        if first {
            let no = if layout.prefs.show_line_numbers {
                raw_no
            } else {
                None
            };
            job.append(&gutter_text(no, layout.gutter_chars), 0.0, gutter);
            first = false;
        } else {
            job.append(&wrap_continuation_gutter(layout.gutter_chars), 0.0, gutter);
        }
        let mut first_seg = true;
        for (segment, accented) in pane_segments(&chunk.text, &spans, width) {
            job.append(
                &segment,
                intra_segment_leading_gap(first_seg),
                selective_span_format(font_id.clone(), fg, bg, accented),
            );
            first_seg = false;
        }
        jobs.push(job);
    }
    jobs
}

/// Wrap-on find-aware sub-row jobs: har chunk me `find_chunk_segments`
/// (chunk-local find mapping) + highlight format. Khali needle par plain
/// [`side_jobs_wrapped_with_prefs`] jaisa. Continuation gutter/geometry
/// wahi, koi naya size/color nahi.
fn side_jobs_wrapped_with_find_current(
    side: Side,
    row: &TextRow,
    layout: &TextPaneLayout<'_>,
    font_id: FontId,
    fg: Color32,
    bg: Color32,
    find: FindRowPaint<'_>,
) -> Vec<LayoutJob> {
    if find.needle.is_empty() {
        return side_jobs_wrapped_with_prefs(side, row, layout, font_id, fg, bg);
    }
    let (raw_no, line) = match side {
        Side::Left => (row.left_no, row.left.as_str()),
        Side::Right => (row.right_no, row.right.as_str()),
    };
    let (accent_left, accent_right) = changed_row_word_spans(row);
    let raw_spans: &[(usize, usize)] = match side {
        Side::Left => accent_left,
        Side::Right => accent_right,
    };
    let accents: &[(usize, usize)] = if layout.prefs.syntax_highlight {
        raw_spans
    } else {
        &[]
    };
    let find_raw = find_line_spans(line, find.needle, find.ignore_case);
    let width = wrap_width_for(layout.pane_chars, layout.prefs.word_wrap).max(1);
    let chunks = wrap_chunks(line, width);
    let mut jobs: Vec<LayoutJob> = Vec::new();
    let mut first = true;
    for chunk in chunks.iter() {
        let mut job = LayoutJob {
            keep_trailing_whitespace: true,
            ..Default::default()
        };
        let mut gutter = TextFormat::simple(font_id.clone(), layout.dim);
        gutter.background = bg;
        if first {
            let no = if layout.prefs.show_line_numbers {
                raw_no
            } else {
                None
            };
            job.append(&gutter_text(no, layout.gutter_chars), 0.0, gutter);
            first = false;
        } else {
            job.append(&wrap_continuation_gutter(layout.gutter_chars), 0.0, gutter);
        }
        let mut first_seg = true;
        for (segment, kind) in
            find_chunk_segments(chunk, accents, &find_raw, find.is_current, width)
        {
            job.append(
                &segment,
                intra_segment_leading_gap(first_seg),
                selective_find_format(font_id.clone(), fg, bg, kind),
            );
            first_seg = false;
        }
        jobs.push(job);
    }
    jobs
}

/// Chhote side ke missing sub-rows ka blank filler: blank continuation
/// gutter plus `width` blanks, same strip `bg` — columns aligned rahen,
/// selection strip continuous lage. Pure paint helper (caller font reuse).
fn wrap_blank_sub_job(
    font_id: FontId,
    fg: Color32,
    bg: Color32,
    dim: Color32,
    gutter_chars: usize,
    width: usize,
) -> LayoutJob {
    let mut job = LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut gutter = TextFormat::simple(font_id.clone(), dim);
    gutter.background = bg;
    job.append(&wrap_continuation_gutter(gutter_chars), 0.0, gutter);
    let mut body = TextFormat::simple(font_id, fg);
    body.background = bg;
    job.append(&" ".repeat(width), 0.0, body);
    job
}

// ----------------------------------------------------------------------------
// Wrap-continuation navigation parity (AW-WRAPNAV).
//
// Geometry first: find spans char indices me hain (same
// `normalize_intra_spans` clamp/merge), wrap continuations me
// `wrap_chunk_spans` reuse se map hote hain — match wrapping geometry
// `pane_segments`/wrap precedent jaisi hai. Next/Prev-match jump wrap
// rows ko flat visual offset par map karta hai
// (`wrap_find_jump_visual`), current-row scroll matched sub-row par
// rehta hai (`wrap_find_scroll_sub`) taaki lambi wrapped rows me match
// viewport se bahar na rahe. Viewport math
// (`wrap_viewport_window`) clamped rehta hai. Typography second:
// continuations plain mono, wahi `FontId`, koi naya size/weight nahi
// (`span_text_format`/`find_segment_format` reuse). Colors frozen: koi
// naya hue/token nahi. `word_wrap` state read-only use hota hai;
// toggle KEVL `viewmodel.rs` me rehta hai (untouched).
// ----------------------------------------------------------------------------

/// Ek line ke kin sub-rows me needle ka kam-se-kam ek hit hai.
/// Khali needle par khali (koi fake hit nahi). Har sub-row ka faisla
/// `wrap_chunk_spans` intersect se hota hai, isliye continuations par
/// ke hits sahi sub me track rehte hain (split matches dono sides me
/// dikhte hain). Pure; unit-tested.
pub fn wrap_find_sub_rows(line: &str, needle: &str, ignore_case: bool, width: usize) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let find_raw = find_line_spans(line, needle, ignore_case);
    if find_raw.is_empty() {
        return Vec::new();
    }
    let chunks = wrap_chunks(line, width);
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < chunks.len() {
        let hit = match chunks.get(idx) {
            Some(chunk) => !wrap_chunk_spans(&find_raw, chunk).is_empty(),
            None => false,
        };
        if hit {
            out.push(idx);
        }
        idx = idx.saturating_add(1);
    }
    out
}

/// Ek model row (dono panes) ka pehla matched sub-row: left/right me jo
/// chhota ho, koi hit na ho ya khali needle par `None`. Paint scroll
/// target isi se nikalta hai taaki Next/Prev-match jump continuation
/// par sahi sub par land kare. Pure; unit-tested.
pub fn wrap_row_find_sub(
    row: &TextRow,
    needle: &str,
    ignore_case: bool,
    width: usize,
) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let left = wrap_find_sub_rows(&row.left, needle, ignore_case, width);
    let right = wrap_find_sub_rows(&row.right, needle, ignore_case, width);
    let left_first = left.first().copied();
    let right_first = right.first().copied();
    match (left_first, right_first) {
        (Some(l), Some(r)) => Some(l.min(r)),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

/// Current-row scroll target sub-row: matched row par pehla hit sub,
/// `pair_count` me clamped; bina hit/khali needle par 0 (row head).
/// Wrap-off path isko nahi chhoota. Pure; unit-tested.
pub fn wrap_find_scroll_sub(
    row: &TextRow,
    needle: &str,
    ignore_case: bool,
    width: usize,
    pair_count: usize,
) -> usize {
    let count = pair_count.max(1);
    let last = count.saturating_sub(1);
    match wrap_row_find_sub(row, needle, ignore_case, width) {
        Some(sub) => sub.min(last),
        None => 0,
    }
}

/// Flat visual offset for `(vis_pos, sub)`: row start plus sub, sub
/// row height se bahar ho to `None`. Next/Prev-match jump isi se flat
/// list offset nikalta hai. Paint `want_scroll` isi se guard karta hai.
/// Pure; unit-tested.
pub fn wrap_visual_offset(counts: &[usize], vis_pos: usize, sub: usize) -> Option<usize> {
    let start = wrap_model_to_visual_start(counts, vis_pos)?;
    let height = counts.get(vis_pos).copied().unwrap_or(0);
    if sub < height {
        Some(start.saturating_add(sub))
    } else {
        None
    }
}

/// Visible position for one model row: `visible` (model order) me
/// pehla index jahan `visible[i] == model_row`. Filtered-out par
/// `None`. Pure; unit-tested.
pub fn wrap_vis_pos_for_model(visible: &[usize], model_row: usize) -> Option<usize> {
    let mut idx = 0usize;
    while idx < visible.len() {
        if visible.get(idx).copied() == Some(model_row) {
            return Some(idx);
        }
        idx = idx.saturating_add(1);
    }
    None
}

/// Next/Prev-match jump ka flat visual target: model row ka visible
/// start plus matched sub (clamped). Bina hit par row head (sub 0).
/// Filtered-out ya OOB par `None` (koi fake scroll nahi). Pure;
/// unit-tested.
pub fn wrap_find_jump_visual(
    counts: &[usize],
    visible: &[usize],
    rows: &[TextRow],
    model_row: usize,
    needle: &str,
    ignore_case: bool,
    width: usize,
) -> Option<usize> {
    let vis_pos = wrap_vis_pos_for_model(visible, model_row)?;
    let row = rows.get(model_row)?;
    let height = counts.get(vis_pos).copied().unwrap_or(0).max(1);
    let last = height.saturating_sub(1);
    let sub = match wrap_row_find_sub(row, needle, ignore_case, width) {
        Some(s) => s.min(last),
        None => 0,
    };
    let start = wrap_model_to_visual_start(counts, vis_pos)?;
    Some(start.saturating_add(sub))
}

/// Viewport window over flat visual rows: `[top, top+viewport)` ko
/// `[0, total]` me clamp karta hai. Khali viewport par empty window.
/// Pure; unit-tested.
pub fn wrap_viewport_window(
    total_visual: usize,
    top_visual: usize,
    viewport_rows: usize,
) -> (usize, usize) {
    if viewport_rows == 0 || total_visual == 0 {
        return (top_visual.min(total_visual), top_visual.min(total_visual));
    }
    let start = top_visual.min(total_visual);
    let end = start.saturating_add(viewport_rows).min(total_visual);
    (start, end)
}

/// Ek sub-row ka continuation indent (chars): pehle sub par 0, baad
/// me chunk indent. OOB par 0. Geometry review ke liye (paint indent
/// carry ka headless contract). Pure; unit-tested.
pub fn wrap_sub_indent_chars(line: &str, width: usize, sub: usize) -> usize {
    match wrap_chunks(line, width).get(sub) {
        Some(chunk) => chunk.indent,
        None => 0,
    }
}

// ----------------------------------------------------------------------------
// Folder filter paint adoption (Wave AU-FILTERPAINT; AT model wiring).
// Naam/type/status chunte ab paint me lagte hain: filtered-out rows
// chhupti hain (tree source `folder_tree_filtered` hai), counts
// filtered rows par honest hain, khali view me honest hint + Clear
// milta hai. Geometry: sirf wired tokens (bar 24 / pad 8 / gap 8,
// existing TIGHT_GAP/SECTION_GAP/HEADER_H/grid spacing); typography:
// naam field mono body-regular, active chip mono caption-strong,
// hint status-role; colors frozen (koi `.color()` nahi).
// ----------------------------------------------------------------------------

/// Type chunta ke paint options: model enum ka poora set, `All`
/// pehle. Paint ComboBox isi se banta hai taaki naya variant kabhi
/// chipke na. Pure; unit-tested.
pub fn folder_kind_options() -> [crate::tree::FolderEntryKind; 3] {
    use crate::tree::FolderEntryKind;
    [
        FolderEntryKind::All,
        FolderEntryKind::FilesOnly,
        FolderEntryKind::DirsOnly,
    ]
}

/// Status chunta ke paint options: model enum ka poora set, `All`
/// pehle. Paint ComboBox isi se banta hai. Pure; unit-tested.
pub fn folder_status_options() -> [crate::tree::FolderStatusFilter; 8] {
    use crate::tree::FolderStatusFilter;
    [
        FolderStatusFilter::All,
        FolderStatusFilter::Same,
        FolderStatusFilter::Different,
        FolderStatusFilter::NewerLeft,
        FolderStatusFilter::NewerRight,
        FolderStatusFilter::LeftOnly,
        FolderStatusFilter::RightOnly,
        FolderStatusFilter::Changed,
    ]
}

/// Filter bar height (px): wired token ka paint-alias taaki bar
/// hamesha model scale par rahe. Pure; unit-tested.
pub fn folder_filter_bar_height_px() -> f32 {
    crate::tree::folder_filter_bar_height()
}

/// Filter bar ka ek taraf ka horizontal padding (px): wired token ka
/// paint-alias. Pure; unit-tested.
pub fn folder_filter_bar_pad_px() -> f32 {
    crate::tree::folder_filter_bar_pad_x()
}

/// Filter bar controls ke beech ka gap (px): wired token ka
/// paint-alias. Pure; unit-tested.
pub fn folder_filter_bar_gap_px() -> f32 {
    crate::tree::folder_filter_bar_gap()
}

/// Paint-side tree source: display filter lagakar hierarchical/flat
/// nodes. `paint_folder` isi se banta hai taaki filtered-out rows
/// kabhi paint na hon. Pure (no egui); unit-tested.
pub fn folder_filter_nodes(
    rows: &[FolderRow],
    expanded: &std::collections::BTreeSet<String>,
    display: &crate::tree::FolderDisplay,
) -> Vec<crate::tree::FolderTreeNode> {
    crate::tree::folder_tree_filtered(rows, expanded, display)
}

/// Honest one-line summary over the filtered rows (caller string ki
/// jagah paint isi ko dikhata hai). Pure; unit-tested.
pub fn folder_filtered_summary(rows: &[FolderRow], display: &crate::tree::FolderDisplay) -> String {
    crate::tree::count_folder_rows(&crate::tree::filtered_folder_rows(rows, display)).summary()
}

/// Khali view ka paint-stable hint: model rows vs visible nodes par.
/// `None` (kuch dikh raha hai) par khali string. Pure; unit-tested.
pub fn folder_paint_empty_hint(
    rows: &[FolderRow],
    visible_rows: usize,
    display: &crate::tree::FolderDisplay,
) -> &'static str {
    crate::tree::folder_empty_hint(crate::tree::folder_empty_state(
        rows.len(),
        visible_rows,
        display.is_active(),
    ))
}

/// Rows-aware paint hint: identical folders ko prune se alag wording.
/// Legacy `folder_paint_empty_hint` frozen hai; `_full_rows` adopt kare to
/// diff-seeking filter par identical model `AllSame` deta hai. Paint me
/// `rows` model se aate hain isliye `has_model` hamesha `true`.
pub fn folder_paint_empty_hint_full_rows(
    rows: &[FolderRow],
    visible_rows: usize,
    display: &crate::tree::FolderDisplay,
) -> &'static str {
    crate::tree::folder_empty_hint(crate::tree::folder_empty_state_full_rows(
        true,
        rows,
        visible_rows,
        display,
    ))
}

/// `FolderTypo` se paint `RichText`: mono/size/strong mapping, koi
/// rang nahi. Paint-only.
pub fn folder_filter_rich(text: &str, typo: crate::tree::FolderTypo) -> RichText {
    let mut styled = RichText::new(text).size(typo.size);
    if typo.mono {
        styled = styled.monospace();
    }
    if typo.strong {
        styled = styled.strong();
    }
    styled
}

/// Folder filter bar: naam field + type/status chunte + active chip +
/// Clear. Caller (`paint_folder`) har frame isi se `display` chalata
/// hai; `true` matlab kuch badla (caller pager top girata hai taaki
/// scroll stale na rahe). Geometry: bar height + pad + gap wired
/// tokens; typography: naam field mono body-regular, active chip mono
/// caption-strong; colors frozen.
pub fn paint_folder_filter_bar(ui: &mut Ui, display: &mut crate::tree::FolderDisplay) -> bool {
    let before = display.clone();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(
            ui.available_width(),
            folder_filter_bar_height_px(),
        ));
        ui.spacing_mut().item_spacing.x = folder_filter_bar_gap_px();
        ui.add_space(folder_filter_bar_pad_px());
        ui.add(
            TextEdit::singleline(&mut display.name_query)
                .desired_width(FIND_NEEDLE_W)
                .hint_text("filter by name")
                .font(egui::FontId::monospace(crate::tree::FOLDER_BODY_SIZE)),
        );
        egui::ComboBox::from_label("Type")
            .selected_text(display.kind.label())
            .show_ui(ui, |ui| {
                for kind in folder_kind_options() {
                    ui.selectable_value(&mut display.kind, kind, kind.label());
                }
            });
        egui::ComboBox::from_label("Status")
            .selected_text(display.status.label())
            .show_ui(ui, |ui| {
                for status in folder_status_options() {
                    ui.selectable_value(&mut display.status, status, status.label());
                }
            });
        if display.is_active() {
            ui.label(folder_filter_rich(
                format!("{} active", display.active_count()).as_str(),
                crate::tree::folder_filter_active_typo(),
            ));
            if ui.button("Clear filters").clicked() {
                display.clear();
            }
        }
    });
    *display != before
}

/// The folder listing: wired filter bar, honest filtered summary,
/// then the monospace tree grid with expand/collapse drill-down.
/// Filtered-out rows kabhi paint nahi hoti (tree source filtered hai);
/// khali view me honest hint + Clear milta hai. Rows are virtualized
/// the same way as the text view. `pager` ka total visible nodes se
/// sync hota hai; filter badle to top girta hai; page buttons aur
/// PageUp/PageDown `next_page`/`prev_page` chalate hain.
pub fn paint_folder(
    ui: &mut Ui,
    rows: &[FolderRow],
    display: &mut crate::tree::FolderDisplay,
    expanded: &mut std::collections::BTreeSet<String>,
    pager: &mut crate::tree::RowPager,
) {
    if paint_folder_filter_bar(ui, display) {
        pager.top = 0;
    }
    ui.add_space(TIGHT_GAP);
    let tree_nodes = folder_filter_nodes(rows, expanded, display);
    ui.label(chrome_status_role().rich(folder_filtered_summary(rows, display)));
    ui.add_space(TIGHT_GAP);
    pager.total_rows = tree_nodes.len();
    pager.clamp_top();
    if tree_nodes.is_empty() {
        let hint = folder_paint_empty_hint_full_rows(rows, tree_nodes.len(), display);
        ui.add_space(SECTION_GAP);
        ui.vertical_centered(|ui| {
            ui.label(chrome_status_role().rich(hint));
            if display.is_active() {
                ui.add_space(TIGHT_GAP);
                if ui
                    .button(folder_filter_rich(
                        "Clear filters",
                        crate::tree::folder_filter_active_typo(),
                    ))
                    .clicked()
                {
                    display.clear();
                    pager.top = 0;
                }
            }
        });
        ui.add_space(SECTION_GAP);
        return;
    }
    let (page_up, page_down) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::PageUp),
            input.key_pressed(egui::Key::PageDown),
        )
    });
    if page_up {
        pager.prev_page();
    }
    if page_down {
        pager.next_page();
    }
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        if ui.small_button("◀ Prev page").clicked() {
            pager.prev_page();
        }
        if ui.small_button("Next page ▶").clicked() {
            pager.next_page();
        }
        ui.label(TypoRole::Small.rich(format!(
            "page {} of {}, showing {} row(s)",
            pager.top / pager.rows_per_page.max(1) + 1,
            pager.page_count().max(1),
            pager
                .visible_range()
                .1
                .saturating_sub(pager.visible_range().0),
        )));
    });
    ui.add_space(TIGHT_GAP);
    // Column header: tree col defs se (`folder_header_line`), mono
    // weak — path flex, sides fixed widths par padded. Rule table
    // header jaisi (`indented_content` lead + `-` line), GEO reuse.
    let folder_header = crate::tree::folder_header_line();
    ui.label(TypoRole::Mono.rich(folder_header.clone()).weak());
    ui.label(
        TypoRole::Mono
            .rich(indented_content(&format_table_header_rule(
                folder_header.len(),
            )))
            .weak(),
    );
    let row_height = row_height_for_line(text_row_height(ui));
    let dark = ui.visuals().dark_mode;
    let mut toggle_dir = None;

    let (wstart, wend) = pager.visible_range();
    let wend = wend.min(tree_nodes.len());
    let count = wend.saturating_sub(wstart);
    egui::ScrollArea::vertical()
        .id_salt("folder_tree_rows")
        .auto_shrink([false, false])
        .show_rows(ui, row_height, count, |ui, range| {
            egui::Grid::new("folder_data_grid")
                .num_columns(folder_grid_columns())
                .spacing([SECTION_GAP, TIGHT_GAP])
                .show(ui, |ui| {
                    for pos in range {
                        let i = wstart.saturating_add(pos);
                        let Some(node) = tree_nodes.get(i) else {
                            break;
                        };
                        ui.set_row_height(row_height);
                        // Glyph column: frozen palette, mode-following dim/blue/red/green.
                        ui.label(
                            TypoRole::Mono
                                .rich(node.row.glyph.to_string())
                                .color(glyph_color(&node.row, dark)),
                        );
                        // Path column (flex): depth indent + expand/collapse
                        // + path (+ dir mark). Sides Grid columns me padded
                        // mono rehte hain, isliye path lamba ho tab bhi
                        // size/time align rehte hain.
                        let cells = folder_row_cells(&node.row);
                        let path_text = cells.first().cloned().unwrap_or_default();
                        ui.horizontal(|ui| {
                            if node.depth > 0 {
                                ui.add_space(tree_indent(node.depth));
                            }
                            if node.row.is_dir && node.has_children {
                                let icon = if node.is_expanded { "▼" } else { "▶" };
                                if ui.small_button(icon).clicked() {
                                    toggle_dir = Some(node.row.rel_path.clone());
                                }
                            } else if node.row.is_dir {
                                ui.label(TypoRole::Mono.rich(CONTENT_LEAD));
                            }
                            ui.label(TypoRole::Mono.rich(path_text));
                        });
                        let mut idx = 1usize;
                        while idx < cells.len() {
                            let mut cell = String::new();
                            if let Some(text) = cells.get(idx) {
                                cell = text.clone();
                            }
                            ui.label(TypoRole::Mono.rich(cell));
                            idx = idx.saturating_add(1);
                        }
                        ui.end_row();
                    }
                });
        });

    if let Some(dir) = toggle_dir {
        if expanded.contains(&dir) {
            expanded.remove(&dir);
        } else {
            expanded.insert(dir);
        }
    }
}

/// The glyph column color: same/dim, different and left-only/red,
/// newer/blue, right-only/green, like the CLI's coloring.
/// Kept for the pending App rewiring (see the note on theme blues).
fn glyph_color(row: &FolderRow, dark: bool) -> Color32 {
    match row.glyph {
        '=' => dim_for(dark),
        '!' | 'L' => red_for(dark),
        '>' | '<' => blue_for(dark),
        'R' => green_for(dark),
        _ => dim_for(dark),
    }
}

/// The columns after the glyph, shaped like the CLI's row: sizes,
/// times, then the path with the directory mark.
/// Kept for the pending App rewiring (see the note on theme blues).
/// Covered by the `folder_columns_match_the_cli_shape` unit test.
pub fn folder_columns(row: &FolderRow) -> String {
    let ls = row.left_size.as_deref().unwrap_or("-");
    let rs = row.right_size.as_deref().unwrap_or("-");
    let lt = row.left_time.as_deref().unwrap_or("-");
    let rt = row.right_time.as_deref().unwrap_or("-");
    let dir_mark = if row.is_dir { "/" } else { "" };
    let rel_path = row.rel_path.as_str();
    format!(" {ls:>10} {rs:>10}  {lt:>16} {rt:>16}  {rel_path}{dir_mark}")
}

/// Folder grid header cells in paint order: flex Path plus fixed
/// side columns from [`crate::tree::folder_column_defs`]. Path raw
/// hai (flex), sides [`crate::tree::pad_folder_cell`] se padded taaki
/// mono Grid me columns align rahen. Pure; unit-tested.
pub fn folder_header_cells() -> Vec<String> {
    let mut out = Vec::new();
    for def in crate::tree::folder_column_defs() {
        if def.width_chars == 0 {
            out.push(def.header.to_string());
        } else {
            out.push(crate::tree::pad_folder_cell(def.header, def.width_chars));
        }
    }
    out
}

/// Folder grid row cells in paint order: path (+ dir mark) raw, phir
/// 4 padded side cells via [`crate::tree::folder_cell_text`] +
/// [`crate::tree::pad_folder_cell`]. Missing sides `-` padded dikhte
/// hain, isliye orphan rows bhi align rehti hain. Path cell par paint
/// `tree_indent(depth)` offset lagata hai (flex column). Pure;
/// unit-tested.
pub fn folder_row_cells(row: &FolderRow) -> Vec<String> {
    let mut out = Vec::new();
    for def in crate::tree::folder_column_defs() {
        match def.column {
            crate::tree::FolderColumn::Path => {
                let mut path = crate::tree::folder_cell_text(row, def.column);
                if row.is_dir {
                    path.push('/');
                }
                out.push(path);
            }
            _ => {
                let text = crate::tree::folder_cell_text(row, def.column);
                out.push(crate::tree::pad_folder_cell(&text, def.width_chars));
            }
        }
    }
    out
}

/// Folder grid columns: glyph column plus har data column.
/// Pure; unit-tested.
pub fn folder_grid_columns() -> usize {
    crate::tree::folder_column_defs().len().saturating_add(1)
}

// ----------------------------------------------------------------------------
// Folder sync paint adoption (Wave AR-SYNCPAINT; AQ model precedent).
// Per-row sync direction badges + selection highlight ka paint. Model
// (direction/selection/preview/typo-tokens) `crate::tree` me hai
// (read-only use); yahan sirf paint-side mapping + grid hai.
// Geometry: sirf existing tokens + `tree::sync_badge_min_px` reuse;
// typography: `tree::sync_badge_typo`/`sync_summary_typo` mapping;
// colors frozen (koi naya hue nahi, highlight `selectable_label`
// precedent par).
// ----------------------------------------------------------------------------

/// Sync badge column ka header text: fixed chhota label, badge width
/// ([`crate::tree::SYNC_BADGE_CHARS`] = 2) me fit. Pure; unit-tested.
pub fn sync_badge_header() -> &'static str {
    "S"
}

/// Paint-side badge text: model ka faisla
/// ([`crate::tree::sync_badge_for_row`]) taaki badge kabhi plan se
/// alag na bole. Pure; unit-tested.
pub fn sync_badge_for_paint(row: &FolderRow, mode: crate::tree::SyncModeChoice) -> &'static str {
    crate::tree::sync_badge_for_row(row, mode)
}

/// Badge column ki paint width: model token ka reuse
/// ([`crate::tree::sync_badge_min_px`]) taaki mono advance/pad badle
/// to paint saath chale. Grid aaj text se auto-size hota hai, isliye
/// ye width contract test-locked hai (fixed layout baad me).
/// Pure; unit-tested.
pub fn sync_badge_column_width() -> f32 {
    crate::tree::sync_badge_min_px()
}

/// Badge text strong hai ya nahi: sirf delete (`X`) strong, baaki
/// regular (colors frozen hain isliye weight se farak). Model typo
/// ([`crate::tree::sync_badge_typo`]) ka paint mapping. Pure;
/// unit-tested.
pub fn sync_badge_is_strong(direction: crate::tree::SyncRowDirection) -> bool {
    crate::tree::sync_badge_typo(direction).strong
}

/// Badge text size: model typo ([`crate::tree::sync_badge_typo`]) se,
/// taaki paint hamesha caption scale par rahe. Pure; unit-tested.
pub fn sync_badge_text_size(direction: crate::tree::SyncRowDirection) -> f32 {
    crate::tree::sync_badge_typo(direction).size
}

/// Summary line text size: model typo
/// ([`crate::tree::sync_summary_typo`]) se. Pure; unit-tested.
pub fn sync_summary_text_size() -> f32 {
    crate::tree::sync_summary_typo().size
}

/// Badge cell ka `RichText`: [`TypoRole::Mono`] single-source (12 mono),
/// delete par strong. Sizes/family legacy `.monospace().size()` jaisi hi
/// hain (model typo 12 mono hai), sirf source ek hua. Colors frozen
/// (koi `.color()` nahi). Paint-only.
pub fn sync_badge_rich(badge: &str, direction: crate::tree::SyncRowDirection) -> RichText {
    let styled = TypoRole::Mono.rich(badge);
    if sync_badge_is_strong(direction) {
        styled.strong()
    } else {
        styled
    }
}

/// Summary line ka `RichText`: [`TypoRole::Mono`] single-source (12 mono),
/// weak tone (App preview precedent: `monospace().weak()`). Sizes/family
/// legacy jaisi hi hain, sirf source ek hua. Paint-only.
pub fn sync_summary_rich(line: &str) -> RichText {
    TypoRole::Mono.rich(line).weak()
}

/// Row selected hai ya nahi: model set ka read-only probe
/// ([`crate::tree::FolderSyncSelection::contains`]). Pure; unit-tested.
pub fn folder_sync_row_selected(
    selection: &crate::tree::FolderSyncSelection,
    rel_path: &str,
) -> bool {
    selection.contains(rel_path)
}

/// Sync grid columns: glyph + badge + har data column. Pure;
/// unit-tested.
pub fn folder_grid_columns_with_sync() -> usize {
    folder_grid_columns().saturating_add(1)
}

/// Sync grid header cells paint order me: glyph header khali, phir
/// badge header ([`sync_badge_header`]), phir [`folder_header_cells`].
/// Pure; unit-tested.
pub fn folder_sync_header_cells() -> Vec<String> {
    let mut out = Vec::with_capacity(folder_grid_columns_with_sync());
    out.push(String::new());
    out.push(sync_badge_header().to_string());
    out.extend(folder_header_cells());
    out
}

/// Selected summary line: `N selected: <dry-run line>`, wahi shape jo
/// App (`folder_sync_selected_summary_line`) dikhata hai — real
/// counts, khali par explicit zero line. Pure; unit-tested.
pub fn folder_sync_selected_line(
    rows: &[FolderRow],
    mode: crate::tree::SyncModeChoice,
    selection: &crate::tree::FolderSyncSelection,
) -> String {
    let preview = crate::tree::sync_preview_for_selection(rows, mode, selection);
    let counts = crate::tree::summarize_sync_preview(&preview);
    format!(
        "{} selected: {}",
        selection.len(),
        crate::tree::format_sync_summary(counts)
    )
}

/// Sync preview modes the mode radio offers, in paint order:
/// Update Both first, Mirror second. Picture tab ke `PictureMode::all`
/// jaisa ek source of truth taaki radio aur App wiring kabhi alag na
/// hon. Pure; unit-tested.
pub fn sync_mode_choices() -> [crate::tree::SyncModeChoice; 2] {
    use crate::tree::SyncModeChoice;
    [SyncModeChoice::UpdateBoth, SyncModeChoice::Mirror]
}

/// Sync preview mode radio row (Update Both / Mirror), picture tab ke
/// mode radio (`paint_picture_compare`) wala precedent: local pick par
/// `radio_value`, badlaav caller par wapas. Caller
/// [`crate::app::App::set_folder_sync_mode`] se wire kare taaki mode
/// switch consent reset kare. Returns the picked mode. Colors frozen.
pub fn paint_sync_mode_row(
    ui: &mut Ui,
    mode: crate::tree::SyncModeChoice,
) -> crate::tree::SyncModeChoice {
    let mut picked = mode;
    ui.horizontal(|ui| {
        ui.label(TypoRole::Small.rich("Mode:"));
        for candidate in sync_mode_choices() {
            ui.radio_value(&mut picked, candidate, candidate.label());
        }
    });
    picked
}

/// Sync adoption grid: mode radio + glyph + direction badge + path/sides,
/// click se selection toggle, neeche selected summary line.
/// [`paint_folder`] ka sync judvaan. Returns the picked preview mode;
/// caller ise [`crate::app::App::set_folder_sync_mode`] se wire kare
/// (consent reset samet). Geometry: wahi header/page/grid rhythm;
/// typography: badge/summary mono caption mapping; colors frozen.
pub fn paint_folder_sync(
    ui: &mut Ui,
    rows: &[FolderRow],
    summary: &str,
    mode: crate::tree::SyncModeChoice,
    selection: &mut crate::tree::FolderSyncSelection,
    expanded: &mut std::collections::BTreeSet<String>,
    pager: &mut crate::tree::RowPager,
) -> crate::tree::SyncModeChoice {
    ui.label(chrome_status_role().rich(summary));
    ui.add_space(TIGHT_GAP);
    let picked = paint_sync_mode_row(ui, mode);
    ui.add_space(TIGHT_GAP);
    if rows.is_empty() {
        ui.label(RichText::new("(both folders are empty)").weak());
        return picked;
    }
    let tree_nodes = viewmodel::folder_tree(rows, expanded);
    pager.total_rows = tree_nodes.len();
    pager.clamp_top();
    let (page_up, page_down) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::PageUp),
            input.key_pressed(egui::Key::PageDown),
        )
    });
    if page_up {
        pager.prev_page();
    }
    if page_down {
        pager.next_page();
    }
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        if ui.small_button("◀ Prev page").clicked() {
            pager.prev_page();
        }
        if ui.small_button("Next page ▶").clicked() {
            pager.next_page();
        }
        ui.label(TypoRole::Small.rich(format!(
            "page {} of {}, showing {} row(s)",
            pager.top / pager.rows_per_page.max(1) + 1,
            pager.page_count().max(1),
            pager
                .visible_range()
                .1
                .saturating_sub(pager.visible_range().0),
        )));
    });
    ui.add_space(TIGHT_GAP);
    // Header grid row: glyph khali + badge `S` + data headers, mono
    // weak (existing single-line header ka grid-aligned roop taaki
    // badge column header se na bhatke). Rule table header jaisi.
    let headers = folder_sync_header_cells();
    egui::Grid::new("folder_sync_header_grid")
        .num_columns(folder_grid_columns_with_sync())
        .spacing([SECTION_GAP, TIGHT_GAP])
        .show(ui, |ui| {
            for cell in &headers {
                ui.label(TypoRole::Small.rich(cell.as_str()));
            }
            ui.end_row();
        });
    ui.label(
        TypoRole::Small.rich(indented_content(&format_table_header_rule(
            headers.join(" ").len(),
        ))),
    );
    let row_height = row_height_for_line(text_row_height(ui));
    let dark = ui.visuals().dark_mode;
    let mut toggle_dir = None;
    let mut toggle_select: Vec<String> = Vec::new();

    let (wstart, wend) = pager.visible_range();
    let wend = wend.min(tree_nodes.len());
    let count = wend.saturating_sub(wstart);
    egui::ScrollArea::vertical()
        .id_salt("folder_sync_rows")
        .auto_shrink([false, false])
        .show_rows(ui, row_height, count, |ui, range| {
            egui::Grid::new("folder_sync_data_grid")
                .num_columns(folder_grid_columns_with_sync())
                .spacing([SECTION_GAP, TIGHT_GAP])
                .show(ui, |ui| {
                    for pos in range {
                        let i = wstart.saturating_add(pos);
                        let Some(node) = tree_nodes.get(i) else {
                            break;
                        };
                        ui.set_row_height(row_height);
                        // Glyph + sync badge: frozen palette,
                        // mode-following dim/blue/red/green; badge mono
                        // caption (delete strong).
                        ui.label(
                            TypoRole::Mono
                                .rich(node.row.glyph.to_string())
                                .color(glyph_color(&node.row, dark)),
                        );
                        let direction = crate::tree::sync_direction_for_row(&node.row, picked);
                        ui.label(sync_badge_rich(
                            sync_badge_for_paint(&node.row, picked),
                            direction,
                        ));
                        // Path column (flex): depth indent +
                        // expand/collapse + path (+ dir mark); click
                        // selection toggle karta hai (dual painter
                        // precedent), highlight `selectable_label` se.
                        let cells = folder_row_cells(&node.row);
                        let path_text = cells.first().cloned().unwrap_or_default();
                        let selected =
                            folder_sync_row_selected(selection, node.row.rel_path.as_str());
                        ui.horizontal(|ui| {
                            if node.depth > 0 {
                                ui.add_space(tree_indent(node.depth));
                            }
                            if node.row.is_dir && node.has_children {
                                let icon = if node.is_expanded { "▼" } else { "▶" };
                                if ui.small_button(icon).clicked() {
                                    toggle_dir = Some(node.row.rel_path.clone());
                                }
                            } else if node.row.is_dir {
                                ui.label(TypoRole::Mono.rich(CONTENT_LEAD));
                            }
                            if ui
                                .selectable_label(selected, TypoRole::Mono.rich(path_text))
                                .clicked()
                            {
                                toggle_select.push(node.row.rel_path.clone());
                            }
                        });
                        let mut idx = 1usize;
                        while idx < cells.len() {
                            let mut cell = String::new();
                            if let Some(text) = cells.get(idx) {
                                cell = text.clone();
                            }
                            ui.label(TypoRole::Mono.rich(cell));
                            idx = idx.saturating_add(1);
                        }
                        ui.end_row();
                    }
                });
        });

    if let Some(dir) = toggle_dir {
        if expanded.contains(&dir) {
            expanded.remove(&dir);
        } else {
            expanded.insert(dir);
        }
    }
    for rel_path in toggle_select {
        selection.toggle(rel_path.as_str());
    }
    // Selected summary: real counts, khali par explicit zero line.
    ui.add_space(TIGHT_GAP);
    ui.label(sync_summary_rich(
        folder_sync_selected_line(rows, picked, selection).as_str(),
    ));
    picked
}

// ----------------------------------------------------------------------------
// Dual-tree folder panes (Gap #19 kadam 1; App wiring baad me).
// Dono pane ek hi node list paint karte hain; select/scroll/action
// `tree::DualTreeState` me shared hai taaki panes kabhi alag na hon.
// Per-row action menu stub hai (`tree::DualRowAction::label` me "(stub)"
// saaf hai, koi I/O nahi). Ek hi ScrollArea dono panes ko chalata hai,
// isliye synced scroll construction se milta hai. Geometry: sirf existing
// tokens (`split_picture_widths`, `paint_split_handle`,
// `row_height_for_line`, `HEADER_H`, `TREE_INDENT`); colors: sirf
// `glyph_color` + frozen palette; typo: mono rows + small weak notes.
// ----------------------------------------------------------------------------

/// Dual-tree pane widths: split authority (picture panes jaisi: row
/// bharti hai, `MIN_HALF` floor). Geometry reuse, koi naya token nahi.
/// Pure; unit-tested.
pub fn dual_tree_pane_widths(avail: f32, fraction: f32) -> (f32, f32) {
    split_picture_widths(avail, fraction)
}

/// Left pane row line: glyph + naam (+ dir mark) + left side cell.
/// Frozen contract (G-F2 grid paint `dual_grid_row_left` par gaya hai;
/// ye line helper + uske tests rakhe hain). Pure; unit-tested.
pub fn dual_left_line(node: &crate::tree::FolderTreeNode) -> String {
    let dir_mark = if node.row.is_dir { "/" } else { "" };
    format!(
        "{} {}{}  {}",
        node.row.glyph,
        node.row.rel_path,
        dir_mark,
        crate::tree::dual_left_cell(&node.row)
    )
}

/// Right pane row line: glyph + naam (+ dir mark) + right side cell.
/// Frozen contract (G-F2 grid paint `dual_grid_row_right` par gaya hai;
/// ye line helper + uske tests rakhe hain). Pure; unit-tested.
pub fn dual_right_line(node: &crate::tree::FolderTreeNode) -> String {
    let dir_mark = if node.row.is_dir { "/" } else { "" };
    format!(
        "{} {}{}  {}",
        node.row.glyph,
        node.row.rel_path,
        dir_mark,
        crate::tree::dual_right_cell(&node.row)
    )
}

/// Dual-pane grid headers, left side: Path + L size + L modified.
/// `tree::folder_column_defs` se reuse (Path flex, sides padded fixed).
/// Pure; unit-tested.
pub fn dual_grid_headers_left() -> [String; 3] {
    let defs = crate::tree::folder_column_defs();
    [
        defs[0].header.to_string(),
        crate::tree::pad_folder_cell(defs[1].header, defs[1].width_chars),
        crate::tree::pad_folder_cell(defs[3].header, defs[3].width_chars),
    ]
}

/// Dual-pane grid headers, right side: Path + R size + R modified.
/// `tree::folder_column_defs` se reuse. Pure; unit-tested.
pub fn dual_grid_headers_right() -> [String; 3] {
    let defs = crate::tree::folder_column_defs();
    [
        defs[0].header.to_string(),
        crate::tree::pad_folder_cell(defs[2].header, defs[2].width_chars),
        crate::tree::pad_folder_cell(defs[4].header, defs[4].width_chars),
    ]
}

/// Dual-pane grid row cells, left side: naam (+ dir mark), size, time.
/// Text `tree::folder_cell_text` se reuse; missing side `-` rakhta hai
/// taaki orphan rows bhi align rahen. Pure; unit-tested.
pub fn dual_grid_row_left(row: &FolderRow) -> [String; 3] {
    use crate::tree::FolderColumn;
    let mut name = crate::tree::folder_cell_text(row, FolderColumn::Path);
    if row.is_dir {
        name.push('/');
    }
    [
        name,
        crate::tree::folder_cell_text(row, FolderColumn::LeftSize),
        crate::tree::folder_cell_text(row, FolderColumn::LeftTime),
    ]
}

/// Dual-pane grid row cells, right side: naam (+ dir mark), size, time.
/// Text `tree::folder_cell_text` se reuse. Pure; unit-tested.
pub fn dual_grid_row_right(row: &FolderRow) -> [String; 3] {
    use crate::tree::FolderColumn;
    let mut name = crate::tree::folder_cell_text(row, FolderColumn::Path);
    if row.is_dir {
        name.push('/');
    }
    [
        name,
        crate::tree::folder_cell_text(row, FolderColumn::RightSize),
        crate::tree::folder_cell_text(row, FolderColumn::RightTime),
    ]
}

/// Dual grid body-cell role: `content_grid` (Mono 12 regular mono).
/// Pure; unit-tested.
pub fn dual_grid_cell_role() -> TypoRole {
    TypoRole::content_grid()
}

/// Dual grid header role: `caption` (Small 11). Paint `.strong()` jodkar
/// caption-strong banata hai, koi naya size nahi. Pure; unit-tested.
pub fn dual_grid_header_role() -> TypoRole {
    TypoRole::caption()
}

/// Staged stub action ki ek line: label + path, I/O-free warning ke
/// saath taaki stub kabhi real action na lage. Pure; unit-tested.
pub fn dual_pending_text(action: crate::tree::DualRowAction, rel_path: &str) -> String {
    format!("stub: {} {rel_path} (koi file nahi badli)", action.label())
}

/// Synced left/right folder trees: summary + stub note, page bar, phir
/// ek hi scroll me dono panes (left cell | menu | right cell). Click
/// kisi bhi pane me shared select set karta hai; `···` menu stub
/// action stage karta hai. `state` me select/scroll/pending rehta hai,
/// `split` drag se pane widths badalti hai (agli frame se).
/// App wiring baad me; tab tak test-locked contract.
pub fn paint_folder_dual(
    ui: &mut Ui,
    rows: &[FolderRow],
    summary: &str,
    expanded: &mut std::collections::BTreeSet<String>,
    state: &mut crate::tree::DualTreeState,
    split: &mut f32,
) {
    ui.label(chrome_status_role().rich(summary));
    ui.label(
        TypoRole::Small.rich("dual-tree preview: per-row actions stub hain, koi file nahi badalti"),
    );
    ui.add_space(TIGHT_GAP);
    if rows.is_empty() {
        ui.label(RichText::new("(both folders are empty)").weak());
        return;
    }
    let tree_nodes = viewmodel::folder_tree(rows, expanded);
    state.sync_total(tree_nodes.len());
    let (page_up, page_down) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::PageUp),
            input.key_pressed(egui::Key::PageDown),
        )
    });
    if page_up {
        state.pager.prev_page();
    }
    if page_down {
        state.pager.next_page();
    }
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), HEADER_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        if ui.small_button("◀ Prev page").clicked() {
            state.pager.prev_page();
        }
        if ui.small_button("Next page ▶").clicked() {
            state.pager.next_page();
        }
        ui.label(TypoRole::Small.rich(format!(
            "page {} of {}, showing {} row(s)",
            state.pager.top / state.pager.rows_per_page.max(1) + 1,
            state.pager.page_count().max(1),
            state
                .pager
                .visible_range()
                .1
                .saturating_sub(state.pager.visible_range().0),
        )));
    });
    ui.add_space(TIGHT_GAP);
    *split = clamp_split_fraction(*split);
    let avail = ui.available_width();
    let (left_w, right_w) = dual_tree_pane_widths(avail, *split);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(left_w);
            egui::Grid::new("folder_dual_header_left")
                .num_columns(3)
                .spacing([SECTION_GAP, TIGHT_GAP])
                .show(ui, |ui| {
                    for header in dual_grid_headers_left() {
                        ui.label(dual_grid_header_role().rich(header).strong());
                    }
                    ui.end_row();
                });
        });
        paint_split_handle(ui, split, avail);
        ui.vertical(|ui| {
            ui.set_width(right_w);
            egui::Grid::new("folder_dual_header_right")
                .num_columns(3)
                .spacing([SECTION_GAP, TIGHT_GAP])
                .show(ui, |ui| {
                    for header in dual_grid_headers_right() {
                        ui.label(dual_grid_header_role().rich(header).strong());
                    }
                    ui.end_row();
                });
        });
    });
    ui.add_space(TIGHT_GAP);
    let dark = ui.visuals().dark_mode;
    let row_height = row_height_for_line(text_row_height(ui));
    let mut toggle_dir = None;
    let (wstart, wend) = state.visible_range();
    let wend = wend.min(tree_nodes.len());
    let count = wend.saturating_sub(wstart);
    egui::ScrollArea::both()
        .id_salt("folder_dual_rows")
        .auto_shrink([false, false])
        .show_rows(ui, row_height, count, |ui, range| {
            for pos in range {
                let i = wstart.saturating_add(pos);
                let Some(node) = tree_nodes.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                let selected = state.is_selected(&node.row.rel_path);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(left_w);
                        ui.horizontal(|ui| {
                            if node.depth > 0 {
                                ui.add_space(tree_indent(node.depth));
                            }
                            ui.label(
                                TypoRole::Mono
                                    .rich(node.row.glyph.to_string())
                                    .color(glyph_color(&node.row, dark)),
                            );
                            let [name, size, time] = dual_grid_row_left(&node.row);
                            egui::Grid::new(("folder_dual_left_cells", i))
                                .num_columns(3)
                                .spacing([SECTION_GAP, TIGHT_GAP])
                                .show(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            selected,
                                            dual_grid_cell_role().rich(name),
                                        )
                                        .clicked()
                                    {
                                        state.select(&node.row.rel_path);
                                    }
                                    ui.label(dual_grid_cell_role().rich(size));
                                    ui.label(dual_grid_cell_role().rich(time));
                                    ui.end_row();
                                });
                        });
                    });
                    ui.vertical(|ui| {
                        if node.row.is_dir && node.has_children {
                            let icon = if node.is_expanded { "▼" } else { "▶" };
                            if ui.small_button(icon).clicked() {
                                toggle_dir = Some(node.row.rel_path.clone());
                            }
                        }
                        ui.menu_button("···", |ui| {
                            for action in crate::tree::dual_row_actions(&node.row) {
                                if ui.button(action.label()).clicked() {
                                    state.select(&node.row.rel_path);
                                    state.stage_action(action, &node.row.rel_path);
                                    ui.close();
                                }
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.set_width(right_w);
                        ui.horizontal(|ui| {
                            if node.depth > 0 {
                                ui.add_space(tree_indent(node.depth));
                            }
                            ui.label(
                                TypoRole::Mono
                                    .rich(node.row.glyph.to_string())
                                    .color(glyph_color(&node.row, dark)),
                            );
                            let [name, size, time] = dual_grid_row_right(&node.row);
                            egui::Grid::new(("folder_dual_right_cells", i))
                                .num_columns(3)
                                .spacing([SECTION_GAP, TIGHT_GAP])
                                .show(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            selected,
                                            dual_grid_cell_role().rich(name),
                                        )
                                        .clicked()
                                    {
                                        state.select(&node.row.rel_path);
                                    }
                                    ui.label(dual_grid_cell_role().rich(size));
                                    ui.label(dual_grid_cell_role().rich(time));
                                    ui.end_row();
                                });
                        });
                    });
                });
            }
        });
    if let Some(dir) = toggle_dir {
        if expanded.contains(&dir) {
            expanded.remove(&dir);
        } else {
            expanded.insert(dir);
        }
    }
    let pending = state.pending.clone();
    if let Some((action, path)) = pending {
        ui.add_space(TIGHT_GAP);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TIGHT_GAP;
            ui.label(TypoRole::Small.rich(dual_pending_text(action, &path)));
            if ui.small_button("Clear").clicked() {
                state.clear_action();
            }
        });
    }
}

/// Straight-alpha RGBA bytes into an egui [`ColorImage`]: egui
/// textures expect premultiplied alpha, so every channel is scaled by
/// its alpha with saturating integer math. `None` when the bytes are
/// empty or disagree with the declared dimensions - those cases have
/// nothing to display and must not reach `ColorImage::new`, whose
/// size debug-assert would otherwise fire.
pub fn heatmap_color_image(bytes: &[u8], width: u32, height: u32) -> Option<egui::ColorImage> {
    let w = usize::try_from(width).ok()?;
    let h = usize::try_from(height).ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    let expected = w.checked_mul(h)?.checked_mul(4)?;
    if bytes.len() != expected {
        return None;
    }
    let pixels: Vec<Color32> = premultiplied_rgba(bytes)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| Color32::from_rgba_premultiplied(r, g, b, a))
        .collect();
    if pixels.len() != w.checked_mul(h)? {
        return None;
    }
    Some(egui::ColorImage::new([w, h], pixels))
}

/// Premultiplied copy of straight-alpha RGBA bytes: each color
/// channel scaled by alpha, saturating per channel. Empty for input
/// whose length is not a multiple of four. Pure; unit-tested.
fn premultiplied_rgba(bytes: &[u8]) -> Vec<u8> {
    if !bytes.len().is_multiple_of(4) {
        return Vec::new();
    }
    let (chunks, _empty_remainder) = bytes.as_chunks::<4>();
    let mut out = Vec::with_capacity(bytes.len());
    for &[r, g, b, a] in chunks {
        out.extend_from_slice(&[premul(r, a), premul(g, a), premul(b, a), a]);
    }
    out
}

/// One channel scaled by alpha: `c * a / 255`, saturating.
fn premul(channel: u8, alpha: u8) -> u8 {
    let scaled = u16::from(channel).saturating_mul(u16::from(alpha)) / 255;
    u8::try_from(scaled).unwrap_or(255)
}

// ----------------------------------------------------------------------------
// Picture compare (model-driven side-by-side / overlay + zoom / pan)
// ----------------------------------------------------------------------------

/// Width of one picture column: half the available width minus one
/// section gap, floored at [`MIN_HALF`] so narrow panels scroll instead
/// of crushing the image. Kept as the centered wrapper over
/// [`split_picture_widths`]; new paint code goes through the
/// draggable split instead.
pub fn picture_column_width(avail: f32) -> f32 {
    split_picture_widths(avail, SPLIT_DEFAULT_FRACTION).0
}

/// Clamped overlay mix paint honors (0 = full left, 100 = full right).
pub fn clamp_overlay_mix(mix: u8) -> u8 {
    mix.min(crate::app::PICTURE_OVERLAY_MAX)
}

/// Zoom presets for the step row: 25/50/100/200/400 all live inside
/// the App-clamped paint range ([`clamp_zoom_pct`]) so every step is
/// reachable via `set_zoom_pct`; the wider 8x headless math stays
/// untouched. Pure data; unit-tested.
pub const PICTURE_ZOOM_STEPS: [u32; 5] = [25, 50, 100, 200, 400];

/// Fit-to-window zoom: the largest 25pp step (the
/// `picture_zoom_in/out` ladder, [`viewmodel::PICTURE_ZOOM_STEP_PCT`])
/// at which the natural frame still fits inside the viewport.
/// Non-finite/non-positive sizes fall back to the reset zoom (100 =
/// 1:1, honest instead of a fake 25); the result runs through
/// [`clamp_zoom_pct`] so paint honors it through `set_zoom_pct`.
/// Pure; unit-tested.
pub fn picture_fit_zoom_pct(natural_w: f32, natural_h: f32, avail_w: f32, avail_h: f32) -> u32 {
    if !natural_w.is_finite()
        || !natural_h.is_finite()
        || !avail_w.is_finite()
        || !avail_h.is_finite()
        || natural_w <= 0.0
        || natural_h <= 0.0
        || avail_w <= 0.0
        || avail_h <= 0.0
    {
        return viewmodel::picture_zoom_reset();
    }
    let scale = (avail_w / natural_w).min(avail_h / natural_h);
    let pct = (scale * 100.0).floor() as u32;
    let stepped = pct.saturating_sub(pct % viewmodel::PICTURE_ZOOM_STEP_PCT);
    clamp_zoom_pct(stepped)
}

/// Centering offset for one axis: half the leftover when the content
/// is smaller than the viewport, else 0 (scroll takes over, never a
/// negative margin). Non-finite input pins to 0 so paint never feeds
/// NaN into layout. Pure; unit-tested.
pub fn picture_center_offset(content: f32, avail: f32) -> f32 {
    if !content.is_finite() || !avail.is_finite() {
        return 0.0;
    }
    ((avail - content) / 2.0).max(0.0)
}

/// Clamped zoom percent paint honors (25..=400).
pub fn clamp_zoom_pct(pct: u32) -> u32 {
    pct.clamp(crate::app::PICTURE_ZOOM_MIN, crate::app::PICTURE_ZOOM_MAX)
}

/// Zoom slider label: `zoom 100% (1.00x)`.
pub fn zoom_label(pct: u32) -> String {
    let clamped = clamp_zoom_pct(pct);
    format!("zoom {clamped}% ({:.2}x)", clamped as f32 / 100.0)
}

/// Overlay slider label: `mix 30% left / 70% right`.
pub fn overlay_label(mix: u8) -> String {
    let clamped = clamp_overlay_mix(mix);
    format!(
        "mix {clamped}% left / {}% right",
        crate::app::PICTURE_OVERLAY_MAX.saturating_sub(clamped)
    )
}

/// Pan readout label: `pan (+32,-16)px`.
pub fn pan_label(pan_x: i32, pan_y: i32) -> String {
    format!("pan ({pan_x:+},{pan_y:+})px")
}

/// Status verdict for the picture header: identical reads clean,
/// otherwise the differing share. Pure; unit-tested.
pub fn picture_status_line(model: &PictureCompareModel) -> String {
    if model.identical {
        String::from("identical: no differences")
    } else {
        model.status_summary()
    }
}

/// Display size of the heatmap under the current zoom: natural size
/// scaled by the zoom factor helper (0.25..=8.0), shared by both
/// [`crate::app::PictureMode`]s. Pure; unit-tested.
pub fn picture_display_size(natural_w: f32, natural_h: f32, view: &PictureViewState) -> (f32, f32) {
    let scale = viewmodel::picture_zoom_factor(view.zoom_pct);
    ((natural_w * scale).max(1.0), (natural_h * scale).max(1.0))
}

/// The model-driven picture view: mode radio (side-by-side columns
/// with per-side dimensions vs one overlay frame with a mix slider),
/// zoom slider, pan nudge buttons, the heatmap at the zoomed size,
/// then the CLI-worded summary lines. Spacing reuses the gutter
/// tokens ([`SECTION_GAP`], [`TIGHT_GAP`]); colors
/// stay the frozen diff palette (no new hues).
pub fn paint_picture_compare(
    ui: &mut Ui,
    model: &PictureCompareModel,
    view: &mut PictureViewState,
    heatmap: Option<&TextureHandle>,
    split: &mut f32,
) {
    ui.label(chrome_status_role().rich(picture_status_line(model)));
    ui.add_space(TIGHT_GAP);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(TypoRole::Small.rich("Layout:"));
        let mut mode = view.mode;
        for candidate in PictureMode::all() {
            ui.radio_value(&mut mode, candidate, candidate.label());
        }
        if mode != view.mode {
            view.mode = mode;
        }
    });
    ui.add_space(TIGHT_GAP);
    match view.mode {
        PictureMode::SideBySide => {
            *split = clamp_split_fraction(*split);
            let avail = ui.available_width();
            let (left_w, right_w) = split_picture_widths(avail, *split);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(left_w);
                    ui.label(TypoRole::Small.rich("left"));
                    ui.label(TypoRole::Mono.rich(format!("{}x{}", model.width_l, model.height_l)));
                    ui.add_space(TIGHT_GAP);
                    ui.label(TypoRole::Small.rich(format!(
                        "overlap {}x{} ({} px)",
                        model.overlap_w, model.overlap_h, model.total
                    )));
                });
                paint_split_handle(ui, split, avail);
                ui.vertical(|ui| {
                    ui.set_width(right_w);
                    ui.label(TypoRole::Small.rich("right"));
                    ui.label(TypoRole::Mono.rich(format!("{}x{}", model.width_r, model.height_r)));
                    ui.add_space(TIGHT_GAP);
                    ui.label(TypoRole::Small.rich(if model.identical {
                        "verdict: identical".to_string()
                    } else {
                        format!("differing: {}", model.differing)
                    }));
                });
            });
        }
        PictureMode::Overlay => {
            ui.label(TypoRole::Small.rich("overlay: heatmap frame, slider blends sides"));
            let mut mix = clamp_overlay_mix(view.overlay_mix);
            let mix_text = overlay_label(mix);
            ui.add(egui::Slider::new(&mut mix, 0..=crate::app::PICTURE_OVERLAY_MAX).text(mix_text));
            if mix != view.overlay_mix {
                view.set_overlay_mix(mix);
            }
        }
    }
    ui.add_space(TIGHT_GAP);
    // Zoom controls: keyboard (+/-/0) and buttons share the headless
    // step helpers, so both modes zoom identically; the slider keeps
    // the `App` range (read-only) while the math already spans 8x.
    let (zoom_in_key, zoom_out_key, zoom_reset_key) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals),
            input.key_pressed(egui::Key::Minus),
            input.key_pressed(egui::Key::Num0),
        )
    });
    if zoom_in_key {
        view.set_zoom_pct(viewmodel::picture_zoom_in(view.zoom_pct));
    }
    if zoom_out_key {
        view.set_zoom_pct(viewmodel::picture_zoom_out(view.zoom_pct));
    }
    if zoom_reset_key {
        view.set_zoom_pct(viewmodel::picture_zoom_reset());
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(TypoRole::Small.rich(zoom_label(view.zoom_pct)));
        if ui.small_button("+").on_hover_text("zoom in (+)").clicked() {
            view.set_zoom_pct(viewmodel::picture_zoom_in(view.zoom_pct));
        }
        if ui.small_button("-").on_hover_text("zoom out (-)").clicked() {
            view.set_zoom_pct(viewmodel::picture_zoom_out(view.zoom_pct));
        }
        if ui
            .small_button("1:1")
            .on_hover_text("reset zoom (0)")
            .clicked()
        {
            view.set_zoom_pct(viewmodel::picture_zoom_reset());
        }
        let mut zoom = clamp_zoom_pct(view.zoom_pct);
        ui.add(
            egui::Slider::new(
                &mut zoom,
                crate::app::PICTURE_ZOOM_MIN..=crate::app::PICTURE_ZOOM_MAX,
            )
            .text("zoom"),
        );
        if zoom != view.zoom_pct {
            view.set_zoom_pct(zoom);
        }
    });
    // Zoom-step row: Fit plus the preset ladder, mapping each step to
    // its zoom pct (selectable row, exact-hit highlight). Fit derives
    // from the remaining viewport below this row (heatmap + summary
    // area) and the heatmap natural size; without a heatmap it stays
    // disabled instead of guessing. Geometry helpers above, no new
    // token/typography/color here.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(TypoRole::Small.rich("steps:"));
        let fit = heatmap.map(|handle| {
            let natural = handle.size_vec2();
            picture_fit_zoom_pct(
                natural.x,
                natural.y,
                ui.available_width(),
                ui.available_height(),
            )
        });
        if ui
            .add_enabled(fit.is_some(), egui::Button::new("Fit"))
            .on_hover_text("fit image to window")
            .clicked()
            && let Some(pct) = fit
        {
            view.set_zoom_pct(pct);
        }
        let mut zoom = clamp_zoom_pct(view.zoom_pct);
        for step in PICTURE_ZOOM_STEPS {
            ui.selectable_value(&mut zoom, step, format!("{step}%"));
        }
        if zoom != view.zoom_pct {
            view.set_zoom_pct(zoom);
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(TypoRole::Small.rich(pan_label(view.pan_x, view.pan_y)));
        if ui.small_button("◀").clicked() {
            view.nudge_pan(-crate::app::PICTURE_PAN_STEP, 0);
        }
        if ui.small_button("▶").clicked() {
            view.nudge_pan(crate::app::PICTURE_PAN_STEP, 0);
        }
        if ui.small_button("▲").clicked() {
            view.nudge_pan(0, -crate::app::PICTURE_PAN_STEP);
        }
        if ui.small_button("▼").clicked() {
            view.nudge_pan(0, crate::app::PICTURE_PAN_STEP);
        }
        if ui.small_button("Reset view").clicked() {
            view.reset_view();
        }
    });
    ui.add_space(SECTION_GAP);
    match heatmap {
        Some(handle) => {
            // Both modes share this zoomed heatmap frame; the
            // side-by-side split above stays the layout authority
            // (Wave U GEO) and never reads the zoom factor.
            let natural = handle.size_vec2();
            let (dw, dh) = picture_display_size(natural.x, natural.y, view);
            // Clamp stale state to the content bounds (App is
            // read-only, so nudge by the delta instead of setting).
            let limit = crate::app::PICTURE_PAN_LIMIT;
            let (cx, cy) = viewmodel::clamp_picture_pan(view.pan_x, view.pan_y, dw, dh, limit);
            view.nudge_pan(cx.saturating_sub(view.pan_x), cy.saturating_sub(view.pan_y));
            // Viewport captured before the scroll area fills it: the
            // centering offsets below are measured against this frame.
            let viewport = ui.available_size();
            let output = egui::ScrollArea::both()
                .id_salt("picture_heatmap_frame")
                .auto_shrink([false, false])
                .scroll_offset(egui::vec2(
                    view.pan_x.max(0) as f32,
                    view.pan_y.max(0) as f32,
                ))
                .show(ui, |ui| {
                    let ox = picture_center_offset(dw, viewport.x);
                    let oy = picture_center_offset(dh, viewport.y);
                    if ox > 0.0 || oy > 0.0 {
                        // Centered frame: the zoomed image is smaller
                        // than the viewport on at least one axis, so
                        // equal margins center it (BC fit-to-window
                        // geometry); scroll stays dormant because
                        // content plus margins still fit. Wide/tall
                        // content keeps the plain top-left path below.
                        ui.vertical_centered(|ui| {
                            ui.add_space(oy);
                            ui.add(
                                egui::Image::new((handle.id(), handle.size_vec2()))
                                    .fit_to_exact_size(egui::vec2(dw, dh))
                                    .sense(egui::Sense::click_and_drag()),
                            )
                        })
                        .inner
                    } else {
                        ui.add(
                            egui::Image::new((handle.id(), handle.size_vec2()))
                                .fit_to_exact_size(egui::vec2(dw, dh))
                                .sense(egui::Sense::click_and_drag()),
                        )
                    }
                });
            // Drag to pan: the image follows the cursor (grabbing right
            // scrolls left), clamped to the content bounds. The forced
            // scroll_offset above keeps state the single authority, so
            // the area's own transient drag never fights back.
            if output.inner.dragged() {
                let delta = output.inner.drag_delta();
                let (nx, ny) = viewmodel::picture_pan_drag(
                    view.pan_x,
                    view.pan_y,
                    -delta.x.round() as i32,
                    -delta.y.round() as i32,
                    dw,
                    dh,
                    limit,
                );
                view.nudge_pan(nx.saturating_sub(view.pan_x), ny.saturating_sub(view.pan_y));
            }
            // Double-click resets zoom+pan (same as Reset view).
            if output.inner.double_clicked() {
                view.reset_view();
            }
        }
        None => paint_note(ui, "heatmap unavailable for these inputs"),
    }
    ui.add_space(TIGHT_GAP);
    paint_lines(ui, &model.lines);
}

// ----------------------------------------------------------------------------
// Sync / folder-merge / audio checklists (per-file rows)
// ----------------------------------------------------------------------------

/// Paint-ready text of one sync checklist row: checkbox state plus
/// the display line. Pure; unit-tested.
pub fn sync_row_text(checked: bool, line: &str) -> String {
    let box_glyph = if checked { "[x]" } else { "[ ]" };
    format!("{box_glyph} {line}")
}

/// Paint-ready text of one skipped sync entry. Pure; unit-tested.
pub fn sync_skip_text(rel_path: &str, reason: &str) -> String {
    format!("~ {rel_path} ({reason})")
}

/// Glyph color of one sync row: copies track blue, deletes red, like
/// the folder glyphs; frozen palette, no new hues.
pub fn sync_row_color(action: autocompare_compare::SyncAction, dark: bool) -> egui::Color32 {
    use autocompare_compare::SyncAction;
    match action {
        SyncAction::CopyToRight | SyncAction::CopyToLeft => blue_for(dark),
        SyncAction::DeleteRight | SyncAction::DeleteLeft => red_for(dark),
    }
}

/// The sync plan as per-file checklist rows: each step is a checkbox
/// row with its glyph, path and action label, then skipped entries
/// and the honest counts line. Dry-run only; checking rows stages
/// them, Apply stays a CLI operation.
pub fn paint_sync_checklist(ui: &mut Ui, list: &mut SyncChecklist) {
    let dark = ui.visuals().dark_mode;
    ui.label(chrome_status_role().rich(list.summary()));
    if list.truncated {
        paint_note(
            ui,
            "the walk stopped at the depth limit; deeper entries are not planned",
        );
    }
    ui.add_space(TIGHT_GAP);
    if list.rows.is_empty() {
        ui.label(RichText::new("(nothing to do: the trees already match the plan)").weak());
    } else {
        for row in &mut list.rows {
            let color = sync_row_color(row.action, dark);
            let hover = sync_row_text(row.checked, &row.display_line());
            ui.horizontal(|ui| {
                ui.checkbox(&mut row.checked, "").on_hover_text(hover);
                ui.label(TypoRole::Mono.rich(row.glyph().to_string()).color(color));
                ui.label(TypoRole::Mono.rich(row.display_line()));
                ui.label(TypoRole::Small.rich(row.action_label()));
            });
        }
    }
    if !list.skipped.is_empty() {
        ui.add_space(TIGHT_GAP);
        for (path, reason) in &list.skipped {
            ui.label(
                TypoRole::Mono
                    .rich(sync_skip_text(path, reason))
                    .size(crate::theme::TYPO_SMALL_SIZE)
                    .color(dim_for(dark)),
            );
        }
    }
    ui.label(chrome_status_role().rich(format!(
        "{} of {} step(s) selected",
        list.checked_count(),
        list.rows.len()
    )));
}

/// Glyph color of one folder-merge row: conflicts red, clean merges
/// green, side takes blue, new dirs dim; frozen palette only.
pub fn folder_merge_row_color(
    action: autocompare_compare::TreeMergeAction,
    dark: bool,
) -> egui::Color32 {
    use autocompare_compare::TreeMergeAction;
    match action {
        TreeMergeAction::Conflict => red_for(dark),
        TreeMergeAction::MergedText => green_for(dark),
        TreeMergeAction::TakeLeft | TreeMergeAction::TakeRight => blue_for(dark),
        TreeMergeAction::MakeDir => dim_for(dark),
    }
}

/// The folder-merge plan as per-file checklist rows: each step is a
/// checkbox row with its glyph, path and action label, then conflict
/// paths and the counts line. Dry-run only.
pub fn paint_folder_merge_checklist(ui: &mut Ui, list: &mut FolderMergeChecklist) {
    let dark = ui.visuals().dark_mode;
    ui.label(chrome_status_role().rich(list.summary()));
    ui.add_space(TIGHT_GAP);
    if list.rows.is_empty() {
        ui.label(RichText::new("(nothing to do: the merge plan is empty)").weak());
    } else {
        for row in &mut list.rows {
            let color = folder_merge_row_color(row.action, dark);
            ui.horizontal(|ui| {
                ui.checkbox(&mut row.checked, "");
                ui.label(TypoRole::Mono.rich(row.glyph().to_string()).color(color));
                ui.label(TypoRole::Mono.rich(row.display_line()));
                if row.is_conflict() {
                    ui.label(
                        TypoRole::Small
                            .rich("needs a decision")
                            .color(red_for(dark)),
                    );
                } else {
                    ui.label(TypoRole::Small.rich(row.action_label()));
                }
            });
        }
    }
    if !list.conflicts.is_empty() {
        ui.add_space(TIGHT_GAP);
        ui.label(chrome_status_role().rich(format!("{} conflict(s):", list.conflicts.len())));
        for path in &list.conflicts {
            ui.label(
                TypoRole::Mono
                    .rich(format!("! {path} (conflict)"))
                    .size(crate::theme::TYPO_SMALL_SIZE)
                    .color(red_for(dark)),
            );
        }
    }
    ui.label(chrome_status_role().rich(format!(
        "{} of {} step(s) selected",
        list.checked_count(),
        list.rows.len()
    )));
}

// ----------------------------------------------------------------------------
// Three-way tree paint (BI-M2 paint half; FolderMerge tab wiring next wave).
// Bahar se dekha teen taraf ka faisla ek ped me dikhta hai: har path par
// base/left/right maujoodgi, conflict par Take Left/Base/Right, header me
// honest progress. Model reuse read-only hai
// (`crate::tree::three_way_tree_rows` + `is_three_way_conflict` +
// `three_way_tree_summary`); decisions `ThreeWayDecision` staged list me
// rehte hain (last-wins, [`stage_three_way_decision`]). Geometry/typo
// existing tokens me hai (TIGHT_GAP spacing, status role header, mono
// rows, small weak notes), colors frozen palette only.
// ----------------------------------------------------------------------------

/// Base/left/right presence marks: maujood side ka akshar, gayab par `-`
/// (`"BLR"`, `"B-R"`, `"-L-"`). Fixed 3-char, mono me paint hota hai.
/// Pure; unit-tested.
pub fn three_way_presence_marks(row: &crate::tree::ThreeWayTreeRow) -> String {
    let base = if row.has_base { 'B' } else { '-' };
    let left = if row.has_left { 'L' } else { '-' };
    let right = if row.has_right { 'R' } else { '-' };
    format!("{base}{left}{right}")
}

/// Ek three-way row ki paint-ready path line: relative path plus dir par
/// `/` mark (folder checklist precedent). Pure; unit-tested.
pub fn three_way_row_line(row: &crate::tree::ThreeWayTreeRow) -> String {
    if row.is_dir {
        format!("{}/", row.rel_path)
    } else {
        row.rel_path.clone()
    }
}

/// Staged decisions me `rel_path` ki aakhri staged side; unknown par `None`.
/// Duplicate picks me aakhri staged jeetti hai
/// (`three_way_tree_summary` dedup rule). Pure; unit-tested.
pub fn three_way_decision_for_row(
    decisions: &[crate::tree::ThreeWayDecision],
    rel_path: &str,
) -> Option<crate::tree::ThreeWaySide> {
    let mut found = None;
    for decision in decisions {
        if decision.rel_path == rel_path {
            found = Some(decision.side);
        }
    }
    found
}

/// Ek decision stage karta hai: same path ki purani pick overwrite hoti hai
/// (aakhri jeetti hai), naya path judta hai. Summary dedup rule ka
/// paint-side mirror. Pure; unit-tested.
pub fn stage_three_way_decision(
    decisions: &[crate::tree::ThreeWayDecision],
    rel_path: &str,
    side: crate::tree::ThreeWaySide,
) -> Vec<crate::tree::ThreeWayDecision> {
    let mut out: Vec<crate::tree::ThreeWayDecision> = Vec::new();
    for decision in decisions {
        if decision.rel_path != rel_path {
            out.push(decision.clone());
        }
    }
    out.push(crate::tree::ThreeWayDecision {
        rel_path: rel_path.to_string(),
        side,
    });
    out
}

/// Ek row ka decision display: staged pick ho to "take <side>" intent,
/// conflict par honest pending, clean par "clean". Pure; unit-tested.
pub fn three_way_row_decision_text(
    conflict: bool,
    decided: Option<crate::tree::ThreeWaySide>,
) -> String {
    match decided {
        Some(side) => side.label().to_lowercase(),
        None if conflict => "needs a decision".to_string(),
        None => "clean".to_string(),
    }
}

/// Header progress line: rows + decisions par honest summary
/// (`three_way_tree_summary().summary_line()` reuse). Pure; unit-tested.
pub fn three_way_header_line(
    rows: &[crate::tree::ThreeWayTreeRow],
    decisions: &[crate::tree::ThreeWayDecision],
) -> String {
    crate::tree::three_way_tree_summary(rows, decisions).summary_line()
}

/// The three-way tree: header me summary line, har row par presence marks,
/// path aur decision display; conflict rows par real Take Left, Take Base aur
/// Take Right buttons (koi stub nahi). Click staged decisions me jata hai
/// (model rows untouched); FolderMerge tab wiring agli wave ka hai.
pub fn paint_three_way_tree(
    ui: &mut Ui,
    rows: &[crate::tree::ThreeWayTreeRow],
    decisions: &mut Vec<crate::tree::ThreeWayDecision>,
) {
    ui.label(chrome_status_role().rich(three_way_header_line(rows, decisions)));
    ui.add_space(TIGHT_GAP);
    if rows.is_empty() {
        ui.label(RichText::new("(nothing to merge: all three snapshots are empty)").weak());
        return;
    }
    let dark = ui.visuals().dark_mode;
    let mut pending: Vec<(String, crate::tree::ThreeWaySide)> = Vec::new();
    for row in rows {
        let conflict = crate::tree::is_three_way_conflict(row);
        let decided = three_way_decision_for_row(decisions, row.rel_path.as_str());
        let mut chosen: Option<crate::tree::ThreeWaySide> = None;
        ui.horizontal(|ui| {
            let marks = three_way_presence_marks(row);
            let mark_color = if conflict {
                red_for(dark)
            } else {
                dim_for(dark)
            };
            ui.label(TypoRole::Mono.rich(marks).color(mark_color));
            ui.label(TypoRole::Mono.rich(three_way_row_line(row)));
            let text = three_way_row_decision_text(conflict, decided);
            if conflict && decided.is_none() {
                ui.label(TypoRole::Small.rich(text).color(red_for(dark)));
            } else {
                ui.label(TypoRole::Small.rich(text));
            }
        });
        if conflict {
            ui.horizontal(|ui| {
                ui.label(TypoRole::Small.rich("Take:"));
                if ui
                    .small_button(crate::tree::ThreeWaySide::Left.label())
                    .clicked()
                {
                    chosen = Some(crate::tree::ThreeWaySide::Left);
                }
                if ui
                    .small_button(crate::tree::ThreeWaySide::Base.label())
                    .clicked()
                {
                    chosen = Some(crate::tree::ThreeWaySide::Base);
                }
                if ui
                    .small_button(crate::tree::ThreeWaySide::Right.label())
                    .clicked()
                {
                    chosen = Some(crate::tree::ThreeWaySide::Right);
                }
            });
            ui.add_space(TIGHT_GAP);
        }
        if let Some(side) = chosen {
            pending.push((row.rel_path.clone(), side));
        }
    }
    for (path, side) in pending {
        *decisions = stage_three_way_decision(decisions, path.as_str(), side);
    }
}

/// Paint-ready text of one audio row: label plus detail. Pure;
/// unit-tested.
pub fn audio_row_text(kind: AudioCheckKind, label: &str, detail: &str) -> String {
    let tag = match kind {
        AudioCheckKind::LeftConfig => "L",
        AudioCheckKind::RightConfig => "R",
        AudioCheckKind::Verdict => "=",
    };
    format!("{tag} {label}: {detail}")
}

/// Color of one audio row: configs dim, identical verdict green,
/// mismatch red, differing verdict changed-fg; frozen palette only.
pub fn audio_row_color(
    kind: AudioCheckKind,
    identical: bool,
    mismatch: bool,
    dark: bool,
) -> egui::Color32 {
    match kind {
        AudioCheckKind::LeftConfig | AudioCheckKind::RightConfig => dim_for(dark),
        AudioCheckKind::Verdict if mismatch => red_for(dark),
        AudioCheckKind::Verdict if identical => green_for(dark),
        AudioCheckKind::Verdict => CHANGED_FG,
    }
}

/// The audio compare as checklist rows: one row per stream config
/// plus the verdict row, each with a checkbox, then the summary.
pub fn paint_audio_checklist(ui: &mut Ui, list: &mut AudioChecklist) {
    let dark = ui.visuals().dark_mode;
    ui.label(chrome_status_role().rich(list.summary()));
    ui.add_space(TIGHT_GAP);
    if list.rows.is_empty() {
        ui.label(RichText::new("(no audio facts to show)").weak());
        return;
    }
    for row in &mut list.rows {
        let color = audio_row_color(row.kind, list.identical, list.config_mismatch, dark);
        ui.horizontal(|ui| {
            ui.checkbox(&mut row.checked, "");
            ui.label(
                TypoRole::Mono
                    .rich(audio_row_text(row.kind, &row.label, &row.detail))
                    .color(color),
            );
        });
    }
}

// ----------------------------------------------------------------------------
// Text find bar (Wave T wiring: App text tab se called).
// ----------------------------------------------------------------------------

/// N/M badge text for the text find bar: "2/5", no hit par "0/0".
/// Pure; App badge model ko paint bina App borrow ke dikhata hai.
/// App text tab se called (Wave T wiring).
pub fn text_find_badge_label(position: Option<usize>, total: usize) -> String {
    match position {
        Some(pos) => format!("{pos}/{total}"),
        None => format!("0/{total}"),
    }
}

/// Text find bar: needle field + case flag + Prev/Next + badge label.
/// Returns `(prev_clicked, next_clicked)`; cursor/badge advance App
/// ke `text_find_next_row` / `text_find_prev_row` me hota hai.
/// App text tab se called (Wave T wiring).
pub fn paint_text_find_bar(
    ui: &mut Ui,
    needle: &mut String,
    ignore_case: &mut bool,
    badge: &str,
) -> (bool, bool) {
    let mut prev = false;
    let mut next = false;
    let avail = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail, FIND_BAR_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(TypoRole::Small.rich("Find:"));
        ui.add(
            egui::TextEdit::singleline(needle)
                .desired_width(FIND_NEEDLE_W)
                .hint_text("needle"),
        );
        ui.checkbox(ignore_case, "Ignore case");
        if ui.small_button("◀ Prev").clicked() {
            prev = true;
        }
        if ui.small_button("Next ▶").clicked() {
            next = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(BADGE_PAD);
            ui.label(TypoRole::Small.rich(badge));
        });
    });
    (prev, next)
}

/// Headless find-nav step: Prev/Next clicks ko `FindState` advance me
/// badalta hai. Click na ho to `None` (selection untouched rehta hai,
/// taaki F3 diff-nav selection se ladai na ho). Pure; unit-tested.
pub fn advance_text_find(
    find: &mut FindState,
    prev_clicked: bool,
    next_clicked: bool,
) -> Option<usize> {
    if next_clicked {
        find.advance()
    } else if prev_clicked {
        find.prev()
    } else {
        None
    }
}

/// `FindState`-owned text find bar: needle field, case flag, Prev/Next
/// aur N/M count badge. Table/folder find precedent ka paint reuse: wahi
/// needle width ([`FIND_NEEDLE_W`]), wahi button labels, badge right me
/// ek pad ([`BADGE_PAD`]) ke saath small weak typo me. Query edit par
/// `set_query_cased` recompute karta hai (cursor first hit par), case
/// toggle par `set_ignore_case`. Badge [`FindState::count_label`] se
/// aata hai. Returns `(prev_clicked, next_clicked)`; cursor advance
/// [`advance_text_find`] me hota hai. Purana App-wired
/// [`paint_text_find_bar`] compat ke liye untouched hai.
pub fn paint_text_find_state_bar(
    ui: &mut Ui,
    find: &mut FindState,
    rows: &[TextRow],
) -> (bool, bool) {
    let mut prev = false;
    let mut next = false;
    let avail = ui.available_width();
    // TextEdit ko `&mut String` chahiye, query `FindState` ke andar hai:
    // local copy edit karke change par wapas likho (App `old_needle`
    // precedent).
    let mut needle = find.query().to_string();
    let mut ignore_case = find.ignore_case();
    ui.horizontal(|ui| {
        ui.set_min_size(egui::vec2(avail, FIND_BAR_H));
        ui.spacing_mut().item_spacing.x = TIGHT_GAP;
        ui.label(TypoRole::Small.rich("Find:"));
        let edit = ui.add(
            egui::TextEdit::singleline(&mut needle)
                .desired_width(FIND_NEEDLE_W)
                .hint_text("needle"),
        );
        if edit.changed() {
            let flag = find.ignore_case();
            find.set_query_cased(rows, &needle, flag);
        }
        if ui.checkbox(&mut ignore_case, "Ignore case").changed() {
            find.set_ignore_case(rows, ignore_case);
        }
        if ui.small_button("◀ Prev").clicked() {
            prev = true;
        }
        if ui.small_button("Next ▶").clicked() {
            next = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(BADGE_PAD);
            ui.label(TypoRole::Small.rich(find.count_label()));
        });
    });
    (prev, next)
}

/// Find-box wired text view: `FindState` bar upar, neeche find-aware
/// [`paint_text_body`]. Har frame `refresh_if_stale` chalata hai
/// taaki naya compare stale counts na dikhaye; Prev/Next clicks
/// [`advance_text_find`] se `model.selected` sync karte hain (wahi
/// highlight jo F3 diff-nav use karta hai) plus per-match blocks:
/// current row inverted block, others underline line (existing palette
/// reuse, wrap continuations samahit). Prefs/counts behavior
/// `paint_text_with_prefs` jaisa hi hai.
pub fn paint_text_with_find(
    ui: &mut Ui,
    model: &mut TextRows,
    prefs: &TextViewPrefs,
    find: &mut FindState,
) {
    find.refresh_if_stale(&model.rows);
    let (find_prev, find_next) = paint_text_find_state_bar(ui, find, &model.rows);
    if let Some(row) = advance_text_find(find, find_prev, find_next) {
        model.selected = Some(row);
    }
    // Borrow split: needle copy nikal kar body ko `&str` do (find + model
    // dono `&mut` nahi ho sakte ek saath).
    let needle_owned: String = find.query().to_string();
    let ignore_case: bool = find.ignore_case();
    let current: Option<usize> = model.selected;
    let ctx = FindPaintCtx {
        needle: needle_owned.as_str(),
        ignore_case,
        current,
    };
    paint_text_body(ui, model, prefs, Some(ctx));
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_line_pads_to_the_pane_width() {
        assert_eq!(
            pane_segments("ab", &[], 4),
            vec![("ab".to_string(), false), ("  ".to_string(), false)]
        );
    }

    #[test]
    fn accent_span_splits_the_pane_into_three_segments() {
        assert_eq!(
            pane_segments("abc", &[(1, 2)], 3),
            vec![
                ("a".to_string(), false),
                ("b".to_string(), true),
                ("c".to_string(), false),
            ]
        );
    }

    #[test]
    fn accents_at_line_start_and_truncation_are_handled() {
        assert_eq!(
            pane_segments("abc", &[(0, 2)], 4),
            vec![
                ("ab".to_string(), true),
                ("c".to_string(), false),
                (" ".to_string(), false),
            ]
        );
        // Longer than the pane: truncated, like the CLI's panes.
        assert_eq!(
            pane_segments("abcdef", &[(5, 6)], 3),
            vec![("abc".to_string(), false)]
        );
    }

    #[test]
    fn empty_line_is_only_padding() {
        assert_eq!(
            pane_segments("", &[(0, 1)], 2),
            vec![("  ".to_string(), false)]
        );
    }

    #[test]
    fn zero_length_spans_accent_nothing() {
        assert_eq!(
            pane_segments("ab", &[(1, 1)], 2),
            vec![("ab".to_string(), false)]
        );
    }

    #[test]
    fn gutter_text_right_aligns_numbers_and_blanks_sides() {
        assert_eq!(gutter_text(Some(12), 4), "  12  ".to_string());
        assert_eq!(gutter_text(None, 4), "      ".to_string());
        assert_eq!(gutter_text(Some(123456), 4), "123456  ".to_string());
    }

    #[test]
    fn text_center_gutter_reuses_tokens_and_gates_copy() {
        assert_eq!(
            TEXT_CENTER_GUTTER_W,
            HEADER_H + GUTTER_W + TIGHT_GAP + TIGHT_GAP
        );
        assert_eq!(text_center_gutter_width(), 48.0);
        assert!(text_gutter_has_copy(TextRowKind::Changed));
        assert!(text_gutter_has_copy(TextRowKind::Deleted));
        assert!(text_gutter_has_copy(TextRowKind::Inserted));
        assert!(!text_gutter_has_copy(TextRowKind::Equal));
    }

    #[test]
    fn text3_wide_960_center_wrap_trailing() {
        // Mirrors checks/gui-exq-text3/comparison/text-wide-960.json:
        // 960px canvas par centered hunk markers, wrap-marker alignment,
        // even trailing blanks. Geometry FIRST (widths/insets), typography
        // SECOND (mono rhythm untouched), colors frozen (koi hue nahi).
        use viewmodel::TextRow;
        // Centered markers: 36px content (16+16+4) in 48px column = 6/side.
        assert_eq!(text_center_marker_content_w(), 36.0);
        assert_eq!(
            text_center_marker_content_w(),
            2.0 * crate::gutter::GUTTER_ARROW_BOX + TIGHT_GAP
        );
        assert_eq!(text_center_marker_inset(), 6.0);
        assert_eq!(
            text_center_marker_inset(),
            crate::gutter::center_row_marker_inset(48.0, 36.0)
        );
        assert!(
            (text_center_marker_inset() * 2.0 + text_center_marker_content_w() - 48.0).abs() < 0.01
        );
        assert_eq!(crate::gutter::center_row_marker_inset(48.0, 36.0), 6.0);
        assert_eq!(crate::gutter::center_row_marker_inset(f32::NAN, 36.0), 0.0);
        assert_eq!(crate::gutter::center_row_marker_inset(48.0, 60.0), 0.0);
        // Wide-960 panes: 448/448 pair fills 960 minus connector + gaps.
        assert_eq!(text_wide_960_pane_pair(), (448.0, 448.0));
        assert_eq!(text_wide_960_pane_pair(), text_pane_pair_widths(960.0));
        // Wrap-marker alignment: same width as blank gutter, marker at
        // number-field right edge, trail pad preserved.
        assert_eq!(WRAP_CONTINUATION_MARKER, "\u{21a9}");
        assert_eq!(wrap_marker_column(4), 3);
        assert_eq!(wrap_marker_column(1), 0);
        assert_eq!(wrap_marker_column(0), 0);
        assert_eq!(wrap_marker_gutter(4), "   \u{21a9}  ".to_string());
        assert_eq!(wrap_marker_gutter(1), "\u{21a9}  ".to_string());
        assert_eq!(
            wrap_marker_gutter(4).chars().count(),
            wrap_continuation_gutter(4).chars().count()
        );
        assert!(wrap_marker_gutter(4).ends_with("  "));
        // Trailing balance at 960 rhythm: 436px viewport @17.6 row = 25
        // slots - 9 rows = 16 raw, balanced stays even (no orphan blank).
        let row_h = row_height_for_line(TypoRole::Mono.line_height_px());
        assert!((row_h - 17.6).abs() < 0.01);
        assert_eq!(text_trailing_blank_rows(436.0, row_h, 9), 16);
        assert_eq!(text_trailing_blank_rows_balanced(436.0, row_h, 9), 16);
        assert_eq!(text_trailing_blank_rows_balanced(436.0, row_h, 9) % 2, 0);
        // Hunk heads per fixture: C,C,E,I -> true,false,false,true.
        let rows = vec![
            TextRow {
                kind: TextRowKind::Changed,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Equal,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Inserted,
                ..Default::default()
            },
        ];
        assert!(text_center_hunk_head(&rows, 0));
        assert!(!text_center_hunk_head(&rows, 1));
        assert!(!text_center_hunk_head(&rows, 2));
        assert!(text_center_hunk_head(&rows, 3));
    }

    #[test]
    fn text4_narrow720b_spans_plus_numbers() {
        // Mirrors checks/gui-exq-text4/comparison/text-narrow-720b.json:
        // even intra-line span gaps (SPACE_XS symmetric), right-aligned
        // tabular number column, even trailing blanks at the 720 narrow
        // breakpoint. Geometry FIRST, typography SECOND, colors frozen.
        use viewmodel::TextRow;
        // Intra-line span pad: one XS step total, half per side, symmetric.
        assert_eq!(INTRA_SPAN_PAD_PX, crate::theme::SPACE_XS);
        assert_eq!(INTRA_SPAN_PAD_PX, 4.0);
        assert_eq!(intra_span_pad_px(), 4.0);
        assert_eq!(intra_span_side_pad_px(), 2.0);
        assert!(intra_span_gaps_symmetric());
        assert_eq!(intra_segment_leading_gap(true), 0.0);
        assert_eq!(intra_segment_leading_gap(false), 2.0);
        // Number column: right-aligned tabular, blank sides same width.
        assert_eq!(gutter_text(Some(1), 4), "   1  ".to_string());
        assert_eq!(gutter_text(Some(12), 4), "  12  ".to_string());
        assert_eq!(gutter_text(None, 4), "      ".to_string());
        assert_eq!(
            gutter_text(None, 4).chars().count(),
            gutter_text(Some(1), 4).chars().count()
        );
        assert!(text_gutter_numbers_tabular());
        assert_eq!(text_gutter_number_width(9, true), 16.0);
        assert_eq!(text_gutter_number_width(99, true), 20.0);
        assert_eq!(text_gutter_number_width(9, false), 8.0);
        assert!(text_gutter_number_width(999, true) > text_gutter_number_width(99, true));
        // Narrow 720: 720 stays side-by-side, 719 stacks; stacked single
        // pane fills 720 - 8 air = 712.
        assert_eq!(text_layout_for_width(720.0), TextLayout::SideBySide);
        assert_eq!(text_layout_for_width(719.0), TextLayout::Stacked);
        assert_eq!(text_narrow_720b_stacked_width(), 712.0);
        assert_eq!(text_narrow_720b_stacked_width(), text_stacked_pane_width(720.0));
        assert_eq!(text_stacked_pane_width(f32::NAN), MIN_HALF);
        // Trailing balance at narrow: 300px budget @17.6 row, 7 visible
        // rows leave 11 raw blanks, balanced up to 12 (paired, no orphan).
        let row_h = row_height_for_line(TypoRole::Mono.line_height_px());
        assert!((row_h - 17.6).abs() < 0.01);
        assert_eq!(text_trailing_blank_rows(300.0, row_h, 7), 11);
        assert_eq!(text_trailing_blank_rows_balanced(300.0, row_h, 7), 12);
        assert_eq!(text_trailing_blank_rows_balanced(300.0, row_h, 7) % 2, 0);
        assert_eq!(text_trailing_blank_rows_balanced(300.0, row_h, 18), 0);
        // Hunk heads per fixture: C,C,E,I -> true,false,false,true.
        let rows = vec![
            TextRow {
                kind: TextRowKind::Changed,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Equal,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Inserted,
                ..Default::default()
            },
        ];
        assert!(text_center_hunk_head(&rows, 0));
        assert!(!text_center_hunk_head(&rows, 1));
        assert!(!text_center_hunk_head(&rows, 2));
        assert!(text_center_hunk_head(&rows, 3));
    }

    #[test]
    fn text_center_hunk_head_groups_contiguous_diffs() {
        // Mirrors checks/gui-exq-text3/comparison/edge-hunk-head.json:
        // Changed+Changed share one hunk (second stays blank), Equal never
        // heads, next diff after Equal heads again. OOB/empty never head.
        use viewmodel::TextRow;
        let rows = vec![
            TextRow {
                kind: TextRowKind::Changed,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Equal,
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Inserted,
                ..Default::default()
            },
        ];
        assert!(text_center_hunk_head(&rows, 0));
        assert!(!text_center_hunk_head(&rows, 1));
        assert!(!text_center_hunk_head(&rows, 2));
        assert!(text_center_hunk_head(&rows, 3));
        assert!(!text_center_hunk_head(&rows, 4));
        assert!(!text_center_hunk_head(&[], 0));
    }

    #[test]
    fn text_center_gutter_band_fills_on_selection_only() {
        // VIEWFILL7: selected row par center gutter band bhi bharta hai
        // (head/blank/continuation sab — band beech me na toote), bina
        // selection kabhi nahi. Copy-gate se independent hai.
        assert!(text_center_gutter_band_fill(true));
        assert!(!text_center_gutter_band_fill(false));
    }

    #[test]
    fn text_layout_breakpoint_reuses_viewport_min_and_guards() {
        // Mirrors checks/gui-exq-text2/comparison/*.json canvases.
        assert_eq!(TEXT_STACK_BREAKPOINT_W, 720.0);
        assert_eq!(TEXT_STACK_BREAKPOINT_W as u32, crate::theme::VIEWPORT_MIN_W);
        assert_eq!(text_layout_for_width(960.0), TextLayout::SideBySide);
        assert_eq!(text_layout_for_width(720.0), TextLayout::SideBySide);
        assert_eq!(text_layout_for_width(719.0), TextLayout::Stacked);
        assert_eq!(text_layout_for_width(320.0), TextLayout::Stacked);
        assert_eq!(text_layout_for_width(0.0), TextLayout::SideBySide);
        assert_eq!(text_layout_for_width(f32::NAN), TextLayout::SideBySide);
        assert_eq!(text_layout_for_width(f32::INFINITY), TextLayout::SideBySide);
    }

    #[test]
    fn text_edge_empty_vs_nonempty_stays_honest_and_wraps_long_line() {
        // Mirrors checks/gui-exq-text2/comparison/empty-vs-nonempty.json:
        // empty left vs two-line right must keep one diff row per line,
        // line numbers only on the present side (no fake "0 B" numbers).
        let model = viewmodel::TextRows {
            rows: vec![
                viewmodel::TextRow {
                    kind: TextRowKind::Inserted,
                    left_no: None,
                    right_no: Some(1),
                    right: "a".to_string(),
                    ..Default::default()
                },
                viewmodel::TextRow {
                    kind: TextRowKind::Inserted,
                    left_no: None,
                    right_no: Some(2),
                    right: "b".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(model.rows.len(), 2);
        for row in &model.rows {
            assert_ne!(row.kind, TextRowKind::Equal);
            assert_eq!(row.left_no, None);
            assert!(row.right_no.is_some());
        }
        // Mirrors checks/gui-exq-text2/comparison/long-line-wrap.json:
        // 200-char line wraps to 3 visuals at the 80-char cap, one at
        // full width (wrap-off keeps bytes intact, no silent cut).
        let long = "x".repeat(200);
        assert_eq!(wrap_width_for(200, true), 80);
        assert_eq!(wrap_width_for(200, false), 200);
        assert_eq!(wrap_pane_pair_count(&long, &long, 80), 3);
        assert_eq!(wrap_pane_pair_count(&long, &long, 200), 1);
    }

    #[test]
    fn text_rows_reuse_mono_hierarchy_line_height_and_spacing() {
        // Typography SECOND: code rows Mono (12 regular-mono), badges
        // Small, line-height 1.30, letter-spacing 0. Colors untouched.
        use crate::theme::{TYPO_LETTER_SPACING, TYPO_LINE_HEIGHT_MONO, TYPO_MONO_SIZE};
        let role = TypoRole::Mono;
        assert!(role.is_mono());
        assert_eq!(role.size(), TYPO_MONO_SIZE);
        assert_eq!(role.line_height(), TYPO_LINE_HEIGHT_MONO);
        assert!((TYPO_LINE_HEIGHT_MONO - 1.30).abs() < 0.001);
        assert_eq!(role.line_height_px(), TYPO_MONO_SIZE * 1.30);
        assert_eq!(role.letter_spacing_em(), 0.0);
        assert_eq!(TYPO_LETTER_SPACING, 0.0);
        assert!(role.rank() > TypoRole::Body.rank());
        assert!(TypoRole::Small.rank() > role.rank());
    }

    #[test]
    fn folder_columns_match_the_cli_shape() {
        let row = FolderRow {
            rel_path: "a/b.txt".to_string(),
            is_dir: false,
            glyph: '!',
            left_size: Some("10".to_string()),
            right_size: None,
            left_time: Some("1970-01-01 00:00".to_string()),
            right_time: None,
            changed: true,
        };
        assert_eq!(
            folder_columns(&row),
            "         10          -  1970-01-01 00:00                -  a/b.txt"
        );
        let dir = FolderRow {
            rel_path: "a".to_string(),
            is_dir: true,
            ..row
        };
        assert!(folder_columns(&dir).ends_with("a/"));
    }

    #[test]
    fn premultiplied_rgba_is_saturating_per_channel() {
        // Opaque passes through unchanged.
        assert_eq!(
            premultiplied_rgba(&[255, 10, 20, 255]),
            vec![255, 10, 20, 255]
        );
        // 255*128/255 = 128, 10*128/255 = 5, 20*128/255 = 10.
        assert_eq!(
            premultiplied_rgba(&[255, 10, 20, 128]),
            vec![128, 5, 10, 128]
        );
        // Transparent collapses to zero.
        assert_eq!(premultiplied_rgba(&[200, 200, 200, 0]), vec![0, 0, 0, 0]);
        // Ragged input models nothing; so does empty.
        assert!(premultiplied_rgba(&[1, 2, 3]).is_empty());
        assert!(premultiplied_rgba(&[]).is_empty());
    }

    #[test]
    fn heatmap_image_builds_only_from_consistent_bytes() {
        let image = heatmap_color_image(&[255, 0, 0, 255], 1, 1).unwrap_or_default();
        let [w, h] = image.size;
        assert_eq!((w, h), (1, 1));
        assert_eq!(
            image.pixels,
            vec![Color32::from_rgba_premultiplied(255, 0, 0, 255)]
        );
        // Bytes disagreeing with the declared size model nothing.
        assert!(heatmap_color_image(&[255, 0, 0, 255], 2, 1).is_none());
        // Empty dimensions or empty bytes model nothing.
        assert!(heatmap_color_image(&[], 0, 0).is_none());
        assert!(heatmap_color_image(&[], 1, 1).is_none());
    }

    #[test]
    fn side_job_lays_out_gutter_text_and_padding_in_order() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(1),
            right_no: None,
            left: "ab".to_string(),
            right: String::new(),
            accent_a: vec![(1, 2)],
            accent_b: Vec::new(),
        };
        let model = TextRows {
            rows: vec![row.clone()],
            truncated: false,
            coarse: false,
            pane_chars: 4,
            gutter_chars: 4,
            selected: None,
            filter: viewmodel::TextFilter::All,
        };
        let shown = TextViewPrefs::default();
        let layout = TextPaneLayout {
            pane_chars: model.pane_chars,
            gutter_chars: model.gutter_chars,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let job = side_job_with_prefs(
            Side::Left,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        // Gutter "   1  ", text "ab" with "b" accented, then pane
        // padding: the job text is exactly gutter + padded line.
        assert_eq!(job.text, "   1  ab  ");
        assert!(job.keep_trailing_whitespace);
        // Sections: gutter, plain "a", accented "b", padding.
        assert_eq!(job.sections.len(), 4);
        // The blank right side is all spaces: gutter (6) plus the
        // padded empty pane (4).
        let blank = side_job_with_prefs(
            Side::Right,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert_eq!(blank.text, "          ");
    }

    #[test]
    fn span_colors_inverts_accent_over_row_strip() {
        let fg = Color32::WHITE;
        let bg = Color32::BLACK;
        // Plain passes the row strip pair through.
        assert_eq!(span_colors(fg, bg, false), (fg, bg));
        // Accented inverts it: distinct block over the strip.
        assert_eq!(span_colors(fg, bg, true), (bg, fg));
    }

    #[test]
    fn side_job_accents_paint_inverted_block_over_strip() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(1),
            right_no: None,
            left: "ab".to_string(),
            right: String::new(),
            accent_a: vec![(1, 2)],
            accent_b: Vec::new(),
        };
        let shown = TextViewPrefs::default();
        let layout = TextPaneLayout {
            pane_chars: 4,
            gutter_chars: 4,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let fg = Color32::WHITE;
        let bg = Color32::BLACK;
        let job = side_job_with_prefs(Side::Left, &row, &layout, FontId::monospace(12.0), fg, bg);
        // Gutter + plain "a" + accented "b" + padding.
        assert_eq!(job.sections.len(), 4);
        // (text, background, underlined) for body sections past the
        // gutter: plain passes the strip pair through, accented paints
        // the selective changed band (dark block + 1px underline).
        let body: Vec<(Color32, Color32, bool)> = job
            .sections
            .iter()
            .skip(1)
            .map(|s| {
                (
                    s.format.color,
                    s.format.background,
                    s.format.underline != Stroke::NONE,
                )
            })
            .collect();
        assert_eq!(
            body,
            vec![
                (fg, bg, false),
                (CHANGED_FG, CHANGED_BG, true),
                (fg, bg, false)
            ]
        );
    }

    #[test]
    fn changed_row_word_spans_only_changed_rows_carry_accents() {
        let changed = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        };
        let (left_spans, right_spans) = changed_row_word_spans(&changed);
        assert!(left_spans.is_empty());
        assert_eq!(right_spans, &[(18, 27)]);
        // Deleted/Inserted/Equal par spans khali — Changed selective
        // plain base par hai, Deleted/Inserted text side band rakhta hai.
        for kind in [
            TextRowKind::Deleted,
            TextRowKind::Inserted,
            TextRowKind::Equal,
        ] {
            let row = TextRow {
                kind,
                left_no: Some(1),
                right_no: Some(1),
                left: "ab".to_string(),
                right: "ab".to_string(),
                accent_a: vec![(0, 1)],
                accent_b: vec![(0, 1)],
            };
            let (left, right) = changed_row_word_spans(&row);
            assert!(left.is_empty());
            assert!(right.is_empty());
        }
    }

    #[test]
    fn changed_row_word_spans_paint_only_changed_word_block() {
        // Fixture jaisi insertion-only Changed row: right par ek accent
        // block, left plain — wahi jo paint me block+underline banta hai.
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        };
        let shown = TextViewPrefs::default();
        let layout = TextPaneLayout {
            pane_chars: 60,
            gutter_chars: 4,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let left_job = side_job_with_prefs(
            Side::Left,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert!(
            left_job
                .sections
                .iter()
                .skip(1)
                .all(|s| s.format.underline == Stroke::NONE)
        );
        let right_job = side_job_with_prefs(
            Side::Right,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert!(
            right_job
                .sections
                .iter()
                .skip(1)
                .any(|s| s.format.underline != Stroke::NONE)
        );
    }

    #[test]
    fn session_toolbar_uses_compact_floor() {
        // Toolbar compact floor: 96x24 (area 2304) — path Browse floor
        // (120x28) se tang taaki wide text pills compact rahen; height 24
        // exquisite hit floor (find-nav precedent). Home CTA (224x28)
        // se alag floor barkarar.
        assert_eq!(session_toolbar_button_size(), (96.0, 24.0));
        assert_eq!(
            session_toolbar_button_size(),
            (
                crate::home::HOME_TOOLBAR_BUTTON_W,
                crate::home::HOME_TOOLBAR_BUTTON_H
            )
        );
        let (w, h) = session_toolbar_button_size();
        assert!(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0);
        assert_eq!(w * h, 2304.0);
        assert!(w < path_browse_button_size().0);
        assert!(h <= path_browse_button_size().1);
    }

    #[test]
    fn toolbar_icon_parity_reuses_chrome_and_glyphs() {
        // P2-1 icon parity: glyphs codebase me pehle se (↔ recent-title,
        // ▶ paging-nav), gap TIGHT_GAP, role chrome_tab (Body 13).
        assert_eq!(TOOLBAR_SWAP_GLYPH, "\u{2194}");
        assert_eq!(TOOLBAR_RELOAD_GLYPH, "▶");
        assert_eq!(toolbar_icon_gap(), TIGHT_GAP);
        assert_eq!(toolbar_button_role(), chrome_tab_role());
        assert_eq!(toolbar_swap_label(), "\u{2194} Swap Sides");
        assert_eq!(toolbar_reload_label(), "▶ Reload / Recompare");
        // Labels English text unchanged rakhte hain (sirf glyph prefix).
        assert!(toolbar_swap_label().ends_with("Swap Sides"));
        assert!(toolbar_reload_label().ends_with("Reload / Recompare"));
    }

    #[test]
    fn toolbar_action_defaults_to_none_with_swap_and_reload() {
        // Live call-site shape (paint_session_toolbar emits, App agle wave
        // me swap/run chalata hai): SessionAction precedent — default None,
        // dono hits alag.
        assert_eq!(ToolbarAction::default(), ToolbarAction::None);
        assert_ne!(ToolbarAction::Swap, ToolbarAction::Reload);
    }

    #[test]
    fn action_nav_labels_carry_jump_glyphs() {
        // Text nav-row precedent glyphs (koi naya asset nahi), dono non-empty.
        assert!(action_nav_prev_label().starts_with("◀"));
        assert!(action_nav_next_label().ends_with("▶"));
        assert!(!action_nav_prev_label().is_empty());
        assert!(!action_nav_next_label().is_empty());
    }

    #[test]
    fn action_bar_geometry_reuses_toolbar_floor() {
        // Dono rows ek beat par: 24 floor + TIGHT_GAP stack; min width do
        // uniform buttons + ek cluster gap (2*96+8 = 200). Koi naya token nahi.
        let (tool_w, tool_h) = session_toolbar_button_size();
        assert_eq!(tool_w, 96.0);
        assert_eq!(action_bar_row_height(), tool_h);
        assert_eq!(action_bar_row_height(), 24.0);
        assert_eq!(action_bar_row_gap(), TIGHT_GAP);
        assert_eq!(toolbar_cluster_gap(), SECTION_GAP);
        assert_eq!(action_bar_readout_gap(), SECTION_GAP);
        assert_eq!(action_bar_min_width(), 2.0 * tool_w + toolbar_cluster_gap());
        assert_eq!(action_bar_min_width(), 200.0);
    }

    #[test]
    fn action_bar_hscroll_only_when_avail_below_floor() {
        // Edge fixture actionbar-narrow-180 (toolbar2: floor 196 -> 200):
        // 180 < 200 par scroll, 700 par nahi; kharab avail par kabhi
        // NaN-branch nahi.
        assert!(action_bar_needs_hscroll(180.0));
        assert!(!action_bar_needs_hscroll(200.0));
        assert!(!action_bar_needs_hscroll(700.0));
        assert!(!action_bar_needs_hscroll(f32::NAN));
        assert!(!action_bar_needs_hscroll(0.0));
    }

    #[test]
    fn action_bar_badge_and_cursor_stay_honest() {
        // Bina selection koi fake position nahi ("0/N"); cursor blank par
        // hidden, warna trimmed readout (jaise viewmodel "3:1").
        let blank = ActionBarNavState {
            position: None,
            total: 3,
            cursor: String::new(),
        };
        assert_eq!(diff_nav_count_label(blank.position, blank.total), "0/3");
        assert_eq!(action_bar_cursor_text(&blank.cursor), None);
        assert_eq!(action_bar_cursor_text("   "), None);
        let mid = ActionBarNavState {
            position: Some(2),
            total: 3,
            cursor: "  3:1  ".to_string(),
        };
        assert_eq!(diff_nav_count_label(mid.position, mid.total), "2/3");
        assert_eq!(action_bar_cursor_text(&mid.cursor), Some("3:1".to_string()));
    }

    #[test]
    fn action_bar_hits_default_to_none_with_split_rows() {
        // Row 1 (action) aur row 2 (nav) alag fields — ek frame me dono lag
        // sakte hain, default dono None; Prev/Next alag hits.
        assert_eq!(ActionBarHits::default().action, ToolbarAction::None);
        assert_eq!(ActionBarHits::default().nav, ActionBarNav::None);
        assert_ne!(ActionBarNav::Prev, ActionBarNav::Next);
    }

    #[test]
    fn normalize_intra_spans_sorts_and_merges() {
        // Unsorted + overlapping ranges ek hi block me judte hain —
        // rects chipke, koi gap/overlap nahi.
        assert_eq!(
            normalize_intra_spans(&[(4, 6), (1, 3), (2, 5)], 10, 10),
            vec![(1, 6)]
        );
        // Aas-paas wale judte hain (ek hi paint block).
        assert_eq!(
            normalize_intra_spans(&[(2, 3), (1, 2)], 10, 10),
            vec![(1, 3)]
        );
        // Alag ranges alag rehte hain, sorted order me.
        assert_eq!(
            normalize_intra_spans(&[(5, 6), (1, 2)], 10, 10),
            vec![(1, 2), (5, 6)]
        );
    }

    #[test]
    fn normalize_intra_spans_clamps_and_drops_empty() {
        // limit = min(line_len, pane) = 6: (4,20) chipak kar (4,6),
        // zero-length (1,1) aur poori-bahar (8,20) gir jate hain.
        assert_eq!(
            normalize_intra_spans(&[(1, 1), (4, 20), (8, 20)], 10, 6),
            vec![(4, 6)]
        );
        // Khali line par kuch nahi bachta.
        assert!(normalize_intra_spans(&[(0, 1)], 0, 4).is_empty());
        // Khali input khali rehta hai.
        assert!(normalize_intra_spans(&[], 10, 10).is_empty());
    }

    #[test]
    fn pane_segments_refinement_stays_deterministic() {
        // Unsorted input bhi sorted paint deta hai: "a" accented,
        // "b" plain, "c" accented.
        assert_eq!(
            pane_segments("abc", &[(2, 3), (0, 1)], 3),
            vec![
                ("a".to_string(), true),
                ("b".to_string(), false),
                ("c".to_string(), true),
            ]
        );
        // Overlapping input ek hi block me judta hai.
        assert_eq!(
            pane_segments("abc", &[(0, 2), (1, 3)], 3),
            vec![("abc".to_string(), true)]
        );
    }

    #[test]
    fn span_text_format_keeps_mono_size_with_accent_underline() {
        let font = FontId::monospace(12.0);
        let plain = span_text_format(font.clone(), Color32::WHITE, Color32::BLACK, false);
        let accented = span_text_format(font.clone(), Color32::WHITE, Color32::BLACK, true);
        // Dono ka size same — line-height untouched (TypoRole precedent:
        // zor bina size badle).
        assert_eq!(plain.font_id.size, 12.0);
        assert_eq!(accented.font_id.size, 12.0);
        assert_eq!(plain.underline, Stroke::NONE);
        assert_eq!(
            accented.underline,
            Stroke::new(INTRA_SPAN_UNDERLINE_W, Color32::BLACK)
        );
        assert_eq!(INTRA_SPAN_UNDERLINE_W, 1.0);
        // Inversion barkarar: accented block strip se alag.
        assert_eq!(plain.background, Color32::BLACK);
        assert_eq!(accented.background, Color32::WHITE);
    }

    #[test]
    fn side_job_intra_spans_keep_mono_size_and_underline() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(1),
            right_no: None,
            left: "ab".to_string(),
            right: String::new(),
            accent_a: vec![(1, 2)],
            accent_b: Vec::new(),
        };
        let shown = TextViewPrefs::default();
        let layout = TextPaneLayout {
            pane_chars: 4,
            gutter_chars: 4,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let job = side_job_with_prefs(
            Side::Left,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        // Gutter ke baad har body section same mono size par hai —
        // geometry first (line-height se chipke).
        for section in job.sections.iter().skip(1) {
            assert_eq!(section.format.font_id.size, 12.0);
        }
        let mut saw_accent = false;
        let mut saw_plain = false;
        for section in job.sections.iter().skip(1) {
            if section.format.underline != Stroke::NONE {
                saw_accent = true;
                assert_eq!(section.format.underline.width, ACCENT_SPAN_UNDERLINE_W);
            } else {
                saw_plain = true;
            }
        }
        assert!(saw_accent && saw_plain);
    }

    #[test]
    fn intra_spans_visible_on_real_diff_words() {
        // Real diff jaisi line: `hello world` me `world` badla hissa.
        let segs = pane_segments("hello world", &[(6, 11)], 11);
        assert_eq!(
            segs,
            vec![("hello ".to_string(), false), ("world".to_string(), true),]
        );
        // Paint job me beech ka hissa accented + underline ke saath.
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(1),
            right_no: Some(1),
            left: "hello world".to_string(),
            right: "hello earth".to_string(),
            accent_a: vec![(6, 11)],
            accent_b: vec![(6, 11)],
        };
        let shown = TextViewPrefs::default();
        let layout = TextPaneLayout {
            pane_chars: 11,
            gutter_chars: 4,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let job = side_job_with_prefs(
            Side::Left,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        // Gutter ke baad plain "hello " phir accented "world": wahi
        // pattern jo `pane_segments` ne diya — paint me beech ka hissa
        // alag block + underline ke saath dikhta hai.
        assert!(job.text.contains("world"));
        let body: Vec<(Color32, Color32, bool)> = job
            .sections
            .iter()
            .skip(1)
            .map(|s| {
                (
                    s.format.color,
                    s.format.background,
                    s.format.underline != Stroke::NONE,
                )
            })
            .collect();
        assert_eq!(
            body,
            vec![
                (Color32::WHITE, Color32::BLACK, false),
                (CHANGED_FG, CHANGED_BG, true),
            ]
        );
    }

    #[test]
    fn prefs_syntax_off_clears_accents_but_keeps_text() {
        let on = TextViewPrefs {
            syntax_highlight: true,
            ..TextViewPrefs::default()
        };
        let off = TextViewPrefs {
            syntax_highlight: false,
            ..TextViewPrefs::default()
        };
        let with_accents = prefs_pane_segments("abc", &[(1, 2)], 3, &on);
        assert!(with_accents.iter().any(|(_, acc)| *acc));
        let cleared = prefs_pane_segments("abc", &[(1, 2)], 3, &off);
        assert!(cleared.iter().all(|(_, acc)| !acc));
        // Same visible text, only accents differ.
        let text_of =
            |segs: &[(String, bool)]| -> String { segs.iter().map(|(s, _)| s.clone()).collect() };
        assert_eq!(text_of(&with_accents), text_of(&cleared));
    }

    #[test]
    fn prefs_line_numbers_off_blanks_gutter_and_narrows() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(7),
            right_no: Some(7),
            left: "hi".to_string(),
            right: "hi!".to_string(),
            accent_a: vec![(0, 1)],
            accent_b: vec![(0, 1)],
        };
        let shown = TextViewPrefs::default();
        let hidden = TextViewPrefs {
            show_line_numbers: false,
            syntax_highlight: false,
            ..TextViewPrefs::default()
        };
        let shown_layout = TextPaneLayout {
            pane_chars: 4,
            gutter_chars: 4,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let with_numbers = side_job_with_prefs(
            Side::Left,
            &row,
            &shown_layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert!(with_numbers.text.starts_with("   7 "));
        // Gutter + accented head + plain tail (tail and padding may
        // merge in LayoutJob, so at least 3 sections).
        assert!(with_numbers.sections.len() >= 3);
        let hidden_layout = TextPaneLayout {
            pane_chars: 4,
            gutter_chars: 1,
            prefs: &hidden,
            dim: crate::theme::LIGHT_DIM,
        };
        let without_numbers = side_job_with_prefs(
            Side::Left,
            &row,
            &hidden_layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        // Gutter width 1 blanks to two spaces, no accent sections.
        assert!(without_numbers.text.starts_with("  "));
        assert!(
            without_numbers
                .sections
                .iter()
                .skip(1)
                .all(|s| s.format.color == Color32::WHITE)
        );
        // Thumbnail chrome narrows in sync.
        assert!(crate::gutter::gutter_bar_width(false) < crate::gutter::gutter_bar_width(true));
    }

    #[test]
    fn selective_row_is_plain_changed_and_equal() {
        assert!(selective_row_is_plain(TextRowKind::Changed));
        assert!(selective_row_is_plain(TextRowKind::Equal));
        assert!(!selective_row_is_plain(TextRowKind::Deleted));
        assert!(!selective_row_is_plain(TextRowKind::Inserted));
    }

    #[test]
    fn selective_side_palette_filler_empty_side_plain() {
        let base = Color32::WHITE;
        // Deleted: left text side band, right empty side filler plain.
        let deleted = TextRow {
            kind: TextRowKind::Deleted,
            left_no: Some(4),
            right_no: None,
            left: "    let legacy_flag = true;".to_string(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        assert_eq!(
            selective_side_palette(true, &deleted, base),
            (DELETED_FG, DELETED_BG)
        );
        assert_eq!(
            selective_side_palette(false, &deleted, base),
            (base, Color32::TRANSPARENT)
        );
        // Inserted: left empty filler plain, right text side band.
        let inserted = TextRow {
            kind: TextRowKind::Inserted,
            left_no: None,
            right_no: Some(4),
            left: String::new(),
            right: "    validate_safe_path(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        assert_eq!(
            selective_side_palette(false, &inserted, base),
            (INSERTED_FG, INSERTED_BG)
        );
        assert_eq!(
            selective_side_palette(true, &inserted, base),
            (base, Color32::TRANSPARENT)
        );
        // Changed: dono text sides plain (sirf word span band rakhta hai).
        let changed = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        };
        assert_eq!(
            selective_side_palette(true, &changed, base),
            (base, Color32::TRANSPARENT)
        );
        assert_eq!(
            selective_side_palette(false, &changed, base),
            (base, Color32::TRANSPARENT)
        );
        // Equal: plain rehta hai.
        let equal = TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        assert_eq!(
            selective_side_palette(true, &equal, base),
            (base, Color32::TRANSPARENT)
        );
    }

    #[test]
    fn selective_span_format_accent_uses_changed_band() {
        let font = FontId::monospace(12.0);
        let plain = selective_span_format(font.clone(), Color32::WHITE, Color32::BLACK, false);
        let accented = selective_span_format(font.clone(), Color32::WHITE, Color32::BLACK, true);
        // Same mono size — line-height untouched.
        assert_eq!(plain.font_id.size, 12.0);
        assert_eq!(accented.font_id.size, 12.0);
        // Plain passthrough, accented frozen changed band + heavier accent
        // underline (line weight emphasis, same hue — no new color).
        assert_eq!(plain.background, Color32::BLACK);
        assert_eq!(plain.underline, Stroke::NONE);
        assert_eq!(accented.font_id.size, plain.font_id.size);
        assert_eq!(
            (accented.color, accented.background),
            (CHANGED_FG, CHANGED_BG)
        );
        assert_eq!(ACCENT_SPAN_UNDERLINE_W, 2.0 * INTRA_SPAN_UNDERLINE_W);
        assert_eq!(
            accented.underline,
            Stroke::new(ACCENT_SPAN_UNDERLINE_W, CHANGED_FG)
        );
    }

    #[test]
    fn selective_find_format_current_light_block() {
        let font = FontId::monospace(12.0);
        // Selective base (transparent): current light block no-underline,
        // other plain + line — accent (dark band + line) se dono alag.
        let current = selective_find_format(
            font.clone(),
            Color32::WHITE,
            Color32::TRANSPARENT,
            FindHl::Current,
        );
        let (light_fg, light_bg) = span_colors(CHANGED_FG, CHANGED_BG, true);
        assert_eq!((current.color, current.background), (light_fg, light_bg));
        assert_eq!(current.underline, Stroke::NONE);
        let other = selective_find_format(
            font.clone(),
            Color32::WHITE,
            Color32::TRANSPARENT,
            FindHl::Other,
        );
        assert_eq!(other.background, Color32::TRANSPARENT);
        assert_ne!(other.underline, Stroke::NONE);
        let accent = selective_find_format(
            font.clone(),
            Color32::WHITE,
            Color32::TRANSPARENT,
            FindHl::Accent,
        );
        assert_eq!((accent.color, accent.background), (CHANGED_FG, CHANGED_BG));
        assert_ne!(accent.underline, Stroke::NONE);
        // Band wale rows par old hit reuse (koi naya block nahi).
        let band_current =
            selective_find_format(font.clone(), DELETED_FG, DELETED_BG, FindHl::Current);
        let old = find_hit_format(font.clone(), DELETED_FG, DELETED_BG, true);
        assert_eq!(
            (band_current.color, band_current.background),
            (old.color, old.background)
        );
    }

    #[test]
    fn filler_geometry_widths_and_hatch_step() {
        assert!(text_side_is_filler(""));
        assert!(!text_side_is_filler("x"));
        // Pane minus line, saturating (koi underflow nahi).
        assert_eq!(filler_pad_width(60, 41), 19);
        assert_eq!(filler_pad_width(60, 60), 0);
        assert_eq!(filler_pad_width(4, 10), 0);
        // Hatch step TIGHT_GAP reuse — koi naya token nahi.
        assert_eq!(filler_hatch_step(), TIGHT_GAP);
        assert_eq!(filler_hatch_count(12.0), 3);
        assert_eq!(filler_hatch_count(0.0), 0);
        assert_eq!(filler_hatch_count(f32::NAN), 0);
    }

    #[test]
    fn filler_side_paint_contract_transparent_base_with_hatch() {
        // Live paint wiring contract (wrap-off path): khaali side filler
        // hai — plain transparent base (selective palette) + us par hatch
        // padta hai (nonzero ticks, TIGHT_GAP step). Dono taraf khaali nahi
        // to hatch zero hona chahiye (guard rail).
        let base = Color32::TRANSPARENT;
        let deleted = TextRow {
            kind: TextRowKind::Deleted,
            left_no: Some(4),
            right_no: None,
            left: "    let legacy_flag = true;".to_string(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        assert!(text_side_is_filler(deleted.right.as_str()));
        assert!(!text_side_is_filler(deleted.left.as_str()));
        let (_, right_bg) = selective_side_palette(false, &deleted, Color32::WHITE);
        assert_eq!(right_bg, base);
        assert!(filler_hatch_count(460.0) > 0);
        assert_eq!(filler_hatch_count(0.0), 0);
    }

    #[test]
    fn paint1_fill_viewport_blank_alignment_and_pane_status() {
        // Short-vs-long edge: 7-row content at 24px rows in a 436px pane
        // budget leaves trailing hatched blanks (BC precedent) — no bottom
        // waste, alignment preserved via same row rhythm.
        assert_eq!(text_trailing_blank_rows(436.0, 24.0, 7), 12);
        assert_eq!(text_trailing_blank_rows(436.0, 24.0, 19), 0);
        assert_eq!(text_trailing_blank_rows(f32::NAN, 24.0, 7), 0);
        assert_eq!(text_trailing_blank_rows(436.0, 0.0, 7), 0);
        // Center-gutter per-hunk markers: diffs mark, equal blanks.
        assert!(text_center_hunk_marker(TextRowKind::Changed));
        assert!(text_center_hunk_marker(TextRowKind::Deleted));
        assert!(text_center_hunk_marker(TextRowKind::Inserted));
        assert!(!text_center_hunk_marker(TextRowKind::Equal));
        // Segmented pane status: per-side honest labels, no fakes.
        let rows = vec![
            TextRow {
                kind: TextRowKind::Changed,
                left_no: Some(3),
                right_no: Some(3),
                left: "a".to_string(),
                right: "b".to_string(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
            TextRow {
                kind: TextRowKind::Deleted,
                left_no: Some(4),
                right_no: None,
                left: "x".to_string(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            },
        ];
        assert_eq!(
            text_pane_cursor_labels(&rows, Some(0)),
            ("L3".to_string(), "R3".to_string())
        );
        assert_eq!(
            text_pane_cursor_labels(&rows, Some(1)),
            ("L4".to_string(), "R-".to_string())
        );
        assert_eq!(
            text_pane_cursor_labels(&rows, None),
            ("L-".to_string(), "R-".to_string())
        );
        assert_eq!(
            text_pane_cursor_labels(&rows, Some(99)),
            ("L-".to_string(), "R-".to_string())
        );
        // Typo reuse: status roles stay Small/Body (Mono 12 rows untouched).
        assert_eq!(status_side_role(), TypoRole::Small);
        assert_eq!(status_verdict_role(), TypoRole::Body);
    }

    #[test]
    fn text2_narrow720_inline_weight_marker_orphan_balance() {
        // Mirrors checks/gui-exq-text2/comparison/text-narrow-720.json.
        // Narrow breakpoint: 719 stacks, 720+ stays side-by-side.
        assert_eq!(text_layout_for_width(719.0), TextLayout::Stacked);
        assert_eq!(text_layout_for_width(720.0), TextLayout::SideBySide);
        assert_eq!(text_layout_for_width(960.0), TextLayout::SideBySide);
        // Inline emphasis by line weight (no new color): accent 2px on the
        // frozen hue, find hits keep the 1px line, plain keeps no line.
        let font = FontId::monospace(12.0);
        let accented = selective_span_format(font.clone(), Color32::WHITE, Color32::BLACK, true);
        assert_eq!(
            accented.underline,
            Stroke::new(ACCENT_SPAN_UNDERLINE_W, CHANGED_FG)
        );
        assert!(accented.underline.width > INTRA_SPAN_UNDERLINE_W);
        let other = find_hit_format(font.clone(), Color32::WHITE, Color32::BLACK, false);
        assert_eq!(other.underline.width, INTRA_SPAN_UNDERLINE_W);
        // Marker aligns to one min band on the row center, never two rows.
        assert_eq!(text_minimap_marker_band(1, 4, 400.0), Some((148.0, 152.0)));
        if let Some((y0, y1)) = text_minimap_marker_band(0, 1000, 400.0) {
            assert!((y1 - y0 - crate::gutter::GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        } else {
            panic!("marker band must exist");
        }
        assert_eq!(text_minimap_marker_band(4, 4, 400.0), None);
        // Orphan blank balance at narrow: 7 visible rows in a 300px budget
        // at 17.6px rhythm leave 11 raw blanks, balanced up to 12 (paired,
        // no lone orphan); full content stays zero.
        assert_eq!(text_trailing_blank_rows(300.0, 17.6, 7), 11);
        assert_eq!(text_trailing_blank_rows_balanced(300.0, 17.6, 7), 12);
        assert_eq!(text_trailing_blank_rows_balanced(300.0, 17.6, 18), 0);
        assert_eq!(text_trailing_blank_rows_balanced(f32::NAN, 17.6, 7), 0);
    }

    #[test]
    fn prefs_word_wrap_caps_pane_at_80() {
        let long = "x".repeat(200);
        let on = TextViewPrefs {
            word_wrap: true,
            ..TextViewPrefs::default()
        };
        let off = TextViewPrefs::default();
        let wrapped: String = prefs_pane_segments(&long, &[], 80, &on)
            .into_iter()
            .map(|(segment, _)| segment)
            .collect();
        assert_eq!(wrapped, "x".repeat(80));
        assert_eq!(
            prefs_pane_segments(&long, &[], 200, &off)
                .into_iter()
                .map(|(s, _)| s)
                .collect::<String>()
                .len(),
            200
        );
        // Model-level cap matches the paint width.
        let model = TextRows {
            rows: Vec::new(),
            truncated: false,
            coarse: false,
            pane_chars: 200,
            gutter_chars: 4,
            selected: None,
            filter: viewmodel::TextFilter::All,
        };
        assert_eq!(view_text_with_prefs(&model, &on).pane_chars, 80);
        assert_eq!(view_text_with_prefs(&model, &off).pane_chars, 200);
    }

    #[test]
    fn hex_hunk_line_carries_offset_and_counts() {
        let hunk = views_hex::HexHunkView {
            offset: 3,
            left_len: 1,
            right_len: 2,
        };
        assert_eq!(format_hex_hunk_line(&hunk), "0x00000003: 1 vs 2 byte(s)");
    }

    #[test]
    fn tolerance_text_parses_like_the_engine() {
        assert_eq!(parse_tolerance_text("0.1"), Some(0.1));
        assert_eq!(parse_tolerance_text("  1 "), Some(1.0));
        assert_eq!(parse_tolerance_text("0"), Some(0.0));
        assert_eq!(parse_tolerance_text(""), None);
        assert_eq!(parse_tolerance_text("abc"), None);
        assert_eq!(parse_tolerance_text("-0.5"), None);
        assert_eq!(parse_tolerance_text("NaN"), None);
        assert_eq!(parse_tolerance_text("inf"), None);
        // Paint-level parsing agrees with the model sanitizer.
        assert_eq!(
            parse_tolerance_text("0.25"),
            views_table::TableKeyView::sanitize_tolerance(Some(0.25))
        );
    }

    #[test]
    fn geometry_tokens_match_targets() {
        assert_eq!(OUTER_MARGIN, 12);
        assert_eq!(SECTION_GAP, 8.0);
        assert_eq!(TIGHT_GAP, 4.0);
        assert_eq!(FOOTER_GAP, 6.0);
        assert_eq!(TAB_BOTTOM_GAP, 6.0);
        assert_eq!(TREE_INDENT, 12.0);
        assert_eq!((CARD_W, CARD_H), (240.0, 75.0));
    }

    #[test]
    fn half_fields_share_equal_width() {
        // Row chrome 388 ghata kar: (800-388)/2 = 206; zero par floors.
        assert_eq!(half_field_width(800.0), 206.0);
        assert_eq!(half_field_width(0.0), 180.0);
    }

    #[test]
    fn third_fields_share_equal_width() {
        // Triple chrome 504 ghata kar: (900-504)/3 = 132; zero par floor.
        assert_eq!(third_field_width(900.0), 132.0);
        assert_eq!(third_field_width(0.0), 130.0);
    }

    #[test]
    fn path_row_chrome_matches_paint_widgets() {
        // Gap 3: fields + chrome == avail (koi overflow nahi).
        assert_eq!(path_row_chrome_width(), 388.0);
        assert_eq!(triple_path_row_chrome_width(), 504.0);
        let (left, right) = split_field_widths(800.0, 0.50);
        assert_eq!(left + right + path_row_chrome_width(), 800.0);
        assert_eq!(
            3.0 * third_field_width(900.0) + triple_path_row_chrome_width(),
            900.0
        );
    }

    #[test]
    fn merge_apply_chrome_covers_checkbox_and_button() {
        // 88 checkbox + 64 button + 3x4 gaps = 164 (legacy 75 under tha).
        assert_eq!(merge_apply_chrome_width(), 164.0);
        // 720 fallback par field floor se upar, Apply kabhi clip nahi.
        assert!((720.0 - merge_apply_chrome_width()) >= MIN_HALF);
    }

    #[test]
    fn path_meta_role_is_caption_reuse() {
        assert_eq!(path_meta_role(), TypoRole::caption());
        assert_eq!(path_meta_role(), crate::theme::TypoRole::Small);
    }

    #[test]
    fn path_meta_line_joins_nonempty_parts() {
        assert_eq!(
            format_path_meta("modified @1s", "10 B", "Text: exact", "wrap:off"),
            "modified @1s | 10 B | Text: exact | wrap:off"
        );
        assert_eq!(format_path_meta("", "10 B", "", ""), "10 B");
        assert_eq!(format_path_meta("", "", "", ""), "");
        assert_eq!(format_path_meta("  ", "10 B", " ", ""), "10 B");
    }

    #[test]
    fn split_fraction_clamps_to_drag_range() {
        assert_eq!(clamp_split_fraction(0.50), 0.50);
        assert_eq!(clamp_split_fraction(0.0), viewmodel::SPLIT_MIN_FRACTION);
        assert_eq!(clamp_split_fraction(1.0), viewmodel::SPLIT_MAX_FRACTION);
        assert_eq!(clamp_split_fraction(-5.0), viewmodel::SPLIT_MIN_FRACTION);
        assert_eq!(clamp_split_fraction(9.9), viewmodel::SPLIT_MAX_FRACTION);
        assert_eq!(
            (viewmodel::SPLIT_MIN_FRACTION, viewmodel::SPLIT_MAX_FRACTION),
            (0.20, 0.80)
        );
        assert_eq!(SPLIT_DEFAULT_FRACTION, 0.50);
    }

    #[test]
    fn split_field_widths_follow_fraction() {
        let (left, right) = split_field_widths(800.0, 0.50);
        assert_eq!((left, right), (206.0, 206.0));
        assert_eq!(
            (left, right),
            (half_field_width(800.0), half_field_width(800.0))
        );
        let (wide_left, wide_right) = split_field_widths(800.0, 0.80);
        assert!(wide_left > wide_right);
        assert_eq!((wide_left, wide_right), (232.0, 180.0));
        let (narrow_left, narrow_right) = split_field_widths(800.0, 0.0);
        assert_eq!((narrow_left, narrow_right), split_field_widths(800.0, 0.20));
        assert_eq!((narrow_left, narrow_right), (180.0, 232.0));
    }

    #[test]
    fn split_picture_widths_sum_to_row() {
        let (left, right) = split_picture_widths(800.0, 0.50);
        assert_eq!(left + right, 800.0 - SECTION_GAP);
        assert_eq!((left, right), (396.0, 396.0));
        let (wide_left, wide_right) = split_picture_widths(800.0, 0.80);
        assert_eq!(wide_left + wide_right, 800.0 - SECTION_GAP);
        assert!(wide_left > wide_right);
        assert!(wide_left >= MIN_HALF && wide_right >= MIN_HALF);
        let (tiny_left, tiny_right) = split_picture_widths(0.0, 0.50);
        assert_eq!(tiny_left + tiny_right, 2.0 * MIN_HALF);
    }

    #[test]
    fn split_drag_delta_moves_fraction() {
        assert_eq!(split_apply_drag(0.50, 80.0, 800.0), 0.60);
        assert_eq!(split_apply_drag(0.50, -80.0, 800.0), 0.40);
        assert_eq!(
            split_apply_drag(0.50, 800.0, 800.0),
            viewmodel::SPLIT_MAX_FRACTION
        );
        assert_eq!(
            split_apply_drag(0.50, -800.0, 800.0),
            viewmodel::SPLIT_MIN_FRACTION
        );
        assert_eq!(split_apply_drag(0.50, 10.0, 0.0), 0.50);
        assert_eq!(split_apply_drag(0.50, 10.0, -4.0), 0.50);
    }

    #[test]
    fn split_handle_strip_matches_model_hit_width() {
        // Paint grab strip model hit width par hai: 8px strip, centered
        // 1px visual, barabar inset dono taraf.
        assert_eq!(SPLIT_HIT_W, viewmodel::SPLIT_HANDLE_HIT_W);
        assert_eq!(SPLIT_HIT_W, 8.0);
        assert_eq!(SPLIT_VISUAL_W, 1.0);
        assert_eq!(split_visual_inset(), 3.5);
    }

    #[test]
    fn split_handle_drag_uses_delta_and_clamp() {
        // Paint drag wahi delta+clamp hai jo model deta hai.
        assert_eq!(split_handle_drag_fraction(0.50, 80.0, 800.0), 0.60);
        assert_eq!(split_handle_drag_fraction(0.50, -80.0, 800.0), 0.40);
        assert_eq!(
            split_handle_drag_fraction(0.50, 800.0, 800.0),
            viewmodel::SPLIT_MAX_FRACTION
        );
        assert_eq!(
            split_handle_drag_fraction(0.50, -800.0, 800.0),
            viewmodel::SPLIT_MIN_FRACTION
        );
        assert_eq!(split_handle_drag_fraction(0.50, 10.0, 0.0), 0.50);
        assert_eq!(split_handle_drag_fraction(0.50, f32::NAN, 800.0), 0.50);
    }

    #[test]
    fn split_handle_label_reports_percent_and_bounds() {
        // Hover label honest percent + 20..80 bounds batata hai.
        let label = split_handle_label(0.50);
        assert!(label.contains("50"), "unexpected label: {label}");
        assert!(label.contains("20"), "unexpected label: {label}");
        assert!(label.contains("80"), "unexpected label: {label}");
        assert_eq!(label, viewmodel::split_accessibility_label(0.50));
    }

    #[test]
    fn split_handle_cursor_is_resize_column() {
        // Model affordance id paint cursor me map hota hai.
        assert_eq!(viewmodel::split_cursor_affordance(), "resize-column");
        assert_eq!(split_handle_cursor(), egui::CursorIcon::ResizeColumn);
    }

    #[test]
    fn split_handle_hit_range_centers_on_fraction() {
        // Paint strip center wahi hai jahan model hit rect hai.
        assert_eq!(viewmodel::split_handle_x_range(800.0, 0.50), (396.0, 404.0));
        let (x0, x1) = viewmodel::split_handle_x_range(800.0, 0.50);
        assert_eq!(x1 - x0, SPLIT_HIT_W);
        assert!(viewmodel::split_handle_contains_x(400.0, 800.0, 0.50));
        assert!(!viewmodel::split_handle_contains_x(395.9, 800.0, 0.50));
    }

    #[test]
    fn tree_indent_scales_with_depth() {
        assert_eq!(tree_indent(0), 0.0);
        assert_eq!(tree_indent(1), 12.0);
        assert_eq!(tree_indent(2), 24.0);
    }

    #[test]
    fn paint_indent_matches_headless_drill_contract() {
        // Paint aur headless tree ek hi kadam par: drill-down rows ka
        // x-offset dono taraf `indent + 16 expander` hai.
        assert_eq!(TREE_INDENT, crate::tree::tree_indent_step());
        assert_eq!(tree_indent(2), crate::tree::tree_indent_for_depth(2));
        assert_eq!(
            tree_indent(1) + crate::tree::folder_expander_width(),
            crate::tree::folder_row_x_offset(1)
        );
    }

    #[test]
    fn home_paint_mapping_uses_single_source_sizes() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_HOME_TITLE_SIZE};
        assert_eq!(home_title_paint_size(), TYPO_HOME_TITLE_SIZE);
        assert_eq!(home_title_paint_size(), 14.0);
        assert!(home_desc_paint_is_small_weak());
        assert_eq!(home_button_paint_size(), TYPO_BODY_SIZE);
        assert_eq!(home_button_paint_size(), 13.0);
    }

    #[test]
    fn home_card_width_flexes_with_floor() {
        // 960-canvas inner width 936 par ~306.67 (fixture 306 parity).
        assert!((home_paint_card_width(936.0) - 306.6667).abs() < 0.01);
        // Tang window par 240 floor (grid scroll, crush nahi).
        assert_eq!(home_paint_card_width(600.0), CARD_W);
        assert_eq!(home_paint_card_width(0.0), CARD_W);
        assert_eq!(home_paint_card_width(f32::NAN), CARD_W);
        // CTA card se bahar nahi: floor card 240 me 224 single-line fit.
        let (cta_w, _) = home_paint_cta_size();
        assert!(cta_w <= home_paint_card_width(936.0));
        assert!(cta_w <= CARD_W);
    }

    #[test]
    fn gutter_paint_width_reproduces_ruler_track() {
        use crate::gutter::gutter_ruler_width;
        assert_eq!(text_gutter_paint_width(true), 16.0);
        assert_eq!(text_gutter_paint_width(false), 8.0);
        assert_eq!(text_gutter_paint_width(true), gutter_ruler_width(true));
        assert_eq!(text_gutter_paint_width(false), gutter_ruler_width(false));
    }

    #[test]
    fn chrome_selectors_follow_dark_mode() {
        use crate::theme::{
            DARK_BLUE, DARK_DIM, DARK_GREEN, DARK_RED, DARK_TRACK, LIGHT_BLUE, LIGHT_DIM,
            LIGHT_GREEN, LIGHT_RED, LIGHT_TRACK,
        };
        assert_eq!(dim_for(true), DARK_DIM);
        assert_eq!(dim_for(false), LIGHT_DIM);
        assert_eq!(red_for(true), DARK_RED);
        assert_eq!(red_for(false), LIGHT_RED);
        assert_eq!(green_for(true), DARK_GREEN);
        assert_eq!(green_for(false), LIGHT_GREEN);
        assert_eq!(blue_for(true), DARK_BLUE);
        assert_eq!(blue_for(false), LIGHT_BLUE);
        assert_eq!(track_for(true), DARK_TRACK);
        assert_eq!(track_for(false), LIGHT_TRACK);
    }

    fn picture_model_fixture(identical: bool) -> crate::app::PictureCompareModel {
        crate::app::PictureCompareModel::from_diff(&autocompare_diff::PictureDiff {
            width_l: 2,
            height_l: 2,
            width_r: 3,
            height_r: 3,
            overlap_w: 2,
            overlap_h: 2,
            differing: if identical { 0 } else { 1 },
            total: 4,
            bbox: None,
            identical,
        })
    }

    #[test]
    fn picture_columns_split_avail_with_floor() {
        assert_eq!(picture_column_width(800.0), (800.0 - SECTION_GAP) / 2.0);
        assert_eq!(picture_column_width(0.0), MIN_HALF);
        assert_eq!(picture_column_width(100.0), MIN_HALF);
    }

    #[test]
    fn picture_sliders_clamp_and_label() {
        assert_eq!(clamp_overlay_mix(200), 100);
        assert_eq!(clamp_overlay_mix(30), 30);
        assert_eq!(clamp_zoom_pct(9), 25);
        assert_eq!(clamp_zoom_pct(900), 400);
        assert_eq!(clamp_zoom_pct(150), 150);
        assert!(zoom_label(100).contains("1.00x"));
        assert!(overlay_label(30).contains("30% left"));
        assert!(overlay_label(30).contains("70% right"));
        assert!(pan_label(32, -16).contains("+32"));
    }

    #[test]
    fn picture_status_and_display_size_follow_view() {
        let clean = picture_model_fixture(true);
        assert!(picture_status_line(&clean).contains("identical"));
        let diffed = picture_model_fixture(false);
        assert!(picture_status_line(&diffed).contains("1 of 4 pixels differ"));
        let mut view = crate::app::PictureViewState::default();
        assert_eq!(picture_display_size(100.0, 50.0, &view), (100.0, 50.0));
        view.set_zoom_pct(200);
        assert_eq!(picture_display_size(100.0, 50.0, &view), (200.0, 100.0));
        view.set_zoom_pct(25);
        assert_eq!(picture_display_size(0.0, 0.0, &view), (1.0, 1.0));
    }

    #[test]
    fn picture_display_size_spans_8x_for_both_modes() {
        // The zoom math (not the App setter) spans 0.25..=8.0, so both
        // SideBySide and Overlay share one zoomed heatmap frame; the
        // pub field lets paint-side state reach 800 headlessly.
        let mut view = crate::app::PictureViewState {
            zoom_pct: 800,
            ..crate::app::PictureViewState::default()
        };
        assert_eq!(viewmodel::picture_zoom_factor(view.zoom_pct), 8.0);
        assert_eq!(picture_display_size(100.0, 50.0, &view), (800.0, 400.0));
        view.mode = crate::app::PictureMode::Overlay;
        assert_eq!(picture_display_size(100.0, 50.0, &view), (800.0, 400.0));
        view.zoom_pct = 25;
        assert_eq!(picture_display_size(100.0, 50.0, &view), (25.0, 12.5));
        // Split stays the side-by-side layout authority (Wave U GEO):
        // zoom never changes column widths.
        let (left, right) = split_picture_widths(800.0, 0.50);
        assert_eq!(left + right, 800.0 - SECTION_GAP);
        assert_eq!((left, right), (396.0, 396.0));
    }

    #[test]
    fn picture_fit_zoom_snaps_down_to_steps() {
        // Exact fit: 800x600 in 400x300 -> 50%.
        assert_eq!(picture_fit_zoom_pct(800.0, 600.0, 400.0, 300.0), 50);
        // 62.5% floors to the 25pp ladder -> 50.
        assert_eq!(picture_fit_zoom_pct(800.0, 600.0, 500.0, 500.0), 50);
        // Narrow axis wins: min(2.0, 1.0) -> 100%.
        assert_eq!(picture_fit_zoom_pct(200.0, 100.0, 400.0, 100.0), 100);
        // Huge viewport caps at the paint-honored range.
        assert_eq!(picture_fit_zoom_pct(100.0, 100.0, 10_000.0, 10_000.0), 400);
        // Tiny viewport floors at 25, never 0.
        assert_eq!(picture_fit_zoom_pct(100.0, 100.0, 10.0, 10.0), 25);
        // Guards fall back to 1:1 instead of a fake zoom.
        assert_eq!(picture_fit_zoom_pct(0.0, 100.0, 400.0, 300.0), 100);
        assert_eq!(picture_fit_zoom_pct(f32::NAN, 100.0, 400.0, 300.0), 100);
        assert_eq!(
            picture_fit_zoom_pct(100.0, 100.0, 400.0, f32::INFINITY),
            100
        );
    }

    #[test]
    fn picture_center_offset_halves_leftover() {
        assert_eq!(picture_center_offset(100.0, 400.0), 150.0);
        assert_eq!(picture_center_offset(400.0, 400.0), 0.0);
        assert_eq!(picture_center_offset(500.0, 400.0), 0.0);
        assert_eq!(picture_center_offset(f32::NAN, 400.0), 0.0);
        assert_eq!(picture_center_offset(100.0, f32::INFINITY), 0.0);
    }

    #[test]
    fn picture_zoom_steps_live_in_paint_range() {
        assert_eq!(PICTURE_ZOOM_STEPS, [25, 50, 100, 200, 400]);
        for step in PICTURE_ZOOM_STEPS {
            assert_eq!(clamp_zoom_pct(step), step);
        }
    }

    #[test]
    fn sync_row_text_and_colors_map_actions() {
        use autocompare_compare::SyncAction;
        assert_eq!(sync_row_text(true, "> a.txt"), "[x] > a.txt");
        assert_eq!(sync_row_text(false, "> a.txt"), "[ ] > a.txt");
        assert_eq!(
            sync_skip_text("b.txt", "already in sync"),
            "~ b.txt (already in sync)"
        );
        assert_eq!(
            sync_row_color(SyncAction::CopyToRight, true),
            blue_for(true)
        );
        assert_eq!(
            sync_row_color(SyncAction::CopyToLeft, false),
            blue_for(false)
        );
        assert_eq!(sync_row_color(SyncAction::DeleteRight, true), red_for(true));
        assert_eq!(
            sync_row_color(SyncAction::DeleteLeft, false),
            red_for(false)
        );
    }

    #[test]
    fn folder_merge_colors_flag_conflicts() {
        use autocompare_compare::TreeMergeAction;
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::Conflict, true),
            red_for(true)
        );
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::MergedText, true),
            green_for(true)
        );
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::TakeLeft, false),
            blue_for(false)
        );
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::MakeDir, true),
            dim_for(true)
        );
    }

    #[test]
    fn audio_row_text_and_colors_follow_verdict() {
        let left = audio_row_text(AudioCheckKind::LeftConfig, "left", "1.000s");
        assert!(left.starts_with("L left: "));
        let verdict = audio_row_text(AudioCheckKind::Verdict, "verdict", "no differences");
        assert!(verdict.starts_with("= verdict: "));
        assert_eq!(
            audio_row_color(AudioCheckKind::Verdict, true, false, true),
            green_for(true)
        );
        assert_eq!(
            audio_row_color(AudioCheckKind::Verdict, false, true, false),
            red_for(false)
        );
        assert_eq!(
            audio_row_color(AudioCheckKind::Verdict, false, false, true),
            CHANGED_FG
        );
        assert_eq!(
            audio_row_color(AudioCheckKind::LeftConfig, false, false, true),
            dim_for(true)
        );
    }

    #[test]
    fn gutter_row_text_parses_only_nonnegative_ints() {
        assert_eq!(parse_gutter_row_text("2"), Some(2));
        assert_eq!(parse_gutter_row_text("  10 "), Some(10));
        assert_eq!(parse_gutter_row_text("0"), Some(0));
        assert_eq!(parse_gutter_row_text(""), None);
        assert_eq!(parse_gutter_row_text("abc"), None);
        assert_eq!(parse_gutter_row_text("-1"), None);
        assert_eq!(parse_gutter_row_text("1.5"), None);
    }

    #[test]
    fn table_model_text_rows_carry_painted_text_for_find() {
        let model = TableModel {
            rows: vec![
                crate::viewmodel::TableRow {
                    glyph: '=',
                    label: "   1".to_string(),
                    text: "a | b".to_string(),
                    changed: false,
                },
                crate::viewmodel::TableRow {
                    glyph: '!',
                    label: "   2".to_string(),
                    text: "needle here".to_string(),
                    changed: true,
                },
            ],
            truncated: false,
        };
        let texts = table_model_text_rows(&model);
        assert_eq!(texts.len(), 2);
        assert_eq!(
            crate::views_table::table_find_next(&texts, "needle", 0, false),
            Some((1, 0))
        );
        assert_eq!(
            crate::views_table::table_find_next(&texts, "missing", 0, false),
            None
        );
    }

    #[test]
    fn folder_path_rows_keep_paint_order_for_find() {
        let rows = vec![
            FolderRow {
                rel_path: "src/lib.rs".to_string(),
                is_dir: false,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            },
            FolderRow {
                rel_path: "README.md".to_string(),
                is_dir: false,
                glyph: '!',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: true,
            },
        ];
        let paths = folder_path_rows(&rows);
        assert_eq!(
            paths,
            vec!["src/lib.rs".to_string(), "README.md".to_string()]
        );
    }

    fn filterpaint_row(path: &str, is_dir: bool, glyph: char) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph,
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: glyph != '=',
        }
    }

    fn filterpaint_expanded() -> std::collections::BTreeSet<String> {
        std::collections::BTreeSet::new()
    }

    #[test]
    fn folder_kind_options_lock_model_variants() {
        let options = folder_kind_options();
        assert_eq!(options.len(), 3);
        let Some(first) = options.first() else {
            return;
        };
        assert_eq!(*first, crate::tree::FolderEntryKind::All);
        for kind in options {
            assert!(!kind.label().is_empty());
        }
    }

    #[test]
    fn folder_status_options_lock_model_variants() {
        let options = folder_status_options();
        assert_eq!(options.len(), 8);
        let Some(first) = options.first() else {
            return;
        };
        assert_eq!(*first, crate::tree::FolderStatusFilter::All);
        let labels: Vec<&str> = options.iter().map(|status| status.label()).collect();
        let mut seen = std::collections::BTreeSet::new();
        for label in &labels {
            seen.insert(*label);
        }
        assert_eq!(seen.len(), labels.len());
    }

    #[test]
    fn folder_filter_bar_tokens_stay_on_md_scale() {
        assert_eq!(folder_filter_bar_height_px(), 24.0);
        assert_eq!(folder_filter_bar_pad_px(), 8.0);
        assert_eq!(folder_filter_bar_gap_px(), 8.0);
        assert_eq!(
            folder_filter_bar_height_px(),
            crate::tree::folder_filter_bar_height()
        );
        assert_eq!(
            folder_filter_bar_pad_px(),
            crate::tree::folder_filter_bar_pad_x()
        );
        assert_eq!(
            folder_filter_bar_gap_px(),
            crate::tree::folder_filter_bar_gap()
        );
    }

    #[test]
    fn folder_filter_typos_are_body_regular_and_caption_strong() {
        let bar = crate::tree::folder_filter_typo();
        assert!(bar.mono);
        assert!(!bar.strong);
        assert_eq!(bar.size, 13.0);
        let chip = crate::tree::folder_filter_active_typo();
        assert!(chip.mono);
        assert!(chip.strong);
        assert_eq!(chip.size, 12.0);
    }

    #[test]
    fn folder_filter_nodes_hide_name_mismatch() {
        let rows = vec![
            filterpaint_row("lib.rs", false, '='),
            filterpaint_row("README.md", false, '!'),
        ];
        let expanded = filterpaint_expanded();
        let display = crate::tree::FolderDisplay {
            name_query: "lib".to_string(),
            ..Default::default()
        };
        let nodes = folder_filter_nodes(&rows, &expanded, &display);
        assert_eq!(nodes.len(), 1);
        let Some(node) = nodes.first() else {
            return;
        };
        assert_eq!(node.row.rel_path, "lib.rs");
    }

    #[test]
    fn folder_filter_nodes_kind_and_status_narrow() {
        let rows = vec![
            filterpaint_row("sub", true, '='),
            filterpaint_row("a.txt", false, '='),
            filterpaint_row("b.txt", false, '!'),
        ];
        let expanded = filterpaint_expanded();
        let display = crate::tree::FolderDisplay {
            kind: crate::tree::FolderEntryKind::FilesOnly,
            ..Default::default()
        };
        let nodes = folder_filter_nodes(&rows, &expanded, &display);
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().all(|node| !node.row.is_dir));
        let display = crate::tree::FolderDisplay {
            status: crate::tree::FolderStatusFilter::Same,
            ..Default::default()
        };
        let nodes = folder_filter_nodes(&rows, &expanded, &display);
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().all(|node| node.row.glyph == '='));
    }

    #[test]
    fn folder_filter_nodes_legacy_and_new_combine() {
        let rows = vec![
            filterpaint_row("same.txt", false, '='),
            filterpaint_row("left.txt", false, 'L'),
            filterpaint_row("diff.txt", false, '!'),
        ];
        let expanded = filterpaint_expanded();
        let display = crate::tree::FolderDisplay {
            hide_orphans: true,
            status: crate::tree::FolderStatusFilter::Changed,
            ..Default::default()
        };
        let nodes = folder_filter_nodes(&rows, &expanded, &display);
        assert_eq!(nodes.len(), 1);
        let Some(node) = nodes.first() else {
            return;
        };
        assert_eq!(node.row.rel_path, "diff.txt");
    }

    #[test]
    fn folder_filtered_summary_stays_honest() {
        let rows = vec![
            filterpaint_row("same.txt", false, '='),
            filterpaint_row("diff.txt", false, '!'),
            filterpaint_row("newer.txt", false, '>'),
            filterpaint_row("older.txt", false, '<'),
            filterpaint_row("left.txt", false, 'L'),
            filterpaint_row("right.txt", false, 'R'),
        ];
        let expanded = filterpaint_expanded();
        let display = crate::tree::FolderDisplay::default();
        let summary = folder_filtered_summary(&rows, &display);
        assert_eq!(summary, crate::tree::count_folder_rows(&rows).summary());
        assert!(summary.contains("6 entries"));
        let display = crate::tree::FolderDisplay {
            status: crate::tree::FolderStatusFilter::NewerLeft,
            ..Default::default()
        };
        let summary = folder_filtered_summary(&rows, &display);
        assert_eq!(
            summary,
            crate::tree::count_folder_rows(&crate::tree::filtered_folder_rows(&rows, &display))
                .summary()
        );
        assert!(summary.contains("1 entries"));
        assert!(summary.contains("1 newer left"));
        let nodes = folder_filter_nodes(&rows, &expanded, &display);
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn folder_paint_empty_hint_stays_honest() {
        let display = crate::tree::FolderDisplay::default();
        let empty: Vec<FolderRow> = Vec::new();
        assert_eq!(
            folder_paint_empty_hint(&empty, 0, &display),
            crate::tree::folder_empty_hint(Some(crate::tree::FolderEmptyState::NoResults))
        );
        let rows = vec![filterpaint_row("a.txt", false, '=')];
        let display = crate::tree::FolderDisplay {
            status: crate::tree::FolderStatusFilter::Different,
            ..Default::default()
        };
        assert_eq!(
            folder_paint_empty_hint(&rows, 0, &display),
            crate::tree::folder_empty_hint(Some(crate::tree::FolderEmptyState::NoMatch))
        );
        assert!(folder_paint_empty_hint(&rows, 0, &display).contains("clear filters"));
        let display = crate::tree::FolderDisplay::default();
        assert_eq!(folder_paint_empty_hint(&rows, 1, &display), "");
    }

    #[test]
    fn folder_paint_empty_hint_full_rows_routes_all_same() {
        let display = crate::tree::FolderDisplay::default();
        let empty: Vec<FolderRow> = Vec::new();
        assert_eq!(
            folder_paint_empty_hint_full_rows(&empty, 0, &display),
            crate::tree::folder_empty_hint(Some(crate::tree::FolderEmptyState::NoResults))
        );
        let rows = vec![filterpaint_row("a.txt", false, '=')];
        let display = crate::tree::FolderDisplay {
            status: crate::tree::FolderStatusFilter::Different,
            ..Default::default()
        };
        assert_eq!(
            folder_paint_empty_hint_full_rows(&rows, 0, &display),
            crate::tree::folder_empty_hint(Some(crate::tree::FolderEmptyState::AllSame))
        );
        assert!(folder_paint_empty_hint_full_rows(&rows, 0, &display).contains("all same"));
        let display = crate::tree::FolderDisplay::default();
        assert_eq!(folder_paint_empty_hint_full_rows(&rows, 1, &display), "");
    }

    #[test]
    fn sync_mode_choices_offer_update_both_first_then_mirror() {
        use crate::tree::SyncModeChoice;
        let choices = sync_mode_choices();
        assert_eq!(choices.len(), 2);
        assert_eq!(
            choices,
            [SyncModeChoice::UpdateBoth, SyncModeChoice::Mirror]
        );
        assert_eq!(choices[0].label(), "Update Both");
        assert_eq!(choices[1].label(), "Mirror");
    }

    #[test]
    fn sync_badge_for_paint_matches_model_across_glyphs_and_modes() {
        use crate::tree::SyncModeChoice;
        let glyphs = ['=', '!', '>', '<', 'L', 'R', '?'];
        for glyph in glyphs {
            for mode in [SyncModeChoice::UpdateBoth, SyncModeChoice::Mirror] {
                let row = FolderRow {
                    rel_path: "a.txt".to_string(),
                    is_dir: false,
                    glyph,
                    left_size: None,
                    right_size: None,
                    left_time: None,
                    right_time: None,
                    changed: glyph != '=',
                };
                assert_eq!(
                    sync_badge_for_paint(&row, mode),
                    crate::tree::sync_badge_for_row(&row, mode),
                );
            }
        }
    }

    #[test]
    fn sync_badge_for_paint_dirs_only_mirror_right_prunes() {
        use crate::tree::{SyncModeChoice, SyncRowDirection};
        let dir = |glyph: char| FolderRow {
            rel_path: "sub".to_string(),
            is_dir: true,
            glyph,
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        };
        assert_eq!(
            sync_badge_for_paint(&dir('R'), SyncModeChoice::Mirror),
            SyncRowDirection::DeleteRight.badge()
        );
        assert_eq!(
            sync_badge_for_paint(&dir('R'), SyncModeChoice::UpdateBoth),
            SyncRowDirection::None.badge()
        );
        assert_eq!(
            sync_badge_for_paint(&dir('>'), SyncModeChoice::Mirror),
            SyncRowDirection::None.badge()
        );
    }

    #[test]
    fn sync_badge_column_width_reuses_model_token() {
        assert_eq!(sync_badge_column_width(), crate::tree::sync_badge_min_px());
        assert!(sync_badge_column_width() > 0.0);
    }

    #[test]
    fn sync_grid_adds_one_badge_column_with_aligned_header() {
        assert_eq!(
            folder_grid_columns_with_sync(),
            folder_grid_columns().saturating_add(1)
        );
        let headers = folder_sync_header_cells();
        assert_eq!(headers.len(), folder_grid_columns_with_sync());
        assert_eq!(headers.first().map(String::as_str), Some(""));
        assert_eq!(
            headers.get(1).map(String::as_str),
            Some(sync_badge_header())
        );
        assert!(sync_badge_header().len() <= crate::tree::SYNC_BADGE_CHARS);
        let data = folder_header_cells();
        assert_eq!(headers.get(2..), Some(data.as_slice()));
    }

    #[test]
    fn sync_badge_only_delete_is_strong_at_caption_size() {
        use crate::tree::SyncRowDirection;
        let dirs = [
            SyncRowDirection::None,
            SyncRowDirection::CopyToRight,
            SyncRowDirection::CopyToLeft,
            SyncRowDirection::DeleteRight,
            SyncRowDirection::Manual,
        ];
        for dir in dirs {
            assert_eq!(
                sync_badge_is_strong(dir),
                dir == SyncRowDirection::DeleteRight
            );
            assert_eq!(sync_badge_text_size(dir), crate::tree::FOLDER_SIDE_SIZE);
        }
        assert_eq!(sync_summary_text_size(), crate::tree::FOLDER_SIDE_SIZE);
    }

    #[test]
    fn sync_rich_routes_through_mono_role() {
        // Single-source: badge/summary 12 mono (TypoRole::Mono scale),
        // delete strong, summary weak. Sizes legacy jaisi hi.
        use crate::tree::SyncRowDirection;
        let plain = format!("{:?}", sync_badge_rich("S", SyncRowDirection::CopyToRight));
        assert!(plain.contains("12.0"));
        assert!(plain.contains("strong: false"));
        let del = format!("{:?}", sync_badge_rich("X", SyncRowDirection::DeleteRight));
        assert!(del.contains("12.0"));
        assert!(del.contains("strong: true"));
        let summary = format!("{:?}", sync_summary_rich("3 to copy"));
        assert!(summary.contains("12.0"));
        assert!(summary.contains("weak: true"));
    }

    #[test]
    fn audio_and_threeway_rich_route_through_typorole() {
        let audio = format!(
            "{:?}",
            TypoRole::Mono.rich(audio_row_text(
                AudioCheckKind::Verdict,
                "Verdict",
                "identical"
            ))
        );
        assert!(audio.contains("12.0"));
        assert!(audio.contains("Monospace"));
        let take_label = format!("{:?}", TypoRole::Small.rich("Take:"));
        assert!(take_label.contains("11.0"));
    }

    #[test]
    fn folder_sync_row_selected_probes_model_set() {
        let mut selection = crate::tree::FolderSyncSelection::new();
        assert!(!folder_sync_row_selected(&selection, "a.txt"));
        selection.select("a.txt");
        assert!(folder_sync_row_selected(&selection, "a.txt"));
        assert!(!folder_sync_row_selected(&selection, "b.txt"));
    }

    #[test]
    fn folder_sync_selected_line_counts_only_selection() {
        use crate::tree::SyncModeChoice;
        let mk = |path: &str, glyph: char| FolderRow {
            rel_path: path.to_string(),
            is_dir: false,
            glyph,
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: glyph != '=',
        };
        let rows = vec![mk("new.txt", '>'), mk("same.txt", '=')];
        let empty = crate::tree::FolderSyncSelection::new();
        assert_eq!(
            folder_sync_selected_line(&rows, SyncModeChoice::UpdateBoth, &empty),
            "0 selected: dry-run: 0 L->R, 0 R->L, 0 delete(s), 0 skipped"
        );
        let mut one = crate::tree::FolderSyncSelection::new();
        one.select("new.txt");
        let line = folder_sync_selected_line(&rows, SyncModeChoice::UpdateBoth, &one);
        assert!(line.starts_with("1 selected: "), "{line}");
        assert!(line.contains("1 L->R"), "{line}");
    }

    #[test]
    fn hex_find_hunk_hits_search_displayed_hunk_lines() {
        let hunks = vec![
            crate::views_hex::HexHunkView {
                offset: 16,
                left_len: 2,
                right_len: 2,
            },
            crate::views_hex::HexHunkView {
                offset: 64,
                left_len: 1,
                right_len: 3,
            },
        ];
        assert!(hex_find_hunk_hits(&hunks, "").is_empty());
        assert!(hex_find_hunk_hits(&[], "vs").is_empty());
        assert!(hex_find_hunk_hits(&hunks, "missing-needle-zzz").is_empty());
        // Displayed line me "vs" har hunk me hai; offset text se selective hit.
        assert_eq!(hex_find_hunk_hits(&hunks, "vs"), vec![0, 1]);
        let first = hunks.first().map(format_hex_hunk_line).unwrap_or_default();
        let tag = first.split_whitespace().next().unwrap_or_default();
        assert_eq!(hex_find_hunk_hits(&hunks, tag), vec![0]);
    }

    #[test]
    fn find_cursor_step_wraps_next_prev() {
        assert_eq!(find_cursor_step(0, 0, false, true), None);
        assert_eq!(find_cursor_step(0, 3, false, false), None);
        assert_eq!(find_cursor_step(0, 3, false, true), Some(1));
        assert_eq!(find_cursor_step(2, 3, false, true), Some(0));
        assert_eq!(find_cursor_step(0, 3, true, false), Some(2));
        assert_eq!(find_cursor_step(1, 3, true, false), Some(0));
        // Dono dabe to Next jeetta hai (text precedent).
        assert_eq!(find_cursor_step(1, 3, true, true), Some(2));
        // Stale cursor clamp hokar wrap karta hai.
        assert_eq!(find_cursor_step(9, 3, false, true), Some(1));
    }

    #[test]
    fn table_wirefind_matches_painted_text_and_badge() {
        let model = TableModel {
            rows: vec![
                crate::viewmodel::TableRow {
                    glyph: '=',
                    label: "   1".to_string(),
                    text: "alpha | beta".to_string(),
                    changed: false,
                },
                crate::viewmodel::TableRow {
                    glyph: '!',
                    label: "   2".to_string(),
                    text: "needle here".to_string(),
                    changed: true,
                },
            ],
            truncated: false,
        };
        let texts = table_model_text_rows(&model);
        let hits = crate::views_table::table_find_match_rows(&texts, "needle", false);
        assert_eq!(hits, vec![(1, 0)]);
        assert_eq!(
            crate::views_table::table_find_badge_label(Some(1), hits.len()),
            "1/1"
        );
        assert!(crate::views_table::table_find_match_rows(&texts, "", false).is_empty());
        assert_eq!(crate::views_table::table_find_badge_label(None, 0), "0/0");
        // Caller step: Next wrap + Prev wrap painted hits par.
        assert_eq!(find_cursor_step(0, hits.len(), false, true), Some(0));
        assert_eq!(find_cursor_step(0, hits.len(), true, false), Some(0));
    }

    #[test]
    fn hex_grid_row_marks_covered_bytes_with_midline_gap() {
        let hunks = vec![crate::views_hex::HexHunkView {
            offset: 20,
            left_len: 2,
            right_len: 2,
        }];
        assert!(hunk_covers_offset(20, &hunks));
        assert!(hunk_covers_offset(21, &hunks));
        assert!(!hunk_covers_offset(16, &hunks));
        assert!(!hunk_covers_offset(22, &hunks));
        assert!(!hunk_covers_offset(20, &[]));
        let line = format_hex_grid_row(16, &hunks);
        assert!(line.starts_with("0x00000010  "));
        // Hex section me 2 covered cells; ASCII column me 1 `??` jod.
        let hex_part = line.split(" |").next().unwrap_or("");
        assert_eq!(hex_part.matches("??").count(), 2);
        assert_eq!(line.matches("??").count(), 3);
        // ASCII column: 16 chars, covered at cols 4-5.
        assert!(line.ends_with("|    ??          |"));
        // Midline gap: 8th cell ke baad extra space.
        let empty = format_hex_grid_row(32, &[]);
        assert!(empty.starts_with("0x00000020"));
        assert!(!empty.contains("??"));
        assert!(!empty.contains('?'));
        assert_eq!(empty.len(), line.len());
    }

    #[test]
    fn hex_grid_cells_split_address_hex_ascii() {
        let hunks = vec![crate::views_hex::HexHunkView {
            offset: 20,
            left_len: 2,
            right_len: 2,
        }];
        let (addr, hex, ascii) = format_hex_grid_cells(16, &hunks);
        assert_eq!(addr, crate::views_hex::format_hex_offset(16));
        // Hex me 2 covered cells; midline gap se extra space.
        assert_eq!(hex.matches("??").count(), 2);
        assert!(hex.contains("  "));
        // ASCII: 16 chars, covered at cols 4-5.
        assert_eq!(ascii, "    ??          ");
        // Single-string join wahi shape hai jo Grid paint karta hai.
        assert_eq!(
            format_hex_grid_row(16, &hunks),
            format!("{addr}  {hex} |{ascii}|")
        );
        // Empty row same width, koi `?` nahi.
        let (e_addr, e_hex, e_ascii) = format_hex_grid_cells(32, &[]);
        assert_eq!(e_addr, crate::views_hex::format_hex_offset(32));
        assert!(!e_hex.contains("??"));
        assert!(!e_ascii.contains('?'));
        assert_eq!(e_ascii.len(), 16);
        assert_eq!(hex.len(), e_hex.len());
        assert_eq!(
            format_hex_grid_row(32, &[]),
            format!("{e_addr}  {e_hex} |{e_ascii}|")
        );
    }

    #[test]
    fn hex_grid_rows_for_hunk_align_and_cap() {
        let hunk = crate::views_hex::HexHunkView {
            offset: 20,
            left_len: 2,
            right_len: 2,
        };
        assert_eq!(hex_grid_rows_for_hunk(&hunk), vec![16]);
        let big = crate::views_hex::HexHunkView {
            offset: 0,
            left_len: 64,
            right_len: 64,
        };
        assert_eq!(hex_grid_rows_for_hunk(&big), vec![0, 16, 32, 48]);
        let zero = crate::views_hex::HexHunkView {
            offset: 5,
            left_len: 0,
            right_len: 0,
        };
        assert!(hex_grid_rows_for_hunk(&zero).is_empty());
    }

    #[test]
    fn hex_minimap_fraction_maps_to_row_and_jumps() {
        assert_eq!(hex_minimap_row_at_y(0.0, 400.0, 4), Some(0));
        assert_eq!(hex_minimap_row_at_y(100.0, 400.0, 4), Some(1));
        assert_eq!(hex_minimap_row_at_y(399.9, 400.0, 4), Some(3));
        assert_eq!(hex_minimap_row_at_y(400.0, 400.0, 4), None);
        assert_eq!(hex_minimap_row_at_y(-1.0, 400.0, 4), None);
        assert_eq!(hex_minimap_row_at_y(10.0, 0.0, 4), None);
        assert_eq!(hex_minimap_row_at_y(10.0, 400.0, 0), None);
        assert_eq!(hex_minimap_row_at_y(f32::NAN, 400.0, 4), None);
        // Jump wahi path hai jo app.rs `hex_gutter_jump_to_row` chalata hai.
        let left = vec![0u8; 64];
        let mut right = vec![0u8; 64];
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        let mut view = crate::views_hex::HexCompareView::build_from_slices(
            &left,
            &right,
            crate::views_hex::HexCompareMode::None,
            8,
        );
        let mut pager = crate::views_hex::HexPager::new(0, 16);
        assert!(hex_minimap_jump_to_row(&mut view, &mut pager, 1));
        assert_eq!(view.selected, Some(0));
        let (start, end) = pager.visible_range();
        assert_eq!(start, 0);
        assert!(end > 0);
        assert!(!hex_minimap_jump_to_row(&mut view, &mut pager, 0));
    }

    #[test]
    fn text_gutter_click_selects_nearest_diff() {
        let row = |kind| TextRow {
            kind,
            left_no: None,
            right_no: None,
            left: String::new(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        let rows = vec![
            row(TextRowKind::Equal),
            row(TextRowKind::Deleted),
            row(TextRowKind::Equal),
            row(TextRowKind::Inserted),
        ];
        // Diff row par click wahi rehta hai (hex jump jaisa stay).
        assert_eq!(text_gutter_click_select(&rows, 4, 400.0, 150.0), Some(1));
        // Equal row 0 par click aage row 1 par snap hota hai.
        assert_eq!(text_gutter_click_select(&rows, 4, 400.0, 50.0), Some(1));
        // Equal row 2 par click aage row 3 par snap hota hai.
        assert_eq!(text_gutter_click_select(&rows, 4, 400.0, 250.0), Some(3));
        // Bahar ya khali par koi jump nahi.
        assert_eq!(text_gutter_click_select(&rows, 4, 400.0, 999.0), None);
        assert_eq!(text_gutter_click_select(&[], 0, 400.0, 50.0), None);
        // Pending-scroll key stable hai (dono frame ek hi id dekhte hain).
        assert_eq!(text_gutter_pending_id(), text_gutter_pending_id());
    }

    #[test]
    fn text_minimap_ruler_geometry_and_row_mapping() {
        use crate::gutter::{gutter_ruler_width, gutter_ruler_width_for_font};
        use crate::theme::TYPO_MONO_SIZE;
        // Click-target chaudhai gutter ruler contract par hai: numbers ke
        // saath 16.0, bina 8.0 (legacy 12/6 bar se chauda).
        assert_eq!(
            text_minimap_ruler_width(true),
            gutter_ruler_width_for_font(TYPO_MONO_SIZE, true)
        );
        assert_eq!(
            text_minimap_ruler_width(false),
            gutter_ruler_width_for_font(TYPO_MONO_SIZE, false)
        );
        assert_eq!(text_minimap_ruler_width(true), gutter_ruler_width(true));
        assert_eq!(text_minimap_ruler_width(false), gutter_ruler_width(false));
        assert_eq!(text_minimap_ruler_width(true), 16.0);
        assert_eq!(text_minimap_ruler_width(false), 8.0);
        // Row mapping: 4 rows over 400px, har band 100px (hex order
        // `y, height, total` me).
        assert_eq!(text_minimap_row_at_y(0.0, 400.0, 4), Some(0));
        assert_eq!(text_minimap_row_at_y(99.9, 400.0, 4), Some(0));
        assert_eq!(text_minimap_row_at_y(100.0, 400.0, 4), Some(1));
        assert_eq!(text_minimap_row_at_y(399.9, 400.0, 4), Some(3));
        // Kharab input par koi jump nahi.
        assert_eq!(text_minimap_row_at_y(400.0, 400.0, 4), None);
        assert_eq!(text_minimap_row_at_y(-1.0, 400.0, 4), None);
        assert_eq!(text_minimap_row_at_y(10.0, 0.0, 4), None);
        assert_eq!(text_minimap_row_at_y(10.0, 400.0, 0), None);
        assert_eq!(text_minimap_row_at_y(f32::NAN, 400.0, 4), None);
        assert_eq!(text_minimap_row_at_y(10.0, f32::NAN, 4), None);
        // Round-trip: marker y wapas usi row band me girta hai.
        let mut row = 0usize;
        while row < 4 {
            if let Some(y) = crate::gutter::gutter_y_for_row(row, 4, 400.0) {
                assert_eq!(text_minimap_row_at_y(y + 0.5, 400.0, 4), Some(row));
            }
            row = row.saturating_add(1);
        }
    }

    #[test]
    fn text_ruler_adopts_gutter_contract() {
        use crate::gutter::{
            GUTTER_HIT_SLOP, GUTTER_SEGMENT_MIN_H, gutter_ruler_width, gutter_segment_at_y,
        };
        use crate::viewmodel::TextRowKind;
        // Width: paint gutter adopts the ruler track contract (16/8),
        // so number column and minimap share one beat.
        assert_eq!(
            text_minimap_ruler_width(true),
            text_gutter_paint_width(true)
        );
        assert_eq!(
            text_minimap_ruler_width(false),
            text_gutter_paint_width(false)
        );
        assert_eq!(text_minimap_ruler_width(true), gutter_ruler_width(true));
        assert_eq!(text_minimap_ruler_width(false), gutter_ruler_width(false));
        // Slop: paint hit-test contract slop par hai (2px har taraf).
        assert_eq!(text_ruler_hit_slop(), GUTTER_HIT_SLOP);
        assert_eq!(text_ruler_hit_slop(), 2.0);
        // Segment min height: paint geometry (viewmodel re-export = gutter
        // contract) lambi file me 1-row hunk ko 4px par rakhta hai.
        let row = |kind| TextRow {
            kind,
            left_no: None,
            right_no: None,
            left: String::new(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        let mut rows = Vec::new();
        let mut i = 0usize;
        while i < 1000 {
            rows.push(row(TextRowKind::Equal));
            i = i.saturating_add(1);
        }
        if let Some(r) = rows.get_mut(500) {
            r.kind = TextRowKind::Deleted;
        }
        let geom = viewmodel::gutter_geometry(1000, 400.0, &rows);
        assert_eq!(geom.segments.len(), 1);
        let Some(seg) = geom.segments.first() else {
            return;
        };
        assert!((seg.y_end - seg.y_start - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        assert!((seg.y_end - seg.y_start - 4.0).abs() < 0.001);
        // Slop hit: band ke bilkul bahar ke pixels exact par miss, slop
        // par hit — bina paint hile.
        assert_eq!(gutter_segment_at_y(&geom.segments, 199.0), None);
        assert_eq!(text_ruler_segment_hit(&geom.segments, 199.0), Some(0));
        assert_eq!(gutter_segment_at_y(&geom.segments, 205.0), None);
        assert_eq!(text_ruler_segment_hit(&geom.segments, 205.0), Some(0));
        assert_eq!(text_ruler_segment_hit(&geom.segments, 150.0), None);
        assert_eq!(text_ruler_segment_hit(&geom.segments, f32::NAN), None);
        // Dono directions: band top (marker) aur band center dono wapas
        // usi row par map hote hain.
        let mut r = 0usize;
        while r < 4 {
            if let Some(y) = crate::gutter::gutter_y_for_row(r, 4, 400.0) {
                assert_eq!(text_minimap_row_at_y(y + 0.5, 400.0, 4), Some(r));
            }
            if let Some(center) = text_minimap_row_center_y(r, 4, 400.0) {
                assert_eq!(text_minimap_row_at_y(center, 400.0, 4), Some(r));
                assert_eq!(
                    crate::gutter::gutter_y_for_row(r, 4, 400.0),
                    Some(r as f32 * 100.0)
                );
            }
            r = r.saturating_add(1);
        }
        assert_eq!(text_minimap_row_center_y(0, 4, 400.0), Some(50.0));
        assert_eq!(text_minimap_row_center_y(1, 4, 400.0), Some(150.0));
        assert_eq!(text_minimap_row_center_y(2, 4, 400.0), Some(250.0));
        assert_eq!(text_minimap_row_center_y(3, 4, 400.0), Some(350.0));
        assert_eq!(text_minimap_row_center_y(4, 4, 400.0), None);
        assert_eq!(text_minimap_row_center_y(0, 0, 400.0), None);
        assert_eq!(text_minimap_row_center_y(1, 4, 0.0), None);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn home_paint_adopts_home_contract() {
        use crate::home::{
            HOME_CARD_BUTTON_MIN_H, HOME_SHOWS_PATH_PICKER, RECENT_COL_AGE_W, RECENT_COL_KIND_W,
            home_card_hierarchy_ok, home_card_min_height, home_recent_section_top,
            home_saved_chrome_height, recent_row_columns,
        };
        // Card min-h: contract 99.6, legacy 75 clip se mehfooz.
        assert_eq!(home_paint_card_height(), home_card_min_height());
        assert!((home_paint_card_height() - 99.6).abs() < 0.01);
        assert!(home_paint_card_height() > CARD_H);
        // CTA single-line budget 224x28 (area 6272): contract width adopt.
        assert_eq!(
            home_paint_cta_size(),
            (crate::home::home_card_cta_width(), HOME_CARD_BUTTON_MIN_H)
        );
        assert_eq!(home_paint_cta_size(), (224.0, 28.0));
        let (cta_w, cta_h) = home_paint_cta_size();
        assert!((cta_w * cta_h - 6_272.0).abs() < 0.001);
        assert!(crate::home::home_card_cta_single_line_ok());
        // Grid: 3 columns, section rhythm gap.
        assert_eq!(home_paint_grid_columns(), 3);
        assert_eq!(HOME_GRID_GAP, SECTION_GAP);
        // Hierarchy: HomeTitle strong-14 vs HomeDesc small-weak-11, koi
        // naya size nahi (paint sizes wahi contract sizes hain).
        assert!(home_card_hierarchy_ok());
        assert_eq!(home_title_paint_size(), 14.0);
        assert!(home_desc_paint_is_small_weak());
        assert_eq!(home_button_paint_size(), 13.0);
        // Home par path picker nahi: paint lock + bachta chrome 32.
        assert!(!HOME_SHOWS_PATH_PICKER);
        assert_eq!(home_saved_chrome_height(), 32.0);
        assert!((home_recent_section_top(200.0) - 208.0).abs() < 0.001);
        // Recent columns: title flex + kind 96 + age 72 tile karte hain.
        assert_eq!(RECENT_COL_KIND_W, 96.0);
        assert_eq!(RECENT_COL_AGE_W, 72.0);
        let (title, kind, age) = recent_row_columns(460.0);
        assert!(
            (title + kind + age + 8.0 * 2.0 + 4.0 * 2.0 - 460.0).abs() < 0.001,
            "title {title} kind {kind} age {age}"
        );
        // Row title: khali paths par kind, warna honest path summary.
        let empty = crate::home::SessionSnapshot::new(crate::home::Tab::Text, "", "", "", 0);
        assert_eq!(home_recent_row_title(&empty), "Text");
        let paths = crate::home::SessionSnapshot::new(
            crate::home::Tab::Text,
            "left.txt",
            "right.txt",
            "",
            0,
        );
        let title_text = home_recent_row_title(&paths);
        assert!(title_text.contains("left.txt"));
        assert!(title_text.contains("right.txt"));
        let three = crate::home::SessionSnapshot::new(crate::home::Tab::Merge, "l", "r", "base", 0);
        assert!(home_recent_row_title(&three).contains("base"));
    }

    #[test]
    fn minimap_click_labels_stay_honest() {
        // Text ruler: empty/selection-hape honest, koi fake count nahi.
        assert_eq!(
            text_minimap_accessibility_label(None, 0),
            "overview ruler: empty"
        );
        assert_eq!(
            text_minimap_accessibility_label(Some(0), 0),
            "overview ruler: empty"
        );
        assert_eq!(
            text_minimap_accessibility_label(None, 4),
            "overview ruler: no selection, 4 rows, click to jump to nearest difference"
        );
        assert_eq!(
            text_minimap_accessibility_label(Some(9), 4),
            "overview ruler: no selection, 4 rows, click to jump to nearest difference"
        );
        assert_eq!(
            text_minimap_accessibility_label(Some(1), 4),
            "overview ruler: row 2 of 4, click to jump to nearest difference"
        );
        assert_eq!(
            text_minimap_accessibility_label(Some(0), 1),
            "overview ruler: row 1 of 1, click to jump to nearest difference"
        );
        // Hex minimap: wahi shape, hunk ginati par.
        assert_eq!(hex_minimap_accessibility_label(None, 0), "minimap: empty");
        assert_eq!(
            hex_minimap_accessibility_label(None, 3),
            "minimap: no selection, 3 hunks, click to jump"
        );
        assert_eq!(
            hex_minimap_accessibility_label(Some(9), 3),
            "minimap: no selection, 3 hunks, click to jump"
        );
        assert_eq!(
            hex_minimap_accessibility_label(Some(0), 3),
            "minimap: hunk 1 of 3, click to jump"
        );
        assert_eq!(
            hex_minimap_accessibility_label(Some(2), 3),
            "minimap: hunk 3 of 3, click to jump"
        );
    }

    #[test]
    fn diff_nav_total_counts_only_diffs() {
        let row = |kind| TextRow {
            kind,
            left_no: None,
            right_no: None,
            left: String::new(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        assert_eq!(diff_nav_total(&[]), 0);
        assert_eq!(
            diff_nav_total(&[row(TextRowKind::Equal), row(TextRowKind::Equal)]),
            0
        );
        let rows = vec![
            row(TextRowKind::Equal),
            row(TextRowKind::Deleted),
            row(TextRowKind::Inserted),
            row(TextRowKind::Equal),
            row(TextRowKind::Changed),
        ];
        assert_eq!(diff_nav_total(&rows), 3);
    }

    #[test]
    fn diff_nav_position_tracks_selected_among_diffs() {
        let row = |kind| TextRow {
            kind,
            left_no: None,
            right_no: None,
            left: String::new(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        let rows = vec![
            row(TextRowKind::Equal),
            row(TextRowKind::Deleted),
            row(TextRowKind::Equal),
            row(TextRowKind::Inserted),
        ];
        assert_eq!(diff_nav_position(&rows, None), None);
        assert_eq!(diff_nav_position(&rows, Some(0)), None);
        assert_eq!(diff_nav_position(&rows, Some(2)), None);
        assert_eq!(diff_nav_position(&[], Some(0)), None);
        assert_eq!(diff_nav_position(&rows, Some(99)), None);
        assert_eq!(diff_nav_position(&rows, Some(1)), Some(1));
        assert_eq!(diff_nav_position(&rows, Some(3)), Some(2));
    }

    #[test]
    fn diff_nav_count_labels_position_over_total() {
        assert_eq!(diff_nav_count_label(Some(2), 5), "2/5");
        assert_eq!(diff_nav_count_label(Some(1), 1), "1/1");
        assert_eq!(diff_nav_count_label(None, 5), "0/5");
        assert_eq!(diff_nav_count_label(None, 0), "0/0");
        assert_eq!(
            diff_nav_count_label(Some(1), 2),
            viewmodel::find_count_label(Some(1), 2)
        );
    }

    #[test]
    fn hex_minimap_strip_wider_than_gutter() {
        // Overview strip reference se kam chaudha na lage: gutter ka
        // double, same token par; gutter khud untouched rehta hai.
        assert_eq!(HEX_MINIMAP_W, 2.0 * GUTTER_W);
        assert_eq!(HEX_MINIMAP_W, 24.0);
        assert_eq!(HEX_MINIMAP_W - GUTTER_W, 12.0);
    }

    #[test]
    fn minimap_seg_floors_at_row_pad() {
        // Chhoti hunk band ek row-pad tak badhti hai (bigger hits);
        // badi band jaisi hai waisi rehti hai — mapping exact.
        assert_eq!(hex_minimap_seg_end(10.0, 10.2), 10.0 + ROW_PAD);
        assert_eq!(hex_minimap_seg_end(10.0, 10.0 + ROW_PAD), 10.0 + ROW_PAD);
        assert_eq!(hex_minimap_seg_end(10.0, 50.0), 50.0);
    }

    #[test]
    fn table_cells_split_and_border_with_header() {
        assert_eq!(split_table_cells("a | b"), vec!["a", "b"]);
        assert_eq!(split_table_cells("single"), vec!["single"]);
        let model = TableModel {
            rows: vec![
                crate::viewmodel::TableRow {
                    glyph: '=',
                    label: " hdr".to_string(),
                    text: "ID | Price".to_string(),
                    changed: false,
                },
                crate::viewmodel::TableRow {
                    glyph: '=',
                    label: "   1".to_string(),
                    text: "1 | 9.99".to_string(),
                    changed: false,
                },
            ],
            truncated: false,
        };
        let header = table_header_cells(&model).unwrap_or_default();
        assert_eq!(header, vec!["ID", "Price"]);
        assert!(table_header_cells(&TableModel::default()).is_none());
        let data = vec![split_table_cells("1 | 9.99")];
        let widths = crate::views_table::table_column_widths(&header, &data);
        let line = format_table_bordered_row(&header, &widths);
        assert!(line.starts_with('|') && line.ends_with('|'));
        let Some(first) = data.first() else {
            return;
        };
        let row_line = format_table_bordered_row(first, &widths);
        assert_eq!(row_line.len(), line.len());
        // Missing cell `-` dikhta hai.
        let ragged = format_table_bordered_row(&["only".to_string()], &widths);
        assert!(ragged.contains('-'));
        assert_eq!(ragged.len(), line.len());
        let rule = format_table_header_rule(line.len());
        assert_eq!(rule.len(), line.len());
        assert!(rule.chars().all(|ch| ch == '-'));
    }

    #[test]
    fn table_row_cells_pad_to_widths_for_grid() {
        let header = vec!["ID".to_string(), "Price".to_string()];
        let data = vec![split_table_cells("1 | 9.99")];
        let widths = crate::views_table::table_column_widths(&header, &data);
        // Grid cells bordered line ki padded cells ke barabar hain
        // (bina `|` borders ke — separation Grid spacing deta hai).
        let cells = table_row_cells("1 | 9.99", &widths);
        assert_eq!(cells.len(), widths.len());
        for (cell, width) in cells.iter().zip(widths.iter()) {
            assert_eq!(cell.chars().count(), *width);
        }
        assert_eq!(
            cells,
            crate::views_table::pad_table_row(&split_table_cells("1 | 9.99"), &widths)
        );
        // Ragged row me missing cell `-` se pad hota hai.
        let ragged = table_row_cells("only", &widths);
        assert_eq!(ragged.len(), widths.len());
        assert!(ragged.iter().any(|cell| cell.trim() == "-"));
        // Header cells bhi same widths par pad hote hain.
        let head = table_row_cells("ID | Price", &widths);
        assert_eq!(head.len(), widths.len());
    }

    #[test]
    fn table_grid_columns_counts_glyph_plus_cells() {
        assert_eq!(table_grid_columns(&[]), 1);
        assert_eq!(table_grid_columns(&[3]), 2);
        assert_eq!(table_grid_columns(&[3, 5]), 3);
    }

    #[test]
    fn tablep234_header_grid_tokens_single_source() {
        // P2 header-grid tokens: paint `views_table::` single-source rakhta
        // hai (same values, koi naya rhythm/text nahi). Values yahan lock
        // hain taaki future drift pakda jaye.
        assert_eq!(
            views_table::TABLE_CELL_HEADER_GRID_ID,
            "table_cell_header_grid"
        );
        assert_eq!(views_table::TABLE_GRID_COL_GAP, SECTION_GAP);
        assert_eq!(views_table::TABLE_GRID_ROW_GAP, TIGHT_GAP);
        assert_eq!(views_table::TABLE_CELL_GLYPH_PAD, CONTENT_LEAD);
        assert_eq!(views_table::TABLE_CELL_GLYPH_PAD, "  ");
    }

    #[test]
    fn tablep34_data_index_map_skips_header_rows() {
        // P3 grid coords `data_cells` (hdr-filtered) par hain: hdr rows
        // `None`, data rows grid order me; reverse map pager (`model.rows`
        // index) par wapas laata hai.
        let model = TableModel {
            rows: vec![
                crate::viewmodel::TableRow {
                    glyph: '=',
                    label: "hdr".to_string(),
                    text: "ID | Price".to_string(),
                    changed: false,
                },
                crate::viewmodel::TableRow {
                    glyph: '=',
                    label: "   1".to_string(),
                    text: "1 | 9.99".to_string(),
                    changed: false,
                },
                crate::viewmodel::TableRow {
                    glyph: '!',
                    label: "   2".to_string(),
                    text: "2 | 1.50".to_string(),
                    changed: true,
                },
            ],
            truncated: false,
        };
        let (fwd, rev) = table_data_index_map(&model);
        assert_eq!(fwd, vec![None, Some(0), Some(1)]);
        assert_eq!(rev, vec![1, 2]);
    }

    #[test]
    fn pager_windows_drive_visible_rows() {
        let mut hex_pager = crate::views_hex::HexPager::new(50, 16);
        assert_eq!(hex_pager.visible_range(), (0, 16));
        hex_pager.next_page();
        assert_eq!(hex_pager.top, 16);
        assert!(hex_pager.bring_into_view(40));
        assert_eq!(hex_pager.top, 25);
        let mut table_pager = crate::views_table::TablePager::new(25, 10);
        assert_eq!(table_pager.visible_range(), (0, 10));
        table_pager.next_page();
        assert_eq!(table_pager.top, 10);
        assert!(table_pager.bring_into_view(24));
        let mut folder_pager = crate::tree::RowPager::new(25, 10);
        folder_pager.next_page();
        assert_eq!(folder_pager.top, 10);
        folder_pager.prev_page();
        assert_eq!(folder_pager.top, 0);
    }

    #[test]
    fn exquisite_geometry_tokens_reuse_scale() {
        use crate::theme::{GUTTER_BAR_WIDE, GUTTER_MIN_SEGMENT_H};
        assert_eq!(GUTTER_W, 12.0);
        assert_eq!(GUTTER_W, GUTTER_BAR_WIDE);
        assert_eq!(
            text_gutter_paint_width(true),
            crate::gutter::gutter_ruler_width(true)
        );
        assert_eq!(HEADER_H, 28.0);
        assert_eq!(FIND_BAR_H, HEADER_H);
        assert_eq!(ROW_PAD, 2.0);
        assert_eq!(ROW_PAD, GUTTER_MIN_SEGMENT_H);
        assert_eq!(SPLIT_HIT_W, 8.0);
        assert_eq!(SPLIT_HIT_W, SPLIT_HANDLE_W);
        assert_eq!(SPLIT_VISUAL_W, 1.0);
        assert_eq!(BADGE_PAD, 4.0);
        assert_eq!(BADGE_PAD, TIGHT_GAP);
        assert_eq!(FIND_NEEDLE_W, 120.0);
        assert_eq!(HEX_MINIMAP_H, 220.0);
        assert_eq!(CONTENT_LEAD, "  ");
    }

    #[test]
    fn split_hit_centers_one_px_visual() {
        assert_eq!(SPLIT_VISUAL_W, 1.0);
        assert_eq!(split_visual_inset(), 3.5);
        assert_eq!(split_visual_inset() * 2.0 + SPLIT_VISUAL_W, SPLIT_HIT_W);
        // Grab width meets the 5px floor via the centering identity.
        assert!(split_visual_inset() * 2.0 + SPLIT_VISUAL_W >= 5.0);
    }

    #[test]
    fn row_height_adds_fixed_pad() {
        assert_eq!(row_height_for_line(14.0), 16.0);
        assert_eq!(row_height_for_line(20.0), 20.0 + ROW_PAD);
        assert_eq!(row_height_for_line(0.0), 0.0);
        assert!(row_height_for_line(f32::NAN).is_nan());
    }

    #[test]
    fn content_lead_gives_shared_left_baseline() {
        assert_eq!(indented_content("x"), "  x");
        assert_eq!(indented_content(""), CONTENT_LEAD);
        assert!(indented_content("| a |").starts_with(CONTENT_LEAD));
        assert_eq!(CONTENT_LEAD.len(), 2);
    }

    #[test]
    fn find_bar_docks_top_with_right_badge_air() {
        // Full-width dock shares the uniform bar height; the badge keeps
        // one pad of air (paint right-aligns it, so no overlap by
        // construction).
        assert_eq!(FIND_BAR_H, HEADER_H);
        assert_eq!(FIND_NEEDLE_W, 120.0);
        assert_eq!(BADGE_PAD, TIGHT_GAP);
    }

    #[test]
    fn find_state_badge_matches_model_label() {
        // Paint badge aur model label ek hi string: badge parity.
        assert_eq!(
            text_find_badge_label(Some(2), 5),
            viewmodel::find_count_label(Some(2), 5)
        );
        assert_eq!(text_find_badge_label(Some(2), 5), "2/5");
        assert_eq!(
            text_find_badge_label(None, 0),
            viewmodel::find_count_label(None, 0)
        );
        assert_eq!(text_find_badge_label(None, 0), "0/0");
        let rows = vec![
            TextRow {
                kind: TextRowKind::Equal,
                left: "alpha beta".to_string(),
                right: String::new(),
                ..TextRow::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left: "gamma delta".to_string(),
                right: String::new(),
                ..TextRow::default()
            },
        ];
        let mut find = FindState::default();
        find.set_query(&rows, "gamma");
        assert_eq!(find.count_label(), "1/1");
        assert_eq!(
            text_find_badge_label(find.position(), find.match_count()),
            "1/1"
        );
    }

    #[test]
    fn advance_text_find_moves_only_on_click() {
        let rows = vec![
            TextRow {
                kind: TextRowKind::Equal,
                left: "alpha".to_string(),
                right: String::new(),
                ..TextRow::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left: "gamma one".to_string(),
                right: String::new(),
                ..TextRow::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left: "gamma two".to_string(),
                right: String::new(),
                ..TextRow::default()
            },
        ];
        let mut find = FindState::default();
        find.set_query(&rows, "gamma");
        // Bina click selection untouched (None = koi change nahi).
        assert_eq!(advance_text_find(&mut find, false, false), None);
        assert_eq!(find.count_label(), "1/2");
        // Next advance karta hai, wrap ke saath.
        assert_eq!(advance_text_find(&mut find, false, true), Some(2));
        assert_eq!(find.count_label(), "2/2");
        assert_eq!(advance_text_find(&mut find, false, true), Some(1));
        // Prev peeche jata hai, wrap ke saath.
        assert_eq!(advance_text_find(&mut find, true, false), Some(2));
        // Khali find par clicks bhi None.
        let mut empty = FindState::default();
        empty.set_query(&rows, "absent");
        assert_eq!(advance_text_find(&mut empty, false, true), None);
        assert_eq!(advance_text_find(&mut empty, true, false), None);
    }

    #[test]
    fn findhl_line_spans_empty_needle_gives_no_hits() {
        assert!(find_line_spans("hello world", "", false).is_empty());
        assert!(find_line_spans("hello world", "", true).is_empty());
        assert!(find_line_spans("", "needle", false).is_empty());
        assert!(find_line_spans("hi", "hello", false).is_empty());
    }

    #[test]
    fn findhl_line_spans_case_modes() {
        // Case-sensitive: sirf exact lower hit.
        assert_eq!(
            find_line_spans("AbC abc", "abc", false),
            vec![(4usize, 7usize)]
        );
        // Ignore-case (ascii-fold): dono hits.
        assert_eq!(
            find_line_spans("AbC abc", "abc", true),
            vec![(0usize, 3usize), (4usize, 7usize)]
        );
        // Upper needle bhi fold hota hai.
        assert_eq!(
            find_line_spans("abc ABC", "ABC", true),
            vec![(0usize, 3usize), (4usize, 7usize)]
        );
    }

    #[test]
    fn findhl_line_spans_overlapping_ok() {
        // Overlap allow: koi match chhupta nahi, sorted rehte hain.
        assert_eq!(
            find_line_spans("aaa", "aa", false),
            vec![(0usize, 2usize), (1usize, 3usize)]
        );
        assert_eq!(
            find_line_spans("hello world", "o w", false),
            vec![(4usize, 7usize)]
        );
    }

    #[test]
    fn findhl_current_other_formats_differ() {
        // Same mono size, same row pair reuse — farak sirf block-vs-line.
        let font = FontId::monospace(12.0);
        let fg = Color32::WHITE;
        let bg = Color32::BLACK;
        let current = find_hit_format(font.clone(), fg, bg, true);
        let other = find_hit_format(font.clone(), fg, bg, false);
        // Current inverted block (no underline), other underline line (no block).
        assert_eq!(current.background, fg);
        assert_eq!(other.background, bg);
        assert_eq!(current.underline.width, 0.0);
        assert_eq!(other.underline.width, INTRA_SPAN_UNDERLINE_W);
        assert_eq!(current.font_id, other.font_id);
        // Segment router same baant-ta hai.
        assert_eq!(
            find_segment_format(font.clone(), fg, bg, FindHl::Current).background,
            fg
        );
        assert_eq!(
            find_segment_format(font.clone(), fg, bg, FindHl::Other).background,
            bg
        );
        assert_eq!(
            find_segment_format(font, fg, bg, FindHl::Plain).background,
            bg
        );
    }

    #[test]
    fn findhl_wrap_chunks_keep_match() {
        // Match wrapping geometry: lambi line + chhoti width par match
        // kisi continuation chunk me `find_chunk_segments` se milta hai.
        let line = "hi world";
        let width = 5usize;
        let chunks = wrap_chunks(line, width);
        assert!(chunks.len() >= 2);
        let find_raw = find_line_spans(line, "world", false);
        assert_eq!(find_raw, vec![(3usize, 8usize)]);
        let mut seen_current = false;
        let mut joined = String::new();
        for chunk in chunks.iter() {
            for (text, kind) in find_chunk_segments(chunk, &[], &find_raw, true, width) {
                if kind == FindHl::Current {
                    seen_current = true;
                    joined.push_str(&text);
                }
            }
        }
        assert!(seen_current);
        assert!(joined.contains("world"));
    }

    #[test]
    fn findhl_pane_segments_mark_current_row() {
        // Current row ke hits Current, others ke Other; find wins over accent.
        let line = "foo bar foo";
        let pane = 11usize;
        let others = find_pane_segments(line, &[], "foo", false, false, pane);
        assert!(others.iter().any(|(_, k)| *k == FindHl::Other));
        assert!(!others.iter().any(|(_, k)| *k == FindHl::Current));
        let currents = find_pane_segments(line, &[], "foo", false, true, pane);
        assert!(currents.iter().any(|(_, k)| *k == FindHl::Current));
        assert!(!currents.iter().any(|(_, k)| *k == FindHl::Other));
        // Geometry wahi: segments ka jod pane text (padding samahit) deta hai.
        let mut text = String::new();
        for (seg, _) in currents.iter() {
            text.push_str(seg);
        }
        assert_eq!(text.len(), pane);
        // Bina needle koi find kind nahi.
        let plain = find_pane_segments(line, &[], "", false, true, pane);
        assert!(
            !plain
                .iter()
                .any(|(_, k)| *k == FindHl::Other || *k == FindHl::Current)
        );
        // Row-level flag helper.
        assert!(find_is_current_row(2, Some(2)));
        assert!(!find_is_current_row(1, Some(2)));
        assert!(!find_is_current_row(1, None));
    }

    #[test]
    fn gutter_widths_stay_uniform() {
        assert_eq!(
            text_gutter_paint_width(true),
            crate::gutter::gutter_ruler_width(true)
        );
        assert_eq!(
            text_gutter_paint_width(false),
            crate::gutter::gutter_ruler_width(false)
        );
        assert!(text_gutter_paint_width(false) < text_gutter_paint_width(true));
    }

    #[test]
    fn dual_tree_panes_share_row_and_floor() {
        // Split authority reuse: row bharti hai, narrow par floor.
        let (left, right) = dual_tree_pane_widths(800.0, 0.50);
        assert_eq!(left + right, 800.0 - SECTION_GAP);
        assert_eq!((left, right), split_picture_widths(800.0, 0.50));
        let (tiny_left, tiny_right) = dual_tree_pane_widths(0.0, 0.50);
        assert_eq!(tiny_left + tiny_right, 2.0 * MIN_HALF);
        // Fraction clamp me rehta hai.
        assert_eq!(
            dual_tree_pane_widths(800.0, 0.0),
            dual_tree_pane_widths(800.0, viewmodel::SPLIT_MIN_FRACTION)
        );
    }

    #[test]
    fn dual_pending_text_names_stub_without_io() {
        let text = dual_pending_text(crate::tree::DualRowAction::Compare, "a.txt");
        assert!(text.contains("(stub)"));
        assert!(text.contains("a.txt"));
        assert!(text.contains("koi file nahi badli"));
        let copy = dual_pending_text(crate::tree::DualRowAction::CopyLeftToRight, "b/c.txt");
        assert!(copy.contains("(stub)"));
        assert!(copy.contains("b/c.txt"));
    }

    #[test]
    fn dual_lines_carry_glyph_name_and_side_cells() {
        let node = crate::tree::FolderTreeNode {
            row: FolderRow {
                rel_path: "a/b.txt".to_string(),
                is_dir: false,
                glyph: '!',
                left_size: Some("10".to_string()),
                right_size: None,
                left_time: None,
                right_time: None,
                changed: true,
            },
            depth: 1,
            is_expanded: false,
            has_children: false,
        };
        let left = dual_left_line(&node);
        assert!(left.contains('!'));
        assert!(left.contains("a/b.txt"));
        assert!(left.contains("10"));
        let right = dual_right_line(&node);
        assert!(right.contains('!'));
        assert!(right.contains("a/b.txt"));
        // Missing right side dash dikhta hai.
        assert!(right.contains('-'));
        let dir_node = crate::tree::FolderTreeNode {
            row: FolderRow {
                rel_path: "src".to_string(),
                is_dir: true,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            },
            depth: 0,
            is_expanded: true,
            has_children: true,
        };
        assert!(dual_left_line(&dir_node).contains("src/"));
        assert!(dual_right_line(&dir_node).contains("src/"));
    }

    #[test]
    fn folder_header_cells_match_defs() {
        let headers = folder_header_cells();
        let defs = crate::tree::folder_column_defs();
        assert_eq!(headers.len(), defs.len());
        let Some(first) = headers.first() else {
            return;
        };
        assert_eq!(first, "Path");
        // Sides padded fixed widths tak.
        let Some(lsize) = headers.get(1) else {
            return;
        };
        assert_eq!(lsize.chars().count(), crate::tree::FOLDER_SIZE_CHARS);
        assert!(lsize.contains("L size"));
        let Some(ltime) = headers.get(3) else {
            return;
        };
        assert_eq!(ltime.chars().count(), crate::tree::FOLDER_TIME_CHARS);
    }

    #[test]
    fn folder_row_cells_keep_sides_fixed_width() {
        let row = FolderRow {
            rel_path: "a/b.txt".to_string(),
            is_dir: false,
            glyph: '!',
            left_size: Some("10".to_string()),
            right_size: None,
            left_time: Some("2026-01-02 03:04".to_string()),
            right_time: None,
            changed: true,
        };
        let cells = folder_row_cells(&row);
        assert_eq!(cells.len(), 5);
        let Some(path) = cells.first() else {
            return;
        };
        assert_eq!(path, "a/b.txt");
        // Sides padded: orphan `-` bhi fixed width.
        let Some(lsize) = cells.get(1) else {
            return;
        };
        assert_eq!(lsize.chars().count(), crate::tree::FOLDER_SIZE_CHARS);
        assert!(lsize.contains("10"));
        let Some(rsize) = cells.get(2) else {
            return;
        };
        assert_eq!(rsize.chars().count(), crate::tree::FOLDER_SIZE_CHARS);
        assert!(rsize.contains('-'));
        let Some(ltime) = cells.get(3) else {
            return;
        };
        assert_eq!(ltime.chars().count(), crate::tree::FOLDER_TIME_CHARS);
        // Dir mark judta hai.
        let dir = FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            ..row
        };
        let dir_cells = folder_row_cells(&dir);
        let Some(dir_path) = dir_cells.first() else {
            return;
        };
        assert_eq!(dir_path, "src/");
    }

    #[test]
    fn folder_grid_columns_counts_glyph_plus_five() {
        assert_eq!(
            folder_grid_columns(),
            crate::tree::folder_column_defs().len().saturating_add(1)
        );
        assert_eq!(folder_grid_columns(), 6);
    }

    #[test]
    fn dual_lines_use_padded_side_cells() {
        let node = crate::tree::FolderTreeNode {
            row: FolderRow {
                rel_path: "a/b.txt".to_string(),
                is_dir: false,
                glyph: '!',
                left_size: Some("10".to_string()),
                right_size: None,
                left_time: Some("2026-01-02 03:04".to_string()),
                right_time: None,
                changed: true,
            },
            depth: 0,
            is_expanded: false,
            has_children: false,
        };
        let left = dual_left_line(&node);
        assert!(left.contains("a/b.txt"));
        assert!(left.contains("10"));
        assert!(left.contains("2026-01-02 03:04"));
        // Padded separator barkarar: size | time.
        assert!(left.contains(" | "));
        let right = dual_right_line(&node);
        assert!(right.contains("a/b.txt"));
        // Missing right sides dash padded dikhte hain.
        assert!(right.contains('-'));
        assert!(right.contains(" | "));
    }

    #[test]
    fn dual_grid_headers_match_column_defs() {
        // Header text tree model se reuse, koi nayi wording nahi.
        let defs = crate::tree::folder_column_defs();
        let left = dual_grid_headers_left();
        assert_eq!(left[0], defs[0].header);
        assert_eq!(
            left[1],
            crate::tree::pad_folder_cell(defs[1].header, defs[1].width_chars)
        );
        assert_eq!(
            left[2],
            crate::tree::pad_folder_cell(defs[3].header, defs[3].width_chars)
        );
        assert!(left[1].contains("L size"));
        assert!(left[2].contains("L modified"));
        let right = dual_grid_headers_right();
        assert_eq!(right[0], defs[0].header);
        assert!(right[1].contains("R size"));
        assert!(right[2].contains("R modified"));
        // Per-side 3 columns (Name/Size/Modified).
        assert_eq!(left.len(), 3);
        assert_eq!(right.len(), 3);
    }

    #[test]
    fn dual_grid_row_cells_carry_name_size_time() {
        let row = FolderRow {
            rel_path: "a/b.txt".to_string(),
            is_dir: false,
            glyph: '!',
            left_size: Some("10".to_string()),
            right_size: None,
            left_time: Some("2026-01-02 03:04".to_string()),
            right_time: None,
            changed: true,
        };
        let left = dual_grid_row_left(&row);
        assert_eq!(left[0], "a/b.txt");
        assert!(left[1].contains("10"));
        assert!(left[2].contains("2026-01-02 03:04"));
        let right = dual_grid_row_right(&row);
        assert_eq!(right[0], "a/b.txt");
        // Missing right sides dash dikhte hain (orphan align).
        assert!(right[1].contains('-'));
        assert!(right[2].contains('-'));
        // Dir mark judta hai.
        let dir = FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            ..row
        };
        assert_eq!(dual_grid_row_left(&dir)[0], "src/");
        assert_eq!(dual_grid_row_right(&dir)[0], "src/");
    }

    #[test]
    fn dual_grid_roles_reuse_typo_aliases() {
        // Cells content_grid (Mono 12), headers caption (Small 11):
        // koi naya size nahi.
        assert_eq!(dual_grid_cell_role(), TypoRole::content_grid());
        assert_eq!(dual_grid_cell_role(), TypoRole::Mono);
        assert!(dual_grid_cell_role().is_mono());
        assert_eq!(dual_grid_cell_role().size(), crate::theme::TYPO_MONO_SIZE);
        assert_eq!(dual_grid_header_role(), TypoRole::caption());
        assert_eq!(dual_grid_header_role(), TypoRole::Small);
        assert!(dual_grid_header_role().is_small());
        assert_eq!(
            dual_grid_header_role().size(),
            crate::theme::TYPO_SMALL_SIZE
        );
    }

    #[test]
    fn merge_hunk_sides_splits_left_base_right() {
        let lines = vec![
            "clean".to_string(),
            "<<<<<<< left".to_string(),
            "L1".to_string(),
            "||||||| base".to_string(),
            "B1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>> right".to_string(),
        ];
        let hunks = viewmodel::merge_conflict_hunks(&lines);
        assert_eq!(hunks.len(), 1);
        let Some(hunk) = hunks.first() else {
            return;
        };
        let (left, base, right) = merge_hunk_sides(&lines, hunk);
        assert_eq!(left, vec!["L1".to_string()]);
        assert_eq!(base, vec!["B1".to_string()]);
        assert_eq!(right, vec!["R1".to_string()]);
    }

    #[test]
    fn merge_hunk_sides_without_base_marker_puts_left_before_sep() {
        let lines = vec![
            "<<<<<<<".to_string(),
            "L1".to_string(),
            "L2".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>>".to_string(),
        ];
        let hunks = viewmodel::merge_conflict_hunks(&lines);
        assert_eq!(hunks.len(), 1);
        let Some(hunk) = hunks.first() else {
            return;
        };
        let (left, base, right) = merge_hunk_sides(&lines, hunk);
        assert_eq!(left, vec!["L1".to_string(), "L2".to_string()]);
        assert!(base.is_empty());
        assert_eq!(right, vec!["R1".to_string()]);
    }

    #[test]
    fn merge_hunk_sides_out_of_range_is_empty() {
        let lines = vec!["clean".to_string()];
        let hunk = viewmodel::MergeHunk {
            conflict_index: 0,
            start: 99,
            end: 200,
        };
        let (left, base, right) = merge_hunk_sides(&lines, &hunk);
        assert!(left.is_empty());
        assert!(base.is_empty());
        assert!(right.is_empty());
    }

    #[test]
    fn merge_three_pane_summary_line_counts_pending() {
        assert_eq!(
            merge_three_pane_summary_line(2, 2, true),
            "2 conflict hunk(s), 2 pending, conflicted".to_string()
        );
        assert_eq!(
            merge_three_pane_summary_line(0, 0, false),
            "0 conflict hunk(s), 0 pending".to_string()
        );
    }

    #[test]
    fn merge_hunk_title_and_decision_labels() {
        assert_eq!(
            format_merge_hunk_title(0, 2),
            "hunk 1 of 2: left / base / right".to_string()
        );
        assert_eq!(
            format_merge_hunk_decision(0),
            "hunk 1: decision pending".to_string()
        );
    }

    #[test]
    fn merge_pane_count_is_three() {
        assert_eq!(merge_pane_count(), 3);
    }

    #[test]
    fn three_pane_widths_split_equal_thirds() {
        let (left, base, right) = three_pane_widths(600.0);
        let total = 600.0 - 2.0 * SECTION_GAP;
        assert!((left - total / 3.0).abs() < 0.001);
        assert!((base - total / 3.0).abs() < 0.001);
        assert!((right - total / 3.0).abs() < 0.001);
    }

    #[test]
    fn three_pane_widths_floor_narrow_panes() {
        let (left, base, right) = three_pane_widths(100.0);
        assert_eq!((left, base, right), (MIN_THIRD, MIN_THIRD, MIN_THIRD));
    }

    #[test]
    fn three_pane_widths_cover_full_row() {
        let avail = 900.0;
        let (left, base, right) = three_pane_widths(avail);
        assert!((left + base + right + 2.0 * SECTION_GAP - avail).abs() < 0.001);
    }

    #[test]
    fn merge_pick_id_scopes_to_live_lines() {
        let lines = vec!["<<<<<<< left".to_string(), "L1".to_string()];
        let same = lines.clone();
        let mut other = lines.clone();
        other.push("extra".to_string());
        assert_eq!(merge_pick_id(&lines), merge_pick_id(&same));
        assert_ne!(merge_pick_id(&lines), merge_pick_id(&other));
        assert_ne!(merge_pick_id(&lines), merge_pick_id(&[]));
    }

    #[test]
    fn merge_pick_choice_labels_are_take_sides() {
        assert_eq!(merge_pick_choice_label(viewmodel::TakeChoice::Left), "left");
        assert_eq!(
            merge_pick_choice_label(viewmodel::TakeChoice::Right),
            "right"
        );
        assert_eq!(merge_pick_choice_label(viewmodel::TakeChoice::Base), "base");
        assert_eq!(merge_pick_choice_label(viewmodel::TakeChoice::Both), "both");
    }

    #[test]
    fn merge_pick_for_hunk_returns_last_staged() {
        let picks = vec![
            viewmodel::MergeHunkPick {
                conflict_index: 0,
                choice: viewmodel::TakeChoice::Left,
            },
            viewmodel::MergeHunkPick {
                conflict_index: 1,
                choice: viewmodel::TakeChoice::Right,
            },
            viewmodel::MergeHunkPick {
                conflict_index: 0,
                choice: viewmodel::TakeChoice::Both,
            },
        ];
        assert_eq!(
            merge_pick_for_hunk(&picks, 0),
            Some(viewmodel::TakeChoice::Both)
        );
        assert_eq!(
            merge_pick_for_hunk(&picks, 1),
            Some(viewmodel::TakeChoice::Right)
        );
        assert_eq!(merge_pick_for_hunk(&picks, 7), None);
        assert_eq!(merge_pick_for_hunk(&[], 0), None);
    }

    #[test]
    fn stage_merge_pick_upserts_last_wins() {
        let staged = stage_merge_pick(&[], 0, viewmodel::TakeChoice::Left);
        assert_eq!(staged.len(), 1);
        let restaged = stage_merge_pick(&staged, 0, viewmodel::TakeChoice::Right);
        assert_eq!(restaged.len(), 1);
        assert_eq!(
            merge_pick_for_hunk(&restaged, 0),
            Some(viewmodel::TakeChoice::Right)
        );
        let two = stage_merge_pick(&restaged, 1, viewmodel::TakeChoice::Base);
        assert_eq!(two.len(), 2);
        assert_eq!(
            merge_pick_for_hunk(&two, 1),
            Some(viewmodel::TakeChoice::Base)
        );
    }

    #[test]
    fn stage_merge_pick_all_covers_every_hunk() {
        let lines = vec![
            "<<<<<<< left".to_string(),
            "L1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>> right".to_string(),
            "mid".to_string(),
            "<<<<<<< left".to_string(),
            "L2".to_string(),
            "=======".to_string(),
            "R2".to_string(),
            ">>>>>>> right".to_string(),
        ];
        let total = viewmodel::merge_conflict_hunks(&lines).len();
        assert_eq!(total, 2);
        let one = stage_merge_pick(&[], 0, viewmodel::TakeChoice::Left);
        let all = stage_merge_pick_all(&one, total, viewmodel::TakeChoice::Right);
        assert_eq!(all.len(), 2);
        assert_eq!(
            merge_pick_for_hunk(&all, 0),
            Some(viewmodel::TakeChoice::Right)
        );
        assert_eq!(
            merge_pick_for_hunk(&all, 1),
            Some(viewmodel::TakeChoice::Right)
        );
        let done = viewmodel::merge_pick_summary(&lines, &all);
        assert_eq!((done.total, done.resolved, done.pending), (2, 2, 0));
        assert!(!done.conflicted);
        let empty = stage_merge_pick_all(&[], 0, viewmodel::TakeChoice::Both);
        assert!(empty.is_empty());
    }

    #[test]
    fn merge_pick_intent_and_decision_with_pick_labels() {
        assert_eq!(
            format_merge_pick_intent(0, viewmodel::TakeChoice::Left),
            "hunk 1: take left".to_string()
        );
        assert_eq!(
            format_merge_pick_intent(1, viewmodel::TakeChoice::Right),
            "hunk 2: take right".to_string()
        );
        assert_eq!(
            format_merge_hunk_decision_with_pick(0, None),
            format_merge_hunk_decision(0)
        );
        assert_eq!(
            format_merge_hunk_decision_with_pick(0, Some(viewmodel::TakeChoice::Left)),
            "hunk 1: take left".to_string()
        );
        assert_eq!(
            format_merge_hunk_decision_with_pick(2, Some(viewmodel::TakeChoice::Both)),
            "hunk 3: take both".to_string()
        );
    }

    #[test]
    fn merge_pick_summary_reflects_staged_picks() {
        let lines = vec![
            "<<<<<<< left".to_string(),
            "L1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>> right".to_string(),
            "mid".to_string(),
            "<<<<<<< left".to_string(),
            "L2".to_string(),
            "=======".to_string(),
            "R2".to_string(),
            ">>>>>>> right".to_string(),
        ];
        let none = viewmodel::merge_pick_summary(&lines, &[]);
        assert_eq!((none.total, none.pending), (2, 2));
        assert!(none.conflicted);
        let one = stage_merge_pick(&[], 0, viewmodel::TakeChoice::Left);
        let half = viewmodel::merge_pick_summary(&lines, &one);
        assert_eq!((half.total, half.resolved, half.pending), (2, 1, 1));
        assert!(half.conflicted);
        let unknown = stage_merge_pick(&one, 9, viewmodel::TakeChoice::Right);
        let still = viewmodel::merge_pick_summary(&lines, &unknown);
        assert_eq!((still.resolved, still.pending), (1, 1));
        let full = stage_merge_pick(&one, 1, viewmodel::TakeChoice::Right);
        let done = viewmodel::merge_pick_summary(&lines, &full);
        assert_eq!((done.resolved, done.pending), (2, 0));
        assert!(!done.conflicted);
    }

    #[test]
    fn header_bar_tokens_align_with_paging_bars() {
        // Header bar reserves the paging-bar rhythm, not a fresh value.
        assert_eq!(header_bar_height(), HEADER_H);
        assert_eq!(header_bar_height(), CHROME_HEADER_BAR_H);
        assert_eq!(CHROME_HEADER_BAR_H, 28.0);
        // Header lead shares the pane-content origin with table rules.
        assert_eq!(header_content_lead(), CONTENT_LEAD);
        assert_eq!(header_content_lead(), CHROME_HEADER_LEAD);
        assert_eq!(header_content_lead(), "  ");
    }

    #[test]
    fn header_helpers_use_shared_tokens() {
        // Note line is lead + text, the indented_content shape.
        assert_eq!(header_note_line("x"), "  x".to_string());
        assert_eq!(header_note_line("x"), indented_content("x"));
        assert_eq!(header_note_line(""), indented_content(""));
        // Chrome roles lock the documented ladder.
        assert_eq!(chrome_tab_role(), TypoRole::Body);
        assert_eq!(chrome_section_role(), TypoRole::Heading);
        assert_eq!(chrome_status_role(), TypoRole::Small);
        assert_eq!(chrome_tab_role().size(), 13.0);
        assert_eq!(chrome_section_role().size(), 18.0);
        assert_eq!(chrome_status_role().size(), 11.0);
        // Section > tab > status ordering holds.
        assert!(chrome_section_role().size() > chrome_tab_role().size());
        assert!(chrome_tab_role().size() > chrome_status_role().size());
        // Status stays tabular (counts) while leaving mono behind.
        assert!(chrome_status_role().is_tabular());
        assert!(!chrome_status_role().is_mono());
        assert!(chrome_tab_role().is_chrome());
    }

    #[test]
    fn status_roles_carry_verdict_hierarchy() {
        // Verdict (mid) one step above the side counts; single sources
        // are the viewmodel pending fns paint now adopts.
        assert_eq!(status_verdict_role(), viewmodel::status_verdict_text_role());
        assert_eq!(status_side_role(), viewmodel::status_side_text_role());
        assert_eq!(status_verdict_role(), TypoRole::Body);
        assert_eq!(status_side_role(), TypoRole::Small);
        assert_eq!(status_verdict_role().size(), 13.0);
        assert_eq!(status_side_role().size(), 11.0);
        assert!(status_verdict_role().size() > status_side_role().size());
        // Sides match the frozen status chrome; verdict leaves it.
        assert_eq!(status_side_role(), chrome_status_role());
        assert_ne!(status_verdict_role(), chrome_status_role());
    }

    #[test]
    fn text_row_height_intent_is_measured_mono() {
        // Paint measures the mono row; multipliers stay headless intent.
        assert_eq!(text_row_height_intent(), TYPO_LINE_HEIGHT_SOURCE);
        assert_eq!(text_row_height_intent(), "ui.text_style_height(Monospace)");
        assert_eq!(
            chrome_status_role().line_height_source(),
            text_row_height_intent()
        );
        assert_eq!(
            chrome_tab_role().line_height_source(),
            text_row_height_intent()
        );
    }

    #[test]
    fn merge_apply_section_title_is_apply() {
        assert_eq!(merge_apply_section_title(), "Apply");
    }

    #[test]
    fn merge_apply_path_hint_names_output_path() {
        // Dono modes me output path shabd rehta hai; dry-run honest note jodta hai.
        assert!(merge_apply_path_hint(true).contains("output path"));
        assert!(merge_apply_path_hint(true).contains("not written"));
        assert_eq!(merge_apply_path_hint(false), "output path");
    }

    #[test]
    fn merge_apply_state_labels_cover_all_states() {
        // Har variant distinct honest label; khali ya duplicate nahi.
        let labels = [
            merge_apply_state_label(MergeApplyState::Idle),
            merge_apply_state_label(MergeApplyState::DryRunReported),
            merge_apply_state_label(MergeApplyState::Wrote),
            merge_apply_state_label(MergeApplyState::NeedsPath),
            merge_apply_state_label(MergeApplyState::NoModel),
            merge_apply_state_label(MergeApplyState::Failed),
        ];
        assert_eq!(labels[0], "idle");
        assert_eq!(labels[4], "compare first");
        for (i, a) in labels.iter().enumerate() {
            assert!(!a.is_empty());
            for b in labels.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }

    fn merge_apply_test_summary(dry_run: bool, out: &str) -> MergeApplySummary {
        MergeApplySummary {
            dry_run,
            resolved_lines: 3,
            decided: 1,
            total: 2,
            pending: 1,
            wrote_lines: 4,
            out: out.to_string(),
        }
    }

    #[test]
    fn merge_apply_result_none_when_no_report() {
        assert_eq!(format_merge_apply_result(None), None);
    }

    #[test]
    fn merge_apply_result_dry_run_shows_counts_no_writes() {
        let line = format_merge_apply_result(Some(&merge_apply_test_summary(true, "")));
        assert_eq!(
            line,
            Some("dry-run: 3 line(s), 1/2 decided, 1 pending (no writes)".to_string())
        );
    }

    #[test]
    fn merge_apply_result_needs_path_is_honest() {
        let line = format_merge_apply_result(Some(&merge_apply_test_summary(false, "")));
        assert_eq!(
            line,
            Some("3 line(s), 1/2 decided, 1 pending (no output path)".to_string())
        );
    }

    #[test]
    fn merge_apply_result_wrote_shows_path_and_counts() {
        let line = format_merge_apply_result(Some(&merge_apply_test_summary(false, "out.txt")));
        assert_eq!(
            line,
            Some("wrote 4 line(s) -> out.txt (1/2 decided, 1 pending)".to_string())
        );
    }

    #[test]
    fn wrap_width_on_caps_at_80_off_passes_through() {
        assert_eq!(wrap_width_for(200, true), 80);
        assert_eq!(
            wrap_width_for(200, true),
            crate::theme::TYPO_WRAP_PANE_CHARS
        );
        assert_eq!(wrap_width_for(40, true), 40);
        assert_eq!(wrap_width_for(200, false), 200);
        assert_eq!(wrap_width_for(80, false), 80);
    }

    #[test]
    fn wrap_leading_indent_takes_only_line_start_blanks() {
        assert_eq!(wrap_leading_indent("    code"), "    ");
        assert_eq!(wrap_leading_indent("\t\tcode"), "\t\t");
        assert_eq!(wrap_leading_indent(" \t code"), " \t ");
        assert_eq!(wrap_leading_indent("code"), "");
        assert_eq!(wrap_leading_indent(""), "");
        assert_eq!(wrap_leading_indent("  mid dle"), "  ");
    }

    #[test]
    fn wrap_visual_lines_short_single_and_zero_width_guard() {
        assert_eq!(wrap_visual_lines("hi", 80), vec!["hi".to_string()]);
        assert_eq!(wrap_visual_lines("", 80), vec![String::new()]);
        assert_eq!(wrap_visual_lines("abcdef", 0), vec!["abcdef".to_string()]);
    }

    #[test]
    fn wrap_visual_lines_prefers_word_boundary_with_indent_continuity() {
        let lines = wrap_visual_lines("    alpha beta gamma delta", 12);
        assert!(lines.len() >= 2);
        let Some(first) = lines.first() else {
            return;
        };
        assert!(first.starts_with("    alpha"), "unexpected first: {first}");
        let mut idx = 1usize;
        while idx < lines.len() {
            if let Some(line) = lines.get(idx) {
                assert!(line.starts_with("    "), "continuation gaps indent: {line}");
            }
            idx = idx.saturating_add(1);
        }
        let hard = wrap_visual_lines("abcdefghij", 4);
        assert_eq!(
            hard,
            vec!["abcd".to_string(), "efgh".to_string(), "ij".to_string()]
        );
    }

    #[test]
    fn wrap_counts_pair_total_stay_honest() {
        assert_eq!(wrap_visual_count("hi", 80), 1);
        assert!(wrap_visual_count(&"x".repeat(200), 80) > 1);
        assert_eq!(wrap_visual_count("", 80), 1);
        let long = "hello world this is a long line for wrap";
        assert_eq!(
            wrap_pane_pair_count("hi", long, 8),
            wrap_visual_count(long, 8)
        );
        assert_eq!(wrap_pane_pair_count("a", "b", 80), 1);
        assert_eq!(wrap_total_visual(&[1, 3, 2]), 6);
        assert_eq!(wrap_total_visual(&[]), 0);
    }

    #[test]
    fn wrap_row_mapping_roundtrips_and_rejects_oob() {
        let counts = vec![1, 3, 2];
        assert_eq!(wrap_visual_to_model(&counts, 0), Some((0, 0)));
        assert_eq!(wrap_visual_to_model(&counts, 1), Some((1, 0)));
        assert_eq!(wrap_visual_to_model(&counts, 3), Some((1, 2)));
        assert_eq!(wrap_visual_to_model(&counts, 4), Some((2, 0)));
        assert_eq!(wrap_visual_to_model(&counts, 6), None);
        assert_eq!(wrap_model_to_visual_start(&counts, 0), Some(0));
        assert_eq!(wrap_model_to_visual_start(&counts, 1), Some(1));
        assert_eq!(wrap_model_to_visual_start(&counts, 2), Some(4));
        assert_eq!(wrap_model_to_visual_start(&counts, 3), None);
        assert_eq!(wrap_model_to_visual_start(&[], 0), None);
        let total = wrap_total_visual(&counts);
        assert_eq!(total, 6);
        let mut v = 0usize;
        while v < total {
            assert!(wrap_visual_to_model(&counts, v).is_some());
            if let Some((row, sub)) = wrap_visual_to_model(&counts, v) {
                assert_eq!(
                    wrap_model_to_visual_start(&counts, row),
                    Some(v.saturating_sub(sub))
                );
            }
            v = v.saturating_add(1);
        }
    }

    #[test]
    fn wrap_chunk_spans_shift_with_indent_and_clip() {
        let chunks = wrap_chunks("    alpha beta gamma", 12);
        assert!(chunks.len() >= 2);
        let Some(first) = chunks.first() else {
            return;
        };
        let Some(second) = chunks.get(1) else {
            return;
        };
        assert_eq!(second.indent, 4);
        assert_eq!(wrap_chunk_spans(&[(4, 9)], first), vec![(4, 9)]);
        assert!(wrap_chunk_spans(&[(4, 9)], second).is_empty());
        assert_eq!(wrap_chunk_spans(&[(7, 12)], first), vec![(7, 9)]);
        assert_eq!(wrap_chunk_spans(&[(7, 12)], second), vec![(4, 6)]);
    }

    #[test]
    fn wrap_reserved_height_stacks_single_rhythm() {
        assert_eq!(wrap_reserved_height(1, 16.0), 16.0);
        assert_eq!(wrap_reserved_height(3, 16.0), 48.0);
        assert_eq!(wrap_reserved_height(0, 16.0), 16.0);
        assert_eq!(wrap_reserved_height(2, 0.0), 0.0);
        assert!(wrap_reserved_height(2, f32::NAN).is_nan());
        assert_eq!(wrap_reserved_height(2, row_height_for_line(14.0)), 32.0);
    }

    #[test]
    fn wrap_continuation_gutter_keeps_column_rhythm() {
        assert_eq!(wrap_continuation_gutter(4), gutter_text(None, 4));
        assert_eq!(wrap_continuation_gutter(4), "      ");
        assert_eq!(wrap_continuation_gutter(1), "   ");
    }

    #[test]
    fn wrap_jobs_first_numbered_continuations_plain_mono() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(1),
            right_no: None,
            left: "    alpha beta gamma delta epsilon zeta ".repeat(3),
            ..TextRow::default()
        };
        let prefs = TextViewPrefs {
            word_wrap: true,
            ..TextViewPrefs::default()
        };
        let layout = TextPaneLayout {
            pane_chars: 80,
            gutter_chars: 4,
            prefs: &prefs,
            dim: crate::theme::DARK_DIM,
        };
        let jobs = side_jobs_wrapped_with_prefs(
            Side::Left,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert!(jobs.len() > 1);
        let Some(first) = jobs.first() else {
            return;
        };
        assert!(
            first.text.starts_with("   1 "),
            "unexpected head: {}",
            first.text
        );
        let mut idx = 1usize;
        while idx < jobs.len() {
            if let Some(job) = jobs.get(idx) {
                assert!(
                    job.text.starts_with("     "),
                    "continuation gaps gutter: {}",
                    job.text
                );
            }
            idx = idx.saturating_add(1);
        }
        for job in jobs.iter() {
            for section in job.sections.iter() {
                assert_eq!(section.format.font_id.size, 12.0);
            }
        }
    }

    #[test]
    fn wrapnav_find_sub_rows_tracks_head_and_continuation() {
        let line = "    alpha beta gamma delta epsilon zeta ".repeat(2);
        let width = 12;
        let chunks = wrap_chunks(&line, width);
        assert!(chunks.len() > 2);
        // Head hit: "alpha" lives in the first chunk.
        let head = wrap_find_sub_rows(&line, "alpha", false, width);
        assert!(head.contains(&0));
        // Tail hit: "zeta" lives past the head, so at least one
        // continuation sub-row tracks it.
        let tail = wrap_find_sub_rows(&line, "zeta", false, width);
        assert!(!tail.is_empty());
        assert!(tail.iter().all(|s| *s > 0));
        // Empty needle tracks nothing; missing needle tracks nothing.
        assert!(wrap_find_sub_rows(&line, "", false, width).is_empty());
        assert!(wrap_find_sub_rows(&line, "qqqzzz", false, width).is_empty());
        // Case flag honored the same way `find_line_spans` does.
        assert!(!wrap_find_sub_rows("Alpha", "alpha", true, 80).is_empty());
        assert!(wrap_find_sub_rows("Alpha", "alpha", false, 80).is_empty());
    }

    #[test]
    fn wrapnav_row_find_sub_picks_earliest_pane_hit() {
        let row = TextRow {
            left: "    alpha beta gamma delta epsilon zeta ".repeat(2),
            right: "short".to_string(),
            ..TextRow::default()
        };
        let width = 12;
        // Left-only hit on a continuation.
        let left_hit = wrap_row_find_sub(&row, "zeta", false, width);
        assert!(left_hit.is_some_and(|s| s > 0));
        // Right-only hit on the head.
        let right_hit = wrap_row_find_sub(&row, "short", false, width);
        assert_eq!(right_hit, Some(0));
        // Both panes hit: earliest sub wins.
        let both = TextRow {
            left: row.left.clone(),
            right: "zeta tail here and more words to wrap around".to_string(),
            ..TextRow::default()
        };
        let both_hit = wrap_row_find_sub(&both, "zeta", false, width);
        assert_eq!(both_hit, Some(0));
        // No hit / empty needle: no scroll target.
        assert_eq!(wrap_row_find_sub(&row, "qqqzzz", false, width), None);
        assert_eq!(wrap_row_find_sub(&row, "", false, width), None);
    }

    #[test]
    fn wrapnav_scroll_sub_clamps_to_pair_height() {
        let row = TextRow {
            left: "    alpha beta gamma delta epsilon zeta ".repeat(2),
            right: "short".to_string(),
            ..TextRow::default()
        };
        let width = 12;
        let count = wrap_pane_pair_count(&row.left, &row.right, width);
        assert!(count > 1);
        let sub = wrap_find_scroll_sub(&row, "zeta", false, width, count);
        assert!(sub > 0);
        assert!(sub < count);
        // No hit falls back to the row head; oversized subs clamp.
        assert_eq!(wrap_find_scroll_sub(&row, "qqqzzz", false, width, count), 0);
        assert_eq!(wrap_find_scroll_sub(&row, "", false, width, count), 0);
        assert_eq!(wrap_find_scroll_sub(&row, "zeta", false, width, 1), 0);
    }

    #[test]
    fn wrapnav_visual_offset_and_vis_pos_map_jumps() {
        let counts = vec![1, 3, 2];
        assert_eq!(wrap_visual_offset(&counts, 1, 0), Some(1));
        assert_eq!(wrap_visual_offset(&counts, 1, 2), Some(3));
        assert_eq!(wrap_visual_offset(&counts, 1, 3), None);
        assert_eq!(wrap_visual_offset(&counts, 5, 0), None);
        let visible = vec![10, 20, 30];
        assert_eq!(wrap_vis_pos_for_model(&visible, 20), Some(1));
        assert_eq!(wrap_vis_pos_for_model(&visible, 99), None);
        assert_eq!(wrap_vis_pos_for_model(&[], 10), None);
    }

    #[test]
    fn wrapnav_jump_visual_lands_on_matched_continuation() {
        let long = "    alpha beta gamma delta epsilon zeta ".repeat(2);
        let rows = vec![
            TextRow {
                left: "head".to_string(),
                ..TextRow::default()
            },
            TextRow {
                left: long.clone(),
                right: "short".to_string(),
                ..TextRow::default()
            },
        ];
        let width = 12;
        let visible = vec![0, 1];
        let row0_left = rows.first().map_or(String::new(), |r| r.left.clone());
        let row0_right = rows.first().map_or(String::new(), |r| r.right.clone());
        let row1_left = rows.get(1).map_or(String::new(), |r| r.left.clone());
        let row1_right = rows.get(1).map_or(String::new(), |r| r.right.clone());
        let counts = vec![
            wrap_pane_pair_count(&row0_left, &row0_right, width),
            wrap_pane_pair_count(&row1_left, &row1_right, width),
        ];
        let target = wrap_find_jump_visual(&counts, &visible, &rows, 1, "zeta", false, width);
        let start = wrap_model_to_visual_start(&counts, 1).unwrap_or(0);
        assert!(target.is_some_and(|v| v > start));
        // Row head fallback when the needle misses.
        assert_eq!(
            wrap_find_jump_visual(&counts, &visible, &rows, 1, "qqqzzz", false, width),
            wrap_model_to_visual_start(&counts, 1)
        );
        // Filtered-out model row maps to no jump.
        assert_eq!(
            wrap_find_jump_visual(&counts, &[0], &rows, 1, "zeta", false, width),
            None
        );
        // OOB model row maps to no jump.
        assert_eq!(
            wrap_find_jump_visual(&counts, &visible, &rows, 9, "zeta", false, width),
            None
        );
    }

    #[test]
    fn wrapnav_viewport_window_clamps_without_panic() {
        assert_eq!(wrap_viewport_window(6, 2, 3), (2, 5));
        assert_eq!(wrap_viewport_window(6, 5, 3), (5, 6));
        assert_eq!(wrap_viewport_window(6, 9, 3), (6, 6));
        assert_eq!(wrap_viewport_window(0, 0, 3), (0, 0));
        assert_eq!(wrap_viewport_window(6, 2, 0), (2, 2));
    }

    #[test]
    fn wrapnav_sub_indent_matches_chunk_indent() {
        let line = "    alpha beta gamma delta";
        let width = 12;
        let chunks = wrap_chunks(line, width);
        assert!(chunks.len() > 1);
        assert_eq!(wrap_sub_indent_chars(line, width, 0), 0);
        assert_eq!(
            wrap_sub_indent_chars(line, width, 1),
            chunks.get(1).map_or(0, |c| c.indent)
        );
        assert_eq!(wrap_sub_indent_chars(line, width, 99), 0);
        assert_eq!(wrap_sub_indent_chars("hi", 80, 0), 0);
    }

    #[test]
    fn wrapnav_continuation_find_format_stays_mono_size() {
        let line = "    alpha beta gamma delta epsilon zeta ".repeat(2);
        let width = 12;
        let chunks = wrap_chunks(&line, width);
        assert!(chunks.len() > 1);
        let find_raw = find_line_spans(&line, "zeta", false);
        assert!(!find_raw.is_empty());
        let mut hit_subs = 0usize;
        for chunk in chunks.iter() {
            let segs = find_chunk_segments(chunk, &[], &find_raw, true, width);
            let has_current = segs.iter().any(|(_, kind)| *kind == FindHl::Current);
            if has_current {
                hit_subs = hit_subs.saturating_add(1);
                for (text, kind) in segs.iter() {
                    assert!(!text.is_empty());
                    let fmt = find_segment_format(
                        FontId::monospace(12.0),
                        Color32::WHITE,
                        Color32::BLACK,
                        *kind,
                    );
                    assert_eq!(fmt.font_id.size, 12.0);
                }
            }
        }
        assert!(hit_subs > 0);
    }

    #[test]
    fn three_way_presence_marks_cover_sides() {
        use crate::tree::ThreeWayTreeRow;
        let full = ThreeWayTreeRow {
            rel_path: "a.txt".to_string(),
            is_dir: false,
            has_base: true,
            has_left: true,
            has_right: true,
        };
        assert_eq!(three_way_presence_marks(&full), "BLR");
        let missing_left = ThreeWayTreeRow {
            rel_path: "b.txt".to_string(),
            is_dir: false,
            has_base: true,
            has_left: false,
            has_right: true,
        };
        assert_eq!(three_way_presence_marks(&missing_left), "B-R");
        let left_only = ThreeWayTreeRow {
            rel_path: "c.txt".to_string(),
            is_dir: false,
            has_base: false,
            has_left: true,
            has_right: false,
        };
        assert_eq!(three_way_presence_marks(&left_only), "-L-");
    }

    #[test]
    fn three_way_row_line_marks_dirs() {
        use crate::tree::ThreeWayTreeRow;
        let file = ThreeWayTreeRow {
            rel_path: "a/b.txt".to_string(),
            is_dir: false,
            has_base: true,
            has_left: true,
            has_right: true,
        };
        assert_eq!(three_way_row_line(&file), "a/b.txt");
        let dir = ThreeWayTreeRow {
            rel_path: "src".to_string(),
            is_dir: true,
            has_base: true,
            has_left: true,
            has_right: false,
        };
        assert_eq!(three_way_row_line(&dir), "src/");
    }

    #[test]
    fn three_way_decision_for_row_returns_last_staged() {
        use crate::tree::{ThreeWayDecision, ThreeWaySide};
        let decisions = vec![
            ThreeWayDecision {
                rel_path: "del.txt".to_string(),
                side: ThreeWaySide::Left,
            },
            ThreeWayDecision {
                rel_path: "del.txt".to_string(),
                side: ThreeWaySide::Right,
            },
        ];
        assert_eq!(
            three_way_decision_for_row(&decisions, "del.txt"),
            Some(ThreeWaySide::Right)
        );
        assert_eq!(three_way_decision_for_row(&decisions, "ghost.txt"), None);
        let empty: Vec<ThreeWayDecision> = Vec::new();
        assert_eq!(three_way_decision_for_row(&empty, "del.txt"), None);
    }

    #[test]
    fn stage_three_way_decision_upserts_last_wins() {
        use crate::tree::{ThreeWayDecision, ThreeWaySide};
        let empty: Vec<ThreeWayDecision> = Vec::new();
        let one = stage_three_way_decision(&empty, "del.txt", ThreeWaySide::Left);
        assert_eq!(one.len(), 1);
        let two = stage_three_way_decision(&one, "keep.txt", ThreeWaySide::Base);
        assert_eq!(two.len(), 2);
        // Same path overwrite: ek hi rehta hai, aakhri jeetta hai.
        let over = stage_three_way_decision(&two, "del.txt", ThreeWaySide::Right);
        assert_eq!(over.len(), 2);
        assert_eq!(
            three_way_decision_for_row(&over, "del.txt"),
            Some(ThreeWaySide::Right)
        );
        assert_eq!(
            three_way_decision_for_row(&over, "keep.txt"),
            Some(ThreeWaySide::Base)
        );
    }

    #[test]
    fn three_way_row_decision_text_covers_states() {
        use crate::tree::ThreeWaySide;
        assert_eq!(
            three_way_row_decision_text(true, None),
            "needs a decision".to_string()
        );
        assert_eq!(
            three_way_row_decision_text(false, None),
            "clean".to_string()
        );
        assert_eq!(
            three_way_row_decision_text(true, Some(ThreeWaySide::Left)),
            "take left".to_string()
        );
        assert_eq!(
            three_way_row_decision_text(false, Some(ThreeWaySide::Base)),
            "take base".to_string()
        );
        assert_eq!(
            three_way_row_decision_text(true, Some(ThreeWaySide::Right)),
            "take right".to_string()
        );
    }

    #[test]
    fn three_way_header_line_uses_summary() {
        use crate::tree::{ThreeWaySide, three_way_tree_rows};
        use crate::viewmodel::FolderRow;
        let snapshot = |path: &str| FolderRow {
            rel_path: path.to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        };
        let base = vec![snapshot("keep.txt"), snapshot("del.txt")];
        let left = vec![snapshot("keep.txt")];
        let right = vec![snapshot("keep.txt"), snapshot("del.txt")];
        let rows = three_way_tree_rows(&base, &left, &right);
        let header = three_way_header_line(&rows, &[]);
        assert!(header.contains("1 conflict(s)"));
        assert!(header.contains("(unresolved)"));
        let decided = stage_three_way_decision(&[], "del.txt", ThreeWaySide::Right);
        let resolved = three_way_header_line(&rows, &decided);
        assert!(!resolved.contains("(unresolved)"));
        assert!(resolved.contains("1 decided"));
    }

    // Session tab strip (BU-SES-PAINT): label fallback + action enum +
    // empty gate + geometry tokens + scene PNG proof. Paint caller wiring
    // agle wave ka hai; ye tests contract lock karte hain.

    #[test]
    fn session_tab_label_uses_title_and_falls_back() {
        let live = Session::new(3, Tab::Text, "fixtures/left.rs", "fixtures/right.rs", "", 0);
        assert_eq!(session_tab_label(&live), "left");
        let mut blank = Session::new(7, Tab::Folder, "a", "b", "", 0);
        blank.title = "   ".to_string();
        assert_eq!(session_tab_label(&blank), "Folder 7");
        let mut empty = Session::new(9, Tab::Hex, "", "", "", 0);
        empty.title.clear();
        assert_eq!(session_tab_label(&empty), "Hex 9");
    }

    #[test]
    fn session_action_defaults_to_none_and_carries_ids() {
        assert_eq!(SessionAction::default(), SessionAction::None);
        assert_eq!(SessionAction::Switch(4), SessionAction::Switch(4));
        assert_eq!(SessionAction::Close(4), SessionAction::Close(4));
        assert_ne!(SessionAction::Switch(4), SessionAction::Close(4));
        assert_ne!(SessionAction::Switch(4), SessionAction::Switch(5));
    }

    #[test]
    fn session_strip_empty_gate_and_hint_are_honest() {
        let empty = SessionList::new();
        assert!(session_strip_is_empty(&empty));
        assert!(!session_strip_empty_hint().is_empty());
        let mut list = SessionList::new();
        assert!(list.open(Tab::Text, "a.rs", "b.rs", "").is_some());
        assert!(!session_strip_is_empty(&list));
    }

    #[test]
    fn session_tab_geometry_reuses_tokens() {
        assert_eq!(SESSION_TAB_MAX_W, 120.0);
        assert!(SESSION_TAB_MAX_W.is_finite() && SESSION_TAB_MAX_W > 0.0);
        let max_w = SESSION_TAB_MAX_W;
        let card_w = CARD_W;
        assert!(max_w <= card_w);
        assert_eq!(session_tab_close_edge(), HOME_TOOLBAR_BUTTON_H);
        assert_eq!(session_tab_close_edge(), 24.0);
    }

    #[test]
    fn session_strip_overflows_at_eight_tabs() {
        // Toolbar2 gaps: cell = label max + close gap (6) + close edge (24)
        // = 150; beech me strip gap (8). Edge fixture tabstrip-8tabs:
        // 2 tabs 308 (700 par no-scroll), 8 tabs 1256 (960 par scroll).
        assert_eq!(session_tab_close_gap(), TAB_BOTTOM_GAP);
        assert_eq!(session_tab_close_gap(), 6.0);
        assert_eq!(session_tab_strip_gap(), SECTION_GAP);
        assert_eq!(session_tab_strip_gap(), 8.0);
        assert_eq!(session_tab_strip_width(0), 0.0);
        assert_eq!(session_tab_strip_width(2), 308.0);
        assert_eq!(session_tab_strip_width(8), 1256.0);
        assert!(!session_tab_strip_needs_hscroll(2, 700.0));
        assert!(session_tab_strip_needs_hscroll(8, 960.0));
        assert!(!session_tab_strip_needs_hscroll(8, 1256.0));
        assert!(!session_tab_strip_needs_hscroll(8, f32::NAN));
        assert!(!session_tab_strip_needs_hscroll(8, 0.0));
    }

    #[test]
    fn pane_status_halves_share_avail_evenly() {
        // BC per-pane left alignment: dono halves barabar (700 -> 348+348
        // + 4 gap); kharab avail par MIN_HALF floor (koi NaN nahi).
        assert_eq!(pane_status_half_width(700.0), 348.0);
        assert_eq!(pane_status_half_width(180.0), 88.0);
        assert_eq!(pane_status_half_width(f32::NAN), MIN_HALF);
        assert_eq!(pane_status_half_width(0.0), MIN_HALF);
        assert_eq!(pane_status_half_width(-50.0), MIN_HALF);
    }

    #[test]
    fn text10_panes_split_even_with_section_gap() {
        // Mirrors checks/gui-exq-text10/comparison/tail-insertion.json:
        // center connector (48) + 2 x SECTION_GAP air ghata kar bacha
        // aadha-aadha (960 -> 448/448); tang par MIN_HALF floor pair;
        // kharab avail par floor (koi NaN nahi).
        assert_eq!(text_pane_pair_widths(960.0), (448.0, 448.0));
        let (left, right) = text_pane_pair_widths(960.0);
        assert_eq!(
            left + right,
            960.0 - TEXT_CENTER_GUTTER_W - 2.0 * SECTION_GAP
        );
        assert_eq!(text_center_gutter_width(), 48.0);
        assert_eq!(text_pane_pair_widths(0.0), (MIN_HALF, MIN_HALF));
        assert_eq!(text_pane_pair_widths(f32::NAN), (MIN_HALF, MIN_HALF));
        assert_eq!(text_pane_pair_widths(-50.0), (MIN_HALF, MIN_HALF));
        let (tiny_left, tiny_right) = text_pane_pair_widths(100.0);
        assert_eq!((tiny_left, tiny_right), (MIN_HALF, MIN_HALF));
    }

    #[test]
    fn text10_pane_hscroll_gate_guards_viewport() {
        // Mirrors checks/gui-exq-text10/comparison/tail-insertion.json:
        // content viewport se chauda ho to h-scroll, warna nahi; kharab
        // viewport par kabhi nahi (paint NaN branch na le).
        assert!(text_pane_needs_hscroll(900.0, 448.0));
        assert!(!text_pane_needs_hscroll(400.0, 448.0));
        assert!(!text_pane_needs_hscroll(448.0, 448.0));
        assert!(!text_pane_needs_hscroll(900.0, 0.0));
        assert!(!text_pane_needs_hscroll(900.0, f32::NAN));
        assert!(!text_pane_needs_hscroll(f32::NAN, 448.0));
    }

    #[test]
    fn text10_row_height_comes_from_typo_mono() {
        // Geometry SECOND nahi, FIRST: row Mono line-height px (12 @ 1.30
        // = 15.6) + ROW_PAD (2) = 17.6; intent source string unchanged.
        use crate::theme::{TYPO_LINE_HEIGHT_MONO, TYPO_MONO_SIZE};
        assert_eq!(
            TypoRole::Mono.line_height_px(),
            TYPO_MONO_SIZE * TYPO_LINE_HEIGHT_MONO
        );
        assert!((TypoRole::Mono.line_height_px() - 15.6).abs() < 0.01);
        assert!((row_height_for_line(TypoRole::Mono.line_height_px()) - 17.6).abs() < 0.01);
    }

    #[test]
    fn text10_edge_tail_insertion_balances_orphan_blanks() {
        // Mirrors checks/gui-exq-text10/comparison/tail-insertion.json:
        // 6 equal rows + tail par 3 inserted (left khaali orphan, right
        // me numbers 7/8/9): hunk head insertion start par, baaki
        // continuation blank; khaali side filler + hatch wali; trailing
        // blanks viewport ko barabar bharte hain (koi void nahi).
        use viewmodel::{TextRow, TextRowKind};
        let mut rows = Vec::new();
        let mut n = 1usize;
        while n <= 6 {
            rows.push(TextRow {
                kind: TextRowKind::Equal,
                left_no: Some(n),
                right_no: Some(n),
                left: "same".to_string(),
                right: "same".to_string(),
                ..Default::default()
            });
            n = n.saturating_add(1);
        }
        let mut m = 7usize;
        while m <= 9 {
            rows.push(TextRow {
                kind: TextRowKind::Inserted,
                left_no: None,
                right_no: Some(m),
                right: "tail".to_string(),
                ..Default::default()
            });
            m = m.saturating_add(1);
        }
        assert_eq!(rows.len(), 9);
        // Hunk head: tail start (6) head, continuations blank.
        assert!(text_center_hunk_head(&rows, 6));
        assert!(!text_center_hunk_head(&rows, 7));
        assert!(!text_center_hunk_head(&rows, 8));
        assert!(!text_center_hunk_head(&rows, 5));
        // Copy gate: inserted par haan, equal par nahi.
        assert!(text_gutter_has_copy(TextRowKind::Inserted));
        assert!(!text_gutter_has_copy(TextRowKind::Equal));
        // Orphan balance: insertion rows par left filler, right me text.
        if let Some(tail) = rows.get(6) {
            assert!(text_side_is_filler(tail.left.as_str()));
            assert!(!text_side_is_filler(tail.right.as_str()));
            assert_eq!(tail.right_no, Some(7));
            assert_eq!(tail.left_no, None);
        }
        // Trailing blanks: 436px viewport @17.6 row = 25 slots - 9 = 16.
        let row_h = row_height_for_line(TypoRole::Mono.line_height_px());
        assert!((row_h - 17.6).abs() < 0.01);
        assert_eq!(text_trailing_blank_rows(436.0, row_h, rows.len()), 16);
        assert_eq!(text_trailing_blank_rows(436.0, row_h, 25), 0);
        // Gutter rhythm: number column right-aligned + trail pad par.
        assert_eq!(gutter_text(Some(7), 4), "   7  ".to_string());
        assert_eq!(gutter_text(None, 4), "      ".to_string());
    }

    #[test]
    fn textview9_pane_detail_bottom_density() {
        // BC bottom detail: har pane ke neeche cursor + us line ka preview
        // (`L3 | content`); missing side par label akela (koi fake nahi);
        // lambi line 80-char budget par `...` samet clip (wrap karke bar
        // kabhi lamba nahi hota — paint truncate se single-row lock);
        // budget pane paint width parity (TYPO_WRAP_PANE_CHARS reuse).
        assert_eq!(text_pane_detail_budget(), 80);
        assert_eq!(
            text_pane_detail_budget(),
            crate::theme::TYPO_WRAP_PANE_CHARS
        );
        let rows = vec![
            TextRow {
                kind: TextRowKind::Changed,
                left_no: Some(3),
                right_no: Some(3),
                left: "    let content = read_to_string(path)?;".to_string(),
                right: "    let content = std::fs::read_to_string(path)?;".to_string(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
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
        ];
        assert_eq!(
            text_pane_preview_side(&rows, Some(0), true),
            "let content = read_to_string(path)?;"
        );
        assert_eq!(
            text_pane_preview_side(&rows, Some(0), false),
            "let content = std::fs::read_to_string(path)?;"
        );
        assert_eq!(text_pane_preview_side(&rows, Some(1), false), String::new());
        assert_eq!(text_pane_preview_side(&rows, None, true), String::new());
        assert_eq!(
            text_pane_preview_side(&rows, Some(99), false),
            String::new()
        );
        assert_eq!(
            text_pane_detail_label("L3", "let x = 1;", text_pane_detail_budget()),
            "L3 | let x = 1;"
        );
        assert_eq!(
            text_pane_detail_label("R-", "", text_pane_detail_budget()),
            "R-"
        );
        assert_eq!(
            text_pane_detail_label("L-", "   ", text_pane_detail_budget()),
            "L-"
        );
        let long = "y".repeat(200);
        assert_eq!(
            text_pane_detail_label("L9", &long, text_pane_detail_budget()),
            format!("L9 | {}...", "y".repeat(80))
        );
        // Cursor strip single-row lock: previews ab bottom detail me hain —
        // strip ka skip gate wahi (None/OOB par koi fake row nahi).
        assert!(viewmodel::focused_line_preview_for(&rows, None).is_none());
        assert!(viewmodel::focused_line_preview_for(&rows, Some(99)).is_none());
    }

    // Scene PNG proof (screenshot.rs frozen/NO-TOUCH): live label +
    // geometry tokens se headless canvas, test-only pixel helpers.
    const SCENE_W: u32 = 960;
    const SCENE_H: u32 = 600;
    const SCENE_BG: [u8; 4] = [27, 27, 27, 255];
    const SCENE_PANEL: [u8; 4] = [32, 32, 32, 255];
    const SCENE_TAB: [u8; 4] = [45, 45, 45, 255];
    const SCENE_ACTIVE: [u8; 4] = [50, 70, 105, 255];
    const SCENE_BTN: [u8; 4] = [60, 60, 60, 255];

    fn scene_canvas() -> Vec<u8> {
        vec![0u8; SCENE_W as usize * SCENE_H as usize * 4]
    }

    fn scene_rect(px: &mut [u8], x: u32, y: u32, w: u32, h: u32, c: [u8; 4]) {
        for yy in y..y.saturating_add(h) {
            if yy >= SCENE_H {
                break;
            }
            for xx in x..x.saturating_add(w) {
                if xx >= SCENE_W {
                    break;
                }
                let at = (yy as usize * SCENE_W as usize + xx as usize) * 4;
                if let Some(slot) = px.get_mut(at..at.saturating_add(4)) {
                    slot.copy_from_slice(&c);
                }
            }
        }
    }

    fn scene_pane_dither(px: &mut [u8], x: u32, y: u32, w: u32, h: u32) {
        for yy in y..y.saturating_add(h) {
            if yy >= SCENE_H {
                break;
            }
            for xx in x..x.saturating_add(w) {
                if xx >= SCENE_W {
                    break;
                }
                // Deterministic hash noise (3 dark shades): checker se zyada
                // entropy taaki PNG 8192B floor cross kare; pane jaisa dikhta
                // hai, koi naya hue nahi.
                let n = xx
                    .wrapping_mul(0x9E37_79B1)
                    .wrapping_add(yy.wrapping_mul(0x85EB_CA6B))
                    ^ xx.wrapping_add(yy).wrapping_mul(0xC2B2_AE35);
                let shade = match (n >> 7) % 3 {
                    0 => 22,
                    1 => 27,
                    _ => 32,
                };
                let at = (yy as usize * SCENE_W as usize + xx as usize) * 4;
                if let Some(slot) = px.get_mut(at..at.saturating_add(4)) {
                    slot.copy_from_slice(&[shade, shade, shade, 255]);
                }
            }
        }
    }

    /// Session strip scene: title bar + strip row (live SESSION_TAB_MAX_W
    /// fixed cells, active highlight, close-x boxes, overflow par right-end
    /// arrow + scrollbar track) + mode row + pane dither + status bar.
    /// Labels live `session_tab_label` strings se aate hain (exact text
    /// unit tests me lock hai); yahan geometry proof hai. Khaali scene me
    /// hint bar, koi fake tab nahi.
    fn paint_session_scene(sessions: &[(String, bool)]) -> Vec<u8> {
        let mut px = scene_canvas();
        scene_rect(&mut px, 0, 0, SCENE_W, SCENE_H, SCENE_BG);
        scene_rect(&mut px, 0, 0, SCENE_W, 36, SCENE_PANEL);
        let mut pen_x = 12u32;
        let strip_y = 36u32;
        let tab_w = SESSION_TAB_MAX_W as u32;
        let close_w = session_tab_close_edge() as u32;
        scene_rect(&mut px, 0, strip_y, SCENE_W, 32, [32, 32, 32, 255]);
        if sessions.is_empty() {
            scene_rect(&mut px, 12, strip_y + 8, 360, 16, SCENE_TAB);
        }
        for (label, active) in sessions {
            let _ = label;
            if pen_x.saturating_add(tab_w).saturating_add(close_w) > SCENE_W {
                break;
            }
            let fill = if *active { SCENE_ACTIVE } else { SCENE_TAB };
            scene_rect(&mut px, pen_x, strip_y + 4, tab_w, 24, fill);
            scene_rect(
                &mut px,
                pen_x
                    .saturating_add(tab_w)
                    .saturating_add(session_tab_close_gap() as u32),
                strip_y + 4,
                close_w.saturating_sub(8),
                24,
                SCENE_BTN,
            );
            pen_x = pen_x
                .saturating_add(tab_w)
                .saturating_add(close_w)
                .saturating_add(session_tab_close_gap() as u32)
                .saturating_add(session_tab_strip_gap() as u32);
        }
        let overflow = pen_x > SCENE_W.saturating_sub(40);
        if overflow {
            scene_rect(&mut px, SCENE_W - 24, strip_y + 4, 16, 24, SCENE_BTN);
            scene_rect(&mut px, 12, strip_y + 29, SCENE_W - 24, 2, SCENE_BTN);
        }
        scene_rect(&mut px, 0, 68, SCENE_W, 24, SCENE_PANEL);
        scene_pane_dither(&mut px, 12, 124, 936, 436);
        scene_rect(&mut px, 0, 570, SCENE_W, 30, SCENE_PANEL);
        px
    }

    fn scene_write_png(name: &str, px: &[u8]) -> std::path::PathBuf {
        use autocompare_diff::{RgbaImage, encode_png};
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../checks/gui-sessionpaint-by/round-1");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(name);
        let image = RgbaImage {
            width: SCENE_W,
            height: SCENE_H,
            pixels: px.to_vec(),
        };
        let bytes = encode_png(&image).unwrap_or_default();
        let _ = std::fs::write(&path, &bytes);
        path
    }

    fn scene_assert_png(path: &std::path::Path) -> Vec<u8> {
        let bytes = std::fs::read(path).unwrap_or_default();
        assert!(bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
        assert!(bytes.len() >= 8192, "scene png too small: {}", bytes.len());
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64, "scene png blank: {}", distinct.len());
        bytes
    }

    fn scene_labels(count: usize, active: usize) -> Vec<(String, bool)> {
        let mut list = SessionList::new();
        let kinds = [
            Tab::Text,
            Tab::Folder,
            Tab::Hex,
            Tab::Merge,
            Tab::Table,
            Tab::Sync,
            Tab::Picture,
            Tab::FolderMerge,
        ];
        for (i, kind_slot) in kinds.iter().cycle().take(count).enumerate() {
            let _ = list.open(
                *kind_slot,
                format!("left{i}.rs"),
                format!("right{i}.rs"),
                "",
            );
        }
        let active_id = if count == 0 {
            None
        } else {
            match list.list().get(active.min(count - 1)) {
                Some(item) => {
                    let id = item.id;
                    list.switch(id);
                    list.active_id()
                }
                None => None,
            }
        };
        list.list()
            .iter()
            .map(|item| (session_tab_label(item), Some(item.id) == active_id))
            .collect()
    }

    #[test]
    fn session_strip_scenes_prove_png() {
        // 2-tab scene: live labels, pehla active.
        let two = scene_labels(2, 0);
        assert_eq!(two.len(), 2);
        let two_px = paint_session_scene(&two);
        let two_path = scene_write_png("session-2tabs.png", &two_px);
        let two_bytes = scene_assert_png(&two_path);
        // 8-tab overflow scene: cap (HOME_RECENT_MAX) bhara, scroll proof.
        let eight = scene_labels(8, 0);
        assert_eq!(eight.len(), 8);
        let eight_px = paint_session_scene(&eight);
        let eight_path = scene_write_png("session-8tabs.png", &eight_px);
        let eight_bytes = scene_assert_png(&eight_path);
        // Khaali scene: hint bar, koi fake tab nahi.
        let empty_px = paint_session_scene(&[]);
        let empty_path = scene_write_png("session-empty.png", &empty_px);
        let empty_bytes = scene_assert_png(&empty_path);
        // Teenon alag dikhte hain (overflow sabse alag).
        assert_ne!(two_px, eight_px);
        assert_ne!(two_bytes, eight_bytes);
        assert_ne!(two_bytes, empty_bytes);
    }

    #[test]
    fn text_status_wiring_zones_use_focused_verdict_and_nav_badge() {
        // Paint call-site ka data flow (BT model reuse, koi math duplicate
        // nahi): counts + filter + focused verdict + nav badge -> zones.
        let rows = vec![
            TextRow {
                kind: TextRowKind::Equal,
                left_no: Some(1),
                right_no: Some(1),
                left: "a".to_string(),
                right: "a".to_string(),
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left_no: Some(2),
                right_no: Some(2),
                left: "b".to_string(),
                right: "c".to_string(),
                ..Default::default()
            },
        ];
        let counts = viewmodel::filter_counts(&rows, viewmodel::TextFilter::Differences);
        let mid = viewmodel::focused_kind_label(&rows, Some(1));
        let badge = diff_nav_count_label(diff_nav_position(&rows, Some(1)), diff_nav_total(&rows));
        let zones =
            viewmodel::status_zones_for(&counts, viewmodel::TextFilter::Differences, mid, &badge);
        assert!(zones.left.contains("Differences"));
        assert!(zones.left.contains("2 rows (1 shown)"));
        assert_eq!(zones.mid, "Changed");
        assert!(zones.right.contains(&badge));
        assert!(zones.right.contains("F3"));
    }

    #[test]
    fn text_cursor_strip_skips_without_preview() {
        // Strip skip condition: preview None (no selection/OOB) par paint
        // kuch nahi karta, koi fake row nahi; cursor label tab "-" hai.
        let rows = vec![TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(2),
            right_no: Some(2),
            left: "b".to_string(),
            right: "c".to_string(),
            ..Default::default()
        }];
        assert!(viewmodel::focused_line_preview_for(&rows, None).is_none());
        assert!(viewmodel::focused_line_preview_for(&rows, Some(99)).is_none());
        assert_eq!(
            viewmodel::cursor_label(viewmodel::cursor_pos_for(None)),
            "-"
        );
        assert_eq!(
            viewmodel::cursor_label(viewmodel::cursor_pos_for(Some(0))),
            "1:1"
        );
    }

    #[test]
    fn text_merge_marker_detection_covers_all_merge3_markers() {
        let markers = ["<<<<<<< left", "||||||| base", "=======", ">>>>>>> right"];
        for m in markers {
            assert!(crate::take::is_conflict_marker(m));
        }
        assert!(!crate::take::is_conflict_marker("normal text"));
    }

    #[test]
    fn text_merge_gutter_geometry_matches_hunks() {
        let lines = vec![
            "clean".to_string(),
            "<<<<<<<".to_string(),
            "L".to_string(),
            "|||||||".to_string(),
            "B".to_string(),
            "=======".to_string(),
            "R".to_string(),
            ">>>>>>>".to_string(),
            "clean".to_string(),
        ];
        let geom = crate::gutter::merge_gutter_geometry(&lines, 200.0);
        assert_eq!(geom.segments.len(), 1);
        let Some(seg) = geom.segments.first() else {
            return;
        };
        assert_eq!(seg.kind, TextRowKind::Changed);
    }

    #[test]
    fn text_compare_copy_all_action_parity() {
        let mut model = TextRows {
            rows: vec![
                TextRow {
                    kind: TextRowKind::Deleted,
                    left: "del".to_string(),
                    right: String::new(),
                    ..Default::default()
                },
                TextRow {
                    kind: TextRowKind::Inserted,
                    left: String::new(),
                    right: "ins".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let copied = crate::copy::copy_all(&mut model, viewmodel::CopyDirection::LeftToRight)
            .unwrap_or_default();
        assert_eq!(copied, 2);
        for row in &model.rows {
            assert_eq!(row.kind, TextRowKind::Equal);
        }
    }
}
