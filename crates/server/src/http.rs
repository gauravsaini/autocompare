//! Pure HTTP/1.1 request parser and router for `autocompare-server`.
//!
//! Operates on generic `Read`/`Write` streams so routing, token verification,
//! and static asset serving are unit-testable in memory without binding sockets.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::json;

use crate::rpc::{
    MAX_RPC_BODY_BYTES, RpcRequest, RpcResponse, dispatch_rpc, enter_request_span, op_for_method,
    resolve_trace_context, rpc_error_kind, with_trace_echo,
};
use autocompare_telemetry::TraceContext;

/// Configuration for the local AutoCompare HTTP RPC server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Loopback bind address (`127.0.0.1` or `::1`).
    pub host: String,
    /// TCP port to bind (`0` lets the OS pick an ephemeral port).
    pub port: u16,
    /// Optional built frontend directory (`apps/desktop/dist`) to serve at `/`.
    pub dist_dir: Option<PathBuf>,
    /// Bearer token required in `X-AutoCompare-Token` for `POST /rpc`.
    pub token: String,
}

/// Pure HTTP request processor operating over any `Read`/`Write` stream.
///
/// # Errors
///
/// Propagates underlying I/O errors.
pub fn handle_http_stream<R: Read, W: Write>(
    reader_src: R,
    mut writer: W,
    cfg: &ServerConfig,
) -> io::Result<()> {
    let mut reader = BufReader::new(reader_src);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_target = parts.next().unwrap_or("/");
    let path = raw_target.split('?').next().unwrap_or("/");

    let mut headers = BTreeMap::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    if method == "GET" && path == "/health" {
        let body = json!({
            "status": "ok",
            "service": "autocompare-server",
            "transport": "http-rpc"
        })
        .to_string();
        return write_http_response(&mut writer, 200, "OK", "application/json", body.as_bytes());
    }

    if method == "GET" && path == "/rpc-config.json" {
        let body = json!({
            "url": "/rpc",
            "token": cfg.token
        })
        .to_string();
        return write_http_response(&mut writer, 200, "OK", "application/json", body.as_bytes());
    }

    if method == "POST" && path == "/rpc" {
        let provided = headers
            .get("x-autocompare-token")
            .map(String::as_str)
            .unwrap_or("");
        // ODD Phase 7: capture the incoming W3C traceparent before any
        // early return so every /rpc response can echo it back out.
        let incoming_tp: Option<String> = headers.get("traceparent").cloned();
        if provided != cfg.token {
            let rejected = header_only_trace(incoming_tp.as_deref());
            let _guard = rejected
                .as_ref()
                .map(|ctx| enter_request_span(ctx, "rpc", &serde_json::Value::Null));
            // Phase 7-remainder: close the rejection span with a classified
            // outcome (no-op when untraced). No raw values recorded.
            let current = tracing::Span::current();
            current.record("status", "error");
            current.record("error_kind", "auth");
            let err = RpcResponse {
                id: None,
                ok: false,
                result: None,
                error: Some("unauthorized: missing or invalid X-AutoCompare-Token".to_string()),
            };
            let body = serde_json::to_vec(&err).unwrap_or_default();
            return write_rpc_response(&mut writer, 401, "Unauthorized", &body, rejected.as_ref());
        }

        let content_len = headers
            .get("content-length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        if content_len > MAX_RPC_BODY_BYTES {
            let rejected = header_only_trace(incoming_tp.as_deref());
            let _guard = rejected
                .as_ref()
                .map(|ctx| enter_request_span(ctx, "rpc", &serde_json::Value::Null));
            let current = tracing::Span::current();
            current.record("status", "error");
            current.record("error_kind", "body_limit");
            let err = RpcResponse {
                id: None,
                ok: false,
                result: None,
                error: Some(format!("request body exceeds {MAX_RPC_BODY_BYTES} bytes")),
            };
            let body = serde_json::to_vec(&err).unwrap_or_default();
            return write_rpc_response(
                &mut writer,
                413,
                "Payload Too Large",
                &body,
                rejected.as_ref(),
            );
        }

        let mut body_buf = vec![0_u8; content_len];
        reader.read_exact(&mut body_buf)?;

        let rpc_req: RpcRequest = match serde_json::from_slice(&body_buf) {
            Ok(req) => req,
            Err(e) => {
                let rejected = header_only_trace(incoming_tp.as_deref());
                let _guard = rejected
                    .as_ref()
                    .map(|ctx| enter_request_span(ctx, "rpc", &serde_json::Value::Null));
                let current = tracing::Span::current();
                current.record("status", "error");
                current.record("error_kind", "bad_json");
                let err = RpcResponse {
                    id: None,
                    ok: false,
                    result: None,
                    error: Some(format!("invalid RPC JSON: {e}")),
                };
                let body = serde_json::to_vec(&err).unwrap_or_default();
                return write_rpc_response(
                    &mut writer,
                    400,
                    "Bad Request",
                    &body,
                    rejected.as_ref(),
                );
            }
        };

        // ODD Phase 7: header wins, `__trace` envelope in params is the
        // fallback (the HTTP client sends both). The child span covers
        // dispatch and carries redacted summaries only; the response
        // echoes the traceparent header plus a `__trace` envelope echo
        // for the client's response-correlation check.
        //
        // ODD Phase 7-remainder: the span closes here with status,
        // classified error kind, and duration. Failure events carry the
        // error kind only: raw dispatch messages may quote absolute
        // paths, so they never enter telemetry.
        let trace_ctx = resolve_trace_context(incoming_tp.as_deref(), &rpc_req.params);
        let _server_span_guard = trace_ctx
            .as_ref()
            .map(|ctx| enter_request_span(ctx, &rpc_req.method, &rpc_req.params));

        let started = Instant::now();
        let response = match dispatch_rpc(&rpc_req.method, &rpc_req.params) {
            Ok(result) => {
                let current = tracing::Span::current();
                current.record("status", "ok");
                current.record("duration_ns", duration_ns_since(started));
                RpcResponse {
                    id: rpc_req.id,
                    ok: true,
                    result: Some(with_trace_echo(result, trace_ctx.as_ref())),
                    error: None,
                }
            }
            Err(message) => {
                let kind = rpc_error_kind(&message);
                let current = tracing::Span::current();
                current.record("status", "error");
                current.record("error_kind", kind);
                current.record("duration_ns", duration_ns_since(started));
                if trace_ctx.is_some() {
                    tracing::error!(
                        component = "rs.server/rpc",
                        op = %op_for_method(&rpc_req.method),
                        error_kind = kind,
                        "server.rpc failed"
                    );
                }
                RpcResponse {
                    id: rpc_req.id,
                    ok: false,
                    result: None,
                    error: Some(message),
                }
            }
        };
        let payload = serde_json::to_vec(&response).unwrap_or_default();
        return write_rpc_response(&mut writer, 200, "OK", &payload, trace_ctx.as_ref());
    }

    if method == "GET" {
        return serve_static(&mut writer, path, cfg);
    }

    write_http_response(
        &mut writer,
        405,
        "Method Not Allowed",
        "text/plain",
        b"Method Not Allowed",
    )
}

