//! Color contrast calculations according to WCAG 2.1 specifications.
//!
//! Provides headless contrast checks plus the typed design-token
//! palette (Dark/Light) that `views.rs` paints with.
//!
//! Geometry and typography live here as headless tokens (this wave);
//! colors below carry the theme26 light mirrors (single color wave).
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
//! [`TypoRole::line_height`] (chrome 1.2 / body 1.40 / mono 1.30;
//! Wave V retune: tighter vertical rhythm so raster rows feel less tall;
//! paint still adopts via the shared measured mono row height),
//! [`TypoRole::letter_spacing_em`] (headers 0.02em, rest 0.0) and
//! [`TypoRole::is_tabular`] (gutter/hex/counts) record the target
//! hierarchy; paint adopts them in a later wave.
//!
//! Pane hierarchy: app header -> Heading; Home cards ->
//! HomeTitle/HomeDesc/HomeButton; diff panes, summaries, status lines
//! -> Mono; hints, subtitles, counts -> Small; plain labels, buttons
//! -> Body.
//!
//! Color model: the diff accents are per-mode washes sharing one hue
//! family (dark keeps the saturated rows, light remaps to pale washes +
//! dark inks), while the chrome (base text, dim/glyph, red/green/blue
//! markers, gutter track) is per-mode so both [`DARK_PALETTE`] and
//! [`LIGHT_PALETTE`] satisfy WCAG AA via [`theme_contrast_ok`].
//! Paint picks a side with [`palette`] / `*_for(dark)` using
//! `ui.visuals().dark_mode`; this wave adds the light mirrors.

#![forbid(unsafe_code)]

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

/// EXQ-TYPO54 system-first candidate chain for proportional/chrome roles
/// (headless, pure): runtime lookup order, no binaries shipped. Head is the
/// macOS system sans, middle reuses [`TYPO_UI_SANS_STACK`] by reference
/// (single source, no duplicated stack string), tail is the egui-safe
/// generic so the chain never resolves blank. Install itself stays in
/// `app.rs` (`typo48_live_font_plan`); this only supplies the order.
pub const TYPO54_SANS_CHAIN: &[&str] = &["SF Pro", TYPO_UI_SANS_STACK, "proportional"];
/// EXQ-TYPO54 system-first candidate chain for [`TypoRole::Mono`] content
/// (headless, pure): macOS system mono head, `Menlo` next, then the single
/// source [`TYPO_MONO_STACK`], tail egui-safe `monospace`. No binaries
/// shipped; install stays in `app.rs`.
pub const TYPO54_MONO_CHAIN: &[&str] = &["SF Mono", "Menlo", TYPO_MONO_STACK, "monospace"];

/// EXQ-TYPO54 ordered runtime family chain for `role` (headless, pure):
/// system names first, egui-safe generic last. Mono content gets
/// [`TYPO54_MONO_CHAIN`], every chrome role [`TYPO54_SANS_CHAIN`]. No new
/// sizes/weights/tokens, no colors, no display calls — `app.rs` install
/// consumes this without duplicating stack names.
pub const fn typo54_candidate_stack(role: TypoRole) -> &'static [&'static str] {
    if role.is_mono() {
        TYPO54_MONO_CHAIN
    } else {
        TYPO54_SANS_CHAIN
    }
}
/// Regular body weight (400).
pub const TYPO_WEIGHT_REGULAR: u16 = 400;
/// Medium headers/chrome weight (500).
pub const TYPO_WEIGHT_MEDIUM: u16 = 500;
/// Semibold badge/counts weight, used sparingly (600).
pub const TYPO_WEIGHT_SEMIBOLD: u16 = 600;
/// Compact chrome line-height multiplier (~1.2).
pub const TYPO_LINE_HEIGHT_CHROME: f32 = 1.2;
/// Reading body line-height multiplier (~1.40; Wave V: tightened from
/// 1.45 so 13px body rows feel less tall next to dense mono diffs).
pub const TYPO_LINE_HEIGHT_BODY: f32 = 1.40;
/// Dense mono line-height multiplier (~1.30; Wave V: tightened from 1.35
/// so 12px diff rows read lighter and the 24px raster row feels airier).
pub const TYPO_LINE_HEIGHT_MONO: f32 = 1.30;
/// Slight header tracking as em ratio (0.02em; egui has no tracking).
pub const TYPO_TRACKING_HEADING_EM: f32 = 0.02;

/// Home3 exquisite scale intent (headless, paint-neutral): visible size steps
/// (18 > 14 > 13 > 12 > 11) + non-zero header tracking (0.02em) while paint
/// tracking no-op ([`TYPO_LETTER_SPACING`] 0.0) untouched rehta hai taaki
/// paint-lock tests green rahen. Sab maujooda tokens reuse, koi naya size
/// nahi, colors untouched. Pure; headless-tested.
pub fn typo_exq_home_scale_ok() -> bool {
    TYPO_HEADING_SIZE > TYPO_HOME_TITLE_SIZE
        && TYPO_HOME_TITLE_SIZE > TYPO_BODY_SIZE
        && TYPO_BODY_SIZE > TYPO_MONO_SIZE
        && TYPO_MONO_SIZE > TYPO_SMALL_SIZE
        && TYPO_TRACKING_HEADING_EM > 0.0
        && TYPO_LETTER_SPACING == 0.0
        && TypoRole::HomeTitle.letter_spacing_em() > 0.0
        && TypoRole::HomeButton.letter_spacing_em() == 0.0
        && TypoRole::HomeDesc.letter_spacing_em() == 0.0
}

// ----------------------------------------------------------------------------
// Chrome hierarchy (headless): tab bar, section headers, home cards and
// status lines share one documented scale. No new point sizes — every
// alias reuses an existing TYPO_* token so the geometry/typography waves
// never drift; only the names plus the role mapping are new. Colors
// untouched (weak tone stays, family mono->proportional on status only).
// Ramp (4 poles, BC-dense ladder; mono content sits between chrome steps):
// header Heading 18 strong-large (app title, section headers) >
// card-title HomeTitle 14 (card titles) > tab Body 13 medium-step
// (tab bar, card buttons) > grid Mono 12 fixed (folder grid, hex/table
// cells, paths — via content_grid(); gutter text — via gutter_text())
// > hint Small 11 (status
// lines, counts, hints, captions — via caption()). Only the alias names
// are new; every size reuses an existing TYPO_* token so the
// geometry/typography waves never drift. Colors untouched (weak tone
// stays, family mono->proportional on status only).
// ----------------------------------------------------------------------------

/// Tab bar size in px: Body scale, default weight, proportional family.
pub const CHROME_TAB_SIZE: f32 = TYPO_BODY_SIZE;
/// Section header size in px: Heading scale, strong weight, proportional.
pub const CHROME_SECTION_SIZE: f32 = TYPO_HEADING_SIZE;
/// Status line size in px: Small scale, weak weight, proportional family.
pub const CHROME_STATUS_SIZE: f32 = TYPO_SMALL_SIZE;
/// Header bar height in px: matches the `views::HEADER_H` paging bars so
/// the app header reserves the same vertical rhythm as the page bars
/// below instead of floating offset above the pane split.
pub const CHROME_HEADER_BAR_H: f32 = 28.0;
/// Header left lead: same two-space origin as `views::CONTENT_LEAD` so the
/// header note starts on the same baseline as table rules/placeholders.
pub const CHROME_HEADER_LEAD: &str = "  ";

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

/// Zoom-aware line-height in px for `role` at `level`: the clamped
/// zoomed point size times the role multiplier (Wave V row-feel knob).
/// Reuses [`zoomed_size`] + [`TypoRole::line_height`] so zoom never
/// escapes the MIN..MAX size clamp; level 0 equals
/// [`TypoRole::line_height_px`]. Pure; unit-tested.
pub fn zoomed_line_height_px(role: TypoRole, level: i8) -> f32 {
    zoomed_size(role, level) * role.line_height()
}

/// Zoom scale factor for `role` at `level`: zoomed size over the base
/// [`TypoRole::size`]. Level 0 gives exactly 1.0; every role x every
/// [`ZOOM_LEVELS`] step stays inside 0.8..=2.0 so code text (Mono)
/// zoom hota hai par chrome (Small) se hierarchy toot-ti nahi. Pure;
/// unit-tested. Paint adoption baad ki wave me; yeh headless contract hai.
pub fn zoom_scale_factor(role: TypoRole, level: i8) -> f32 {
    zoomed_size(role, level) / role.size()
}

impl TypoRole {
    /// Zoomed point size for this role at `level` (see [`zoomed_size`]).
    pub fn zoomed_point_size(self, level: i8) -> f32 {
        zoomed_size(self, level)
    }

    /// Zoom scale factor for this role at `level` (see [`zoom_scale_factor`]).
    pub fn zoomed_scale_factor(self, level: i8) -> f32 {
        zoom_scale_factor(self, level)
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

// ----------------------------------------------------------------------------
// Viewport geometry (headless, px): canonical shot + narrow fallback.
// ----------------------------------------------------------------------------

/// Canonical viewport width in px: every fixture PNG proves this size.
pub const VIEWPORT_W: u32 = 960;
/// Canonical viewport height in px.
pub const VIEWPORT_H: u32 = 600;
/// Narrow fallback width in px: live window minimum; dual panes shrink
/// to fit, text ellipsis-clips instead of painting past the edge.
pub const VIEWPORT_MIN_W: u32 = 720;
/// Narrow fallback height in px: keeps the canonical 8:5 aspect.
pub const VIEWPORT_MIN_H: u32 = 450;
/// Fixed chrome around the dual panes in px: 12 outer lead + 4 pane
/// gap + 16 overview gutter + 8 right margin (fixture budget parity).
pub const DUAL_PANE_CHROME_W: u32 = 40;
/// Text-center gutter width in px: two independent panes ke beech
/// fixed-width center gutter jisme per-hunk copy arrows rehte hain.
/// Ponytail reuse: `CHROME_HEADER_BAR_H + GUTTER_BAR_WIDE +
/// 2 * GEO_TIGHT_GAP` = 48.0 — do chhote copy buttons side-by-side fit
/// hote hain, equal rows par blank click-to-select rehta hai. No new
/// literal, no color; paint (`views::TEXT_CENTER_GUTTER_W`) adopts in
/// a later wave. Pure token; unit-tested.
pub const TEXT_CENTER_GUTTER_W: f32 =
    CHROME_HEADER_BAR_H + GUTTER_BAR_WIDE + GEO_TIGHT_GAP + GEO_TIGHT_GAP;
/// Inline accent span underline width in px: sharp 1px line under the
/// changed word block (1px visual precedent, same as the split visual).
/// Spans hug the word exactly (zero char padding — the headless
/// `accent_a`/`accent_b` ranges are already half-open char offsets), so
/// emphasis comes from block + 1px line, never a full-width wash.
/// No color move; pure token; unit-tested.
pub const TEXT_INLINE_SPAN_UNDERLINE_W: f32 = 1.0;

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

/// Single-source resolved font for one [`TypoRole`]: exact px size,
/// egui font id (px + family), paint weight spelling, numeric weight,
/// line-height multiplier and px, letter-tracking and tabular flag.
/// Every field delegates to the matching [`TypoRole`] accessor so paint
/// never re-derives sizes per call-site; the future wiring pass builds
/// from [`TypoRole::spec`] / [`TypoRole::font_id`] instead of ad-hoc
/// `.monospace()`/`.size()` chains. Pure; unit-tested. Colors untouched
/// (paint owns them).
#[derive(Debug, Clone, PartialEq)]
pub struct TypoSpec {
    /// Role this spec was built from.
    pub role: TypoRole,
    /// Point size px ([`TypoRole::size`]).
    pub size: f32,
    /// Resolved font id (px + family, [`TypoRole::font_id`]).
    pub font_id: egui::FontId,
    /// Paint weight spelling (`strong`/`weak`/`default`/`regular-mono`).
    pub weight: &'static str,
    /// Numeric weight (400/500/600).
    pub weight_value: u16,
    /// Line-height multiplier ([`TypoRole::line_height`]).
    pub line_height: f32,
    /// Line-height px (`size * line_height`).
    pub line_height_px: f32,
    /// Letter-tracking em ratio (headers 0.02, rest 0.0).
    pub letter_spacing_em: f32,
    /// Tabular numerals intent (Mono + Small).
    pub is_tabular: bool,
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

    /// Hierarchy rank: 0 app header, 1 card title, 2 tab/body,
    /// 3 mono content, 4 small hints. Ranks follow point size, so a
    /// higher rank always paints smaller. Pure; unit-tested.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Heading => 0,
            Self::HomeTitle => 1,
            Self::Body | Self::HomeButton => 2,
            Self::Mono => 3,
            Self::Small | Self::HomeDesc => 4,
        }
    }

