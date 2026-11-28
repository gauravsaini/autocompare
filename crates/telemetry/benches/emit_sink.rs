//! ODD Phase 12 — offline micro-benchmarks for the span emit + sink write path.
//!
//! Scope (§11 budgets): `Span::begin` / `set_attr` / `finish` (emit path) and
//! the two local sink write paths (`InMemorySink::emit`, `JsonlFileSink::emit`).
//! These benches inform the §11 budgets (UI-thread tracing <1ms/op; p99 added
//! latency <3% on a 10k-line diff) — they do not assert them; the e2e overhead
//! assert in `apps/desktop/e2e/v2/v2-trace.spec.ts` owns the budget gate.
//!
//! Offline guarantee: no network, no OTLP exporter, no collector. The JSONL
//! bench writes to a fresh per-process temp-dir file and removes it afterwards.

use autocompare_telemetry::{emit, InMemorySink, JsonlFileSink, Sink, TraceContext};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

/// Representative span close for a `text.compare` op, matching the envelope
/// built by `emit::Span::finish` (`telemetry/schema/v1/span.json`).
fn build_closed_span() -> serde_json::Value {
    let ctx = TraceContext::fresh(true);
    let mut span = emit::Span::begin(
        &ctx,
        None,
        "text.compare",
        "rs.cmd/text",
        Some("sess_bench"),
        Some("text"),
        "2026-11-28T00:00:00Z",
    );
    span.set_attr("left_lines", serde_json::json!(10_000));
    span.set_attr("right_lines", serde_json::json!(9_998));
    span.set_attr("hunks", serde_json::json!(6));
    span.set_attr("equal", serde_json::json!(false));
    span.finish(true)
}

/// Full emit path: fresh context + begin + one attr + close to a schema record.
fn bench_span_emit_close(c: &mut Criterion) {
    c.bench_function("span_emit/close_text_compare", |b| {
        b.iter(|| {
            let ctx = TraceContext::fresh(true);
            let mut span = emit::Span::begin(
                black_box(&ctx),
                None,
                "text.compare",
                "rs.cmd/text",
                Some("sess_bench"),
                Some("text"),
                "2026-11-28T00:00:00Z",
            );
            span.set_attr("hunks", serde_json::json!(6));
            black_box(span.finish(true));
        });
    });
}

/// In-memory ring write path. Includes one clone of a representative record
/// (callers hand ownership to the sink), so this is an upper bound on the
/// per-record cost; overflow degrades to drop-oldest + counter.
fn bench_sink_inmemory_emit(c: &mut Criterion) {
    let sink = InMemorySink::new(4096);
    let record = build_closed_span();
    c.bench_function("sink_write/inmemory_emit", |b| {
        b.iter(|| {
            sink.emit(black_box(record.clone()));
        });
    });
    black_box(sink.dropped());
}

/// JSONL file write path (open-append-close per record, errors counted as
/// drops). Uses a per-process temp file, removed after the bench.
fn bench_sink_jsonl_emit(c: &mut Criterion) {
    let dir = std::env::temp_dir().join(format!("ac-tel-bench-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("bench.jsonl");
    let sink = JsonlFileSink::new(path.clone());
    let record = build_closed_span();
    c.bench_function("sink_write/jsonl_emit", |b| {
        b.iter(|| {
            sink.emit(black_box(record.clone()));
        });
    });
    black_box(sink.dropped());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

criterion_group!(benches, bench_span_emit_close, bench_sink_inmemory_emit, bench_sink_jsonl_emit);
criterion_main!(benches);
