//! Head-based sampling with kill-switch and error bypass.
//!
//! ODD §11 budget map (see `docs/OBSERVABILITY_DRIVEN_DEVELOPMENT.md` §11;
//! Phase 12 review — no default below violates a budget, so this change is
//! documentation-only with two additive named rate aliases):
//!
//! | Budget (§11) | Default that satisfies it |
//! |---|-------------------------------------------------------------|
//! | p99 added latency <3% on 10k-line diff | emit path ~µs (see `benches/emit_sink.rs`); sampling keeps the hot path allocation-free for dropped spans |
//! | UI-thread tracing <1ms/op | `Span::begin`/`finish` do no I/O; sink writes are fire-and-forget via [`crate::sink`] |
//! | Sink memory cap 8MB ring | [`crate::sink::InMemorySink`] capacity is caller-sized; TS mirror caps at 1024 records (`SINK_BUFFER_LIMIT`, ≈1–2MB worst case) |
//! | JSONL volume <2MB per 1k ops (dev) | ~2KB/op ceiling; span records are shape-only summaries, never contents |
//! | Errors always kept | [`Sampling::keep`] returns `true` for errors at any rate |
//! | Kill-switch | [`Sampling::enabled`]: `AUTOCOMPARE_TELEMETRY=off` disables everything |
//!
//! Sampling profile (§7): local-dev keeps 100% ([`DEV_SAMPLE_RATE`]);
//! prod-like keeps 5% ([`PROD_SAMPLE_RATE`], inside the §2 "1–5% prod-like"
//! band) with a `sample_always` op allowlist carried in [`Sampling::always`].

/// Local-dev keep probability: 100% (§7 default-dev rule).
///
/// Additive alias for documentation — behaviour unchanged.
pub const DEV_SAMPLE_RATE: f64 = 1.0;

/// Prod-like keep probability: 5% (§7 prod-like target, §2 "1–5% prod-like").
///
/// Additive alias for documentation — behaviour unchanged.
pub const PROD_SAMPLE_RATE: f64 = 0.05;

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
