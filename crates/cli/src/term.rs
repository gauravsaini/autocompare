//! Terminal helpers: colors and time formatting.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::ColorWhen;

pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const BLUE: &str = "\x1b[34m";
pub const YELLOW: &str = "\x1b[33m";

/// Accent for changed characters inside a highlighted line.
pub const ACCENT: &str = "\x1b[1;4m";
/// Ends the accent without touching a wrapping background color.
pub const ACCENT_OFF: &str = "\x1b[22;24m";

/// Backgrounds for whole lines.
pub const BG_RED: &str = "\x1b[41m";
pub const BG_GREEN: &str = "\x1b[42m";
pub const BG_YELLOW: &str = "\x1b[43m";

pub fn color_enabled(when: ColorWhen) -> bool {
    use std::io::IsTerminal;
    match when {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => std::io::stdout().is_terminal(),
    }
}

/// Wraps `text` in `codes` when color is on; otherwise plain text.
pub fn paint(color: bool, codes: &str, text: &str) -> String {
    if color && !codes.is_empty() {
        format!("{codes}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Formats a timestamp as "2026-11-18 09:41" (UTC), or "-" for times
/// before the Unix epoch.
pub fn format_time(t: SystemTime) -> String {
    let secs = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => return "-".to_string(),
    };
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let (y, m, d) = civil_from_days(cast_days(days));
    let hh = tod / 3_600;
    let mm = (tod % 3_600) / 60;
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

/// `u64` days since the epoch fit comfortably in `i64`; this keeps the
/// civil-date math total without any overflow risk.
fn cast_days(days: u64) -> i64 {
    if days <= i64::MAX as u64 {
        days as i64
    } else {
        i64::MAX
    }
}

/// Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
