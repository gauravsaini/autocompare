//! ODD telemetry/v1 primitives (Phase 2).
//!
//! Wire format is snake_case everywhere. File contents, tokens and raw
//! paths must never be emitted — see [`redact`].

#![forbid(unsafe_code)]

pub mod context;
pub mod emit;
pub mod redact;
pub mod sample;
pub mod sink;

pub use context::TraceContext;
pub use sample::Sampling;
pub use sink::{InMemorySink, JsonlFileSink, Sink};
