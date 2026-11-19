//! Color contrast calculations according to WCAG 2.1 specifications.
//!
//! Provides pure headless contrast checks for dark and light theme palettes.

#![forbid(unsafe_code)]
#![allow(dead_code)]

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
}
