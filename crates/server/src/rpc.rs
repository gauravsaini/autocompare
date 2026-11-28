//! JSON-RPC command dispatch for `autocompare-server`.

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use autocompare_compare::format_converter::FormatConverterRegistry;
use autocompare_compare::{
    FolderOptions, SyncDirection, SyncOptions, TableOptions, TextOptions, compare_folders,
    compare_table_files, compare_text_files, plan_sync,
};
use autocompare_diff::Op;
use autocompare_telemetry::{Sampling, TraceContext, emit, redact};
use autocompare_vfs::registry::VfsRegistry;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum file/body read limit for RPC payloads (16 MiB).
pub const MAX_RPC_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Incoming JSON-RPC request payload for `POST /rpc`.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcRequest {
    /// Optional caller-supplied correlation identifier.
    #[serde(default)]
    pub id: Option<Value>,
    /// Command/method name matching Tauri IPC command conventions.
    pub method: String,
    /// JSON object of named parameters.
    #[serde(default)]
    pub params: Value,
}

/// Outgoing JSON-RPC response payload from `POST /rpc`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// Echoed correlation identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    /// `true` when the method completed without error.
    pub ok: bool,
    /// Successful JSON result payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Human-readable error message on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn req_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string parameter '{key}'"))
}

/// ODD Phase 7: maps an RPC method name to its telemetry op.
///
/// The four engine ops join the registry in `telemetry/schema/v1/ops.json`;
/// everything else is namespaced under `server.` so op names stay
/// free-form but collision-free.
#[must_use]
pub fn op_for_method(method: &str) -> String {
    match method {
        "compare_text" => "text.compare".to_string(),
        "compare_table" => "table.compare".to_string(),
        "compare_folders" => "folder.compare".to_string(),
        "plan_sync" => "sync.plan".to_string(),
        "ping" | "health" => "server.health".to_string(),
        _ => format!("server.{}", method.replace('_', ".")),
    }
}

/// Process-unique salt for path hashing: stable in-process so one run
/// correlates, never persisted anywhere. Mirrors the compare crate's salt
/// convention so tokens stay opaque across crates.
fn odd_server_salt() -> String {
    format!("odd-server-v1-{}", std::process::id())
}

/// Opaque token for a path-like RPC parameter. The raw value never leaves
/// the process.
fn odd_path_token(value: &str) -> String {
    redact::hash_path(value, &odd_server_salt())
}

/// ODD Phase 7: derives a child trace context for one RPC request.
///
/// Prefers the W3C `traceparent` header, falls back to the Tauri-style
/// `__trace` envelope inside `params` (the HTTP client sends both).
/// Returns `None` when telemetry is disabled or no valid context was
/// supplied — in which case handling and serialized shapes are unchanged.
#[must_use]
/// ODD Phase 7-remainder: derives the incoming parent trace context.
pub fn incoming_parent_context(traceparent: Option<&str>, params: &Value) -> Option<TraceContext> {
    if !Sampling::enabled() {
        return None;
    }
    if let Some(header) = traceparent
        && let Some(parent) = TraceContext::from_traceparent(header.trim())
    {
        return Some(parent);
    }
    TraceContext::from_envelope(params)
}

/// ODD Phase 7: derives a child trace context for one RPC request.
///
/// Header wins, `__trace` envelope in params is the fallback. Returns
/// `None` when telemetry is disabled or no valid context was supplied:
/// handling and serialized shapes are then unchanged.
#[must_use]
pub fn resolve_trace_context(traceparent: Option<&str>, params: &Value) -> Option<TraceContext> {
    incoming_parent_context(traceparent, params).map(|parent| parent.child())
}