fn serve_static<W: Write>(writer: &mut W, url_path: &str, cfg: &ServerConfig) -> io::Result<()> {
    let Some(dist_root) = cfg.dist_dir.as_ref() else {
        let body = json!({
            "service": "autocompare-server",
            "message": "Pass --dist apps/desktop/dist to serve the web UI."
        })
        .to_string();
        return write_http_response(writer, 200, "OK", "application/json", body.as_bytes());
    };

    let rel = url_path.trim_start_matches('/');
    let rel_path = if rel.is_empty() { "index.html" } else { rel };
    if rel_path.contains("..") {
        return write_http_response(writer, 403, "Forbidden", "text/plain", b"Forbidden");
    }

    let candidate = dist_root.join(rel_path);
    let file_path = if candidate.is_file() {
        candidate
    } else {
        dist_root.join("index.html")
    };

    let Ok(canon_root) = fs::canonicalize(dist_root) else {
        return write_http_response(writer, 404, "Not Found", "text/plain", b"dist not found");
    };
    let Ok(canon_file) = fs::canonicalize(&file_path) else {
        return write_http_response(writer, 404, "Not Found", "text/plain", b"Not Found");
    };
    if !canon_file.starts_with(&canon_root) {
        return write_http_response(writer, 403, "Forbidden", "text/plain", b"Forbidden");
    }

    let bytes = match fs::read(&canon_file) {
        Ok(b) => b,
        Err(_) => return write_http_response(writer, 404, "Not Found", "text/plain", b"Not Found"),
    };

    let mime = content_type_for_path(&canon_file);
    write_http_response(writer, 200, "OK", mime, &bytes)
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn write_http_response<W: Write>(
    writer: &mut W,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    write_http_response_with_headers(writer, status, reason, content_type, body, &[])
}

fn write_http_response_with_headers<W: Write>(
    writer: &mut W,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, &str)],
) -> io::Result<()> {
    let mut header = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n",
        body.len()
    );
    for pair in extra_headers {
        let (name, value) = *pair;
        header.push_str(name);
        header.push_str(": ");
        header.push_str(value);
        header.push_str("\r\n");
    }
    header.push_str("Connection: close\r\n\r\n");
    writer.write_all(header.as_bytes())?;
    writer.write_all(body)?;
    writer.flush()
}

