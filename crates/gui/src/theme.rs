//! Color contrast calculations according to WCAG 2.1 specifications.
//!
//! Provides headless contrast checks plus the typed design-token
//! palette (Dark/Light) that `views.rs` paints with.
//!
//! Geometry and typography live here as headless tokens (this wave);
//! colors below are FROZEN from the COLOR wave — no hue changes.
//!
//! Geometry scale (px): canonical spacing 4/8/12/16/24. Panel
//! gutters, tree indent and gutter bars all snap to this scale;
//! `gutter.rs` / `tree.rs` apply the tokens purely so paint stays
//! aligned without touching `views.rs`.
//!
//! Typography scale (family, weight, size, line-height,
//! letter-spacing): heading 18 strong, body 13, small 11, mono diff 12
//! (13 is the tolerated upper bound), Home card title strong 14. Every
//! [`TypoRole`] maps to one egui text style via
//! [`TypoRole::text_style`] and paints via [`TypoRole::rich`], which
//! applies family + weight + size + letter-spacing
//! ([`TYPO_LETTER_SPACING`], a documented 0.0 no-op: egui has no
//! tracking, so weight + line-height carry hierarchy). Line-height is
//! always the measured monospace row height at paint time in
//! `views.rs`; there is no per-role override
//! ([`TYPO_LINE_HEIGHT_SOURCE`]).
//!
//! Exquisite intent (headless, paint-neutral this wave):
//! [`TypoRole::font_stack`] (UI sans vs mono stack),
//! [`TypoRole::weight_value`] (400 regular / 500 medium / 600 semibold),
//! [`TypoRole::line_height`] (chrome 1.2 / body 1.45 / mono 1.35),
//! [`TypoRole::letter_spacing_em`] (headers 0.02em, rest 0.0) and
//! [`TypoRole::is_tabular`] (gutter/hex/counts) record the target
//! hierarchy; paint adopts them in a later wave.
//!
//! Pane hierarchy: app header -> Heading; Home cards ->
//! HomeTitle/HomeDesc/HomeButton; diff panes, summaries, status lines
//! -> Mono; hints, subtitles, counts -> Small; plain labels, buttons
//! -> Body.
//!
//! Color model: the diff accents are fixed (a deleted row reads the
//! same in light and dark mode, so those colors carry their own
//! contrast), while the chrome (base text, dim/glyph, red/green/blue
//! markers, gutter track) is per-mode so both [`DARK_PALETTE`] and
//! [`LIGHT_PALETTE`] satisfy WCAG AA via [`theme_contrast_ok`].
//! Paint picks a side with [`palette`] / `*_for(dark)` using
//! `ui.visuals().dark_mode`; no color values move in this wave.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use eframe::egui::{self, Color32};

// ----------------------------------------------------------------------------
// Typography scale (headless, ponytail minimal)
// ----------------------------------------------------------------------------

/// Heading size in px: strong weight, proportional family.
pub const TYPO_HEADING_SIZE: f32 = 18.0;
/// Body size in px: default weight, proportional family.
pub const TYPO_BODY_SIZE: f32 = 13.0;
/// Small size in px: weak weight, proportional family.
pub const TYPO_SMALL_SIZE: f32 = 11.0;
/// Mono diff size in px: regular weight, monospace family.
pub const TYPO_MONO_SIZE: f32 = 12.0;
/// Tolerated mono upper bound in px (12-13 range for dense diffs).
pub const TYPO_MONO_SIZE_MAX: f32 = 13.0;
/// Home card title size in px: strong weight, proportional family.
/// Matches the single `.size(14.0)` in `views::paint_home`.
pub const TYPO_HOME_TITLE_SIZE: f32 = 14.0;
/// Wrap cap in chars: panes paint at most this wide when word wrap is on.
pub const TYPO_WRAP_PANE_CHARS: usize = 80;
/// Documented full-width example in chars (paint keeps 200 when wrap off).
pub const TYPO_FULL_PANE_CHARS: usize = 200;
/// Letter-spacing in px: egui has no tracking support, always 0.0.
/// Hierarchy comes from size + weight + line-height instead. No dep added.
pub const TYPO_LETTER_SPACING: f32 = 0.0;
/// Line-height source: always the measured monospace row height.
/// Every [`TypoRole`] paints with this; no per-role override.
pub const TYPO_LINE_HEIGHT_SOURCE: &str = "ui.text_style_height(Monospace)";

// ----------------------------------------------------------------------------
// Exquisite type tokens (headless intent, paint-neutral this wave).
// Paint still uses `rich()` / `rich_zoomed()` unchanged (0.0 tracking,
// shared measured mono row height) so PNG stays bit-identical; the
// tokens below record the intended hierarchy for a future paint pass.
// ----------------------------------------------------------------------------

/// UI sans stack for body/small/chrome roles (system sans first).
pub const TYPO_UI_SANS_STACK: &str = "system-ui, sans-serif";
/// Monospace stack for hex/text-cells/table-cells/gutter (system mono first).
pub const TYPO_MONO_STACK: &str = "ui-monospace, monospace";
/// Regular body weight (400).
pub const TYPO_WEIGHT_REGULAR: u16 = 400;
/// Medium headers/chrome weight (500).
pub const TYPO_WEIGHT_MEDIUM: u16 = 500;
/// Semibold badge/counts weight, used sparingly (600).
pub const TYPO_WEIGHT_SEMIBOLD: u16 = 600;
/// Compact chrome line-height multiplier (~1.2).
pub const TYPO_LINE_HEIGHT_CHROME: f32 = 1.2;
/// Reading body line-height multiplier (~1.45).
pub const TYPO_LINE_HEIGHT_BODY: f32 = 1.45;
/// Dense mono line-height multiplier (~1.35).
pub const TYPO_LINE_HEIGHT_MONO: f32 = 1.35;
/// Slight header tracking as em ratio (0.02em; egui has no tracking).
pub const TYPO_TRACKING_HEADING_EM: f32 = 0.02;