/// ODD Phase 7: builds a redaction-safe attribute map for one RPC request.
///
/// Contains the method/op names, sorted parameter keys, opaque tokens for
/// path-like parameters, and byte counts for inline content — never raw
/// paths, tokens, or file contents.
#[must_use]
pub fn redacted_summary(method: &str, params: &Value) -> Value {
    let mut keys: Vec<String> = Vec::new();
    if let Some(obj) = params.as_object() {
        for key in obj.keys() {
            keys.push(key.clone());
        }
    }
    keys.sort();

    let mut paths = serde_json::Map::new();
    for key in ["left", "right", "path", "uri", "base"] {
        if let Some(raw) = params.get(key).and_then(Value::as_str) {
            paths.insert(key.to_string(), Value::from(odd_path_token(raw)));
        }
    }
    let content_bytes = params
        .get("content")
        .and_then(Value::as_str)
        .map(|c| Value::from(c.len() as u64));

    let mut summary = serde_json::Map::new();
    summary.insert("method".to_string(), Value::from(method));
    summary.insert("op".to_string(), Value::from(op_for_method(method)));
    let key_values: Vec<Value> = keys.iter().map(|k| Value::from(k.as_str())).collect();
    summary.insert("param_keys".to_string(), Value::from(key_values));
    summary.insert("paths".to_string(), Value::Object(paths));
    if let Some(bytes) = content_bytes {
        summary.insert("content_bytes".to_string(), bytes);
    }
    Value::Object(summary)
}

/// ODD Phase 7: enters the per-request server span for `method`.
///
/// All span fields come from [`redacted_summary`]: op/method names plus
/// opaque trace ids and path tokens only. `status`, `error_kind`, and
/// `duration_ns` start empty so the HTTP layer can record the outcome
/// once dispatch completes (Phase 7-remainder: web-RPC traces close).
pub fn enter_request_span(ctx: &TraceContext, method: &str, params: &Value) -> tracing::span::EnteredSpan {
    let envelope = ctx.to_envelope();
    let trace_id = envelope
        .get("trace_id")
        .cloned()
        .unwrap_or(Value::Null);
    let span_id = envelope
        .get("span_id")
        .cloned()
        .unwrap_or(Value::Null);
    let summary = redacted_summary(method, params);
    tracing::info_span!(
        "server.rpc",
        op = %op_for_method(method),
        method = %method,
        trace_id = ?trace_id,
        span_id = ?span_id,
        summary = ?summary,
        component = "rs.server/rpc",
        status = tracing::field::Empty,
        error_kind = tracing::field::Empty,
        duration_ns = tracing::field::Empty,
    )
    .entered()
}

/// ODD Phase 7: injects the `__trace` echo into an object result so the
/// HTTP client can correlate the response to its outgoing span (the same
/// contract as the Tauri `trace_echo` DTO field).
///
/// Non-object results and `None` contexts pass through untouched, keeping
/// serialized shapes byte-identical whenever telemetry is off.
#[must_use]
pub fn with_trace_echo(result: Value, ctx: Option<&TraceContext>) -> Value {
    let Some(ctx) = ctx else {
        return result;
    };
    let mut result = result;
    if let Some(obj) = result.as_object_mut() {
        obj.insert("__trace".to_string(), ctx.to_envelope());
    }
    result
}

/// ODD Phase 7-remainder: extracts the session join key from RPC params.
///
/// Accepts the `session_id` / `sessionId` / `session` spellings used across
/// the TS IPC modules. Overlong values are treated as absent so a hostile
/// param can never bloat a span attribute. Returns `None` when telemetry
/// is disabled: no join key, no linkage, zero behavior change.
#[must_use]
pub fn session_id_of(params: &Value) -> Option<String> {
    if !Sampling::enabled() {
        return None;
    }
    for key in ["session_id", "sessionId", "session"] {
        if let Some(id) = params.get(key).and_then(Value::as_str)
            && !id.is_empty()
            && id.len() <= 128
        {
            return Some(id.to_string());
        }
    }
    None
}