/// ODD Phase 7-remainder: elapsed nanos since `started`, saturated to
/// `u64` for span recording. Never blocks, never fails.
fn duration_ns_since(started: Instant) -> u64 {
    let nanos = started.elapsed().as_nanos();
    if nanos > u128::from(u64::MAX) {
        u64::MAX
    } else {
        nanos as u64
    }
}

/// ODD Phase 7: derives a child span context from the `traceparent`
/// header alone, for `/rpc` rejections that happen before the body is
/// available (auth / size / JSON errors). `None` keeps those responses
/// byte-identical when telemetry is off or untraced.
fn header_only_trace(traceparent: Option<&str>) -> Option<TraceContext> {
    resolve_trace_context(traceparent, &serde_json::Value::Null)
}

/// ODD Phase 7: writes a `/rpc` response, echoing the server span's W3C
/// `traceparent` back out when the request was traced. Untraced responses
/// take the legacy path byte-identically.
fn write_rpc_response<W: Write>(
    writer: &mut W,
    status: u16,
    reason: &str,
    body: &[u8],
    trace: Option<&TraceContext>,
) -> io::Result<()> {
    if let Some(ctx) = trace {
        let outgoing = ctx.to_traceparent();
        write_http_response_with_headers(
            writer,
            status,
            reason,
            "application/json",
            body,
            &[("traceparent", outgoing.as_str())],
        )
    } else {
        write_http_response(writer, status, reason, "application/json", body)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn run_in_memory(cfg: &ServerConfig, request: &str) -> String {
        let mut out = Vec::new();
        handle_http_stream(Cursor::new(request.as_bytes()), &mut out, cfg).expect("handle");
        String::from_utf8(out).expect("utf8")
    }

    #[test]
    fn serves_health_enforces_token_and_dispatches_rpc() {
        let cfg = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            dist_dir: None,
            token: "test-token-123".to_string(),
        };

        let health = run_in_memory(
            &cfg,
            "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(health.starts_with("HTTP/1.1 200 OK"));
        assert!(health.contains("\"status\":\"ok\""));

        let body = r#"{"method":"ping","params":{}}"#;
        let unauth = run_in_memory(
            &cfg,
            &format!(
                "POST /rpc HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
        );
        assert!(unauth.starts_with("HTTP/1.1 401 Unauthorized"));

        let schemes_body = r#"{"id":7,"method":"vfs_schemes","params":{}}"#;
        let ok_resp = run_in_memory(
            &cfg,
            &format!(
                "POST /rpc HTTP/1.1\r\nHost: localhost\r\nX-AutoCompare-Token: test-token-123\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{schemes_body}",
                schemes_body.len()
            ),
        );
        assert!(ok_resp.starts_with("HTTP/1.1 200 OK"));
        assert!(ok_resp.contains("\"ok\":true"));
        assert!(ok_resp.contains("\"zip\""));
        assert!(ok_resp.contains("\"bcss\""));
    }

    #[test]
    fn traceparent_propagates_in_and_out_with_redacted_child() {
        use autocompare_telemetry::TraceContext;

        let cfg = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            dist_dir: None,
            token: "trace-token".to_string(),
        };

        // Untraced requests stay byte-identical: no header echo, no echo field.
        let plain_body = r#"{"id":1,"method":"ping","params":{}}"#;
        let plain = run_in_memory(
            &cfg,
            &format!(
                "POST /rpc HTTP/1.1\r\nHost: localhost\r\nX-AutoCompare-Token: trace-token\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{plain_body}",
                plain_body.len()
            ),
        );
        assert!(plain.starts_with("HTTP/1.1 200 OK"));
        assert!(!plain.contains("traceparent:"));
        assert!(!plain.contains("__trace"));

        // Traced request: same trace_id flows out via header + envelope echo.
        let parent = TraceContext::fresh(true);
        let incoming = parent.to_traceparent();
        let traced_body = r#"{"id":2,"method":"ping","params":{}}"#;
        let traced = run_in_memory(
            &cfg,
            &format!(
                "POST /rpc HTTP/1.1\r\nHost: localhost\r\nX-AutoCompare-Token: trace-token\r\ntraceparent: {incoming}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{traced_body}",
                traced_body.len()
            ),
        );
        assert!(traced.starts_with("HTTP/1.1 200 OK"));
        let trace_hex = hex_of_trace(&parent);
        assert!(traced.contains(&format!("traceparent: 00-{trace_hex}-")));
        // The echoed span shares the trace but mints its own span id.
        assert!(!traced.contains(&incoming));
        let body_json = response_body_json(&traced);
        let echoed = body_json
            .get("result")
            .and_then(|r| r.get("__trace"))
            .and_then(|t| t.get("trace_id"))
            .and_then(serde_json::Value::as_str)
            .expect("trace echo");
        assert_eq!(echoed, trace_hex);

        // Rejections echo the header too (auth path, body never parsed).
        let rejected = run_in_memory(
            &cfg,
            &format!(
                "POST /rpc HTTP/1.1\r\nHost: localhost\r\ntraceparent: {incoming}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{traced_body}",
                traced_body.len()
            ),
        );
        assert!(rejected.starts_with("HTTP/1.1 401 Unauthorized"));
        assert!(rejected.contains(&format!("traceparent: 00-{trace_hex}-")));

        // Malformed traceparent means untraced: no echo, dispatch unaffected.
        let bogus_body = r#"{"id":3,"method":"ping","params":{}}"#;
        let bogus = run_in_memory(
            &cfg,
            &format!(
                "POST /rpc HTTP/1.1\r\nHost: localhost\r\nX-AutoCompare-Token: trace-token\r\ntraceparent: bogus\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{bogus_body}",
                bogus_body.len()
            ),
        );
        assert!(bogus.starts_with("HTTP/1.1 200 OK"));
        assert!(!bogus.contains("traceparent:"));
    }

    fn hex_of_trace(ctx: &autocompare_telemetry::TraceContext) -> String {
        ctx.to_envelope()
            .get("trace_id")
            .and_then(serde_json::Value::as_str)
            .expect("trace_id hex")
            .to_string()
    }

    fn response_body_json(raw: &str) -> serde_json::Value {
        let sep = raw.find("\r\n\r\n").expect("header/body separator");
        serde_json::from_str(&raw[sep + 4..]).expect("response json")
    }

    /// ODD Phase 7-remainder: minimal capturing layer proving the server
    /// span lifecycle without touching the wire format.
    mod capture {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::registry::LookupSpan;

        #[derive(Clone, Default)]
        pub struct Spans(pub Arc<Mutex<Vec<SpanRecord>>>);

        #[derive(Clone, Default, Debug)]
        pub struct SpanRecord {
            pub name: String,
            pub opened: Vec<String>,
            pub recorded: Vec<String>,
        }

        #[derive(Clone, Default)]
        pub struct Events(pub Arc<Mutex<Vec<String>>>);

        pub struct Cap {
            pub spans: Spans,
            pub events: Events,
        }

        struct Collect<'a> {
            out: &'a mut Vec<String>,
        }

        impl tracing::field::Visit for Collect<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                self.out.push(format!("{}={:?}", field.name(), value));
            }
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                self.out.push(format!("{}={}", field.name(), value));
            }
            fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
                self.out.push(format!("{}={}", field.name(), value));
            }
            fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
                self.out.push(format!("{}={}", field.name(), value));
            }
            fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
                self.out.push(format!("{}={}", field.name(), value));
            }
        }

        impl<S> Layer<S> for Cap
        where
            S: tracing::Subscriber + for<'a> LookupSpan<'a>,
        {
            fn on_new_span(
                &self,
                attrs: &tracing::span::Attributes<'_>,
                id: &tracing::span::Id,
                ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                let mut opened = Vec::new();
                attrs.record(&mut Collect { out: &mut opened });
                let key = id.into_u64();
                if let Some(span) = ctx.span(id) {
                    let mut ext = span.extensions_mut();
                    ext.insert(key);
                }
                if let Ok(mut guard) = self.spans.0.lock() {
                    guard.push(SpanRecord {
                        name: attrs.metadata().name().to_string(),
                        opened,
                        recorded: Vec::new(),
                    });
                    let idx = guard.len().wrapping_sub(1);
                    if let Some(span) = ctx.span(id) {
                        let mut ext = span.extensions_mut();
                        ext.insert(idx);
                    }
                }
            }

            fn on_record(
                &self,
                id: &tracing::span::Id,
                values: &tracing::span::Record<'_>,
                ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                let mut extra = Vec::new();
                values.record(&mut Collect { out: &mut extra });
                if let Ok(mut guard) = self.spans.0.lock() {
                    let slot = ctx
                        .span(id)
                        .and_then(|span| {
                            let ext = span.extensions();
                            ext.get::<usize>().copied()
                        })
                        .and_then(|idx| guard.get_mut(idx));
                    if let Some(record) = slot {
                        record.recorded.extend(extra);
                    }
                }
            }

            fn on_event(
                &self,
                event: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                let mut fields = Vec::new();
                event.record(&mut Collect { out: &mut fields });
                if let Ok(mut guard) = self.events.0.lock() {
                    guard.push(fields.join(" "));
                }
            }
        }

        pub fn with_capture<T>(f: impl FnOnce() -> T) -> (T, Spans, Events) {
            let spans = Spans::default();
            let events = Events::default();
            let layer = Cap { spans: spans.clone(), events: events.clone() };
            let subscriber = tracing_subscriber::registry().with(layer);
            let out = tracing::subscriber::with_default(subscriber, f);
            (out, spans, events)
        }
    }

    #[test]
    fn traced_requests_close_spans_with_status_and_redacted_errors() {
        use autocompare_telemetry::TraceContext;

        let cfg = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            dist_dir: None,
            token: "span-close-token".to_string(),
        };

        let ((), spans, events) = capture::with_capture(|| {
            // Success path: ping closes ok with a duration.
            let parent = TraceContext::fresh(true);
            let incoming = parent.to_traceparent();
            let body = r#"{"id":1,"method":"ping","params":{}}"#;
            let ok_raw = run_in_memory(
                &cfg,
                &format!(
                    "POST /rpc HTTP/1.1\r\nHost: localhost\r\nX-AutoCompare-Token: span-close-token\r\ntraceparent: {incoming}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                ),
            );
            assert!(ok_raw.starts_with("HTTP/1.1 200 OK"));
            assert!(ok_raw.contains("\"ok\":true"));

            // Failure path: secret-looking absolute paths in params must
            // never reach telemetry; only the classified kind is recorded.
            let secret = "/secret/home/autocompare/missing.txt";
            let bad_body = format!(
                r#"{{"id":2,"method":"compare_text","params":{{"left":"{secret}","right":"{secret}"}}}}"#
            );
            let bad_raw = run_in_memory(
                &cfg,
                &format!(
                    "POST /rpc HTTP/1.1\r\nHost: localhost\r\nX-AutoCompare-Token: span-close-token\r\ntraceparent: {incoming}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{bad_body}",
                    bad_body.len()
                ),
            );
            assert!(bad_raw.starts_with("HTTP/1.1 200 OK"));
            assert!(bad_raw.contains("\"ok\":false"));

            // The error dispatch still echoes correlation headers.
            assert!(bad_raw.contains("traceparent: 00-"));
        });

        let spans = spans.0.lock().map(|v| v.clone()).unwrap_or_default();
        let server_spans: Vec<_> = spans.iter().filter(|s| s.name == "server.rpc").collect();
        assert_eq!(server_spans.len(), 2, "one span per request: {spans:?}");

        let closed: Vec<String> = server_spans
            .iter()
            .flat_map(|s| s.recorded.iter().cloned())
            .collect();
        assert!(
            closed.iter().any(|f| f == "status=ok"),
            "success span closes ok: {closed:?}"
        );
        assert!(
            closed.iter().any(|f| f == "status=error"),
            "failure span closes error: {closed:?}"
        );
        assert!(
            closed.iter().any(|f| f == "error_kind=io_missing"),
            "failure kind classified without raw message: {closed:?}"
        );
        assert!(
            closed.iter().any(|f| f.starts_with("duration_ns=")),
            "both spans carry durations: {closed:?}"
        );

        // Opened span fields carry tokens/summaries only, never raw paths.
        let opened: String = spans
            .iter()
            .flat_map(|s| s.opened.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!opened.contains("/secret/home"), "raw path leaked: {opened}");

        // The failure event is kind-only: no paths, no secrets, no prose.
        let events = events.0.lock().map(|v| v.clone()).unwrap_or_default();
        assert!(
            events.iter().any(|e| e.contains("error_kind=io_missing")),
            "expected classified error event: {events:?}"
        );
        let flat_events = events.join("\n");
        assert!(!flat_events.contains("/secret/home"));
        assert!(!flat_events.contains("missing.txt"));
    }

    #[test]
    fn traced_rejections_close_spans_without_touching_dispatch() {
        let cfg = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            dist_dir: None,
            token: "reject-token".to_string(),
        };

        let ((), spans, _) = capture::with_capture(|| {
            use autocompare_telemetry::TraceContext;
            let parent = TraceContext::fresh(true);
            let incoming = parent.to_traceparent();
            let body = r#"{"id":9,"method":"ping","params":{}}"#;
            let rejected = run_in_memory(
                &cfg,
                &format!(
                    "POST /rpc HTTP/1.1\r\nHost: localhost\r\ntraceparent: {incoming}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                ),
            );
            assert!(rejected.starts_with("HTTP/1.1 401 Unauthorized"));
            assert!(rejected.contains("traceparent: 00-"));
        });

        let spans = spans.0.lock().map(|v| v.clone()).unwrap_or_default();
        let server_spans: Vec<_> = spans.iter().filter(|s| s.name == "server.rpc").collect();
        assert_eq!(server_spans.len(), 1);
        let recorded: Vec<_> = server_spans
            .iter()
            .flat_map(|s| s.recorded.iter().cloned())
            .collect();
        assert!(recorded.iter().any(|f| f == "status=error"), "{recorded:?}");
        assert!(recorded.iter().any(|f| f == "error_kind=auth"), "{recorded:?}");
    }
}