    /// Whether this role uses the small scale.
    pub const fn is_small(self) -> bool {
        matches!(self, Self::Small | Self::HomeDesc)
    }

    /// Whether this role uses the monospace family.
    pub const fn is_mono(self) -> bool {
        matches!(self, Self::Mono)
    }

    /// Whether this role is chrome (proportional UI) rather than content.
    /// Mono is the only content role (diff/table/folder rows); everything
    /// else paints chrome (titles, tabs, buttons, hints, counts, status).
    pub const fn is_chrome(self) -> bool {
        !self.is_mono()
    }

    /// Tab bar role: Body (13 default proportional). The tab
    /// `selectable_value` labels paint at Body scale; this alias locks
    /// the contract so tabs never drift to mono.
    pub const fn chrome_tab() -> Self {
        Self::Body
    }

    /// Section header role: Heading (18 strong proportional). App title
    /// plus section headers share the top chrome step; card titles step
    /// down to HomeTitle so app > section > card reads top-down.
    pub const fn chrome_section() -> Self {
        Self::Heading
    }

    /// Status line role: Small (11 weak proportional, tabular intent).
    /// Counts stay tabular (`is_tabular`); family leaves mono so status
    /// stops reading as code. Weak tone keeps the frozen dim, no hue move.
    pub const fn chrome_status() -> Self {
        Self::Small
    }

    /// Grid/data-cell role: Mono (12 regular monospace, tabular). Folder
    /// grid, hex/table cells, paths and gutter-adjacent rows all paint as
    /// Mono content, never chrome; the size stays fixed at
    /// [`TYPO_MONO_SIZE`] so dense rows align. Paint adoption is a later
    /// wave; this alias is the single named target (today paint still uses
    /// direct `.monospace()` at the call sites REPORT lists).
    pub const fn content_grid() -> Self {
        Self::Mono
    }

    /// Hint/caption role: Small (11 weak proportional, tabular intent).
    /// Hints, subtitles, captions and counts share the bottom chrome step;
    /// caption-strong (folder headers) stays a paint-side composition for
    /// a later wave, never a new size. Defines the `Caption` precedent the
    /// folder columns reference; caption-regular is Small exactly.
    pub const fn caption() -> Self {
        Self::Small
    }

    /// Gutter-text role: Mono (12 regular monospace, tabular). Line numbers
    /// and gutter-adjacent rows read in the same family as pane rows
    /// (BC hierarchy: gutter text matches diff-body mono, never chrome).
    /// Value-stable alias over [`TypoRole::content_grid`]; the local
    /// `gutter::gutter_text_role` helper keeps working unchanged and
    /// delegates here in a later wave (`gutter.rs` is out of scope now).
    /// Paint adoption is a later wave; no size/color/geometry moves here.
    pub const fn gutter_text() -> Self {
        Self::Mono
    }

    /// Text-center diff-body role: Mono (12 regular monospace, tabular).
    /// Both text panes paint as Mono content, never chrome — gutter text
    /// (`gutter_text`) same family me rehta hai taaki gutter + body ek
    /// rhythm me padhe. Value-stable alias over [`TypoRole::content_grid`];
    /// paint adoption later wave; no size/color/geometry move here.
    pub const fn content_textcenter() -> Self {
        Self::Mono
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

    /// EXQ-TYPO54 ordered runtime family chain for this role (see
    /// [`typo54_candidate_stack`]): system-first head, egui-safe generic
    /// tail. Delegates to the free function so `app.rs` install has one
    /// source per role and never duplicates stack names.
    pub const fn candidate_stack(self) -> &'static [&'static str] {
        typo54_candidate_stack(self)
    }

    /// egui font family for this role: monospace diff cells, proportional
    /// UI chrome. String twin is [`TypoRole::family`]; this is the resolver
    /// paint feeds to [`FontId`](egui::FontId).
    pub const fn font_family(self) -> egui::FontFamily {
        match self {
            Self::Mono => egui::FontFamily::Monospace,
            _ => egui::FontFamily::Proportional,
        }
    }

    /// Exact resolved [`FontId`](egui::FontId) for this role: point size
    /// from [`TypoRole::size`] plus [`TypoRole::font_family`]. Mono roles
    /// resolve to `monospace(size)`, everything else to
    /// `proportional(size)` (UI sans stack intent, [`TypoRole::font_stack`]).
    /// Pure; unit-tested. Paint wiring (later wave) uses this instead of
    /// re-deriving sizes per call-site.
    pub const fn font_id(self) -> egui::FontId {
        egui::FontId::new(self.size(), self.font_family())
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

    /// Line-height multiplier for this role: chrome ~1.2, body ~1.40,
    /// mono ~1.30 (Wave V retune, tighter rows). Paint still measures
    /// the shared mono row height ([`TYPO_LINE_HEIGHT_SOURCE`]); this is
    /// headless intent.
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

    /// Zoom-aware [`FontId`](egui::FontId): [`zoomed_size`] px plus the
    /// role family. Level 0 equals [`TypoRole::font_id`]; out-of-range
    /// levels clamp via [`clamp_zoom_level`], sizes via ZOOM_MIN/MAX.
    /// Pure; unit-tested. Paint adoption later wave me.
    pub fn font_id_zoomed(self, level: i8) -> egui::FontId {
        egui::FontId::new(zoomed_size(self, level), self.font_family())
    }

    /// Style-resolved [`FontId`](egui::FontId): this role's
    /// [`TypoRole::text_style`] resolved against `style` (honours runtime
    /// font overrides the same way `views.rs` does today with
    /// `TextStyle::Monospace.resolve(ui.style())`). Headless: needs no
    /// `Ui`/`Context`. Pure; unit-tested against `Style::default`.
    pub fn resolved_font_id(self, style: &egui::Style) -> egui::FontId {
        self.text_style().resolve(style)
    }

    /// Single-source bundle for this role: every exact value paint needs
    /// (px, [`FontId`](egui::FontId), weight spelling + numeric, line
    /// height, tracking, tabular) from one call. Each field equals the
    /// matching accessor; tests lock the delegation per role.
    pub const fn spec(self) -> TypoSpec {
        TypoSpec {
            role: self,
            size: self.size(),
            font_id: self.font_id(),
            weight: self.weight(),
            weight_value: self.weight_value(),
            line_height: self.line_height(),
            line_height_px: self.line_height_px(),
            letter_spacing_em: self.letter_spacing_em(),
            is_tabular: self.is_tabular(),
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
pub const DARK_TRACK_RGB: [u8; 3] = [48, 48, 48];
/// Light gutter track fill (decorative, not text).
pub const LIGHT_TRACK_RGB: [u8; 3] = [225, 225, 225];
/// Light deleted-row wash (dark-red ink [`LIGHT_RED_RGB`] on top).
/// Light-mirror of [`DELETED_BG_RGB`]: same red hue, lightness remapped
/// so the pair passes AA on the near-white pane.
pub const LIGHT_DELETED_BG_RGB: [u8; 3] = [255, 222, 222];
/// Light inserted-row wash (dark-green ink [`LIGHT_GREEN_RGB`] on top).
/// Light-mirror of [`INSERTED_BG_RGB`].
pub const LIGHT_INSERTED_BG_RGB: [u8; 3] = [214, 242, 220];
/// Light changed-row wash (dark-amber ink [`LIGHT_CHANGED_FG_RGB`] on top).
/// Light-mirror of [`CHANGED_BG_RGB`].
pub const LIGHT_CHANGED_BG_RGB: [u8; 3] = [255, 242, 204];
/// Light changed-row/span ink on the wash above. Light-mirror of
/// [`CHANGED_FG_RGB`]: same amber hue, darkened so AA holds on light.
pub const LIGHT_CHANGED_FG_RGB: [u8; 3] = [105, 85, 15];
/// Light error-panel wash (text reuses [`LIGHT_RED_RGB`]).
/// Light-mirror of [`ERROR_BG_RGB`].
pub const LIGHT_ERROR_BG_RGB: [u8; 3] = [252, 230, 230];

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
pub const DARK_TRACK: Color32 = Color32::from_rgb(48, 48, 48);
/// Light gutter track paint token.
pub const LIGHT_TRACK: Color32 = Color32::from_rgb(225, 225, 225);
/// Light deleted-row wash paint token.
pub const LIGHT_DELETED_BG: Color32 = Color32::from_rgb(255, 222, 222);
/// Light inserted-row wash paint token.
pub const LIGHT_INSERTED_BG: Color32 = Color32::from_rgb(214, 242, 220);
/// Light changed-row wash paint token.
pub const LIGHT_CHANGED_BG: Color32 = Color32::from_rgb(255, 242, 204);
/// Light changed-row/span ink paint token.
pub const LIGHT_CHANGED_FG: Color32 = Color32::from_rgb(105, 85, 15);
/// Light error-panel wash paint token.
pub const LIGHT_ERROR_BG: Color32 = Color32::from_rgb(252, 230, 230);

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

/// Deleted-row background for the active mode (saturated dark vs pale wash).
pub fn deleted_bg_for(dark: bool) -> Color32 {
    if dark { DELETED_BG } else { LIGHT_DELETED_BG }
}

/// Deleted-row ink for the active mode (pale pink vs reused dark red).
pub fn deleted_fg_for(dark: bool) -> Color32 {
    if dark { DELETED_FG } else { LIGHT_RED }
}

/// Inserted-row background for the active mode (saturated dark vs pale wash).
pub fn inserted_bg_for(dark: bool) -> Color32 {
    if dark { INSERTED_BG } else { LIGHT_INSERTED_BG }
}

/// Inserted-row ink for the active mode (pale mint vs reused dark green).
pub fn inserted_fg_for(dark: bool) -> Color32 {
    if dark { INSERTED_FG } else { LIGHT_GREEN }
}

/// Changed-row background for the active mode (olive dark vs amber wash).
pub fn changed_bg_for(dark: bool) -> Color32 {
    if dark { CHANGED_BG } else { LIGHT_CHANGED_BG }
}

/// Changed-row ink for the active mode (pale gold vs dark amber).
pub fn changed_fg_for(dark: bool) -> Color32 {
    if dark { CHANGED_FG } else { LIGHT_CHANGED_FG }
}

/// Accent span ink for the active mode (pale gold vs reused dark amber).
pub fn accent_fg_for(dark: bool) -> Color32 {
    if dark { ACCENT_FG } else { LIGHT_CHANGED_FG }
}

/// Error-panel wash for the active mode (dark maroon vs pale wash).
pub fn error_bg_for(dark: bool) -> Color32 {
    if dark { ERROR_BG } else { LIGHT_ERROR_BG }
}

/// Error-panel ink for the active mode (pale pink vs reused dark red).
pub fn error_fg_for(dark: bool) -> Color32 {
    if dark { DELETED_FG } else { LIGHT_RED }
}

/// Light diff-accent contract: every light wash/ink pair (plus the span
/// ink on the pane) meets WCAG AA. Pure; `theme26_light_contrast` proves it.
pub fn light_diff_accents_aa() -> bool {
    theme_contrast_ok(LIGHT_RED_RGB, LIGHT_DELETED_BG_RGB)
        && theme_contrast_ok(LIGHT_GREEN_RGB, LIGHT_INSERTED_BG_RGB)
        && theme_contrast_ok(LIGHT_CHANGED_FG_RGB, LIGHT_CHANGED_BG_RGB)
        && theme_contrast_ok(LIGHT_CHANGED_FG_RGB, LIGHT_BG_RGB)
        && theme_contrast_ok(LIGHT_RED_RGB, LIGHT_ERROR_BG_RGB)
}

// ----------------------------------------------------------------------------
// Dark/Light contrast typography (second): geometry/typo levers only.
// Hues FROZEN — farak sirf weight + size floor se, taaki dim/gutter text
// dono modes me AA par rahe bina ek bhi `Color32` chhue.
// ----------------------------------------------------------------------------

/// Dim secondary text role: Small semibold (11 weak + 600 tabular).
/// Weak tone frozen hai; weight (400->600) contrast carry karta hai —
/// dono palettes ke dim AA par hain, ye role wahi weight pakdta hai.
pub const fn contrast_dim_text_role() -> TypoRole {
    TypoRole::Small
}

/// Dim text min size px floor: Small token (11.0) — isse chhota dim text
/// kabhi paint na ho. Pure; unit-tested.
pub const fn contrast_dim_min_size() -> f32 {
    TYPO_SMALL_SIZE
}

/// Dim text numeric weight floor: semibold 600 — regular 400 dim text
/// halke background par dubla padta; weight gap color ke bina antar
/// pakdta hai. Pure; unit-tested.
pub const fn contrast_dim_min_weight() -> u16 {
    TYPO_WEIGHT_SEMIBOLD
}

/// Gutter/line-number text role: Mono tabular (12 regular) — dono modes
/// me 1px columns tabular rehte hain, mode switch par shimmer nahi.
/// Pure; unit-tested.
pub const fn contrast_gutter_text_role() -> TypoRole {
    TypoRole::Mono
}

/// Dim-text contrast contract: role small + weak + tabular, weight
/// semibold floor par, size floor par, aur dono mode palettes AA.
/// Koi hue move nahi — sirf type carry karta hai. Pure; unit-tested.
pub fn contrast_dim_text_ok() -> bool {
    let role = contrast_dim_text_role();
    role.is_small()
        && role.is_weak()
        && role.is_tabular()
        && role.weight_value() >= contrast_dim_min_weight()
        && role.size() >= contrast_dim_min_size()
        && theme_mode_contrast_ok(true)
        && theme_mode_contrast_ok(false)
}

// ----------------------------------------------------------------------------
// WCAG AA Design-Token Contrast Verification Tokens & Helpers
// ----------------------------------------------------------------------------

/// Standard WCAG 2.1 AA contrast ratio threshold for normal text (4.5:1).
pub const WCAG_AA_NORMAL_TEXT_RATIO: f64 = 4.5;

/// Standard WCAG 2.1 AA contrast ratio threshold for large text / UI components (3.0:1).
pub const WCAG_AA_LARGE_TEXT_RATIO: f64 = 3.0;

/// Standard WCAG 2.1 AAA contrast ratio threshold for normal text (7.0:1).
pub const WCAG_AAA_NORMAL_TEXT_RATIO: f64 = 7.0;

/// Contrast ratio calculation using [`Color32`] instances.
pub fn theme_contrast_ratio_color32(fg: Color32, bg: Color32) -> f64 {
    theme_contrast_ratio([fg.r(), fg.g(), fg.b()], [bg.r(), bg.g(), bg.b()])
}

/// Checks whether [`Color32`] foreground and background meet WCAG AA (4.5:1).
pub fn theme_contrast_ok_color32(fg: Color32, bg: Color32) -> bool {
    theme_contrast_ratio_color32(fg, bg) >= WCAG_AA_NORMAL_TEXT_RATIO
}

/// Verifies that all text tokens in the given palette satisfy the WCAG AA contrast threshold (>= 4.5:1).
pub fn palette_all_text_tokens_aa(palette: &Palette) -> bool {
    theme_contrast_ok(palette.base_fg, palette.base_bg)
        && theme_contrast_ok(palette.dim, palette.base_bg)
        && theme_contrast_ok(palette.red, palette.base_bg)
        && theme_contrast_ok(palette.green, palette.base_bg)
        && theme_contrast_ok(palette.blue, palette.base_bg)
}

/// Verifies that all Dark and Light design-token palettes satisfy WCAG AA contrast.
pub fn theme_all_palettes_satisfy_aa() -> bool {
    palette_all_text_tokens_aa(&DARK_PALETTE) && palette_all_text_tokens_aa(&LIGHT_PALETTE)
}

// ----------------------------------------------------------------------------
// Typed Keymap Table (Headless macOS UX tokens & bindings)
// ----------------------------------------------------------------------------

/// Supported keymap preset names in GUI layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeymapPreset {
    /// Default macOS / Beyond Compare keyboard shortcuts.
    Default,
    /// Modal Vim-style navigation and editing shortcuts.
    Vim,
    /// Visual Studio Code compatible shortcuts.
    Vscode,
    /// GNU Emacs compatible keybindings.
    Emacs,
}

impl KeymapPreset {
    /// Canonical identifier for the preset.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Vim => "vim",
            Self::Vscode => "vscode",
            Self::Emacs => "emacs",
        }
    }

    /// Parses a string into a [`KeymapPreset`], case-insensitively with alias support.
    pub fn from_name(name: &str) -> Option<Self> {
        let trimmed = name.trim();
        if trimmed.eq_ignore_ascii_case("default")
            || trimmed.eq_ignore_ascii_case("standard")
            || trimmed.eq_ignore_ascii_case("mac")
            || trimmed.eq_ignore_ascii_case("macos")
        {
            Some(Self::Default)
        } else if trimmed.eq_ignore_ascii_case("vim") || trimmed.eq_ignore_ascii_case("vi") {
            Some(Self::Vim)
        } else if trimmed.eq_ignore_ascii_case("vscode") || trimmed.eq_ignore_ascii_case("code") {
            Some(Self::Vscode)
        } else if trimmed.eq_ignore_ascii_case("emacs") {
            Some(Self::Emacs)
        } else {
            None
        }
    }

    /// All supported presets.
    pub const fn all() -> &'static [Self] {
        &[Self::Default, Self::Vim, Self::Vscode, Self::Emacs]
    }
}

