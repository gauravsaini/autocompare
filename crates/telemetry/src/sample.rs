//! Head-based sampling with kill-switch and error bypass.

/// Decides whether a fresh root span is kept.
#[derive(Debug, Clone)]
pub struct Sampling {
    /// Keep probability in [0.0, 1.0] for non-error spans.
    pub rate: f64,
    /// Ops that bypass sampling and are always kept.
    pub always: Vec<String>,
}

impl Sampling {
    pub fn new(rate: f64) -> Self {
        Self { rate: rate.clamp(0.0, 1.0), always: Vec::new() }
    }

    /// Global kill-switch: `AUTOCOMPARE_TELEMETRY=off` disables everything.
    pub fn enabled() -> bool {
        std::env::var("AUTOCOMPARE_TELEMETRY")
            .map(|v| !v.eq_ignore_ascii_case("off"))
            .unwrap_or(true)
    }

    /// Error spans are always kept; allowlisted ops are always kept.
    pub fn keep(&self, op: &str, is_error: bool, dice: f64) -> bool {
        if is_error {
            return true;
        }
        if self.always.iter().any(|a| a == op) {
            return true;
        }
        dice < self.rate
    }
}