/// ODD Phase 7-remainder: current UTC time as `YYYY-MM-DDTHH:MM:SS.sssZ`.
///
/// Dependency-free (civil-day math mirrors `autocompare-telemetry` sink
/// date helpers, which stay untouched): the server crate formats its own
/// `started_at` / `at` fields for [`emit`] records.
#[must_use]
pub fn now_rfc3339_millis() -> String {
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since_epoch.as_secs();
    let millis = since_epoch.subsec_millis();
    let secs_of_day = secs % 86_400;
    let (year, month, day) = civil_from_days(u64_days_to_i64(secs / 86_400));
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn u64_days_to_i64(days: u64) -> i64 {
    if days > i64::MAX as u64 { i64::MAX } else { days as i64 }
}

// Howard Hinnant's civil-day algorithm (mirrors the telemetry sink's
// date math; the sink itself is untouched per Phase 7-remainder scope).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days.saturating_add(719_468);
    let era = if shifted >= 0 { shifted } else { shifted - 146_096 } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 { month_prime + 3 } else { month_prime - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// ODD Phase 7-remainder: open server span for one RPC request.
///
/// Builds a schema-valid [`emit::Span`] (closed exactly once via
/// [`RpcServerSpan::finish`]) carrying the redacted request summary plus
/// the `session_id` join key into the sessions undo history. Returns
/// `None` when telemetry is disabled, so callers can skip all span work.
#[must_use]
pub fn begin_rpc_span(
    parent: &TraceContext,
    method: &str,
    params: &Value,
) -> Option<RpcServerSpan> {
    if !Sampling::enabled() {
        return None;
    }
    let child = parent.child();
    let op = op_for_method(method);
    let started_at = now_rfc3339_millis();
    let mut inner = emit::Span::begin(
        &child,
        Some(parent),
        &op,
        "rs.server/rpc",
        session_id_of(params).as_deref(),
        None,
        &started_at,
    );
    if let Some(summary) = redacted_summary(method, params).as_object() {
        for (key, value) in summary {
            inner.set_attr(key, value.clone());
        }
    }
    inner.set_attr("transport", Value::from("http-rpc"));
    Some(RpcServerSpan { inner, ctx: child, method: method.to_string() })
}

/// Open server span handle. Finish exactly once via [`RpcServerSpan::finish`].
pub struct RpcServerSpan {
    inner: emit::Span,
    ctx: TraceContext,
    method: String,
}

impl RpcServerSpan {
    /// Child context this span runs under (links error events to the span).
    #[must_use]
    pub const fn ctx(&self) -> &TraceContext {
        &self.ctx
    }

    /// RPC method this span covers.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Closes the span into a schema-valid telemetry/v1 record.
    #[must_use]
    pub fn finish(self, status_ok: bool) -> Value {
        self.inner.finish(status_ok)
    }
}

/// ODD Phase 7-remainder: classifies a dispatch failure without raw values.
///
/// Every branch matches a fixed message prefix emitted by [`dispatch_rpc`]
/// or the HTTP layer, so the returned kind never carries paths, tokens,
/// or file contents into telemetry.
#[must_use]
pub fn rpc_error_kind(message: &str) -> &'static str {
    if message.starts_with("unknown RPC method") {
        "unknown_method"
    } else if message.starts_with("missing string parameter") {
        "missing_param"
    } else if message.starts_with("stat ") || message.starts_with("path not found") {
        "io_missing"
    } else if message.starts_with("read ") || message.starts_with("reading ") {
        "io_read"
    } else if message.contains("exceeds") {
        "body_limit"
    } else if message.starts_with("invalid RPC JSON") {
        "bad_json"
    } else if message.starts_with("unauthorized") {
        "auth"
    } else if message.starts_with("operation cancelled") {
        "cancelled"
    } else {
        "engine"
    }
}

/// ODD Phase 7-remainder: builds the error event linked to a server span.
///
/// The event carries `op`, `method`, and the classified `error_kind` only:
/// the raw dispatch message (which may quote absolute paths) never enters
/// telemetry. Returns `None` when telemetry is disabled.
#[must_use]
pub fn rpc_error_event(ctx: &TraceContext, method: &str, message: &str) -> Option<Value> {
    if !Sampling::enabled() {
        return None;
    }
    let kind = rpc_error_kind(message);
    let mut event = emit::error_event(
        ctx,
        "server.rpc_error",
        "rs.server/rpc",
        &format!("{method}:{kind}"),
        &now_rfc3339_millis(),
    );
    if let Some(attrs) = event
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
    {
        attrs.insert("op".to_string(), Value::from(op_for_method(method)));
        attrs.insert("method".to_string(), Value::from(method));
        attrs.insert("error_kind".to_string(), Value::from(kind));
    }
    Some(event)
}

/// Outcome of a traced dispatch: the wire-identical result plus optional
/// closed telemetry records. `span` / `error` are `None` whenever
/// telemetry is disabled or the request was untraced.
#[derive(Debug, Clone)]
pub struct RpcTraceOutcome {
    /// Exactly what [`dispatch_rpc`] returns: untouched by tracing.
    pub result: Result<Value, String>,
    /// Closed telemetry/v1 span record for the request.
    pub span: Option<Value>,
    /// Error event linked to the span, present only on failure.
    pub error: Option<Value>,
}

/// ODD Phase 7-remainder: dispatches one RPC method with a full server span.
///
/// Reuses [`incoming_parent_context`] + [`emit`]: resolves the parent,
/// opens the child span, runs [`dispatch_rpc`], then closes the span with
/// `ok`/`error` status and classifies failures via [`rpc_error_event`].
/// Wire values are byte-identical to [`dispatch_rpc`]; when telemetry is
/// off or the request is untraced both records are `None`.
#[must_use]
pub fn dispatch_traced(
    method: &str,
    params: &Value,
    traceparent: Option<&str>,
) -> RpcTraceOutcome {
    let Some(parent) = incoming_parent_context(traceparent, params) else {
        return RpcTraceOutcome { result: dispatch_rpc(method, params), span: None, error: None };
    };
    let Some(span) = begin_rpc_span(&parent, method, params) else {
        return RpcTraceOutcome { result: dispatch_rpc(method, params), span: None, error: None };
    };
    let result = dispatch_rpc(method, params);
    let status_ok = result.is_ok();
    let error = if let Err(message) = &result {
        rpc_error_event(span.ctx(), method, message)
    } else {
        None
    };
    let record = span.finish(status_ok);
    RpcTraceOutcome { result, span: Some(record), error }
}

/// Dispatches an RPC command to the AutoCompare core crates.
///
/// # Errors
///
/// Returns an error message string if parameters are missing or the operation fails.
pub fn dispatch_rpc(method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "ping" | "health" => Ok(json!({
            "status": "ok",
            "transport": "http-rpc",
            "version": env!("CARGO_PKG_VERSION"),
        })),
        "vfs_schemes" => {
            let reg = VfsRegistry::with_builtins();
            Ok(json!({ "schemes": reg.schemes() }))
        }
        "vfs_list_dir" => {
            let uri = req_str(params, "uri")?;
            let reg = VfsRegistry::with_builtins();
            let (vfs, subpath) = reg.open_with_subpath(uri).map_err(|e| e.to_string())?;
            let entries = vfs.list_dir(&subpath).map_err(|e| e.to_string())?;
            let mapped: Vec<Value> = entries
                .into_iter()
                .map(|e| {
                    json!({
                        "name": e.name,
                        "path": e.path.as_str(),
                        "is_dir": e.meta.is_dir,
                        "len": e.meta.len,
                    })
                })
                .collect();
            Ok(json!({ "entries": mapped }))
        }
        "read_text_file" => {
            let path = req_str(params, "path")?;
            let meta = fs::metadata(path).map_err(|e| format!("stat '{path}': {e}"))?;
            if meta.len() > MAX_RPC_BODY_BYTES as u64 {
                return Err(format!("file '{path}' exceeds 16 MiB read limit"));
            }
            let content = fs::read_to_string(path).map_err(|e| format!("read '{path}': {e}"))?;
            Ok(json!({ "path": path, "content": content }))
        }
        "format_convert" => {
            let path = req_str(params, "path")?;
            let content = req_str(params, "content")?;
            let reg = FormatConverterRegistry::with_builtins();
            match reg.convert(path, content.as_bytes()) {
                Ok(Some(bytes)) => Ok(json!({
                    "converted": true,
                    "content": String::from_utf8_lossy(&bytes),
                })),
                Ok(None) => Ok(json!({
                    "converted": false,
                    "content": content,
                })),
                Err(e) => Err(e.to_string()),
            }
        }
        "compare_text" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let opts = TextOptions {
                ignore_case: params
                    .get("ignore_case")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                ignore_space_change: params
                    .get("ignore_space_change")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                ..Default::default()
            };
            let cmp = compare_text_files(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            let equal = cmp
                .script
                .ops
                .iter()
                .all(|op| matches!(op, Op::Equal { .. }));
            Ok(json!({
                "left_lines": cmp.left_lines,
                "right_lines": cmp.right_lines,
                "left_count": cmp.left_lines.len(),
                "right_count": cmp.right_lines.len(),
                "equal": equal,
            }))
        }
        "compare_folders" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let opts = FolderOptions {
                content_compare: params
                    .get("content_compare")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                case_insensitive: params
                    .get("case_insensitive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                depth_limit: params
                    .get("depth_limit")
                    .and_then(Value::as_u64)
                    .and_then(|d| usize::try_from(d).ok())
                    .unwrap_or(32),
                ..Default::default()
            };
            let cmp = compare_folders(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&cmp).map_err(|e| e.to_string())
        }
        "compare_table" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let delimiter = params
                .get("delimiter")
                .and_then(Value::as_str)
                .and_then(|s| s.chars().next())
                .unwrap_or(',');
            let header = params
                .get("header")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let opts = TableOptions { delimiter, header };
            let cmp = compare_table_files(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "header_l": cmp.header_l,
                "header_r": cmp.header_r,
                "rows_l": cmp.rows_l,
                "rows_r": cmp.rows_r,
                "equal": cmp.script.ops.iter().all(|op| matches!(op, Op::Equal { .. })),
            }))
        }
        "plan_sync" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let direction = match params
                .get("direction")
                .and_then(Value::as_str)
                .unwrap_or("l2r")
            {
                "r2l" | "right_to_left" => SyncDirection::RightToLeft,
                "newer" | "newer_wins" => SyncDirection::NewerWins,
                _ => SyncDirection::LeftToRight,
            };
            let prune = params
                .get("prune")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let opts = SyncOptions { direction, prune };
            let plan = plan_sync(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&plan).map_err(|e| e.to_string())
        }
        other => Err(format!("unknown RPC method '{other}'")),
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

    #[test]
    fn traceparent_propagates_to_child_with_same_trace() {
        use autocompare_telemetry::TraceContext;

        // Header path: child shares the trace, mints a fresh span.
        let parent = TraceContext::fresh(true);
        let header = parent.to_traceparent();
        let params = json!({});
        let child = resolve_trace_context(Some(&header), &params).expect("child");
        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);

        // Envelope fallback path (Tauri-style __trace in params).
        let via_envelope = resolve_trace_context(
            None,
            &json!({ "__trace": parent.to_envelope() }),
        )
        .expect("envelope child");
        assert_eq!(via_envelope.trace_id, parent.trace_id);

        // Header wins over envelope when both are present.
        let other = TraceContext::fresh(true);
        let both = resolve_trace_context(
            Some(&header),
            &json!({ "__trace": other.to_envelope() }),
        )
        .expect("both");
        assert_eq!(both.trace_id, parent.trace_id);

        // Absent or malformed input yields no span (shapes unchanged).
        assert!(resolve_trace_context(None, &json!({})).is_none());
        assert!(resolve_trace_context(Some("bogus"), &json!({})).is_none());
        assert!(
            resolve_trace_context(None, &json!({ "__trace": { "trace_id": "zz" } })).is_none()
        );
    }

    #[test]
    fn redacted_summary_never_carries_raw_values() {
        let params = json!({
            "left": "/secret/home/left.txt",
            "right": "/secret/home/right.txt",
            "content": "super-secret-bytes",
            "ignore_case": true,
        });
        let summary = redacted_summary("compare_text", &params);
        let flat = summary.to_string();
        assert!(!flat.contains("/secret/home"));
        assert!(!flat.contains("super-secret-bytes"));
        assert_eq!(
            summary.get("op").and_then(Value::as_str),
            Some("text.compare")
        );
        // Opaque tokens + shapes survive redaction.
        let paths = summary.get("paths").expect("paths");
        assert!(paths.get("left").and_then(Value::as_str).is_some_and(|t| t.starts_with("p_")));
        assert_eq!(
            summary.get("content_bytes").and_then(Value::as_u64),
            Some(18)
        );

        // Registry joins for the engine ops, namespaced fallback otherwise.
        assert_eq!(op_for_method("compare_folders"), "folder.compare");
        assert_eq!(op_for_method("plan_sync"), "sync.plan");
        assert_eq!(op_for_method("vfs_list_dir"), "server.vfs.list.dir");

        // Trace echo only touches object results with a live context.
        use autocompare_telemetry::TraceContext;
        let ctx = TraceContext::fresh(true);
        let echoed = with_trace_echo(json!({"ok": true}), Some(&ctx));
        assert!(echoed.get("__trace").is_some());
        assert!(with_trace_echo(json!({"ok": true}), None).get("__trace").is_none());
        let scalar = with_trace_echo(json!(42), Some(&ctx));
        assert_eq!(scalar, json!(42));
    }

    #[test]
    fn dispatches_compare_text_table_folder_and_plan_sync() {
        let base = std::env::temp_dir().join(format!(
            "autocompare_rpc_test_{}",
            std::process::id()
        ));
        let left_dir = base.join("left");
        let right_dir = base.join("right");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&left_dir).expect("mkdir left");
        fs::create_dir_all(&right_dir).expect("mkdir right");

        let l_txt = left_dir.join("a.txt");
        let r_txt = right_dir.join("a.txt");
        fs::write(&l_txt, "hello\nworld\n").expect("write l_txt");
        fs::write(&r_txt, "hello\nrust\n").expect("write r_txt");

        let l_csv = left_dir.join("t.csv");
        let r_csv = right_dir.join("t.csv");
        fs::write(&l_csv, "id,name\n1,Alice\n").expect("write l_csv");
        fs::write(&r_csv, "id,name\n1,Alice\n").expect("write r_csv");

        let txt_res = dispatch_rpc(
            "compare_text",
            &json!({ "left": l_txt.to_string_lossy(), "right": r_txt.to_string_lossy() }),
        )
        .expect("compare_text");
        assert_eq!(txt_res.get("equal"), Some(&json!(false)));
        assert_eq!(txt_res.get("left_count"), Some(&json!(2)));

        let tbl_res = dispatch_rpc(
            "compare_table",
            &json!({ "left": l_csv.to_string_lossy(), "right": r_csv.to_string_lossy() }),
        )
        .expect("compare_table");
        assert_eq!(tbl_res.get("equal"), Some(&json!(true)));

        let fld_res = dispatch_rpc(
            "compare_folders",
            &json!({ "left": left_dir.to_string_lossy(), "right": right_dir.to_string_lossy() }),
        )
        .expect("compare_folders");
        assert!(fld_res.get("entries").and_then(Value::as_array).is_some());

        let sync_res = dispatch_rpc(
            "plan_sync",
            &json!({ "left": left_dir.to_string_lossy(), "right": right_dir.to_string_lossy(), "direction": "l2r" }),
        )
        .expect("plan_sync");
        assert!(sync_res.get("steps").and_then(Value::as_array).is_some());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn dispatch_traced_closes_complete_schema_valid_span() {
        use autocompare_telemetry::TraceContext;

        let parent = TraceContext::fresh(true);
        let header = parent.to_traceparent();
        let trace_hex = parent
            .to_envelope()
            .get("trace_id")
            .and_then(Value::as_str)
            .expect("trace hex")
            .to_string();

        // Traced success: wire result identical to dispatch_rpc, plus a
        // closed span record joined to the incoming trace and session.
        let params = json!({ "session_id": "sess-join-1" });
        let outcome = dispatch_traced("ping", &params, Some(&header));
        let plain = dispatch_rpc("ping", &params).expect("ping");
        assert_eq!(outcome.result.expect("traced ping"), plain);

        let span = outcome.span.expect("span record");
        assert_eq!(span.get("schema_version").and_then(Value::as_u64), Some(1));
        assert_eq!(span.get("kind").and_then(Value::as_str), Some("span"));
        assert_eq!(span.get("trace_id").and_then(Value::as_str), Some(trace_hex.as_str()));
        assert_eq!(span.get("op").and_then(Value::as_str), Some("server.health"));
        assert_eq!(span.get("component").and_then(Value::as_str), Some("rs.server/rpc"));
        assert_eq!(span.get("status").and_then(Value::as_str), Some("ok"));
        assert_eq!(span.get("session_id").and_then(Value::as_str), Some("sess-join-1"));
        // Same trace, fresh span id: the server span is a child, not a clone.
        assert_ne!(
            span.get("span_id").and_then(Value::as_str),
            parent.to_envelope().get("span_id").and_then(Value::as_str)
        );
        assert!(span.get("started_at").and_then(Value::as_str).is_some());
        assert!(span.get("duration_ns").and_then(Value::as_u64).is_some());
        assert!(outcome.error.is_none());

        // Foreign parent trace links correctly (header wins over envelope).
        let other = TraceContext::fresh(true);
        let other_hex = other
            .to_envelope()
            .get("trace_id")
            .and_then(Value::as_str)
            .expect("other hex")
            .to_string();
        let both = dispatch_traced(
            "ping",
            &json!({ "__trace": other.to_envelope() }),
            Some(&header),
        );
        assert!(both.result.is_ok());
        let both_span = both.span.as_ref().expect("span");
        assert_eq!(
            both_span.get("trace_id").and_then(Value::as_str),
            Some(trace_hex.as_str())
        );
        assert_ne!(
            both_span.get("trace_id").and_then(Value::as_str),
            Some(other_hex.as_str())
        );

        // Untraced dispatch: byte-identical values, no records at all.
        let untraced = dispatch_traced("ping", &json!({}), None);
        let untraced_plain = dispatch_rpc("ping", &json!({})).expect("ping");
        assert_eq!(untraced.result.expect("ping"), untraced_plain);
        assert!(untraced.span.is_none());
        assert!(untraced.error.is_none());
    }

    #[test]
    fn dispatch_traced_errors_carry_kind_not_raw_values() {
        use autocompare_telemetry::TraceContext;

        let parent = TraceContext::fresh(true);
        let header = parent.to_traceparent();

        // Unknown method: Err value untouched, error event classified.
        let outcome = dispatch_traced("nope_missing", &json!({}), Some(&header));
        let Err(message) = outcome.result else {
            panic!("expected unknown-method error");
        };
        assert!(message.contains("unknown RPC method"));
        let span = outcome.span.expect("error span");
        assert_eq!(span.get("status").and_then(Value::as_str), Some("error"));
        let event = outcome.error.expect("error event");
        assert_eq!(event.get("schema_version").and_then(Value::as_u64), Some(1));
        assert_eq!(event.get("kind").and_then(Value::as_str), Some("event"));
        assert_eq!(event.get("severity").and_then(Value::as_str), Some("error"));
        let attrs = event.get("attributes").expect("attributes");
        assert_eq!(attrs.get("error_kind").and_then(Value::as_str), Some("unknown_method"));

        // Missing-param failure quotes nothing raw: classification only.
        let missing = dispatch_traced("compare_text", &json!({}), Some(&header));
        assert!(missing.result.is_err());
        let missing_event = missing.error.expect("missing event");
        assert_eq!(
            missing_event
                .get("attributes")
                .and_then(|a| a.get("error_kind"))
                .and_then(Value::as_str),
            Some("missing_param")
        );
        let flat = format!("{} {}", missing.span.expect("span"), missing_event);
        assert!(!flat.contains("super-secret"));

        // IO-shaped failure never leaks the absolute path into telemetry.
        let secret_path = "/secret/home/autocompare/missing.txt";
        let io = dispatch_traced(
            "compare_text",
            &json!({ "left": secret_path, "right": secret_path }),
            Some(&header),
        );
        assert!(io.result.is_err());
        let io_event = io.error.expect("io event");
        assert_eq!(
            io_event
                .get("attributes")
                .and_then(|a| a.get("error_kind"))
                .and_then(Value::as_str),
            Some("io_missing")
        );
        for record in [io.span.expect("io span"), io_event] {
            let flat = record.to_string();
            assert!(!flat.contains("/secret/home"), "raw path leaked: {flat}");
        }

        // Error-kind taxonomy covers every dispatch failure shape.
        assert_eq!(rpc_error_kind("unknown RPC method 'x'"), "unknown_method");
        assert_eq!(rpc_error_kind("missing string parameter 'left'"), "missing_param");
        assert_eq!(rpc_error_kind("stat '/a': nope"), "io_missing");
        assert_eq!(rpc_error_kind("path not found: /a"), "io_missing");
        assert_eq!(rpc_error_kind("read '/a': nope"), "io_read");
        assert_eq!(rpc_error_kind("reading /a failed: nope"), "io_read");
        assert_eq!(rpc_error_kind("request body exceeds 1 bytes"), "body_limit");
        assert_eq!(rpc_error_kind("invalid RPC JSON: boom"), "bad_json");
        assert_eq!(rpc_error_kind("unauthorized: nope"), "auth");
        assert_eq!(rpc_error_kind("operation cancelled"), "cancelled");
        assert_eq!(rpc_error_kind("something else entirely"), "engine");
    }

    #[test]
    fn session_join_key_accepts_aliases_and_rejects_bloat() {
        assert_eq!(
            session_id_of(&json!({ "session_id": "sess-1" })).as_deref(),
            Some("sess-1")
        );
        assert_eq!(
            session_id_of(&json!({ "sessionId": "sess-2" })).as_deref(),
            Some("sess-2")
        );
        assert_eq!(
            session_id_of(&json!({ "session": "sess-3" })).as_deref(),
            Some("sess-3")
        );
        assert!(session_id_of(&json!({})).is_none());
        assert!(session_id_of(&json!({ "session_id": "" })).is_none());
        // Overlong values are treated as absent, never truncated into spans.
        let bloated = "s".repeat(129);
        assert!(session_id_of(&json!({ "session_id": bloated })).is_none());
    }

    #[test]
    fn engine_ops_join_ops_registry_which_stays_read_only() {
        // ops.json is read-only for this phase: the test loads it from disk
        // and asserts the join, never writes it.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let registry_path = format!("{manifest}/../../telemetry/schema/v1/ops.json");
        let text = std::fs::read_to_string(&registry_path).expect("ops.json readable");
        let registry: Value = serde_json::from_str(&text).expect("ops.json valid");
        let ops = registry.get("ops").expect("ops map");

        // Every engine op the server maps must exist in the registry with a
        // matching component, so traces join the documented taxonomy.
        for (method, expected_op) in [
            ("compare_text", "text.compare"),
            ("compare_table", "table.compare"),
            ("compare_folders", "folder.compare"),
            ("plan_sync", "sync.plan"),
        ] {
            let op = op_for_method(method);
            assert_eq!(op, expected_op);
            let entry = ops.get(op.as_str()).expect("registry entry");
            assert!(
                entry
                    .get("component")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.starts_with("rs.")),
                "registry entry for {op} must name an rs component"
            );
        }
        // Non-engine methods stay namespaced under server.* (collision-free,
        // never claiming a registry op they do not implement).
        assert_eq!(op_for_method("vfs_list_dir"), "server.vfs.list.dir");
        assert!(ops.get("server.vfs.list.dir").is_none());

        // Timestamps are schema-shaped RFC 3339 UTC millis.
        let stamp = now_rfc3339_millis();
        assert!(stamp.len() == 24, "unexpected stamp shape: {stamp}");
        assert!(stamp.ends_with('Z'));
        assert!(stamp.contains('T'));
    }
}