/// Typed application actions triggerable via keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    /// Search / find in active pane (Cmd-F / Ctrl-F / / in vim / C-s in emacs).
    Find,
    /// Toggle file / metadata info or inspector panel (Cmd-I / Ctrl-I).
    ToggleInfo,
    /// Reset zoom level to default (Cmd-0 / Ctrl-0).
    ResetZoom,
    /// Zoom in font size (Cmd-+ / Cmd-=).
    ZoomIn,
    /// Zoom out font size (Cmd--).
    ZoomOut,
    /// Jump to next difference hunk (F3 / Ctrl-N / ]c in vim).
    NextDiff,
    /// Jump to previous difference hunk (Shift-F3 / Ctrl-P / [c in vim).
    PrevDiff,
    /// Toggle line numbers / gutter visibility.
    ToggleLineNumbers,
    /// Toggle word wrap in text panes.
    ToggleWrap,
}

/// Typed keyboard shortcut representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    /// Target primary key.
    pub key: egui::Key,
    /// Requires Command (macOS) or Control (other platforms).
    pub command: bool,
    /// Requires Shift modifier.
    pub shift: bool,
    /// Requires Alt / Option modifier.
    pub alt: bool,
    /// Requires Control modifier specifically (e.g. for Emacs C-x chords).
    pub ctrl: bool,
}

impl KeyBinding {
    /// Creates a Command (or Ctrl on non-Mac) shortcut with the given key.
    pub const fn cmd(key: egui::Key) -> Self {
        Self {
            key,
            command: true,
            shift: false,
            alt: false,
            ctrl: false,
        }
    }

    /// Creates a Command+Shift shortcut with the given key.
    pub const fn cmd_shift(key: egui::Key) -> Self {
        Self {
            key,
            command: true,
            shift: true,
            alt: false,
            ctrl: false,
        }
    }

    /// Creates a Control modifier shortcut with the given key.
    pub const fn ctrl(key: egui::Key) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: false,
            ctrl: true,
        }
    }

    /// Creates a plain, un-modified shortcut with the given key.
    pub const fn plain(key: egui::Key) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: false,
            ctrl: false,
        }
    }

    /// Builder to add Shift modifier.
    pub const fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Builder to add Alt / Option modifier.
    pub const fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// Tests if this binding matches an input key and modifier state.
    pub fn matches(&self, key: egui::Key, modifiers: &egui::Modifiers) -> bool {
        if self.key != key {
            return false;
        }
        let has_cmd = modifiers.command || modifiers.ctrl;
        if self.command && !has_cmd {
            return false;
        }
        if !self.command && has_cmd && !self.ctrl {
            return false;
        }
        if self.ctrl && !modifiers.ctrl {
            return false;
        }
        if self.shift != modifiers.shift {
            return false;
        }
        if self.alt != modifiers.alt {
            return false;
        }
        true
    }
}

const DEFAULT_FIND: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::F)];
const DEFAULT_INFO: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::I)];
const DEFAULT_RESET_ZOOM: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::Num0)];
const DEFAULT_ZOOM_IN: [KeyBinding; 2] = [
    KeyBinding::cmd(egui::Key::Equals),
    KeyBinding::cmd(egui::Key::Plus),
];
const DEFAULT_ZOOM_OUT: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::Minus)];
const DEFAULT_NEXT_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3),
    KeyBinding::ctrl(egui::Key::N),
];
const DEFAULT_PREV_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3).with_shift(),
    KeyBinding::ctrl(egui::Key::P),
];
const DEFAULT_TOGGLE_LINE_NUMBERS: [KeyBinding; 1] = [KeyBinding::cmd_shift(egui::Key::L)];
const DEFAULT_TOGGLE_WRAP: [KeyBinding; 1] = [KeyBinding::cmd_shift(egui::Key::W)];

const VIM_FIND: [KeyBinding; 2] = [
    KeyBinding::cmd(egui::Key::F),
    KeyBinding::plain(egui::Key::Slash),
];
const VIM_INFO: [KeyBinding; 2] = [
    KeyBinding::cmd(egui::Key::I),
    KeyBinding::plain(egui::Key::K),
];
const VIM_RESET_ZOOM: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::Num0)];
const VIM_NEXT_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3),
    KeyBinding::plain(egui::Key::J),
];
const VIM_PREV_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3).with_shift(),
    KeyBinding::plain(egui::Key::K).with_shift(),
];

const VSCODE_FIND: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::F)];
const VSCODE_INFO: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::I)];
const VSCODE_RESET_ZOOM: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::Num0)];
const VSCODE_NEXT_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3),
    KeyBinding::plain(egui::Key::F7),
];
const VSCODE_PREV_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3).with_shift(),
    KeyBinding::plain(egui::Key::F7).with_shift(),
];

const EMACS_FIND: [KeyBinding; 2] = [
    KeyBinding::cmd(egui::Key::F),
    KeyBinding::ctrl(egui::Key::S),
];
const EMACS_INFO: [KeyBinding; 2] = [
    KeyBinding::cmd(egui::Key::I),
    KeyBinding::ctrl(egui::Key::H),
];
const EMACS_RESET_ZOOM: [KeyBinding; 1] = [KeyBinding::cmd(egui::Key::Num0)];
const EMACS_NEXT_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3),
    KeyBinding::ctrl(egui::Key::N),
];
const EMACS_PREV_DIFF: [KeyBinding; 2] = [
    KeyBinding::plain(egui::Key::F3).with_shift(),
    KeyBinding::ctrl(egui::Key::P),
];

