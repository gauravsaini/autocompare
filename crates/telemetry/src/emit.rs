//! Span/error record builders matching telemetry/schema/v1.
//!
//! Spans are emitted exactly once, at close, as complete records
//! (`started_at` + `duration_ns` + `status`), mirroring the TS tracer.

use crate::TraceContext;
use serde_json::{Map, Value};

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

/// Open span handle. Call [`Span::set_attr`] during execution, then
/// [`Span::finish`] exactly once to produce the schema-valid record.
pub struct Span {
    ctx: TraceContext,
    parent_span_id: Option<[u8; 8]>,
    op: String,
    component: String,
    session_id: Option<String>,
    view_id: Option<String>,
    started_at_rfc3339: String,
    started: std::time::Instant,
    attributes: Map<String, Value>,
}

impl Span {
    pub fn begin(
        ctx: &TraceContext,
        parent: Option<&TraceContext>,
        op: &str,
        component: &str,
        session_id: Option<&str>,
        view_id: Option<&str>,
        started_at_rfc3339: &str,
    ) -> Self {
        Self {
            ctx: *ctx,
            parent_span_id: parent.map(|p| p.span_id),
            op: op.to_string(),
            component: component.to_string(),
            session_id: session_id.map(str::to_string),
            view_id: view_id.map(str::to_string),
            started_at_rfc3339: started_at_rfc3339.to_string(),
            started: std::time::Instant::now(),
            attributes: Map::new(),
        }
    }

    pub fn set_attr(&mut self, key: &str, value: Value) {
        self.attributes.insert(key.to_string(), value);
    }

    /// Consume the span and build the complete closed record.
    pub fn finish(self, status_ok: bool) -> Value {
        let mut m = Map::new();
        m.insert("schema_version".to_string(), Value::from(1));
        m.insert("kind".to_string(), Value::from("span"));
        m.insert("trace_id".to_string(), Value::from(hex_of(&self.ctx.trace_id)));
        m.insert("span_id".to_string(), Value::from(hex_of(&self.ctx.span_id)));
        if let Some(p) = self.parent_span_id {
            m.insert("parent_span_id".to_string(), Value::from(hex_of(&p)));
        }
        m.insert("op".to_string(), Value::from(self.op));
        m.insert("component".to_string(), Value::from(self.component));
        if let Some(s) = self.session_id {
            m.insert("session_id".to_string(), Value::from(s));
        }
        if let Some(v) = self.view_id {
            m.insert("view_id".to_string(), Value::from(v));
        }
        m.insert("started_at".to_string(), Value::from(self.started_at_rfc3339));
        let duration_ns = self.started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
        m.insert("duration_ns".to_string(), Value::from(duration_ns));
        m.insert(
            "status".to_string(),
            Value::from(if status_ok { "ok" } else { "error" }),
        );
        m.insert("attributes".to_string(), Value::Object(self.attributes));
        m.insert("redaction".to_string(), Value::from("v1-default"));
        Value::Object(m)
    }
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
    let mut m = Map::new();
    m.insert("schema_version".to_string(), Value::from(1));
    m.insert("kind".to_string(), Value::from("event"));
    m.insert("name".to_string(), Value::from(name));
    m.insert("trace_id".to_string(), Value::from(hex_of(&ctx.trace_id)));
    m.insert("span_id".to_string(), Value::from(hex_of(&ctx.span_id)));
    m.insert("at".to_string(), Value::from(at_rfc3339));
    m.insert("severity".to_string(), Value::from("error"));
    m.insert("attributes".to_string(), Value::Object(attrs));
    Value::Object(m)
}
