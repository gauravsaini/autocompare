//! Color contrast calculations according to WCAG 2.1 specifications.
//!
//! Provides pure headless contrast checks for dark and light theme palettes.
//!
//! Typography scale (family, weight, line-height only; colors out of
//! scope): heading 18 strong, body 13, small 11 weak, mono diff 12
//! (13 is the tolerated upper bound), Home card title strong 14.
//! Line-height is always `text_style_height(Monospace)` at paint time
//! (5 call sites in `views.rs`); egui has no letter-tracking so
//! letter-spacing is a documented no-op ([`TYPO_LETTER_SPACING`]) with
//! weight + line-height carrying hierarchy. No `Color32`/`Visuals`
//! touches here.

#![forbid(unsafe_code)]
#![allow(dead_code)]

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
        "ui.text_style_height(Monospace)"
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
        theme_contrast_ok([230, 230, 230], [27, 27, 27])
    } else {
        // Light theme: foreground text [20, 20, 20] on background [248, 248, 248]
        theme_contrast_ok([20, 20, 20], [248, 248, 248])
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
        assert!(TYPO_MONO_SIZE <= TYPO_MONO_SIZE_MAX);
        assert!(TYPO_SMALL_SIZE < TYPO_BODY_SIZE);
        assert!(TYPO_BODY_SIZE < TYPO_HOME_TITLE_SIZE);
        assert!(TYPO_HOME_TITLE_SIZE < TYPO_HEADING_SIZE);
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
        assert_eq!(
            TypoRole::Mono.line_height_source(),
            "ui.text_style_height(Monospace)"
        );
        assert_eq!(
            TypoRole::Heading.line_height_source(),
            "ui.text_style_height(Monospace)"
        );
    }
}