/// Returns the slice of [`KeyBinding`]s registered for an action in the given preset.
pub fn keymap_bindings(preset: KeymapPreset, action: KeyAction) -> &'static [KeyBinding] {
    match (preset, action) {
        (KeymapPreset::Default, KeyAction::Find) => &DEFAULT_FIND,
        (KeymapPreset::Default, KeyAction::ToggleInfo) => &DEFAULT_INFO,
        (KeymapPreset::Default, KeyAction::ResetZoom) => &DEFAULT_RESET_ZOOM,
        (KeymapPreset::Default, KeyAction::ZoomIn) => &DEFAULT_ZOOM_IN,
        (KeymapPreset::Default, KeyAction::ZoomOut) => &DEFAULT_ZOOM_OUT,
        (KeymapPreset::Default, KeyAction::NextDiff) => &DEFAULT_NEXT_DIFF,
        (KeymapPreset::Default, KeyAction::PrevDiff) => &DEFAULT_PREV_DIFF,
        (KeymapPreset::Default, KeyAction::ToggleLineNumbers) => &DEFAULT_TOGGLE_LINE_NUMBERS,
        (KeymapPreset::Default, KeyAction::ToggleWrap) => &DEFAULT_TOGGLE_WRAP,

        (KeymapPreset::Vim, KeyAction::Find) => &VIM_FIND,
        (KeymapPreset::Vim, KeyAction::ToggleInfo) => &VIM_INFO,
        (KeymapPreset::Vim, KeyAction::ResetZoom) => &VIM_RESET_ZOOM,
        (KeymapPreset::Vim, KeyAction::ZoomIn) => &DEFAULT_ZOOM_IN,
        (KeymapPreset::Vim, KeyAction::ZoomOut) => &DEFAULT_ZOOM_OUT,
        (KeymapPreset::Vim, KeyAction::NextDiff) => &VIM_NEXT_DIFF,
        (KeymapPreset::Vim, KeyAction::PrevDiff) => &VIM_PREV_DIFF,
        (KeymapPreset::Vim, KeyAction::ToggleLineNumbers) => &DEFAULT_TOGGLE_LINE_NUMBERS,
        (KeymapPreset::Vim, KeyAction::ToggleWrap) => &DEFAULT_TOGGLE_WRAP,

        (KeymapPreset::Vscode, KeyAction::Find) => &VSCODE_FIND,
        (KeymapPreset::Vscode, KeyAction::ToggleInfo) => &VSCODE_INFO,
        (KeymapPreset::Vscode, KeyAction::ResetZoom) => &VSCODE_RESET_ZOOM,
        (KeymapPreset::Vscode, KeyAction::ZoomIn) => &DEFAULT_ZOOM_IN,
        (KeymapPreset::Vscode, KeyAction::ZoomOut) => &DEFAULT_ZOOM_OUT,
        (KeymapPreset::Vscode, KeyAction::NextDiff) => &VSCODE_NEXT_DIFF,
        (KeymapPreset::Vscode, KeyAction::PrevDiff) => &VSCODE_PREV_DIFF,
        (KeymapPreset::Vscode, KeyAction::ToggleLineNumbers) => &DEFAULT_TOGGLE_LINE_NUMBERS,
        (KeymapPreset::Vscode, KeyAction::ToggleWrap) => &DEFAULT_TOGGLE_WRAP,

        (KeymapPreset::Emacs, KeyAction::Find) => &EMACS_FIND,
        (KeymapPreset::Emacs, KeyAction::ToggleInfo) => &EMACS_INFO,
        (KeymapPreset::Emacs, KeyAction::ResetZoom) => &EMACS_RESET_ZOOM,
        (KeymapPreset::Emacs, KeyAction::ZoomIn) => &DEFAULT_ZOOM_IN,
        (KeymapPreset::Emacs, KeyAction::ZoomOut) => &DEFAULT_ZOOM_OUT,
        (KeymapPreset::Emacs, KeyAction::NextDiff) => &EMACS_NEXT_DIFF,
        (KeymapPreset::Emacs, KeyAction::PrevDiff) => &EMACS_PREV_DIFF,
        (KeymapPreset::Emacs, KeyAction::ToggleLineNumbers) => &DEFAULT_TOGGLE_LINE_NUMBERS,
        (KeymapPreset::Emacs, KeyAction::ToggleWrap) => &DEFAULT_TOGGLE_WRAP,
    }
}

/// Resolves a key press and modifier state into a [`KeyAction`] under the active preset.
pub fn keymap_resolve(
    preset: KeymapPreset,
    key: egui::Key,
    modifiers: &egui::Modifiers,
) -> Option<KeyAction> {
    const ACTIONS: [KeyAction; 9] = [
        KeyAction::Find,
        KeyAction::ToggleInfo,
        KeyAction::ResetZoom,
        KeyAction::ZoomIn,
        KeyAction::ZoomOut,
        KeyAction::NextDiff,
        KeyAction::PrevDiff,
        KeyAction::ToggleLineNumbers,
        KeyAction::ToggleWrap,
    ];

    for action in ACTIONS {
        let bindings = keymap_bindings(preset, action);
        for binding in bindings {
            if binding.matches(key, modifiers) {
                return Some(action);
            }
        }
    }
    None
}

/// Returns human-readable shortcut hint text for display in menus or status bars (e.g. "⌘F").
pub fn keymap_shortcut_hint(preset: KeymapPreset, action: KeyAction) -> &'static str {
    match (preset, action) {
        (KeymapPreset::Default, KeyAction::Find) => "⌘F",
        (KeymapPreset::Default, KeyAction::ToggleInfo) => "⌘I",
        (KeymapPreset::Default, KeyAction::ResetZoom) => "⌘0",
        (KeymapPreset::Default, KeyAction::ZoomIn) => "⌘+",
        (KeymapPreset::Default, KeyAction::ZoomOut) => "⌘-",
        (KeymapPreset::Default, KeyAction::NextDiff) => "F3",
        (KeymapPreset::Default, KeyAction::PrevDiff) => "⇧F3",
        (KeymapPreset::Default, KeyAction::ToggleLineNumbers) => "⇧⌘L",
        (KeymapPreset::Default, KeyAction::ToggleWrap) => "⇧⌘W",

        (KeymapPreset::Vim, KeyAction::Find) => "⌘F / /",
        (KeymapPreset::Vim, KeyAction::ToggleInfo) => "⌘I / K",
        (KeymapPreset::Vim, KeyAction::ResetZoom) => "⌘0",
        (KeymapPreset::Vim, KeyAction::ZoomIn) => "⌘+",
        (KeymapPreset::Vim, KeyAction::ZoomOut) => "⌘-",
        (KeymapPreset::Vim, KeyAction::NextDiff) => "j / F3",
        (KeymapPreset::Vim, KeyAction::PrevDiff) => "k / ⇧F3",
        (KeymapPreset::Vim, KeyAction::ToggleLineNumbers) => "⇧⌘L",
        (KeymapPreset::Vim, KeyAction::ToggleWrap) => "⇧⌘W",

        (KeymapPreset::Vscode, KeyAction::Find) => "⌘F",
        (KeymapPreset::Vscode, KeyAction::ToggleInfo) => "⌘I",
        (KeymapPreset::Vscode, KeyAction::ResetZoom) => "⌘0",
        (KeymapPreset::Vscode, KeyAction::ZoomIn) => "⌘+",
        (KeymapPreset::Vscode, KeyAction::ZoomOut) => "⌘-",
        (KeymapPreset::Vscode, KeyAction::NextDiff) => "F7",
        (KeymapPreset::Vscode, KeyAction::PrevDiff) => "⇧F7",
        (KeymapPreset::Vscode, KeyAction::ToggleLineNumbers) => "⇧⌘L",
        (KeymapPreset::Vscode, KeyAction::ToggleWrap) => "⇧⌘W",

        (KeymapPreset::Emacs, KeyAction::Find) => "⌘F / C-s",
        (KeymapPreset::Emacs, KeyAction::ToggleInfo) => "⌘I / C-h",
        (KeymapPreset::Emacs, KeyAction::ResetZoom) => "⌘0",
        (KeymapPreset::Emacs, KeyAction::ZoomIn) => "⌘+",
        (KeymapPreset::Emacs, KeyAction::ZoomOut) => "⌘-",
        (KeymapPreset::Emacs, KeyAction::NextDiff) => "C-n",
        (KeymapPreset::Emacs, KeyAction::PrevDiff) => "C-p",
        (KeymapPreset::Emacs, KeyAction::ToggleLineNumbers) => "⇧⌘L",
        (KeymapPreset::Emacs, KeyAction::ToggleWrap) => "⇧⌘W",
    }
}

/// Verifies that all required core macOS bindings (Cmd-F, Cmd-I, Cmd-0) are active in `preset`.
pub fn keymap_has_core_mac_bindings(preset: KeymapPreset) -> bool {
    let cmd_modifiers = egui::Modifiers {
        command: true,
        ..Default::default()
    };
    let has_find = keymap_resolve(preset, egui::Key::F, &cmd_modifiers) == Some(KeyAction::Find);
    let has_info =
        keymap_resolve(preset, egui::Key::I, &cmd_modifiers) == Some(KeyAction::ToggleInfo);
    let has_reset =
        keymap_resolve(preset, egui::Key::Num0, &cmd_modifiers) == Some(KeyAction::ResetZoom);
    has_find && has_info && has_reset
}

// ----------------------------------------------------------------------------
// TEXT CENTER exquisite type steps (second): diff-body Mono 12, chrome
// Body 13, hints Small 11 — calm hierarchy, no new sizes. Sab maujooda
// tokens reuse, colors untouched. Paint adoption baad ki wave me; ye
// headless intent hai.
// ----------------------------------------------------------------------------

/// TEXT CENTER type steps ok: Body 13 > Mono 12 > Small 11; line-height
/// px order body (18.2) > mono (15.6) > small (13.2); weights 400
/// body/mono < 600 small-counts; tracking trio 0.0, headers 0.02em;
/// tabular Mono + Small, Body never. TypoSpec delegation locked per role.
/// Pure; headless-tested.
pub fn typo_textcenter_steps_ok() -> bool {
    let body = TypoRole::Body;
    let mono = TypoRole::content_textcenter();
    let small = TypoRole::Small;
    mono.size() == TYPO_MONO_SIZE
        && body.size() == TYPO_BODY_SIZE
        && small.size() == TYPO_SMALL_SIZE
        && mono.size() < body.size()
        && small.size() < mono.size()
        && body.line_height_px() > mono.line_height_px()
        && mono.line_height_px() > small.line_height_px()
        && mono.weight_value() == TYPO_WEIGHT_REGULAR
        && body.weight_value() == TYPO_WEIGHT_REGULAR
        && small.weight_value() == TYPO_WEIGHT_SEMIBOLD
        && mono.letter_spacing_em() == 0.0
        && body.letter_spacing_em() == 0.0
        && small.letter_spacing_em() == 0.0
        && TypoRole::Heading.letter_spacing_em() == TYPO_TRACKING_HEADING_EM
        && mono.is_tabular()
        && small.is_tabular()
        && !body.is_tabular()
        && mono.is_mono()
        && mono.spec().weight_value == TYPO_WEIGHT_REGULAR
        && body.spec().size == TYPO_BODY_SIZE
        && small.spec().size == TYPO_SMALL_SIZE
}

// ----------------------------------------------------------------------------
// HOME4 exquisite type steps (second): family/weight/line-height/tracking.
// Sab TypoRole reuse, koi naya size nahi, colors untouched. Paint adoption
// baad ki wave me; ye headless intent hai.
// ----------------------------------------------------------------------------

/// HOME4 card type steps ok: Title 14 > Button 13 > Desc 11, weights
/// 500 > 400, title tracking 0.02 baaki 0, sab chrome proportional,
/// line-height px order title > button > desc. Pure; headless-tested.
pub fn typo_home4_steps_ok() -> bool {
    let title = TypoRole::HomeTitle;
    let button = TypoRole::HomeButton;
    let desc = TypoRole::HomeDesc;
    title.size() > button.size()
        && button.size() > desc.size()
        && title.weight_value() > desc.weight_value()
        && title.letter_spacing_em() > 0.0
        && button.letter_spacing_em() == 0.0
        && desc.letter_spacing_em() == 0.0
        && title.is_chrome()
        && button.is_chrome()
        && desc.is_chrome()
        && title.family() == "proportional"
        && button.family() == "proportional"
        && desc.family() == "proportional"
        && title.line_height_px() > button.line_height_px()
        && button.line_height_px() > desc.line_height_px()
}

// ----------------------------------------------------------------------------
// TYPO8 exquisite roles plus rhythm (headless, paint-neutral): geometry FIRST
// (paragraph/section gaps reuse the canonical SPACE scale via GEO_* aliases
// for margins/padding rhythm), typography SECOND (every gap maps to one
// TypoRole, no new point sizes — family/weight/line-height/letter-spacing
// unchanged), colors FROZEN (zero Color32/Visuals). Paint adoption later wave.
// ----------------------------------------------------------------------------

/// Body paragraph gap: one SM step (== [`GEO_SECTION_GAP`], 8.0).
pub const TYPO_PARA_GAP_BODY: f32 = GEO_SECTION_GAP;
/// Small/hint paragraph gap: one XS step (== [`GEO_TIGHT_GAP`], 4.0).
pub const TYPO_PARA_GAP_SMALL: f32 = GEO_TIGHT_GAP;
/// Chrome section gap: one MD step (== [`GEO_OUTER_MARGIN`], 12.0).
pub const TYPO_SECTION_GAP_CHROME: f32 = GEO_OUTER_MARGIN;
/// Mono content row gap: one XS step keeps dense 12px rows airy, no new size.
pub const TYPO_ROW_GAP_MONO: f32 = GEO_TIGHT_GAP;

