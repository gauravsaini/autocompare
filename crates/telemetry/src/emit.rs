//! Span/error record builders matching telemetry/schema/v1.

use crate::TraceContext;
use serde_json::{Map, Value};

fn base_envelope(ctx: &TraceContext, session_id: Option<&str>, view_id: Option<&str>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("schema_version".to_string(), Value::from(1));
    m.insert("trace_id".to_string(), Value::from(hex_of(&ctx.trace_id)));
    m.insert("span_id".to_string(), Value::from(hex_of(&ctx.span_id)));
    if let Some(s) = session_id {
        m.insert("session_id".to_string(), Value::from(s));
    }
    if let Some(v) = view_id {
        m.insert("view_id".to_string(), Value::from(v));
    }
    m
}

fn hex_of(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let hi = HEX.get((b >> 4) as usize).copied().unwrap_or(b'?');
        let lo = HEX.get((b & 0x0f) as usize).copied().unwrap_or(b'?');
        out.push(hi as char);
        out.push(lo as char);
    }
    out
}

/// Build a span-open record (caller fills `attributes`).
pub fn span_open(
    ctx: &TraceContext,
    parent: Option<&TraceContext>,
    op: &str,
    component: &str,
    session_id: Option<&str>,
    view_id: Option<&str>,
    started_at_rfc3339: &str,
) -> Value {
    let mut m = base_envelope(ctx, session_id, view_id);
    m.insert("kind".to_string(), Value::from("span"));
    m.insert("op".to_string(), Value::from(op));
    m.insert("component".to_string(), Value::from(component));
    if let Some(p) = parent {
        m.insert("parent_span_id".to_string(), Value::from(hex_of(&p.span_id)));
    }
    m.insert("started_at".to_string(), Value::from(started_at_rfc3339));
    m.insert("status".to_string(), Value::from("open"));
    m.insert("redaction".to_string(), Value::from("v1-default"));
    Value::Object(m)
}

/// Build a span-close record.
pub fn span_close(
    ctx: &TraceContext,
    op: &str,
    status_ok: bool,
    duration_ns: u64,
    attributes: Map<String, Value>,
) -> Value {
    let mut m = base_envelope(ctx, None, None);
    m.insert("kind".to_string(), Value::from("span"));
    m.insert("op".to_string(), Value::from(op));
    m.insert(
        "status".to_string(),
        Value::from(if status_ok { "ok" } else { "error" }),
    );
    m.insert("duration_ns".to_string(), Value::from(duration_ns));
    m.insert("attributes".to_string(), Value::Object(attributes));
    m.insert("redaction".to_string(), Value::from("v1-default"));
    Value::Object(m)
}

/// Build an error event linked to a span.
pub fn error_event(
    ctx: &TraceContext,
    name: &str,
    component: &str,
    message: &str,
    at_rfc3339: &str,
) -> Value {
    let mut attrs = Map::new();
    attrs.insert("component".to_string(), Value::from(component));
    let short: String = message.chars().take(300).collect();
    attrs.insert("message".to_string(), Value::from(short));
    let mut m = base_envelope(ctx, None, None);
    m.insert("kind".to_string(), Value::from("event"));
    m.insert("name".to_string(), Value::from(name));
    m.insert("at".to_string(), Value::from(at_rfc3339));
    m.insert("severity".to_string(), Value::from("error"));
    m.insert("attributes".to_string(), Value::Object(attrs));
    Value::Object(m)
}