// ----------------------------------------------------------------------------
// Font zoom model (headless): step table over TypoRole sizes.
// ----------------------------------------------------------------------------

/// Lowest zoom step: two ticks below default.
pub const ZOOM_MIN_LEVEL: i8 = -2;
/// Highest zoom step: four ticks above default.
pub const ZOOM_MAX_LEVEL: i8 = 4;
/// Default zoom step: base [`TypoRole::size`] values unchanged.
pub const ZOOM_DEFAULT_LEVEL: i8 = 0;
/// Size delta per step in px: one point per level keeps role gaps intact.
pub const ZOOM_STEP_PX: f32 = 1.0;
/// Clamp floor for any zoomed size so small text never vanishes.
pub const ZOOM_MIN_SIZE: f32 = 8.0;
/// Clamp ceiling for any zoomed size so headings never blow out layout.
pub const ZOOM_MAX_SIZE: f32 = 24.0;
/// Every accepted step from MIN..=MAX, in order, for docs plus tests.
pub const ZOOM_LEVELS: [i8; 7] = [-2, -1, 0, 1, 2, 3, 4];

/// Clamps a raw zoom step into MIN..=MAX; out-of-range input sticks.
pub const fn clamp_zoom_level(level: i8) -> i8 {
    if level < ZOOM_MIN_LEVEL {
        ZOOM_MIN_LEVEL
    } else if level > ZOOM_MAX_LEVEL {
        ZOOM_MAX_LEVEL
    } else {
        level
    }
}

/// Pixel offset for a step: clamped level times STEP (level 0 gives 0.0).
pub fn zoom_offset_px(level: i8) -> f32 {
    f32::from(clamp_zoom_level(level)) * ZOOM_STEP_PX
}

/// Zoomed point size for `role` at `level`, clamped to MIN..MAX size.
/// The level clamps first, so very low/high input behaves like the
/// nearest edge. Base comes from [`TypoRole::size`]; zoom never moves
/// geometry or color tokens.
pub fn zoomed_size(role: TypoRole, level: i8) -> f32 {
    let stepped = role.size() + zoom_offset_px(level);
    stepped.clamp(ZOOM_MIN_SIZE, ZOOM_MAX_SIZE)
}

impl TypoRole {
    /// Zoomed point size for this role at `level` (see [`zoomed_size`]).
    pub fn zoomed_point_size(self, level: i8) -> f32 {
        zoomed_size(self, level)
    }

    /// Zoom-aware text builder: same family, weight and style as
    /// [`TypoRole::rich`] but the point size comes from [`zoomed_size`].
    /// Line-height stays the shared measured mono row height.
    pub fn rich_zoomed(self, text: impl Into<String>, level: i8) -> egui::RichText {
        let styled = egui::RichText::new(text)
            .text_style(self.text_style())
            .size(self.zoomed_point_size(level))
            .extra_letter_spacing(TYPO_LETTER_SPACING);
        match self {
            Self::Heading | Self::HomeTitle => styled.strong(),
            Self::Small | Self::HomeDesc => styled.weak(),
            Self::Body | Self::Mono | Self::HomeButton => styled,
        }
    }
}

// ----------------------------------------------------------------------------
// Geometry scale (headless, px): canonical spacing 4/8/12/16/24.
// ----------------------------------------------------------------------------

/// XS step: tight gaps, marker padding.
pub const SPACE_XS: f32 = 4.0;
/// SM step: section gaps.
pub const SPACE_SM: f32 = 8.0;
/// MD step: outer margin, panel gutter, gutter wide bar, tree indent.
pub const SPACE_MD: f32 = 12.0;
/// LG step: large panel gutter.
pub const SPACE_LG: f32 = 16.0;
/// XL step: card / large section gap.
pub const SPACE_XL: f32 = 24.0;
/// Tight gap (one XS step).
pub const GEO_TIGHT_GAP: f32 = SPACE_XS;
/// Section gap (one SM step).
pub const GEO_SECTION_GAP: f32 = SPACE_SM;
/// Outer margin / panel gutter (one MD step).
pub const GEO_OUTER_MARGIN: f32 = SPACE_MD;
/// Large panel gutter (one LG step).
pub const GEO_PANEL_GUTTER: f32 = SPACE_LG;
/// Card / large section gap (one XL step).
pub const GEO_CARD_GAP: f32 = SPACE_XL;
/// Gutter bar wide: one MD step, reproduces the 12.0 chrome.
pub const GUTTER_BAR_WIDE: f32 = SPACE_MD;
/// Gutter bar narrow: half MD step, reproduces the 6.0 chrome.
pub const GUTTER_BAR_NARROW: f32 = 6.0;
/// Minimum gutter segment height so 1px hunks stay visible.
pub const GUTTER_MIN_SEGMENT_H: f32 = 2.0;
/// Merge gap: same-kind gutter segments within this fuse into one.
pub const GUTTER_MERGE_GAP: f32 = 2.0;
/// Canonical tree indent step: one MD step on the scale.
pub const GEO_TREE_INDENT_STEP: f32 = SPACE_MD;

/// One typographic role: size + family + weight, line-height shared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TypoRole {
    /// 18px strong heading (`ui.heading` scale).
    Heading,
    /// 13px default body (buttons, plain labels).
    #[default]
    Body,
    /// 11px weak small (hints, counts, subtitles).
    Small,
    /// 12px regular monospace (diff panes, summaries).
    Mono,
    /// 14px strong Home card title.
    HomeTitle,
    /// 11px weak small Home card description.
    HomeDesc,
    /// 13px default Home card button.
    HomeButton,
}