/// Rhythm contract: gaps reuse the geometry scale in strict order
/// (small 4 < body 8 < section 12, mono row == small) and line-height px
/// keeps Small < Mono < Body so vertical rhythm never inverts. Pure.
pub fn typo_rhythm_ok() -> bool {
    TYPO_PARA_GAP_SMALL == GEO_TIGHT_GAP
        && TYPO_PARA_GAP_BODY == GEO_SECTION_GAP
        && TYPO_SECTION_GAP_CHROME == GEO_OUTER_MARGIN
        && TYPO_ROW_GAP_MONO == GEO_TIGHT_GAP
        && TYPO_PARA_GAP_SMALL < TYPO_PARA_GAP_BODY
        && TYPO_PARA_GAP_BODY < TYPO_SECTION_GAP_CHROME
        && TYPO_ROW_GAP_MONO == TYPO_PARA_GAP_SMALL
        && TypoRole::Small.line_height_px() < TypoRole::Mono.line_height_px()
        && TypoRole::Mono.line_height_px() < TypoRole::Body.line_height_px()
}

/// Roles plus rhythm: existing ladders (home scale, text-center steps, home4
/// steps) plus the rhythm gaps above. Single gate for TYPO8. Pure.
pub fn typo_roles_plus_rhythm_ok() -> bool {
    typo_exq_home_scale_ok()
        && typo_textcenter_steps_ok()
        && typo_home4_steps_ok()
        && typo_rhythm_ok()
}

impl TypoRole {
    /// Vertical gap after a paragraph/section in this role: small roles hug
    /// (XS), body/mono breathe (SM), headers separate sections (MD). Reuses
    /// [`TYPO_PARA_GAP_SMALL`] / [`TYPO_PARA_GAP_BODY`] /
    /// [`TYPO_SECTION_GAP_CHROME`]; no new sizes, no colors. Pure.
    pub const fn para_gap(self) -> f32 {
        match self {
            Self::Small | Self::HomeDesc => TYPO_PARA_GAP_SMALL,
            Self::Body | Self::Mono | Self::HomeButton => TYPO_PARA_GAP_BODY,
            Self::Heading | Self::HomeTitle => TYPO_SECTION_GAP_CHROME,
        }
    }
}

// ----------------------------------------------------------------------------
// THEME10 exquisite pane hierarchy (headless, paint-neutral): geometry FIRST
// (pane header/body/meta gaps plus header height/pad reuse the canonical
// SPACE scale via GEO_*/CHROME_* aliases for margins/padding rhythm),
// typography SECOND (header Heading 18 strong > body Mono 12 > meta Small 11
// — TypoRole reuse only, no new point sizes), colors FROZEN (zero
// Color32/Visuals). Paint adoption later wave.
// ----------------------------------------------------------------------------

/// Pane header gap: one MD step (== [`GEO_OUTER_MARGIN`], 12.0).
pub const THEME10_PANE_HEADER_GAP: f32 = GEO_OUTER_MARGIN;
/// Pane body gap: one SM step (== [`GEO_SECTION_GAP`], 8.0).
pub const THEME10_PANE_BODY_GAP: f32 = GEO_SECTION_GAP;
/// Pane meta gap: one XS step (== [`GEO_TIGHT_GAP`], 4.0).
pub const THEME10_PANE_META_GAP: f32 = GEO_TIGHT_GAP;
/// Pane header bar height: same rhythm as [`CHROME_HEADER_BAR_H`] (28.0).
pub const THEME10_PANE_HEADER_H: f32 = CHROME_HEADER_BAR_H;
/// Pane horizontal pad: one MD step (== [`GEO_OUTER_MARGIN`], 12.0).
pub const THEME10_PANE_PAD_X: f32 = GEO_OUTER_MARGIN;

/// Pane hierarchy contract: gaps reuse the geometry scale in strict order
/// (meta 4 < body 8 < header-gap 12), header bar keeps the 28px chrome
/// rhythm, sizes step header 18 > body 12 > meta 11 with line-height px in
/// the same order, weights strong > regular-mono > weak carry the tone.
/// Pure.
pub fn theme10_pane_hierarchy_ok() -> bool {
    THEME10_PANE_META_GAP == GEO_TIGHT_GAP
        && THEME10_PANE_BODY_GAP == GEO_SECTION_GAP
        && THEME10_PANE_HEADER_GAP == GEO_OUTER_MARGIN
        && THEME10_PANE_HEADER_H == CHROME_HEADER_BAR_H
        && THEME10_PANE_PAD_X == GEO_OUTER_MARGIN
        && THEME10_PANE_META_GAP < THEME10_PANE_BODY_GAP
        && THEME10_PANE_BODY_GAP < THEME10_PANE_HEADER_GAP
        && TypoRole::pane_header().size() > TypoRole::pane_body().size()
        && TypoRole::pane_body().size() > TypoRole::pane_meta().size()
        && TypoRole::pane_header().line_height_px() > TypoRole::pane_body().line_height_px()
        && TypoRole::pane_body().line_height_px() > TypoRole::pane_meta().line_height_px()
}

// ----------------------------------------------------------------------------
// EXQ-TYPO10 rhythm + line-height + density contract (headless, paint-neutral).
// Single gate proving monospace consistency across code panes, proportional
// chrome with weight hierarchy, line-height vertical rhythm without inversion,
// and zero text clipping across 960x600 and 720px fallback.
// ----------------------------------------------------------------------------

/// Wave EXQ-TYPO10 contract: TypoRole rhythm + line-height audit, monospace consistency,
/// proportional chrome hierarchy, letter-tracking and vertical rhythm tokens without any
/// new font sizes or color changes. Single gate for EXQ-TYPO10. Pure; unit-tested.
pub fn exq_typo10_roles_plus_rhythm_ok() -> bool {
    typo_roles_plus_rhythm_ok()
        && theme10_pane_hierarchy_ok()
        && TypoRole::Mono.is_mono()
        && TypoRole::Mono.is_tabular()
        && TypoRole::Mono.size() == TYPO_MONO_SIZE
        && TypoRole::Heading.is_chrome()
        && TypoRole::Body.is_chrome()
        && TypoRole::Small.is_chrome()
        && TypoRole::Heading.letter_spacing_em() == TYPO_TRACKING_HEADING_EM
        && TypoRole::Body.letter_spacing_em() == 0.0
        && TypoRole::Mono.letter_spacing_em() == 0.0
        && TypoRole::Small.letter_spacing_em() == 0.0
        && TypoRole::Small.line_height_px() < TypoRole::Mono.line_height_px()
        && TypoRole::Mono.line_height_px() < TypoRole::Body.line_height_px()
        && TypoRole::Body.line_height_px() < TypoRole::Heading.line_height_px()
}

impl TypoRole {
    /// Pane header role: Heading (18 strong proportional, tabular off).
    /// Same value as [`TypoRole::chrome_section`]: app title plus section
    /// plus pane headers share the top chrome step so pane headers read
    /// above card titles. Paint adoption later wave; no size/color move.
    pub const fn pane_header() -> Self {
        Self::Heading
    }

    /// Pane body role: Mono (12 regular monospace, tabular). Same value as
    /// [`TypoRole::content_textcenter`]: diff rows read as code, never
    /// chrome. Paint adoption later wave; no size/color move.
    pub const fn pane_body() -> Self {
        Self::Mono
    }

    /// Pane meta role: Small (11 weak proportional, tabular intent). Same
    /// value as [`TypoRole::caption`]: hints, counts, subtitles hug the
    /// bottom step. Paint adoption later wave; no size/color move.
    pub const fn pane_meta() -> Self {
        Self::Small
    }
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
    fn theme26_light_contrast() {
        // Edge fixture: `checks/gui-exq-theme26/round-1/comparison/theme-contrast-720.json`.
        // 720px narrow fallback reuses the identical light palette
        // (width-independent hues); every light text ink below must meet
        // WCAG AA (>= 4.5) on its pane surface.
        assert!(theme_contrast_ok(LIGHT_FG_RGB, LIGHT_BG_RGB));
        assert!(theme_contrast_ok(LIGHT_DIM_RGB, LIGHT_BG_RGB));
        assert!(theme_contrast_ok(LIGHT_RED_RGB, LIGHT_BG_RGB));
        assert!(theme_contrast_ok(LIGHT_GREEN_RGB, LIGHT_BG_RGB));
        assert!(theme_contrast_ok(LIGHT_BLUE_RGB, LIGHT_BG_RGB));
        assert!(light_diff_accents_aa());
        assert!(palette_all_text_tokens_aa(&LIGHT_PALETTE));
        // Washes stay pale tints of the near-white pane (hue, not depth,
        // carries the signal) while inks stay dark enough for AA.
        for wash in [
            LIGHT_DELETED_BG_RGB,
            LIGHT_INSERTED_BG_RGB,
            LIGHT_CHANGED_BG_RGB,
            LIGHT_ERROR_BG_RGB,
        ] {
            assert!(wash != LIGHT_BG_RGB);
            assert!(theme_contrast_ratio(wash, LIGHT_BG_RGB) < 1.5);
        }
        // Selectors agree with the consts on both arms.
        assert_eq!(deleted_bg_for(true), DELETED_BG);
        assert_eq!(deleted_bg_for(false), LIGHT_DELETED_BG);
        assert_eq!(inserted_fg_for(true), INSERTED_FG);
        assert_eq!(inserted_fg_for(false), LIGHT_GREEN);
        assert_eq!(changed_fg_for(false), LIGHT_CHANGED_FG);
        assert_eq!(accent_fg_for(false), LIGHT_CHANGED_FG);
        assert_eq!(error_bg_for(false), LIGHT_ERROR_BG);
        assert_eq!(error_fg_for(false), LIGHT_RED);
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
        assert_eq!(DARK_TRACK_RGB, [48, 48, 48]);
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
        assert!((TYPO_LINE_HEIGHT_BODY - 1.40).abs() < 0.001);
        assert!((TYPO_LINE_HEIGHT_MONO - 1.30).abs() < 0.001);
        // px height follows size * multiplier.
        assert!((TypoRole::Body.line_height_px() - 13.0 * 1.40).abs() < 0.01);
        assert!((TypoRole::Mono.line_height_px() - 12.0 * 1.30).abs() < 0.01);
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
    fn typo54_candidate_chains_system_first() {
        let roles = [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ];
        // Har role ki chain non-empty; method free function se milti hai.
        for role in roles {
            assert!(!typo54_candidate_stack(role).is_empty());
            assert_eq!(role.candidate_stack(), typo54_candidate_stack(role));
        }
        // Mono chain proportional chain se alag hai.
        assert_ne!(
            typo54_candidate_stack(TypoRole::Mono),
            typo54_candidate_stack(TypoRole::Body)
        );
        // Chrome roles ek hi sans chain, Mono apni mono chain.
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            assert_eq!(typo54_candidate_stack(role), TYPO54_SANS_CHAIN);
        }
        assert_eq!(typo54_candidate_stack(TypoRole::Mono), TYPO54_MONO_CHAIN);
        // Single source: stacks duplicate nahi, const reference se reuse.
        assert!(TYPO54_SANS_CHAIN.contains(&TYPO_UI_SANS_STACK));
        assert!(TYPO54_MONO_CHAIN.contains(&TYPO_MONO_STACK));
        // System-first head, egui-safe generic tail (kabhi blank nahi).
        assert_eq!(TYPO54_SANS_CHAIN.first(), Some(&"SF Pro"));
        assert_eq!(TYPO54_SANS_CHAIN.last(), Some(&"proportional"));
        assert_eq!(TYPO54_MONO_CHAIN.first(), Some(&"SF Mono"));
        assert_eq!(TYPO54_MONO_CHAIN.last(), Some(&"monospace"));
    }

    #[test]
    fn glyph_zoomed_line_height_clamps() {
        // Level 0 is the base passthrough for every role.
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            assert!((zoomed_line_height_px(role, 0) - role.line_height_px()).abs() < 0.001);
            // One px step up grows the row by exactly one multiplier.
            assert!(
                (zoomed_line_height_px(role, 1)
                    - zoomed_line_height_px(role, 0)
                    - role.line_height())
                .abs()
                    < 0.001
            );
            // Extreme input clamps through the existing zoom resolver.
            assert_eq!(
                zoomed_line_height_px(role, 100),
                zoomed_line_height_px(role, ZOOM_MAX_LEVEL)
            );
            assert_eq!(
                zoomed_line_height_px(role, -100),
                zoomed_line_height_px(role, ZOOM_MIN_LEVEL)
            );
        }
        // Wave V feel numbers: mono row tightens 16.2 -> 15.6px,
        // body row tightens 18.85 -> 18.2px, chrome unchanged at 21.6px.
        assert!((TypoRole::Mono.line_height_px() - 15.6).abs() < 0.01);
        assert!((TypoRole::Body.line_height_px() - 18.2).abs() < 0.01);
        assert!((TypoRole::Heading.line_height_px() - 21.6).abs() < 0.01);
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

