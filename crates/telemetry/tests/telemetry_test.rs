#![allow(clippy::unwrap_used, clippy::expect_used)]

use autocompare_telemetry::{InMemorySink, JsonlFileSink, Sampling, Sink, TraceContext};
use autocompare_telemetry::{emit, redact};

#[test]
fn context_round_trip_envelope() {
    let ctx = TraceContext::fresh(true);
    let params = serde_json::json!({ "left": "a", "__trace": ctx.to_envelope() });
    let back = TraceContext::from_envelope(&params).expect("envelope round-trip");
    assert_eq!(ctx, back);
}

#[test]
fn context_rejects_absent_or_malformed() {
    assert!(TraceContext::from_envelope(&serde_json::json!({})).is_none());
    assert!(TraceContext::from_envelope(&serde_json::json!({
        "__trace": { "trace_id": "zz", "span_id": "00", "sampled": true }
    }))
    .is_none());
}

#[test]
fn context_traceparent_round_trip() {
    let ctx = TraceContext::fresh(false);
    let header = ctx.to_traceparent();
    let back = TraceContext::from_traceparent(&header).expect("traceparent round-trip");
    assert_eq!(ctx, back);
    assert!(TraceContext::from_traceparent("bogus").is_none());
}

#[test]
fn sampling_keeps_errors_and_kills_switch() {
    let s = Sampling::new(0.0);
    assert!(s.keep("text.compare", true, 0.99));
    assert!(!s.keep("text.compare", false, 0.99));
    let _ = Sampling::enabled();
}

#[test]
fn redact_never_leaks_content() {
    let data = b"secret Rosetta line one\nline two\n";
    let sum = redact::summarize_bytes(data);
    assert_eq!(sum.bytes, data.len() as u64);
    assert_eq!(sum.lines, 2);
    assert_eq!(sum.sha12.len(), 12);
    assert!(!sum.sha12.contains("secret"));

    let token = redact::hash_path("/Users/ekta/Documents/File 1.txt", "salt-1");
    assert!(token.starts_with("p_"));
    assert!(!token.contains("ekta"));

    assert!(redact::looks_like_secret("api_token=abc"));
    assert!(redact::scrub_external_output("api_token=abc").is_none());
    assert!(redact::scrub_external_output("plain failure").is_some());
}

#[test]
fn emit_shapes_match_schema_fields() {
    let ctx = TraceContext::fresh(true);
    let mut span = emit::Span::begin(&ctx, None, "text.compare", "rs.cmd/text", Some("s1"), Some("text"), "2026-11-28T00:00:00Z");
    span.set_attr("hunks", serde_json::json!(6));
    let rec = span.finish(true);
    assert_eq!(rec.get("schema_version").and_then(serde_json::Value::as_u64), Some(1));
    assert_eq!(rec.get("kind").and_then(serde_json::Value::as_str), Some("span"));
    assert_eq!(rec.get("op").and_then(serde_json::Value::as_str), Some("text.compare"));
    assert_eq!(rec.get("redaction").and_then(serde_json::Value::as_str), Some("v1-default"));
    assert_eq!(rec.get("status").and_then(serde_json::Value::as_str), Some("ok"));
    assert!(rec.get("started_at").is_some());
    assert!(rec.get("duration_ns").and_then(serde_json::Value::as_u64).is_some());
    assert_eq!(rec.get("attributes").and_then(|a| a.get("hunks")).and_then(serde_json::Value::as_u64), Some(6));

    let err = emit::error_event(&ctx, "ipc.serde_error", "rs.compare/text", "bad field", "2026-11-28T00:00:00Z");
    assert_eq!(err.get("kind").and_then(serde_json::Value::as_str), Some("event"));
    assert_eq!(err.get("severity").and_then(serde_json::Value::as_str), Some("error"));
}

#[test]
fn sinks_buffer_and_persist() {
    let mem = InMemorySink::new(2);
    mem.emit(serde_json::json!({"a": 1}));
    mem.emit(serde_json::json!({"a": 2}));
    mem.emit(serde_json::json!({"a": 3}));
    assert_eq!(mem.snapshot().len(), 2);
    assert_eq!(mem.dropped(), 1);

    let dir = std::env::temp_dir().join(format!("ac-tel-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("t.jsonl");
    let file = JsonlFileSink::new(path.clone());
    file.emit(serde_json::json!({"schema_version": 1, "kind": "span"}));
    assert_eq!(file.dropped(), 0);
    let content = std::fs::read_to_string(&path).expect("read jsonl");
    assert!(content.contains("schema_version"));
}