impl TypoRole {
    /// Point size for this role.
    pub const fn size(self) -> f32 {
        match self {
            Self::Heading => TYPO_HEADING_SIZE,
            Self::Body => TYPO_BODY_SIZE,
            Self::Small => TYPO_SMALL_SIZE,
            Self::Mono => TYPO_MONO_SIZE,
            Self::HomeTitle => TYPO_HOME_TITLE_SIZE,
            Self::HomeDesc => TYPO_SMALL_SIZE,
            Self::HomeButton => TYPO_BODY_SIZE,
        }
    }

    /// Whether this role paints strong (bright/bold emphasis).
    pub const fn is_strong(self) -> bool {
        matches!(self, Self::Heading | Self::HomeTitle)
    }

    /// Whether this role paints weak (dimmed secondary text).
    pub const fn is_weak(self) -> bool {
        matches!(self, Self::Small | Self::HomeDesc)
    }

    /// Whether this role uses the small scale.
    pub const fn is_small(self) -> bool {
        matches!(self, Self::Small | Self::HomeDesc)
    }

    /// Whether this role uses the monospace family.
    pub const fn is_mono(self) -> bool {
        matches!(self, Self::Mono)
    }

    /// Line-height source: always the measured monospace row height.
    pub const fn line_height_source(self) -> &'static str {
        TYPO_LINE_HEIGHT_SOURCE
    }

    /// Paint weight for this role: strong / default / weak / regular-mono.
    /// Hierarchy carrier next to size since egui has no tracking.
    pub const fn weight(self) -> &'static str {
        match self {
            Self::Heading | Self::HomeTitle => "strong",
            Self::Small | Self::HomeDesc => "weak",
            Self::Mono => "regular-mono",
            Self::Body | Self::HomeButton => "default",
        }
    }

    /// Font family for this role: monospace diffs, proportional rest.
    pub const fn family(self) -> &'static str {
        match self {
            Self::Mono => "monospace",
            _ => "proportional",
        }
    }

    /// Font stack for this role: UI sans for body/small/chrome,
    /// monospace stack for hex/text-cells/table-cells/gutter
    /// (which all paint as [`TypoRole::Mono`]). Headless intent;
    /// egui resolves the family via [`TypoRole::text_style`].
    pub const fn font_stack(self) -> &'static str {
        match self {
            Self::Mono => TYPO_MONO_STACK,
            _ => TYPO_UI_SANS_STACK,
        }
    }

    /// Numeric weight for this role: regular body, medium
    /// headers/chrome, semibold badge+counts sparingly (only
    /// [`TypoRole::Small`]). [`TypoRole::weight`] keeps the egui
    /// strong/weak spelling so paint stays unchanged.
    pub const fn weight_value(self) -> u16 {
        match self {
            Self::Heading | Self::HomeTitle | Self::HomeButton => TYPO_WEIGHT_MEDIUM,
            Self::Small => TYPO_WEIGHT_SEMIBOLD,
            Self::Body | Self::Mono | Self::HomeDesc => TYPO_WEIGHT_REGULAR,
        }
    }

    /// Line-height multiplier for this role: chrome ~1.2, body ~1.45,
    /// mono ~1.35. Paint still measures the shared mono row height
    /// ([`TYPO_LINE_HEIGHT_SOURCE`]); this is headless intent.
    pub const fn line_height(self) -> f32 {
        match self {
            Self::Heading | Self::HomeTitle | Self::HomeButton | Self::Small => {
                TYPO_LINE_HEIGHT_CHROME
            }
            Self::Mono => TYPO_LINE_HEIGHT_MONO,
            Self::Body | Self::HomeDesc => TYPO_LINE_HEIGHT_BODY,
        }
    }

    /// Line-height in px for this role: size * multiplier.
    pub const fn line_height_px(self) -> f32 {
        self.size() * self.line_height()
    }

    /// Letter-tracking as em ratio: slight for headers, zero elsewhere.
    /// Headless intent (egui `extra_letter_spacing` stays 0.0 in paint).
    pub const fn letter_spacing_em(self) -> f32 {
        match self {
            Self::Heading | Self::HomeTitle => TYPO_TRACKING_HEADING_EM,
            _ => 0.0,
        }
    }

    /// Whether this role uses tabular numerals: gutter/hex/counts.
    /// Mono covers gutter/hex/text-cells/table-cells, Small covers counts.
    /// Headless intent (egui has no tabular toggle).
    pub const fn is_tabular(self) -> bool {
        matches!(self, Self::Mono | Self::Small)
    }

    /// egui text style carrying this role's family. Explicit
    /// [`TypoRole::size`] overrides the style default at paint time via
    /// [`TypoRole::rich`]; the style itself picks the family (plus the
    /// semantic slot for buttons). HomeTitle stays on Body: it is
    /// proportional 14px strong, not a full Heading jump.
    pub fn text_style(self) -> egui::TextStyle {
        match self {
            Self::Heading => egui::TextStyle::Heading,
            Self::Mono => egui::TextStyle::Monospace,
            Self::Small | Self::HomeDesc => egui::TextStyle::Small,
            Self::HomeButton => egui::TextStyle::Button,
            Self::Body | Self::HomeTitle => egui::TextStyle::Body,
        }
    }

    /// Builds the egui text for this role: family via
    /// [`TypoRole::text_style`], point size via [`TypoRole::size`],
    /// weight via strong/weak, letter-spacing via
    /// [`TYPO_LETTER_SPACING`]. Line-height stays unset so paint keeps
    /// measuring the shared monospace row height
    /// ([`TYPO_LINE_HEIGHT_SOURCE`]). Headless: needs no `Ui` or
    /// `Context`. Colors untouched (paint owns them).
    pub fn rich(self, text: impl Into<String>) -> egui::RichText {
        let styled = egui::RichText::new(text)
            .text_style(self.text_style())
            .size(self.size())
            .extra_letter_spacing(TYPO_LETTER_SPACING);
        match self {
            Self::Heading | Self::HomeTitle => styled.strong(),
            Self::Small | Self::HomeDesc => styled.weak(),
            Self::Body | Self::Mono | Self::HomeButton => styled,
        }
    }
}