    #[test]
    fn zoom_scale_stays_in_factor_band_and_keeps_hierarchy() {
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
            // Level 0 par factor exactly 1.0 (passthrough invariant).
            assert!((zoom_scale_factor(role, 0) - 1.0).abs() < 0.001);
            assert!((role.zoomed_scale_factor(0) - 1.0).abs() < 0.001);
            assert!((role.zoomed_scale_factor(ZOOM_DEFAULT_LEVEL) - 1.0).abs() < 0.001);
        }
        for level in ZOOM_LEVELS {
            // Har role ka factor 0.8..=2.0 band me (Gap #9 contract).
            for role in roles {
                let factor = zoom_scale_factor(role, level);
                assert!(factor >= 0.8, "{role:?} lv{level}: {factor}");
                assert!(factor <= 2.0, "{role:?} lv{level}: {factor}");
            }
            // Uniform 1px steps se order barkarar: mono code vs small chrome.
            assert!(zoomed_size(TypoRole::Small, level) < zoomed_size(TypoRole::Body, level));
            assert!(zoomed_size(TypoRole::Mono, level) < zoomed_size(TypoRole::Body, level));
            assert!(zoomed_size(TypoRole::Body, level) < zoomed_size(TypoRole::HomeTitle, level));
            assert!(
                zoomed_size(TypoRole::HomeTitle, level) < zoomed_size(TypoRole::Heading, level)
            );
            assert_eq!(
                zoomed_size(TypoRole::HomeDesc, level),
                zoomed_size(TypoRole::Small, level)
            );
            assert_eq!(
                zoomed_size(TypoRole::HomeButton, level),
                zoomed_size(TypoRole::Body, level)
            );
        }
    }

    #[test]
    fn chrome_sizes_match_typo_scale() {
        // Aliases reuse existing tokens: no new point sizes this wave.
        assert_eq!(CHROME_TAB_SIZE, TYPO_BODY_SIZE);
        assert_eq!(CHROME_TAB_SIZE, 13.0);
        assert_eq!(CHROME_SECTION_SIZE, TYPO_HEADING_SIZE);
        assert_eq!(CHROME_SECTION_SIZE, 18.0);
        assert_eq!(CHROME_STATUS_SIZE, TYPO_SMALL_SIZE);
        assert_eq!(CHROME_STATUS_SIZE, 11.0);
        // Header bar reserves the paging-bar rhythm; lead shares the
        // pane-content origin (checked against views tokens downstream).
        assert_eq!(CHROME_HEADER_BAR_H, 28.0);
        assert_eq!(CHROME_HEADER_LEAD, "  ");
        const {
            assert!(CHROME_STATUS_SIZE < TYPO_MONO_SIZE);
            assert!(TYPO_MONO_SIZE < CHROME_TAB_SIZE);
            assert!(CHROME_TAB_SIZE < TYPO_HOME_TITLE_SIZE);
            assert!(TYPO_HOME_TITLE_SIZE < CHROME_SECTION_SIZE);
        }
    }

    #[test]
    fn chrome_roles_carry_hierarchy() {
        // Chrome mapping: tab Body, section Heading, status Small.
        assert_eq!(TypoRole::chrome_tab(), TypoRole::Body);
        assert_eq!(TypoRole::chrome_section(), TypoRole::Heading);
        assert_eq!(TypoRole::chrome_status(), TypoRole::Small);
        assert_eq!(TypoRole::chrome_tab().size(), CHROME_TAB_SIZE);
        assert_eq!(TypoRole::chrome_section().size(), CHROME_SECTION_SIZE);
        assert_eq!(TypoRole::chrome_status().size(), CHROME_STATUS_SIZE);
        // Weights: section strong, tab default, status weak.
        assert_eq!(TypoRole::chrome_section().weight(), "strong");
        assert_eq!(TypoRole::chrome_tab().weight(), "default");
        assert_eq!(TypoRole::chrome_status().weight(), "weak");
        // Chrome flag: every role except Mono is chrome.
        assert!(TypoRole::Heading.is_chrome());
        assert!(TypoRole::Body.is_chrome());
        assert!(TypoRole::Small.is_chrome());
        assert!(TypoRole::HomeTitle.is_chrome());
        assert!(TypoRole::HomeDesc.is_chrome());
        assert!(TypoRole::HomeButton.is_chrome());
        assert!(!TypoRole::Mono.is_chrome());
        // Full ladder: Small < Mono < Body < HomeTitle < Heading.
        assert!(TypoRole::Small.size() < TypoRole::Mono.size());
        assert!(TypoRole::Mono.size() < TypoRole::Body.size());
        assert!(TypoRole::Body.size() < TypoRole::HomeTitle.size());
        assert!(TypoRole::HomeTitle.size() < TypoRole::Heading.size());
        // Line-height px follows size * multiplier per role family.
        assert!(
            TypoRole::chrome_section().line_height_px() > TypoRole::chrome_tab().line_height_px()
        );
        assert!(
            TypoRole::chrome_tab().line_height_px() > TypoRole::chrome_status().line_height_px()
        );
    }

    #[test]
    fn content_grid_and_caption_aliases_carry_ramp() {
        // Aliases reuse existing roles: no new point sizes this wave.
        assert_eq!(TypoRole::content_grid(), TypoRole::Mono);
        assert_eq!(TypoRole::caption(), TypoRole::Small);
        // Grid: fixed mono 12, tabular, regular-mono, mono line-height.
        assert_eq!(TypoRole::content_grid().size(), TYPO_MONO_SIZE);
        assert_eq!(TypoRole::content_grid().size(), 12.0);
        assert!(TypoRole::content_grid().is_mono());
        assert!(!TypoRole::content_grid().is_chrome());
        assert!(TypoRole::content_grid().is_tabular());
        assert_eq!(TypoRole::content_grid().weight(), "regular-mono");
        assert_eq!(TypoRole::content_grid().family(), "monospace");
        assert_eq!(
            TypoRole::content_grid().line_height(),
            TYPO_LINE_HEIGHT_MONO
        );
        assert_eq!(
            TypoRole::content_grid().font_id(),
            egui::FontId::monospace(12.0)
        );
        // Hint/caption: small weak proportional, tabular counts, chrome LH.
        assert_eq!(TypoRole::caption().size(), TYPO_SMALL_SIZE);
        assert_eq!(TypoRole::caption().size(), 11.0);
        assert!(TypoRole::caption().is_small());
        assert!(TypoRole::caption().is_weak());
        assert!(TypoRole::caption().is_chrome());
        assert!(TypoRole::caption().is_tabular());
        assert_eq!(TypoRole::caption().weight(), "weak");
        assert_eq!(TypoRole::caption().family(), "proportional");
        assert_eq!(TypoRole::caption().line_height(), TYPO_LINE_HEIGHT_CHROME);
        // Single-source delegation through spec().
        assert_eq!(
            TypoRole::content_grid().spec().size,
            TypoRole::Mono.spec().size
        );
        assert_eq!(TypoRole::caption().spec().size, TypoRole::Small.spec().size);
        assert_eq!(
            TypoRole::content_grid().spec().font_id,
            TypoRole::Mono.font_id()
        );
        assert_eq!(
            TypoRole::caption().spec().font_id,
            TypoRole::Small.font_id()
        );
        // Ramp order: hint 11 < grid 12 < tab 13 < card-title 14 < section 18.
        assert!(TypoRole::caption().size() < TypoRole::content_grid().size());
        assert!(TypoRole::content_grid().size() < TypoRole::chrome_tab().size());
        assert!(TypoRole::chrome_tab().size() < TypoRole::HomeTitle.size());
        assert!(TypoRole::HomeTitle.size() < TypoRole::chrome_section().size());
        const {
            assert!(TYPO_SMALL_SIZE < TYPO_MONO_SIZE);
            assert!(TYPO_MONO_SIZE < TYPO_BODY_SIZE);
        }
    }

    #[test]
    fn gutter_text_alias_matches_grid_mono() {
        // Central gutter pole: same value as the grid alias, fixed mono 12.
        assert_eq!(TypoRole::gutter_text(), TypoRole::Mono);
        assert_eq!(TypoRole::gutter_text(), TypoRole::content_grid());
        assert_eq!(TypoRole::gutter_text().size(), TYPO_MONO_SIZE);
        assert_eq!(TypoRole::gutter_text().size(), 12.0);
        assert!(TypoRole::gutter_text().is_mono());
        assert!(!TypoRole::gutter_text().is_chrome());
        assert!(TypoRole::gutter_text().is_tabular());
        assert_eq!(TypoRole::gutter_text().weight(), "regular-mono");
        assert_eq!(TypoRole::gutter_text().family(), "monospace");
        assert_eq!(TypoRole::gutter_text().line_height(), TYPO_LINE_HEIGHT_MONO);
        assert_eq!(
            TypoRole::gutter_text().font_id(),
            egui::FontId::monospace(12.0)
        );
        // Single-source delegation through spec().
        assert_eq!(
            TypoRole::gutter_text().spec().size,
            TypoRole::Mono.spec().size
        );
        assert_eq!(
            TypoRole::gutter_text().spec().font_id,
            TypoRole::Mono.font_id()
        );
        // BC ladder holds with the gutter pole: hint 11 < gutter/grid 12.
        assert!(TypoRole::caption().size() < TypoRole::gutter_text().size());
        assert_eq!(
            TypoRole::gutter_text().size(),
            TypoRole::content_grid().size()
        );
    }

    #[test]
    fn typo_font_ids_carry_exact_px_and_family() {
        // Mono diff panes resolve monospace; all chrome resolves proportional.
        assert_eq!(
            TypoRole::Mono.font_id(),
            egui::FontId::monospace(TYPO_MONO_SIZE)
        );
        assert_eq!(
            TypoRole::Body.font_id(),
            egui::FontId::proportional(TYPO_BODY_SIZE)
        );
        assert_eq!(
            TypoRole::Small.font_id(),
            egui::FontId::proportional(TYPO_SMALL_SIZE)
        );
        assert_eq!(
            TypoRole::Heading.font_id(),
            egui::FontId::proportional(TYPO_HEADING_SIZE)
        );
        assert_eq!(
            TypoRole::HomeTitle.font_id(),
            egui::FontId::proportional(TYPO_HOME_TITLE_SIZE)
        );
        assert_eq!(
            TypoRole::HomeDesc.font_id(),
            egui::FontId::proportional(TYPO_SMALL_SIZE)
        );
        assert_eq!(
            TypoRole::HomeButton.font_id(),
            egui::FontId::proportional(TYPO_BODY_SIZE)
        );
        // font_family matches is_mono + the family()/font_stack() twins.
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            assert_eq!(role.font_id().size, role.size());
            if role.is_mono() {
                assert_eq!(role.font_family(), egui::FontFamily::Monospace);
                assert_eq!(role.family(), "monospace");
                assert_eq!(role.font_stack(), TYPO_MONO_STACK);
            } else {
                assert_eq!(role.font_family(), egui::FontFamily::Proportional);
                assert_eq!(role.family(), "proportional");
                assert_eq!(role.font_stack(), TYPO_UI_SANS_STACK);
            }
        }
    }

    #[test]
    fn typo_spec_bundles_each_role_single_source() {
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            let spec = role.spec();
            assert_eq!(spec.role, role);
            assert_eq!(spec.size, role.size());
            assert_eq!(spec.font_id, role.font_id());
            assert_eq!(spec.font_id.size, role.size());
            assert_eq!(spec.weight, role.weight());
            assert_eq!(spec.weight_value, role.weight_value());
            assert_eq!(spec.line_height, role.line_height());
            assert_eq!(spec.line_height_px, role.line_height_px());
            assert_eq!(spec.letter_spacing_em, role.letter_spacing_em());
            assert_eq!(spec.is_tabular, role.is_tabular());
        }
        // Exact mapping table, single-sourced (px / family / weight spelling / numeric).
        assert_eq!(
            TypoRole::Heading.spec().font_id,
            egui::FontId::proportional(18.0)
        );
        assert_eq!(TypoRole::Heading.spec().weight, "strong");
        assert_eq!(TypoRole::Heading.spec().weight_value, 500);
        assert_eq!(
            TypoRole::Body.spec().font_id,
            egui::FontId::proportional(13.0)
        );
        assert_eq!(TypoRole::Body.spec().weight, "default");
        assert_eq!(TypoRole::Body.spec().weight_value, 400);
        assert_eq!(
            TypoRole::Small.spec().font_id,
            egui::FontId::proportional(11.0)
        );
        assert_eq!(TypoRole::Small.spec().weight, "weak");
        assert_eq!(TypoRole::Small.spec().weight_value, 600);
        assert_eq!(TypoRole::Mono.spec().font_id, egui::FontId::monospace(12.0));
        assert_eq!(TypoRole::Mono.spec().weight, "regular-mono");
        assert_eq!(TypoRole::Mono.spec().weight_value, 400);
        assert_eq!(
            TypoRole::HomeTitle.spec().font_id,
            egui::FontId::proportional(14.0)
        );
        // Ladder intact through the bundle: Small < Mono < Body < HomeTitle < Heading.
        assert!(TypoRole::Small.spec().size < TypoRole::Mono.spec().size);
        assert!(TypoRole::Mono.spec().size < TypoRole::Body.spec().size);
        assert!(TypoRole::Body.spec().size < TypoRole::HomeTitle.spec().size);
        assert!(TypoRole::HomeTitle.spec().size < TypoRole::Heading.spec().size);
    }

    #[test]
    fn typo_font_id_zoomed_follows_zoom_table() {
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            // Level 0 is the base passthrough.
            assert_eq!(role.font_id_zoomed(0), role.font_id());
            assert_eq!(role.font_id_zoomed(ZOOM_DEFAULT_LEVEL), role.font_id());
            // ±1px steps keep the family; extreme input clamps to the edge.
            assert_eq!(role.font_id_zoomed(1).size, role.size() + 1.0);
            assert_eq!(role.font_id_zoomed(-1).size, role.size() - 1.0);
            assert_eq!(role.font_id_zoomed(1).family, role.font_family());
            assert_eq!(
                role.font_id_zoomed(100),
                role.font_id_zoomed(ZOOM_MAX_LEVEL)
            );
            assert_eq!(
                role.font_id_zoomed(-100),
                role.font_id_zoomed(ZOOM_MIN_LEVEL)
            );
            for level in ZOOM_LEVELS {
                let resolved = role.font_id_zoomed(level);
                assert_eq!(resolved.family, role.font_family());
                assert_eq!(resolved.size, zoomed_size(role, level));
                assert!(resolved.size >= ZOOM_MIN_SIZE);
                assert!(resolved.size <= ZOOM_MAX_SIZE);
            }
        }
        // Mono zoom keeps mono family (code text zoom hota hai, chrome se alag rehta hai).
        assert_eq!(
            TypoRole::Mono.font_id_zoomed(2),
            egui::FontId::monospace(TYPO_MONO_SIZE + 2.0)
        );
    }

    #[test]
    fn typo_resolved_font_id_uses_role_text_style() {
        let style = egui::Style::default();
        for role in [
            TypoRole::Heading,
            TypoRole::Body,
            TypoRole::Small,
            TypoRole::Mono,
            TypoRole::HomeTitle,
            TypoRole::HomeDesc,
            TypoRole::HomeButton,
        ] {
            // Delegation lock: helper equals text_style().resolve(style).
            assert_eq!(
                role.resolved_font_id(&style),
                role.text_style().resolve(&style)
            );
        }
        // Family contract against the default style: mono diff vs proportional chrome.
        assert_eq!(
            TypoRole::Mono.resolved_font_id(&style).family,
            egui::FontFamily::Monospace
        );
        assert_eq!(
            TypoRole::Body.resolved_font_id(&style).family,
            egui::FontFamily::Proportional
        );
        assert_eq!(
            TypoRole::Small.resolved_font_id(&style).family,
            egui::FontFamily::Proportional
        );
        assert_eq!(
            TypoRole::Heading.resolved_font_id(&style).family,
            egui::FontFamily::Proportional
        );
    }

    #[test]
    fn viewport_tokens_match_canvas_and_fallback() {
        assert_eq!(VIEWPORT_W, 960);
        assert_eq!(VIEWPORT_H, 600);
        assert_eq!(VIEWPORT_MIN_W, 720);
        assert_eq!(VIEWPORT_MIN_H, 450);
        assert_eq!(DUAL_PANE_CHROME_W, 40);
        const {
            assert!(VIEWPORT_MIN_W < VIEWPORT_W);
            assert!(VIEWPORT_MIN_H < VIEWPORT_H);
        }
    }

    #[test]
    fn typo_rank_follows_point_size() {
        assert!(TypoRole::Heading.rank() < TypoRole::HomeTitle.rank());
        assert!(TypoRole::HomeTitle.rank() < TypoRole::Body.rank());
        assert_eq!(TypoRole::Body.rank(), TypoRole::HomeButton.rank());
        assert!(TypoRole::Body.rank() < TypoRole::Mono.rank());
        assert!(TypoRole::Mono.rank() < TypoRole::Small.rank());
        assert_eq!(TypoRole::Small.rank(), TypoRole::HomeDesc.rank());
        // Rank steps down as point size steps down.
        assert!(TypoRole::Heading.size() > TypoRole::HomeTitle.size());
        assert!(TypoRole::HomeTitle.size() > TypoRole::Body.size());
        assert!(TypoRole::Body.size() > TypoRole::Mono.size());
        assert!(TypoRole::Mono.size() > TypoRole::Small.size());
    }

    #[test]
    fn contrast_dim_text_carries_weight_not_hue() {
        assert_eq!(contrast_dim_text_role(), TypoRole::Small);
        assert_eq!(contrast_dim_min_size(), 11.0);
        assert_eq!(contrast_dim_min_size(), TYPO_SMALL_SIZE);
        assert_eq!(contrast_dim_min_weight(), 600);
        assert_eq!(contrast_dim_min_weight(), TYPO_WEIGHT_SEMIBOLD);
        assert_eq!(contrast_gutter_text_role(), TypoRole::Mono);
        assert!(contrast_dim_text_ok());
    }

    #[test]
    fn test_wcag_aa_design_token_contrast_all_palettes() {
        assert!(theme_all_palettes_satisfy_aa());
        assert!(palette_all_text_tokens_aa(&DARK_PALETTE));
        assert!(palette_all_text_tokens_aa(&LIGHT_PALETTE));
        assert!(theme_contrast_ok_color32(
            Color32::from_rgb(
                DARK_FG_RGB.first().copied().unwrap_or(0),
                DARK_FG_RGB.get(1).copied().unwrap_or(0),
                DARK_FG_RGB.get(2).copied().unwrap_or(0),
            ),
            Color32::from_rgb(
                DARK_BG_RGB.first().copied().unwrap_or(0),
                DARK_BG_RGB.get(1).copied().unwrap_or(0),
                DARK_BG_RGB.get(2).copied().unwrap_or(0),
            ),
        ));
        assert!(theme_contrast_ok_color32(
            Color32::from_rgb(
                LIGHT_FG_RGB.first().copied().unwrap_or(0),
                LIGHT_FG_RGB.get(1).copied().unwrap_or(0),
                LIGHT_FG_RGB.get(2).copied().unwrap_or(0),
            ),
            Color32::from_rgb(
                LIGHT_BG_RGB.first().copied().unwrap_or(0),
                LIGHT_BG_RGB.get(1).copied().unwrap_or(0),
                LIGHT_BG_RGB.get(2).copied().unwrap_or(0),
            ),
        ));
    }

    #[test]
    fn test_keymap_preset_parsing_and_names() {
        for preset in KeymapPreset::all() {
            assert_eq!(KeymapPreset::from_name(preset.as_str()), Some(*preset));
        }
        assert_eq!(KeymapPreset::from_name("vi"), Some(KeymapPreset::Vim));
        assert_eq!(KeymapPreset::from_name("code"), Some(KeymapPreset::Vscode));
        assert_eq!(
            KeymapPreset::from_name("standard"),
            Some(KeymapPreset::Default)
        );
        assert_eq!(
            KeymapPreset::from_name("macos"),
            Some(KeymapPreset::Default)
        );
        assert_eq!(KeymapPreset::from_name("unknown"), None);
    }

    #[test]
    fn test_keymap_core_mac_bindings_cmd_f_i_0() {
        for preset in KeymapPreset::all() {
            assert!(
                keymap_has_core_mac_bindings(*preset),
                "Preset {preset:?} must bind Cmd-F, Cmd-I, Cmd-0"
            );
        }
    }

    #[test]
    fn test_keymap_resolve_shortcuts_across_presets() {
        let cmd = egui::Modifiers {
            command: true,
            ..Default::default()
        };
        let plain = egui::Modifiers::default();
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };

        // Cmd-F, Cmd-I, Cmd-0 resolve on all presets
        for preset in KeymapPreset::all() {
            assert_eq!(
                keymap_resolve(*preset, egui::Key::F, &cmd),
                Some(KeyAction::Find)
            );
            assert_eq!(
                keymap_resolve(*preset, egui::Key::I, &cmd),
                Some(KeyAction::ToggleInfo)
            );
            assert_eq!(
                keymap_resolve(*preset, egui::Key::Num0, &cmd),
                Some(KeyAction::ResetZoom)
            );
        }

        // Vim specific '/' resolves to Find
        assert_eq!(
            keymap_resolve(KeymapPreset::Vim, egui::Key::Slash, &plain),
            Some(KeyAction::Find)
        );

        // F3 resolves to NextDiff, Shift-F3 to PrevDiff
        assert_eq!(
            keymap_resolve(KeymapPreset::Default, egui::Key::F3, &plain),
            Some(KeyAction::NextDiff)
        );
        assert_eq!(
            keymap_resolve(KeymapPreset::Default, egui::Key::F3, &shift),
            Some(KeyAction::PrevDiff)
        );
    }

    #[test]
    fn test_keymap_shortcut_hints() {
        for preset in KeymapPreset::all() {
            assert!(keymap_shortcut_hint(*preset, KeyAction::Find).contains("⌘F"));
            assert!(keymap_shortcut_hint(*preset, KeyAction::ToggleInfo).contains("⌘I"));
            assert_eq!(keymap_shortcut_hint(*preset, KeyAction::ResetZoom), "⌘0");
        }
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn typo_exq_home_scale_has_visible_steps_and_tracking() {
        // Edge: Home3 exquisite scale — visible size steps + non-zero header
        // tracking, paint no-op untouched. Koi naya size nahi, colors frozen.
        assert!(typo_exq_home_scale_ok());
        assert_eq!(TYPO_HEADING_SIZE, 18.0);
        assert_eq!(TYPO_HOME_TITLE_SIZE, 14.0);
        assert_eq!(TYPO_BODY_SIZE, 13.0);
        assert_eq!(TYPO_MONO_SIZE, 12.0);
        assert_eq!(TYPO_SMALL_SIZE, 11.0);
        assert!(TYPO_TRACKING_HEADING_EM > 0.0);
        assert_eq!(TYPO_LETTER_SPACING, 0.0);
        assert_eq!(TypoRole::HomeTitle.letter_spacing_em(), 0.02);
        assert_eq!(TypoRole::HomeButton.letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::HomeDesc.letter_spacing_em(), 0.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn typo_textcenter_steps_carry_hierarchy_without_new_sizes() {
        // TEXT CENTER: diff-body Mono 12, chrome Body 13, hints Small 11.
        // TypoRole + TYPO_* reuse only; koi naya size, koi color move nahi.
        assert!(typo_textcenter_steps_ok());
        assert_eq!(TypoRole::content_textcenter(), TypoRole::Mono);
        assert_eq!(TypoRole::content_textcenter(), TypoRole::content_grid());
        assert_eq!(TypoRole::content_textcenter(), TypoRole::gutter_text());
        assert_eq!(TypoRole::content_textcenter().size(), TYPO_MONO_SIZE);
        assert_eq!(TypoRole::Body.size(), TYPO_BODY_SIZE);
        assert_eq!(TypoRole::Small.size(), TYPO_SMALL_SIZE);
        // Weights: 400 body/mono, 600 small-counts, 500 headers.
        assert_eq!(
            TypoRole::content_textcenter().weight_value(),
            TYPO_WEIGHT_REGULAR
        );
        assert_eq!(TypoRole::Body.weight_value(), TYPO_WEIGHT_REGULAR);
        assert_eq!(TypoRole::Small.weight_value(), TYPO_WEIGHT_SEMIBOLD);
        assert_eq!(TypoRole::Heading.weight_value(), TYPO_WEIGHT_MEDIUM);
        // Line-height px order: body 18.2 > mono 15.6 > small 13.2.
        assert_eq!(
            TypoRole::Body.line_height_px(),
            13.0 * TYPO_LINE_HEIGHT_BODY
        );
        assert_eq!(
            TypoRole::content_textcenter().line_height_px(),
            12.0 * TYPO_LINE_HEIGHT_MONO
        );
        assert_eq!(
            TypoRole::Small.line_height_px(),
            11.0 * TYPO_LINE_HEIGHT_CHROME
        );
        // Tracking: trio 0.0, sirf headers 0.02em.
        assert_eq!(TypoRole::content_textcenter().letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::Body.letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::Small.letter_spacing_em(), 0.0);
        assert_eq!(
            TypoRole::Heading.letter_spacing_em(),
            TYPO_TRACKING_HEADING_EM
        );
        // Tabular: mono + small stable, body kabhi nahi.
        assert!(TypoRole::content_textcenter().is_tabular());
        assert!(TypoRole::Small.is_tabular());
        assert!(!TypoRole::Body.is_tabular());
        // TypoSpec delegation: har field accessor se match.
        let spec = TypoRole::content_textcenter().spec();
        assert_eq!(spec.size, TYPO_MONO_SIZE);
        assert_eq!(spec.weight_value, TYPO_WEIGHT_REGULAR);
        assert_eq!(spec.line_height, TYPO_LINE_HEIGHT_MONO);
        assert_eq!(spec.line_height_px, 12.0 * TYPO_LINE_HEIGHT_MONO);
        assert!(spec.is_tabular);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn textcenter_geometry_tokens_reuse_scale() {
        // 48 = header-bar 28 + gutter-bar 12 + 2 x tight-gap 4.
        // Underline 1px visual precedent. Koi naya literal/color nahi.
        assert_eq!(TEXT_CENTER_GUTTER_W, 48.0);
        assert_eq!(
            TEXT_CENTER_GUTTER_W,
            CHROME_HEADER_BAR_H + GUTTER_BAR_WIDE + GEO_TIGHT_GAP + GEO_TIGHT_GAP
        );
        assert_eq!(TEXT_INLINE_SPAN_UNDERLINE_W, 1.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn typo_home4_steps_carry_hierarchy_without_new_sizes() {
        // HOME4: family/weight/line-height/tracking steps, TypoRole reuse only.
        assert!(typo_home4_steps_ok());
        assert_eq!(TypoRole::HomeTitle.size(), TYPO_HOME_TITLE_SIZE);
        assert_eq!(TypoRole::HomeButton.size(), TYPO_BODY_SIZE);
        assert_eq!(TypoRole::HomeDesc.size(), TYPO_SMALL_SIZE);
        assert_eq!(TypoRole::HomeTitle.weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(TypoRole::HomeDesc.weight_value(), TYPO_WEIGHT_REGULAR);
        assert!(TypoRole::HomeTitle.line_height_px() > TypoRole::HomeButton.line_height_px());
        assert!(TypoRole::HomeButton.line_height_px() > TypoRole::HomeDesc.line_height_px());
        assert_eq!(TypoRole::HomeTitle.family(), "proportional");
        assert_eq!(TypoRole::HomeButton.family(), "proportional");
        assert_eq!(TypoRole::HomeDesc.family(), "proportional");
        assert!(typo_exq_home_scale_ok());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn typo8_roles_plus_rhythm_reuses_scale_without_new_sizes() {
        // TYPO8: gaps reuse GEO_* scale only, har gap ek TypoRole se bandha,
        // koi naya point size nahi, koi color move nahi. Paint-neutral.
        assert!(typo_rhythm_ok());
        assert!(typo_roles_plus_rhythm_ok());
        assert_eq!(TYPO_PARA_GAP_SMALL, GEO_TIGHT_GAP);
        assert_eq!(TYPO_PARA_GAP_SMALL, SPACE_XS);
        assert_eq!(TYPO_PARA_GAP_BODY, GEO_SECTION_GAP);
        assert_eq!(TYPO_PARA_GAP_BODY, SPACE_SM);
        assert_eq!(TYPO_SECTION_GAP_CHROME, GEO_OUTER_MARGIN);
        assert_eq!(TYPO_SECTION_GAP_CHROME, SPACE_MD);
        assert_eq!(TYPO_ROW_GAP_MONO, GEO_TIGHT_GAP);
        // Role -> gap mapping: small hug, body/mono breathe, headers separate.
        assert_eq!(TypoRole::Small.para_gap(), TYPO_PARA_GAP_SMALL);
        assert_eq!(TypoRole::HomeDesc.para_gap(), TYPO_PARA_GAP_SMALL);
        assert_eq!(TypoRole::Body.para_gap(), TYPO_PARA_GAP_BODY);
        assert_eq!(TypoRole::Mono.para_gap(), TYPO_PARA_GAP_BODY);
        assert_eq!(TypoRole::HomeButton.para_gap(), TYPO_PARA_GAP_BODY);
        assert_eq!(TypoRole::Heading.para_gap(), TYPO_SECTION_GAP_CHROME);
        assert_eq!(TypoRole::HomeTitle.para_gap(), TYPO_SECTION_GAP_CHROME);
        // Sizes untouched: rhythm gaps change no TypoRole point size.
        assert_eq!(TypoRole::Body.size(), TYPO_BODY_SIZE);
        assert_eq!(TypoRole::Mono.size(), TYPO_MONO_SIZE);
        assert_eq!(TypoRole::Small.size(), TYPO_SMALL_SIZE);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn theme10_pane_hierarchy_sharp_without_new_sizes() {
        // THEME10: pane headers vs body vs meta — geometry FIRST (gaps reuse
        // GEO_*/CHROME_* scale only), typography SECOND (TypoRole reuse,
        // koi naya point size nahi), colors FROZEN. Paint-neutral.
        assert!(theme10_pane_hierarchy_ok());
        // Geometry: meta 4 < body 8 < header-gap 12, header bar 28 rhythm.
        assert_eq!(THEME10_PANE_META_GAP, GEO_TIGHT_GAP);
        assert_eq!(THEME10_PANE_META_GAP, SPACE_XS);
        assert_eq!(THEME10_PANE_BODY_GAP, GEO_SECTION_GAP);
        assert_eq!(THEME10_PANE_BODY_GAP, SPACE_SM);
        assert_eq!(THEME10_PANE_HEADER_GAP, GEO_OUTER_MARGIN);
        assert_eq!(THEME10_PANE_HEADER_GAP, SPACE_MD);
        assert_eq!(THEME10_PANE_HEADER_H, CHROME_HEADER_BAR_H);
        assert_eq!(THEME10_PANE_HEADER_H, 28.0);
        assert_eq!(THEME10_PANE_PAD_X, GEO_OUTER_MARGIN);
        // Typography: header Heading 18 > body Mono 12 > meta Small 11.
        assert_eq!(TypoRole::pane_header(), TypoRole::Heading);
        assert_eq!(TypoRole::pane_header(), TypoRole::chrome_section());
        assert_eq!(TypoRole::pane_body(), TypoRole::Mono);
        assert_eq!(TypoRole::pane_body(), TypoRole::content_textcenter());
        assert_eq!(TypoRole::pane_meta(), TypoRole::Small);
        assert_eq!(TypoRole::pane_meta(), TypoRole::caption());
        assert_eq!(TypoRole::pane_header().size(), TYPO_HEADING_SIZE);
        assert_eq!(TypoRole::pane_body().size(), TYPO_MONO_SIZE);
        assert_eq!(TypoRole::pane_meta().size(), TYPO_SMALL_SIZE);
        // Weight + family + tracking carry hierarchy (no new sizes).
        assert_eq!(TypoRole::pane_header().weight(), "strong");
        assert_eq!(TypoRole::pane_body().weight(), "regular-mono");
        assert_eq!(TypoRole::pane_meta().weight(), "weak");
        assert_eq!(TypoRole::pane_header().family(), "proportional");
        assert_eq!(TypoRole::pane_body().family(), "monospace");
        assert_eq!(TypoRole::pane_meta().family(), "proportional");
        assert!(TypoRole::pane_header().letter_spacing_em() > 0.0);
        assert_eq!(TypoRole::pane_body().letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::pane_meta().letter_spacing_em(), 0.0);
        // Tabular: body + meta on, header off; chrome: body content only.
        assert!(!TypoRole::pane_header().is_tabular());
        assert!(TypoRole::pane_body().is_tabular());
        assert!(TypoRole::pane_meta().is_tabular());
        assert!(TypoRole::pane_header().is_chrome());
        assert!(!TypoRole::pane_body().is_chrome());
        assert!(TypoRole::pane_meta().is_chrome());
        // Narrow edge (720px fallback): hierarchy width-independent rehti
        // hai — gaps plus size order same, koi flat single-size rhythm nahi.
        assert!(TypoRole::pane_header().size() > TypoRole::pane_body().size());
        assert!(TypoRole::pane_body().size() > TypoRole::pane_meta().size());
        assert!(THEME10_PANE_META_GAP < THEME10_PANE_BODY_GAP);
        assert!(THEME10_PANE_BODY_GAP < THEME10_PANE_HEADER_GAP);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn exq_typo10_empty_vs_nonempty_rhythm_and_density_edge() {
        assert!(exq_typo10_roles_plus_rhythm_ok());

        // Line-height monotonic ladder: small 13.2 < mono 15.6 < body 18.2 < heading 21.6 px
        assert_eq!(
            TypoRole::Small.line_height_px(),
            11.0 * TYPO_LINE_HEIGHT_CHROME
        );
        assert_eq!(
            TypoRole::Mono.line_height_px(),
            12.0 * TYPO_LINE_HEIGHT_MONO
        );
        assert_eq!(
            TypoRole::Body.line_height_px(),
            13.0 * TYPO_LINE_HEIGHT_BODY
        );
        assert_eq!(
            TypoRole::Heading.line_height_px(),
            18.0 * TYPO_LINE_HEIGHT_CHROME
        );

        // Tracking: heading only
        assert_eq!(
            TypoRole::Heading.letter_spacing_em(),
            TYPO_TRACKING_HEADING_EM
        );
        assert_eq!(TypoRole::Mono.letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::Body.letter_spacing_em(), 0.0);
        assert_eq!(TypoRole::Small.letter_spacing_em(), 0.0);

        // Monospace consistency
        assert_eq!(TypoRole::Mono.size(), TYPO_MONO_SIZE);
        assert!(TypoRole::Mono.is_mono());
        assert!(!TypoRole::Mono.is_chrome());
        assert!(TypoRole::Mono.is_tabular());

        // Edge comparison empty vs nonempty text models
        let empty_cmp = autocompare_compare::TextComparison {
            left_lines: Vec::new(),
            right_lines: Vec::new(),
            script: autocompare_diff::DiffScript {
                ops: Vec::new(),
                exact: true,
            },
        };
        let empty_rows = crate::viewmodel::build_text_rows(&empty_cmp);
        assert_eq!(empty_rows.rows.len(), 0);
        assert_eq!(empty_rows.gutter_chars, 4);

        let nonempty_cmp = autocompare_compare::TextComparison {
            left_lines: vec!["alpha".to_string(), "beta".to_string()],
            right_lines: vec!["alpha".to_string(), "gamma".to_string()],
            script: autocompare_diff::DiffScript {
                ops: vec![
                    autocompare_diff::Op::Equal { len: 1 },
                    autocompare_diff::Op::Delete { len: 1 },
                    autocompare_diff::Op::Insert { len: 1 },
                ],
                exact: true,
            },
        };
        let nonempty_rows = crate::viewmodel::build_text_rows(&nonempty_cmp);
        assert_eq!(nonempty_rows.rows.len(), 2);
        assert_eq!(nonempty_rows.gutter_chars, 4);

        // Density verification: zero vertical clipping in both
        let audit = crate::viewmodel::audit_viewmodel_density();
        assert!(audit.zero_clipping_verified);
    }
}