/// Calculates the WCAG 2.1 relative luminance of an sRGB color.
pub fn relative_luminance(rgb: [u8; 3]) -> f64 {
    let [r_raw, g_raw, b_raw] = rgb;
    let transform = |c: u8| -> f64 {
        let v = f64::from(c) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = transform(r_raw);
    let g = transform(g_raw);
    let b = transform(b_raw);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Calculates the WCAG 2.1 contrast ratio between two sRGB colors.
pub fn theme_contrast_ratio(fg: [u8; 3], bg: [u8; 3]) -> f64 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Checks whether the contrast ratio between foreground and background colors
/// meets the standard WCAG AA threshold (4.5:1).
pub fn theme_contrast_ok(fg: [u8; 3], bg: [u8; 3]) -> bool {
    theme_contrast_ratio(fg, bg) >= 4.5
}

/// Checks whether standard dark/light base themes satisfy WCAG AA contrast.
pub fn theme_mode_contrast_ok(dark: bool) -> bool {
    if dark {
        // Dark theme: foreground text [230, 230, 230] on background [27, 27, 27]
        theme_contrast_ok(DARK_FG_RGB, DARK_BG_RGB)
    } else {
        // Light theme: foreground text [20, 20, 20] on background [248, 248, 248]
        theme_contrast_ok(LIGHT_FG_RGB, LIGHT_BG_RGB)
    }
}

// ----------------------------------------------------------------------------
// Color tokens (typed design tokens, Dark/Light)
// ----------------------------------------------------------------------------

/// Dark base text (headless). Paint reads it through egui visuals.
pub const DARK_FG_RGB: [u8; 3] = [230, 230, 230];
/// Dark base background (headless).
pub const DARK_BG_RGB: [u8; 3] = [27, 27, 27];
/// Light base text (headless).
pub const LIGHT_FG_RGB: [u8; 3] = [20, 20, 20];
/// Light base background (headless).
pub const LIGHT_BG_RGB: [u8; 3] = [248, 248, 248];

/// Fixed diff deleted foreground (same both modes).
pub const DELETED_FG_RGB: [u8; 3] = [255, 170, 170];
/// Fixed diff deleted background (same both modes).
pub const DELETED_BG_RGB: [u8; 3] = [120, 32, 32];
/// Fixed diff inserted foreground (same both modes).
pub const INSERTED_FG_RGB: [u8; 3] = [170, 240, 180];
/// Fixed diff inserted background (same both modes).
pub const INSERTED_BG_RGB: [u8; 3] = [24, 90, 46];
/// Fixed diff changed foreground (same both modes).
pub const CHANGED_FG_RGB: [u8; 3] = [255, 235, 160];
/// Fixed diff changed background (same both modes).
pub const CHANGED_BG_RGB: [u8; 3] = [122, 102, 20];
/// Fixed accent for changed spans (same both modes).
pub const ACCENT_FG_RGB: [u8; 3] = [255, 255, 210];
/// Fixed error-panel fill (same both modes; text is [`DELETED_FG_RGB`]).
pub const ERROR_BG_RGB: [u8; 3] = [60, 20, 20];

/// Dark dim gray for gutters and `=` glyphs.
pub const DARK_DIM_RGB: [u8; 3] = [150, 150, 150];
/// Light dim gray for gutters and `=` glyphs (darkened so it passes AA on light).
pub const LIGHT_DIM_RGB: [u8; 3] = [90, 90, 90];
/// Dark red marker.
pub const DARK_RED_RGB: [u8; 3] = [255, 100, 100];
/// Light red marker (darkened so it passes AA on light).
pub const LIGHT_RED_RGB: [u8; 3] = [180, 20, 20];
/// Dark green marker.
pub const DARK_GREEN_RGB: [u8; 3] = [120, 220, 130];
/// Light green marker (darkened so it passes AA on light).
pub const LIGHT_GREEN_RGB: [u8; 3] = [20, 110, 50];
/// Dark newer-side blue.
pub const DARK_BLUE_RGB: [u8; 3] = [130, 170, 255];
/// Light newer-side blue (darkened so it passes AA on light).
pub const LIGHT_BLUE_RGB: [u8; 3] = [30, 80, 180];
/// Dark gutter track fill (decorative, not text).
pub const DARK_TRACK_RGB: [u8; 3] = [32, 32, 32];
/// Light gutter track fill (decorative, not text).
pub const LIGHT_TRACK_RGB: [u8; 3] = [225, 225, 225];

/// Fixed diff deleted foreground paint token.
pub const DELETED_FG: Color32 = Color32::from_rgb(255, 170, 170);
/// Fixed diff deleted background paint token.
pub const DELETED_BG: Color32 = Color32::from_rgb(120, 32, 32);
/// Fixed diff inserted foreground paint token.
pub const INSERTED_FG: Color32 = Color32::from_rgb(170, 240, 180);
/// Fixed diff inserted background paint token.
pub const INSERTED_BG: Color32 = Color32::from_rgb(24, 90, 46);
/// Fixed diff changed foreground paint token.
pub const CHANGED_FG: Color32 = Color32::from_rgb(255, 235, 160);
/// Fixed diff changed background paint token.
pub const CHANGED_BG: Color32 = Color32::from_rgb(122, 102, 20);
/// Fixed accent paint token for changed spans.
pub const ACCENT_FG: Color32 = Color32::from_rgb(255, 255, 210);
/// Fixed error-panel fill paint token.
pub const ERROR_BG: Color32 = Color32::from_rgb(60, 20, 20);
/// Dark dim paint token.
pub const DARK_DIM: Color32 = Color32::from_rgb(150, 150, 150);
/// Light dim paint token.
pub const LIGHT_DIM: Color32 = Color32::from_rgb(90, 90, 90);
/// Dark red marker paint token.
pub const DARK_RED: Color32 = Color32::from_rgb(255, 100, 100);
/// Light red marker paint token.
pub const LIGHT_RED: Color32 = Color32::from_rgb(180, 20, 20);
/// Dark green marker paint token.
pub const DARK_GREEN: Color32 = Color32::from_rgb(120, 220, 130);
/// Light green marker paint token.
pub const LIGHT_GREEN: Color32 = Color32::from_rgb(20, 110, 50);
/// Dark newer-side blue paint token.
pub const DARK_BLUE: Color32 = Color32::from_rgb(130, 170, 255);
/// Light newer-side blue paint token.
pub const LIGHT_BLUE: Color32 = Color32::from_rgb(30, 80, 180);
/// Dark gutter track paint token.
pub const DARK_TRACK: Color32 = Color32::from_rgb(32, 32, 32);
/// Light gutter track paint token.
pub const LIGHT_TRACK: Color32 = Color32::from_rgb(225, 225, 225);

/// Per-mode chrome: base pair plus the markers that paint on it.
/// The diff accents stay fixed outside this struct so one deleted row
/// reads the same in both modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Base text on [`Palette::base_bg`].
    pub base_fg: [u8; 3],
    /// Base background.
    pub base_bg: [u8; 3],
    /// Dim gray (gutters, `=` glyphs) on [`Palette::base_bg`].
    pub dim: [u8; 3],
    /// Red marker on [`Palette::base_bg`].
    pub red: [u8; 3],
    /// Green marker on [`Palette::base_bg`].
    pub green: [u8; 3],
    /// Newer-side blue on [`Palette::base_bg`].
    pub blue: [u8; 3],
    /// Gutter track fill (decorative).
    pub track: [u8; 3],
}

/// Dark chrome (same hues the paint has always used).
pub const DARK_PALETTE: Palette = Palette {
    base_fg: DARK_FG_RGB,
    base_bg: DARK_BG_RGB,
    dim: DARK_DIM_RGB,
    red: DARK_RED_RGB,
    green: DARK_GREEN_RGB,
    blue: DARK_BLUE_RGB,
    track: DARK_TRACK_RGB,
};

/// Light chrome (same hue families, darkened markers so AA holds on light).
pub const LIGHT_PALETTE: Palette = Palette {
    base_fg: LIGHT_FG_RGB,
    base_bg: LIGHT_BG_RGB,
    dim: LIGHT_DIM_RGB,
    red: LIGHT_RED_RGB,
    green: LIGHT_GREEN_RGB,
    blue: LIGHT_BLUE_RGB,
    track: LIGHT_TRACK_RGB,
};

/// Paint-side palette selector (mirrors `ui.visuals().dark_mode`).
pub const fn palette(dark: bool) -> Palette {
    if dark { DARK_PALETTE } else { LIGHT_PALETTE }
}

/// Dim paint token for the active mode.
pub fn dim_for(dark: bool) -> Color32 {
    if dark { DARK_DIM } else { LIGHT_DIM }
}

/// Red marker paint token for the active mode.
pub fn red_for(dark: bool) -> Color32 {
    if dark { DARK_RED } else { LIGHT_RED }
}

/// Green marker paint token for the active mode.
pub fn green_for(dark: bool) -> Color32 {
    if dark { DARK_GREEN } else { LIGHT_GREEN }
}

/// Newer-side blue paint token for the active mode.
pub fn blue_for(dark: bool) -> Color32 {
    if dark { DARK_BLUE } else { LIGHT_BLUE }
}

/// Gutter track paint token for the active mode.
pub fn track_for(dark: bool) -> Color32 {
    if dark { DARK_TRACK } else { LIGHT_TRACK }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_has_maximum_contrast() {
        let ratio = theme_contrast_ratio([0, 0, 0], [255, 255, 255]);
        assert!((ratio - 21.0).abs() < 0.1);
        assert!(theme_contrast_ok([0, 0, 0], [255, 255, 255]));
    }

    #[test]
    fn low_contrast_fails_threshold() {
        assert!(!theme_contrast_ok([120, 120, 120], [130, 130, 130]));
    }

    #[test]
    fn theme_modes_satisfy_contrast() {
        assert!(theme_mode_contrast_ok(true));
        assert!(theme_mode_contrast_ok(false));
    }

    #[test]
    fn typo_scale_matches_paint_contract() {
        assert_eq!(TYPO_HEADING_SIZE, 18.0);
        assert_eq!(TYPO_BODY_SIZE, 13.0);
        assert_eq!(TYPO_SMALL_SIZE, 11.0);
        assert_eq!(TYPO_MONO_SIZE, 12.0);
        assert_eq!(TYPO_MONO_SIZE_MAX, 13.0);
        assert_eq!(TYPO_HOME_TITLE_SIZE, 14.0);
        assert_eq!(TYPO_WRAP_PANE_CHARS, 80);
        assert_eq!(TYPO_FULL_PANE_CHARS, 200);
        assert_eq!(TYPO_LETTER_SPACING, 0.0);
        const {
            assert!(TYPO_MONO_SIZE <= TYPO_MONO_SIZE_MAX);
            assert!(TYPO_SMALL_SIZE < TYPO_BODY_SIZE);
            assert!(TYPO_BODY_SIZE < TYPO_HOME_TITLE_SIZE);
            assert!(TYPO_HOME_TITLE_SIZE < TYPO_HEADING_SIZE);
        }
    }

    #[test]
    fn typo_roles_carry_hierarchy_without_tracking() {
        assert!(TypoRole::Heading.is_strong());
        assert!(TypoRole::HomeTitle.is_strong());
        assert!(!TypoRole::Body.is_strong());
        assert!(TypoRole::Small.is_weak());
        assert!(TypoRole::HomeDesc.is_weak());
        assert!(TypoRole::HomeDesc.is_small());
        assert!(TypoRole::Mono.is_mono());
        assert!(!TypoRole::Body.is_mono());
        // Body-scale buttons: neither strong nor weak/small.
        assert!(!TypoRole::HomeButton.is_strong());
        assert!(!TypoRole::HomeButton.is_weak());
        assert!(!TypoRole::HomeButton.is_small());
        assert_eq!(TypoRole::HomeButton.size(), TYPO_BODY_SIZE);
        assert_eq!(TypoRole::HomeDesc.size(), TYPO_SMALL_SIZE);
        // Line-height is the measured mono row height everywhere.
        assert_eq!(TypoRole::Mono.line_height_source(), TYPO_LINE_HEIGHT_SOURCE);
        assert_eq!(
            TypoRole::Heading.line_height_source(),
            "ui.text_style_height(Monospace)"
        );
        // Weight + family carry hierarchy (no letter-tracking in egui).
        assert_eq!(TypoRole::Heading.weight(), "strong");
        assert_eq!(TypoRole::HomeTitle.weight(), "strong");
        assert_eq!(TypoRole::Small.weight(), "weak");
        assert_eq!(TypoRole::HomeDesc.weight(), "weak");
        assert_eq!(TypoRole::Mono.weight(), "regular-mono");
        assert_eq!(TypoRole::Body.weight(), "default");
        assert_eq!(TypoRole::Mono.family(), "monospace");
        assert_eq!(TypoRole::Body.family(), "proportional");
        assert_eq!(TypoRole::Heading.family(), "proportional");
    }

    #[test]
    fn typo_roles_map_to_egui_text_styles() {
        assert_eq!(TypoRole::Heading.text_style(), egui::TextStyle::Heading);
        assert_eq!(TypoRole::Body.text_style(), egui::TextStyle::Body);
        assert_eq!(TypoRole::Small.text_style(), egui::TextStyle::Small);
        assert_eq!(TypoRole::Mono.text_style(), egui::TextStyle::Monospace);
        assert_eq!(TypoRole::HomeDesc.text_style(), egui::TextStyle::Small);
        assert_eq!(TypoRole::HomeButton.text_style(), egui::TextStyle::Button);
        // HomeTitle stays proportional-body + explicit 14px strong.
        assert_eq!(TypoRole::HomeTitle.text_style(), egui::TextStyle::Body);
    }

    #[test]
    fn typo_rich_carries_size_and_weight_per_role() {
        let debug = |role: TypoRole| format!("{role:?}::{:?}", role.rich("x"));
        assert!(debug(TypoRole::HomeTitle).contains("14.0"));
        assert!(debug(TypoRole::Mono).contains("12.0"));
        assert!(debug(TypoRole::Body).contains("13.0"));
        assert!(debug(TypoRole::Small).contains("11.0"));
        // Hierarchy: distinct roles differ, same-scale roles match.
        assert_ne!(TypoRole::Heading.rich("a"), TypoRole::Body.rich("a"));
        assert_eq!(TypoRole::Small.rich("a"), TypoRole::HomeDesc.rich("a"));
        // Weight carriers survive the applicator.
        assert!(debug(TypoRole::Heading).contains("strong: true"));
        assert!(debug(TypoRole::HomeDesc).contains("weak: true"));
        assert!(debug(TypoRole::Mono).contains("strong: false"));
    }

    #[test]
    fn geometry_scale_unifies_spacing() {
        assert_eq!(SPACE_XS, 4.0);
        assert_eq!(SPACE_SM, 8.0);
        assert_eq!(SPACE_MD, 12.0);
        assert_eq!(SPACE_LG, 16.0);
        assert_eq!(SPACE_XL, 24.0);
        assert_eq!(GEO_TIGHT_GAP, SPACE_XS);
        assert_eq!(GEO_SECTION_GAP, SPACE_SM);
        assert_eq!(GEO_OUTER_MARGIN, SPACE_MD);
        assert_eq!(GEO_PANEL_GUTTER, SPACE_LG);
        assert_eq!(GEO_CARD_GAP, SPACE_XL);
        assert_eq!(GUTTER_BAR_WIDE, 12.0);
        assert_eq!(GUTTER_BAR_NARROW, 6.0);
        assert_eq!(GEO_TREE_INDENT_STEP, SPACE_MD);
        const {
            assert!(SPACE_XS < SPACE_SM);
            assert!(SPACE_SM < SPACE_MD);
            assert!(SPACE_MD < SPACE_LG);
            assert!(SPACE_LG < SPACE_XL);
        }
    }

    #[test]
    fn fixed_diff_pairs_satisfy_aa() {
        assert!(theme_contrast_ok(DELETED_FG_RGB, DELETED_BG_RGB));
        assert!(theme_contrast_ok(INSERTED_FG_RGB, INSERTED_BG_RGB));
        assert!(theme_contrast_ok(CHANGED_FG_RGB, CHANGED_BG_RGB));
        assert!(theme_contrast_ok(ACCENT_FG_RGB, CHANGED_BG_RGB));
        assert!(theme_contrast_ok(ACCENT_FG_RGB, DELETED_BG_RGB));
        assert!(theme_contrast_ok(ACCENT_FG_RGB, INSERTED_BG_RGB));
        assert!(theme_contrast_ok(DELETED_FG_RGB, ERROR_BG_RGB));
    }

    #[test]
    fn palettes_satisfy_aa_per_mode() {
        for pal in [DARK_PALETTE, LIGHT_PALETTE] {
            assert!(theme_contrast_ok(pal.base_fg, pal.base_bg));
            assert!(theme_contrast_ok(pal.dim, pal.base_bg));
            assert!(theme_contrast_ok(pal.red, pal.base_bg));
            assert!(theme_contrast_ok(pal.green, pal.base_bg));
            assert!(theme_contrast_ok(pal.blue, pal.base_bg));
        }
        assert!(theme_mode_contrast_ok(true));
        assert!(theme_mode_contrast_ok(false));
    }

    #[test]
    fn palette_selector_matches_mode() {
        assert_eq!(palette(true), DARK_PALETTE);
        assert_eq!(palette(false), LIGHT_PALETTE);
        assert_eq!(palette(true).base_bg, DARK_BG_RGB);
        assert_eq!(palette(false).base_bg, LIGHT_BG_RGB);
    }

    #[test]
    fn tokens_keep_frozen_hues() {
        assert_eq!(DELETED_FG_RGB, [255, 170, 170]);
        assert_eq!(DELETED_BG_RGB, [120, 32, 32]);
        assert_eq!(INSERTED_FG_RGB, [170, 240, 180]);
        assert_eq!(INSERTED_BG_RGB, [24, 90, 46]);
        assert_eq!(CHANGED_FG_RGB, [255, 235, 160]);
        assert_eq!(CHANGED_BG_RGB, [122, 102, 20]);
        assert_eq!(ACCENT_FG_RGB, [255, 255, 210]);
        assert_eq!(ERROR_BG_RGB, [60, 20, 20]);
        assert_eq!(DARK_DIM_RGB, [150, 150, 150]);
        assert_eq!(DARK_RED_RGB, [255, 100, 100]);
        assert_eq!(DARK_GREEN_RGB, [120, 220, 130]);
        assert_eq!(DARK_BLUE_RGB, [130, 170, 255]);
        assert_eq!(DARK_TRACK_RGB, [32, 32, 32]);
    }

    #[test]
    fn zoom_steps_cover_min_to_max_in_order() {
        assert_eq!(ZOOM_MIN_LEVEL, -2);
        assert_eq!(ZOOM_MAX_LEVEL, 4);
        assert_eq!(ZOOM_DEFAULT_LEVEL, 0);
        assert_eq!(ZOOM_LEVELS, [-2, -1, 0, 1, 2, 3, 4]);
        assert_eq!(ZOOM_STEP_PX, 1.0);
        assert_eq!(ZOOM_MIN_SIZE, 8.0);
        assert_eq!(ZOOM_MAX_SIZE, 24.0);
        const {
            assert!(ZOOM_MIN_SIZE < TYPO_SMALL_SIZE);
            assert!(TYPO_HEADING_SIZE < ZOOM_MAX_SIZE);
        }
    }

    #[test]
    fn zoom_level_clamps_to_table_edges() {
        assert_eq!(clamp_zoom_level(0), 0);
        assert_eq!(clamp_zoom_level(-2), -2);
        assert_eq!(clamp_zoom_level(4), 4);
        assert_eq!(clamp_zoom_level(-100), ZOOM_MIN_LEVEL);
        assert_eq!(clamp_zoom_level(100), ZOOM_MAX_LEVEL);
        assert_eq!(zoom_offset_px(0), 0.0);
        assert_eq!(zoom_offset_px(1), 1.0);
        assert_eq!(zoom_offset_px(-2), -2.0);
        assert_eq!(zoom_offset_px(4), 4.0);
        assert_eq!(zoom_offset_px(100), zoom_offset_px(ZOOM_MAX_LEVEL));
        assert_eq!(zoom_offset_px(-100), zoom_offset_px(ZOOM_MIN_LEVEL));
    }

    #[test]
    fn zoomed_size_zero_level_is_base() {
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            assert_eq!(zoomed_size(role, 0), role.size());
            assert_eq!(role.zoomed_point_size(0), role.size());
            assert_eq!(role.zoomed_point_size(ZOOM_DEFAULT_LEVEL), role.size());
        }
    }

    #[test]
    fn zoomed_size_steps_one_px_per_level_then_clamps() {
        assert_eq!(zoomed_size(TypoRole::Body, 1), TYPO_BODY_SIZE + 1.0);
        assert_eq!(zoomed_size(TypoRole::Body, -1), TYPO_BODY_SIZE - 1.0);
        assert_eq!(zoomed_size(TypoRole::Mono, 2), TYPO_MONO_SIZE + 2.0);
        assert_eq!(zoomed_size(TypoRole::Small, -2), TYPO_SMALL_SIZE - 2.0);
        // Level clamps before sizing: huge input behaves like the edge.
        assert_eq!(
            zoomed_size(TypoRole::Body, 100),
            zoomed_size(TypoRole::Body, ZOOM_MAX_LEVEL)
        );
        assert_eq!(
            zoomed_size(TypoRole::Body, -100),
            zoomed_size(TypoRole::Body, ZOOM_MIN_LEVEL)
        );
        // Size clamps to MIN..MAX even for extreme roles.
        assert!(zoomed_size(TypoRole::Small, -100) >= ZOOM_MIN_SIZE);
        assert!(zoomed_size(TypoRole::Heading, 100) <= ZOOM_MAX_SIZE);
        // Full step table stays inside MIN..MAX for every role.
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            for level in ZOOM_LEVELS {
                let sized = zoomed_size(role, level);
                assert!(sized >= ZOOM_MIN_SIZE);
                assert!(sized <= ZOOM_MAX_SIZE);
            }
        }
    }

    #[test]
    fn zoomed_rich_carries_zoomed_size_per_role() {
        let debug =
            |role: TypoRole, level: i8| format!("{role:?}::{:?}", role.rich_zoomed("x", level));
        assert!(debug(TypoRole::Body, 0).contains("13.0"));
        assert!(debug(TypoRole::Body, 2).contains("15.0"));
        assert!(debug(TypoRole::Mono, 1).contains("13.0"));
        assert!(debug(TypoRole::Small, -1).contains("10.0"));
        // Weight carriers survive the zoomed applicator.
        assert!(debug(TypoRole::Heading, 1).contains("strong: true"));
        assert!(debug(TypoRole::HomeDesc, -1).contains("weak: true"));
        assert!(debug(TypoRole::Mono, 0).contains("strong: false"));
    }

    #[test]
    fn typo_exquisite_tokens_carry_hierarchy() {
        // Stacks: UI sans everywhere except mono cells.
        assert_eq!(TypoRole::Mono.font_stack(), TYPO_MONO_STACK);
        assert_eq!(TypoRole::Body.font_stack(), TYPO_UI_SANS_STACK);
        assert_eq!(TypoRole::Small.font_stack(), TYPO_UI_SANS_STACK);
        assert_eq!(TypoRole::Heading.font_stack(), TYPO_UI_SANS_STACK);
        assert_eq!(TypoRole::HomeTitle.font_stack(), TYPO_UI_SANS_STACK);
        assert_eq!(TypoRole::HomeButton.font_stack(), TYPO_UI_SANS_STACK);
        assert_eq!(TypoRole::HomeDesc.font_stack(), TYPO_UI_SANS_STACK);
        // Weights: regular body, medium headers/chrome, semibold counts only.
        assert_eq!(TYPO_WEIGHT_REGULAR, 400);
        assert_eq!(TYPO_WEIGHT_MEDIUM, 500);
        assert_eq!(TYPO_WEIGHT_SEMIBOLD, 600);
        assert_eq!(TypoRole::Body.weight_value(), TYPO_WEIGHT_REGULAR);
        assert_eq!(TypoRole::Mono.weight_value(), TYPO_WEIGHT_REGULAR);
        assert_eq!(TypoRole::HomeDesc.weight_value(), TYPO_WEIGHT_REGULAR);
        assert_eq!(TypoRole::Heading.weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(TypoRole::HomeTitle.weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(TypoRole::HomeButton.weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(TypoRole::Small.weight_value(), TYPO_WEIGHT_SEMIBOLD);
        // Line-height multipliers per role family.
        assert_eq!(TypoRole::Heading.line_height(), TYPO_LINE_HEIGHT_CHROME);
        assert_eq!(TypoRole::HomeTitle.line_height(), TYPO_LINE_HEIGHT_CHROME);
        assert_eq!(TypoRole::HomeButton.line_height(), TYPO_LINE_HEIGHT_CHROME);
        assert_eq!(TypoRole::Small.line_height(), TYPO_LINE_HEIGHT_CHROME);
        assert_eq!(TypoRole::Mono.line_height(), TYPO_LINE_HEIGHT_MONO);
        assert_eq!(TypoRole::Body.line_height(), TYPO_LINE_HEIGHT_BODY);
        assert_eq!(TypoRole::HomeDesc.line_height(), TYPO_LINE_HEIGHT_BODY);
        assert!((TYPO_LINE_HEIGHT_CHROME - 1.2).abs() < 0.001);
        assert!((TYPO_LINE_HEIGHT_BODY - 1.45).abs() < 0.001);
        assert!((TYPO_LINE_HEIGHT_MONO - 1.35).abs() < 0.001);
        // px height follows size * multiplier.
        assert!((TypoRole::Body.line_height_px() - 13.0 * 1.45).abs() < 0.01);
        assert!((TypoRole::Mono.line_height_px() - 12.0 * 1.35).abs() < 0.01);
        assert!((TypoRole::Heading.line_height_px() - 18.0 * 1.2).abs() < 0.01);
        // Tracking: headers slight, rest zero; paint stays 0.0 no-op.
        assert_eq!(TYPO_LETTER_SPACING, 0.0);
        assert!((TYPO_TRACKING_HEADING_EM - 0.02).abs() < 0.0001);
        assert_eq!(TypoRole::Heading.letter_spacing_em(), 0.02);
        assert_eq!(TypoRole::HomeTitle.letter_spacing_em(), 0.02);
        assert_eq!(TypoRole::Body.letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::Mono.letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::Small.letter_spacing_em(), 0.0);
        // Tabular: gutter/hex/counts only (Mono + Small).
        assert!(TypoRole::Mono.is_tabular());
        assert!(TypoRole::Small.is_tabular());
        assert!(!TypoRole::Body.is_tabular());
        assert!(!TypoRole::Heading.is_tabular());
        assert!(!TypoRole::HomeTitle.is_tabular());
    }

    #[test]
    fn typo_exquisite_zoom_clamps_every_role() {
        let roles = [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ];
        for role in roles {
            // Level 0 is the base passthrough.
            assert_eq!(zoomed_size(role, 0), role.size());
            // Full step table stays inside MIN..MAX.
            for level in ZOOM_LEVELS {
                let sized = zoomed_size(role, level);
                assert!(sized >= ZOOM_MIN_SIZE);
                assert!(sized <= ZOOM_MAX_SIZE);
            }
            // Extreme input clamps to the edge, then to MIN..MAX.
            assert_eq!(zoomed_size(role, 100), zoomed_size(role, ZOOM_MAX_LEVEL));
            assert_eq!(zoomed_size(role, -100), zoomed_size(role, ZOOM_MIN_LEVEL));
            assert!(zoomed_size(role, 100) <= ZOOM_MAX_SIZE);
            assert!(zoomed_size(role, -100) >= ZOOM_MIN_SIZE);
            assert_eq!(
                role.zoomed_point_size(100),
                role.zoomed_point_size(ZOOM_MAX_LEVEL)
            );
        }
    }
}
