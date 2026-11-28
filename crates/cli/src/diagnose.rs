//! ODD Phase 11: automated diagnostics + fix-validation (`trace diagnose`).
//!
//! Heuristics for the top-5 known failure shapes (contract drift, serde
//! mismatch, missing echo, timeout, panic). Each maps trace patterns to a
//! probable cause plus a suggested fix. Also computes trace completeness
//! (percent of spans carrying both endpoints: `started_at` + `duration_ns`)
//! and the error-span-zero check, exposed in both human and `--json`
//! (stable snake_case) output. Offline only; never touches the network.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::error::CliError;
use crate::outcome::Outcome;
use crate::trace::{
    TRACE_JSON_SCHEMA_VERSION, is_error_record, is_span, load_telemetry_dir, records_for_trace,
    redact_record,
};

/// Duration at or above which a span looks like a timeout (5s in nanos).
pub const TIMEOUT_NS_THRESHOLD: u64 = 5_000_000_000;

/// Failure shapes surfaced by `trace diagnose` (snake_case, stable).
pub const SHAPE_CONTRACT_DRIFT: &str = "contract_drift";
pub const SHAPE_SERDE_MISMATCH: &str = "serde_mismatch";
pub const SHAPE_MISSING_ECHO: &str = "missing_echo";
pub const SHAPE_TIMEOUT: &str = "timeout";
pub const SHAPE_PANIC: &str = "panic";
pub const SHAPE_UNKNOWN: &str = "unknown";
pub const SHAPE_OK: &str = "ok";

/// One classified failure with cause + fix guidance.
#[derive(Debug, Clone)]
pub struct Finding {
    /// One of the `SHAPE_*` constants.
    pub failure_shape: String,
    /// Human-readable probable cause (mentions the evidence).
    pub likely_cause: String,
    /// Actionable fix suggestion (file/layer + concrete edit).
    pub suggested_fix: String,
    /// Owning trace.
    pub trace_id: String,
    /// Best-effort offending span (may be empty for trace-level shapes).
    pub span_id: String,
    /// Op of the offending span (may be empty).
    pub op: String,
    /// Component of the offending span/event (may be empty).
    pub component: String,
    /// Small matched-evidence map for debugging.
    pub evidence: BTreeMap<String, String>,
}

/// Completeness for one trace (ODD section 7 + endpoint check).
#[derive(Debug, Clone)]
pub struct TraceCompleteness {
    /// Closed spans seen.
    pub spans_total: u64,
    /// Spans carrying both `started_at` and `duration_ns`.
    pub spans_complete: u64,
    /// `spans_total - spans_complete`.
    pub spans_incomplete: u64,
    /// `100 * spans_complete / spans_total` (100 when empty).
    pub completeness_pct: u64,
    /// Spans with `status: error`.
    pub error_spans: u64,
    /// Events with `severity: error`.
    pub error_events: u64,
    /// True when no error spans and no error events.
    pub error_span_zero: bool,
    /// Spans with null/missing `parent_span_id`.
    pub roots: u64,
    /// Non-root spans whose parent id resolves to no known span.
    pub orphans: u64,
    /// Overall verdict (one root, no orphans, all statuses ok|error, full endpoints).
    pub complete: bool,
    /// Machine-readable violation tokens (empty when complete).
    pub reasons: Vec<String>,
}

/// Full diagnosis for one trace.
#[derive(Debug, Clone)]
pub struct TraceDiagnosis {
    /// Trace under diagnosis.
    pub trace_id: String,
    /// Completeness scoring.
    pub completeness: TraceCompleteness,
    /// Classified findings (empty on clean traces).
    pub findings: Vec<Finding>,
}

/// Aggregate diagnosis over a telemetry root.
#[derive(Debug, Clone)]
pub struct DiagnoseReport {
    /// Distinct traces seen.
    pub traces_total: u64,
    /// Closed spans seen.
    pub spans_total: u64,
    /// Spans with both endpoints.
    pub spans_complete: u64,
    /// Aggregate `100 * spans_complete / spans_total` (100 when empty).
    pub completeness_pct: u64,
    /// Error spans across all traces.
    pub error_spans: u64,
    /// Error events across all traces.
    pub error_events: u64,
    /// True when zero error spans and zero error events.
    pub error_span_zero: bool,
    /// Traces whose per-trace verdict is complete.
    pub traces_complete: u64,
    /// Per-trace breakdown, sorted by trace id.
    pub traces: Vec<TraceDiagnosis>,
}

fn record_str(record: &serde_json::Value, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn attr_str(record: &serde_json::Value, key: &str) -> Option<String> {
    record
        .get("attributes")
        .and_then(serde_json::Value::as_object)
        .and_then(|attrs| attrs.get(key))
        .and_then(|value| match value {
            serde_json::Value::String(text) => Some(text.clone()),
            serde_json::Value::Number(number) => Some(number.to_string()),
            serde_json::Value::Bool(flag) => Some(flag.to_string()),
            _ => None,
        })
}

fn attr_map(record: &serde_json::Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(attrs) = record
        .get("attributes")
        .and_then(serde_json::Value::as_object)
    {
        for (key, value) in attrs {
            let rendered = match value {
                serde_json::Value::String(text) => text.clone(),
                serde_json::Value::Number(number) => number.to_string(),
                serde_json::Value::Bool(flag) => flag.to_string(),
                serde_json::Value::Null => String::new(),
                _ => String::new(),
            };
            if !rendered.is_empty() {
                out.insert(key.clone(), rendered);
            }
        }
    }
    out
}

fn component_of(record: &serde_json::Value) -> String {
    if let Some(component) = record_str(record, "component")
        && !component.is_empty()
    {
        return component;
    }
    attr_str(record, "component").unwrap_or_default()
}

fn message_of(record: &serde_json::Value) -> String {
    for key in ["message", "error", "detail", "panic_message", "stderr_tail"] {
        if let Some(text) = attr_str(record, key)
            && !text.is_empty()
        {
            return text;
        }
    }
    attr_str(record, "likely_cause").unwrap_or_default()
}

fn event_name_of(record: &serde_json::Value) -> String {
    record_str(record, "name").unwrap_or_default()
}

/// Lowercase + strip `_`/`-` so `dry_run` and `dryRun` compare equal.
fn normalize_shape(text: &str) -> String {
    text.bytes()
        .filter(|byte| *byte != b'_' && *byte != b'-')
        .map(|byte| byte.to_ascii_lowercase())
        .map(char::from)
        .collect()
}

fn contains_fold(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    haystack.to_ascii_lowercase().contains(needle)
}

/// True when the span carries both endpoints (`started_at` + `duration_ns`).
pub fn span_has_both_endpoints(record: &serde_json::Value) -> bool {
    let has_start = record
        .get("started_at")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|text| !text.is_empty());
    let has_duration = record
        .get("duration_ns")
        .and_then(serde_json::Value::as_u64)
        .is_some();
    has_start && has_duration
}

fn status_ok_or_error(record: &serde_json::Value) -> bool {
    matches!(
        record.get("status").and_then(serde_json::Value::as_str),
        Some("ok") | Some("error")
    )
}

/// Compute completeness for one trace's records (spans + events mixed).
pub fn completeness_for_trace(trace_id: &str, trace_records: &[&serde_json::Value]) -> TraceCompleteness {
    let mut spans_total: u64 = 0;
    let mut spans_complete: u64 = 0;
    let mut error_spans: u64 = 0;
    let mut error_events: u64 = 0;
    let mut roots: u64 = 0;
    let mut ids: BTreeSet<String> = BTreeSet::new();
    let mut reasons: Vec<String> = Vec::new();

    for record in trace_records {
        if is_span(record) {
            spans_total = spans_total.saturating_add(1);
            if span_has_both_endpoints(record) {
                spans_complete = spans_complete.saturating_add(1);
            }
            if record.get("status").and_then(serde_json::Value::as_str) == Some("error") {
                error_spans = error_spans.saturating_add(1);
            }
            if let Some(span_id) = record_str(record, "span_id") {
                ids.insert(span_id);
            }
        } else if record.get("kind").and_then(serde_json::Value::as_str) == Some("event")
            && is_error_record(record)
        {
            error_events = error_events.saturating_add(1);
        }
    }

    let mut orphan_refs: Vec<String> = Vec::new();
    let mut bad_status: u64 = 0;
    let mut missing_endpoint: u64 = 0;
    for record in trace_records {
        if !is_span(record) {
            continue;
        }
        let parent = record.get("parent_span_id");
        let is_root = parent.is_none_or(serde_json::Value::is_null);
        if is_root {
            roots = roots.saturating_add(1);
        } else if let Some(parent_id) = parent.and_then(serde_json::Value::as_str) {
            if !ids.contains(parent_id) {
                orphan_refs.push(parent_id.to_string());
            }
        } else {
            orphan_refs.push(String::from("bad_parent"));
        }
        if !status_ok_or_error(record) {
            bad_status = bad_status.saturating_add(1);
        }
        if !span_has_both_endpoints(record) {
            missing_endpoint = missing_endpoint.saturating_add(1);
        }
    }
    let orphans = orphan_refs.len() as u64;
    let _ = trace_id;

    if roots != 1 {
        reasons.push(format!("roots:{roots}"));
    }
    if orphans > 0 {
        reasons.push(format!("orphans:{orphans}"));
    }
    if bad_status > 0 {
        reasons.push(format!("bad_status:{bad_status}"));
    }
    if missing_endpoint > 0 {
        reasons.push(format!("missing_endpoints:{missing_endpoint}"));
    }

    let spans_incomplete = spans_total.saturating_sub(spans_complete);
    let completeness_pct = if spans_total == 0 {
        100
    } else {
        spans_complete.saturating_mul(100).saturating_div(spans_total)
    };
    let complete = roots == 1 && orphans == 0 && bad_status == 0 && missing_endpoint == 0;
    TraceCompleteness {
        spans_total,
        spans_complete,
        spans_incomplete,
        completeness_pct,
        error_spans,
        error_events,
        error_span_zero: error_spans == 0 && error_events == 0,
        roots,
        orphans,
        complete,
        reasons,
    }
}

/// Distinct trace ids in load order.
pub fn distinct_trace_ids(records: &[crate::trace::LoadedRecord]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for loaded in records {
        if let Some(trace_id) = loaded
            .record
            .get("trace_id")
            .and_then(serde_json::Value::as_str)
            && !ids.iter().any(|known| known == trace_id)
        {
            ids.push(trace_id.to_string());
        }
    }
    ids.sort();
    ids
}

fn has_uppercase_key(record: &serde_json::Value) -> Option<String> {
    record
        .get("attributes")
        .and_then(serde_json::Value::as_object)
        .and_then(|attrs| {
            attrs
                .keys()
                .find(|key| key.bytes().any(|byte| byte.is_ascii_uppercase()))
                .cloned()
        })
}

/// Contract drift: `field` (snake) vs `got` (camel) that match once case and
/// underscores are ignored, or any camelCase attribute key leaking through.
fn drift_evidence(record: &serde_json::Value) -> Option<(String, String)> {
    let attrs = attr_map(record);
    if let (Some(field), Some(got)) = (attrs.get("field"), attrs.get("got"))
        && field != got
        && normalize_shape(field) == normalize_shape(got)
    {
        return Some((field.clone(), got.clone()));
    }
    if let (Some(field), Some(expected)) = (attrs.get("field"), attrs.get("expected"))
        && field != expected
        && normalize_shape(field) == normalize_shape(expected)
    {
        return Some((field.clone(), expected.clone()));
    }
    if let Some(camel) = has_uppercase_key(record) {
        let snake_guess: String = camel
            .bytes()
            .flat_map(|byte| {
                if byte.is_ascii_uppercase() {
                    vec![b'_', byte.to_ascii_lowercase()]
                } else {
                    vec![byte]
                }
            })
            .map(char::from)
            .collect();
        return Some((snake_guess, camel));
    }
    None
}

fn is_serde_record(record: &serde_json::Value) -> bool {
    let name = event_name_of(record);
    if contains_fold(&name, "serde") || contains_fold(&name, "deserial") {
        return true;
    }
    let attrs = attr_map(record);
    if attrs.contains_key("expected") || attrs.contains_key("got") || attrs.contains_key("field") {
        return true;
    }
    let message = message_of(record);
    for marker in [
        "serde",
        "deserial",
        "missing field",
        "invalid type",
        "expected",
        "got",
    ] {
        if contains_fold(&message, marker) {
            return true;
        }
    }
    false
}

fn is_timeout_record(record: &serde_json::Value) -> bool {
    let name = event_name_of(record);
    if contains_fold(&name, "timeout") || contains_fold(&name, "deadline") {
        return true;
    }
    let attrs = attr_map(record);
    for key in attrs.keys() {
        if contains_fold(key, "timeout") || contains_fold(key, "deadline") {
            return true;
        }
    }
    for value in attrs.values() {
        if contains_fold(value, "timeout")
            || contains_fold(value, "timed out")
            || contains_fold(value, "deadline exceeded")
            || contains_fold(value, "deadline_exceeded")
        {
            return true;
        }
    }
    if let Some(duration) = record.get("duration_ns").and_then(serde_json::Value::as_u64)
        && duration >= TIMEOUT_NS_THRESHOLD
    {
        return true;
    }
    contains_fold(&message_of(record), "timeout")
        || contains_fold(&message_of(record), "timed out")
        || contains_fold(&message_of(record), "deadline")
}

fn is_panic_record(record: &serde_json::Value) -> bool {
    let name = event_name_of(record);
    if contains_fold(&name, "panic") {
        return true;
    }
    let attrs = attr_map(record);
    for key in attrs.keys() {
        if contains_fold(key, "panic") {
            return true;
        }
    }
    for value in attrs.values() {
        if contains_fold(value, "panic") || contains_fold(value, "catch_unwind") {
            return true;
        }
    }
    let component = component_of(record);
    if contains_fold(&component, "panic") {
        return true;
    }
    let message = message_of(record);
    contains_fold(&message, "panic") || contains_fold(&message, "catch_unwind")
}

fn is_missing_echo_trace(trace_records: &[&serde_json::Value]) -> bool {
    for record in trace_records {
        let attrs = attr_map(record);
        for (key, value) in &attrs {
            if contains_fold(key, "missing_echo") {
                return true;
            }
            if contains_fold(key, "echo") && contains_fold(value, "missing") {
                return true;
            }
        }
        let message = message_of(record);
        if contains_fold(&message, "missing echo") || contains_fold(&message, "echo missing") {
            return true;
        }
        if contains_fold(&event_name_of(record), "missing_echo") {
            return true;
        }
    }
    let mut has_ts = false;
    let mut has_rs = false;
    for record in trace_records {
        if !is_span(record) {
            continue;
        }
        let component = component_of(record);
        if component.starts_with("ts.") {
            has_ts = true;
        }
        if component.starts_with("rs.") {
            has_rs = true;
        }
    }
    has_ts && !has_rs
}

fn error_span_or_event<'a>(trace_records: &'a [&'a serde_json::Value]) -> Vec<&'a serde_json::Value> {
    trace_records
        .iter()
        .copied()
        .filter(|record| is_error_record(record))
        .collect()
}

/// Classify one trace into findings (empty on clean traces).
pub fn diagnose_trace_records(
    trace_id: &str,
    trace_records: &[&serde_json::Value],
) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let errors = error_span_or_event(trace_records);
    // Events carry no `op`; backfill from the sibling span with the same id.
    let mut op_by_span: BTreeMap<String, String> = BTreeMap::new();
    for record in trace_records {
        if is_span(record)
            && let (Some(span_id), Some(op)) = (record_str(record, "span_id"), record_str(record, "op"))
        {
            op_by_span.insert(span_id, op);
        }
    }

    // Per-record shapes first (drift beats generic serde on the same record).
    for record in &errors {
        let span_id = record_str(record, "span_id").unwrap_or_default();
        let mut op = record_str(record, "op").unwrap_or_else(|| attr_str(record, "op").unwrap_or_default());
        if op.is_empty() {
            op = op_by_span.get(&span_id).cloned().unwrap_or_default();
        }
        let component = component_of(record);
        if let Some((field, got)) = drift_evidence(record) {
            let mut evidence = BTreeMap::new();
            evidence.insert("field".to_string(), field.clone());
            evidence.insert("got".to_string(), got.clone());
            findings.push(Finding {
                failure_shape: SHAPE_CONTRACT_DRIFT.to_string(),
                likely_cause: format!(
                    "contract drift: layer emitted `{got}` but the telemetry contract expects snake_case `{field}`"
                ),
                suggested_fix: format!(
                    "align field `{got}` to snake_case `{field}` at {component}; fix the emitter (mockAdapter/serde struct) and re-validate against telemetry/schema/v1"
                ),
                trace_id: trace_id.to_string(),
                span_id,
                op,
                component,
                evidence,
            });
        } else if is_timeout_record(record) {
            let mut evidence = BTreeMap::new();
            if let Some(duration) = record.get("duration_ns").and_then(serde_json::Value::as_u64) {
                evidence.insert("duration_ns".to_string(), duration.to_string());
            }
            findings.push(Finding {
                failure_shape: SHAPE_TIMEOUT.to_string(),
                likely_cause: format!(
                    "timeout: span exceeded budget or carried a timeout marker (op `{op}`)"
                ),
                suggested_fix: format!(
                    "increase the timeout for op `{op}` at {component} or fix the hang; check retry/backoff attempts on the parent span"
                ),
                trace_id: trace_id.to_string(),
                span_id,
                op,
                component,
                evidence,
            });
        } else if is_panic_record(record) {
            let mut evidence = BTreeMap::new();
            let message = message_of(record);
            if !message.is_empty() {
                let mut short = message.clone();
                short.truncate(160);
                evidence.insert("message".to_string(), short);
            }
            findings.push(Finding {
                failure_shape: SHAPE_PANIC.to_string(),
                likely_cause: format!("panic: command boundary caught an unwind at {component}"),
                suggested_fix: format!(
                    "fix the panicking code path at {component}; add input guards and keep catch_unwind at the command boundary emitting an error span"
                ),
                trace_id: trace_id.to_string(),
                span_id,
                op,
                component,
                evidence,
            });
        } else if is_serde_record(record) {
            let mut evidence = BTreeMap::new();
            for key in ["field", "expected", "got"] {
                if let Some(value) = attr_str(record, key) {
                    evidence.insert(key.to_string(), value);
                }
            }
            let field = evidence.get("field").cloned().unwrap_or_else(|| "?".to_string());
            findings.push(Finding {
                failure_shape: SHAPE_SERDE_MISMATCH.to_string(),
                likely_cause: format!(
                    "serde mismatch: shape or type break at {component} around field `{field}`"
                ),
                suggested_fix: format!(
                    "fix the serde shape at {component} for field `{field}` (type/rename/optional); validate the envelope against telemetry/schema/v1 before resending"
                ),
                trace_id: trace_id.to_string(),
                span_id,
                op,
                component,
                evidence,
            });
        }
    }

    // Trace-level shape: missing echo (engine spans never arrived).
    if is_missing_echo_trace(trace_records) {
        let already = findings
            .iter()
            .any(|finding| finding.failure_shape == SHAPE_MISSING_ECHO);
        if !already {
            let (span_id, op, component) = trace_records
                .iter()
                .find(|record| is_span(record))
                .map(|record| {
                    (
                        record_str(record, "span_id").unwrap_or_default(),
                        record_str(record, "op").unwrap_or_default(),
                        component_of(record),
                    )
                })
                .unwrap_or_default();
            let mut evidence = BTreeMap::new();
            evidence.insert("layers".to_string(), "ts_present_rs_missing".to_string());
            findings.push(Finding {
                failure_shape: SHAPE_MISSING_ECHO.to_string(),
                likely_cause: "missing echo: ui/ipc spans exist but no rs.* engine spans joined the trace (response __trace echo absent)".to_string(),
                suggested_fix: "echo __trace in every mockAdapter/handler response envelope and verify transport propagation; rerun and expect rs.* spans under the same trace_id".to_string(),
                trace_id: trace_id.to_string(),
                span_id,
                op,
                component,
                evidence,
            });
        }
    }

    // Unclassified errors still surface so the gate never goes silent.
    if findings.is_empty() && !errors.is_empty() {
        let first = errors.first();
        let (span_id, op, component) = first
            .map(|record| {
                (
                    record_str(record, "span_id").unwrap_or_default(),
                    record_str(record, "op").unwrap_or_else(|| attr_str(record, "op").unwrap_or_default()),
                    component_of(record),
                )
            })
            .unwrap_or_default();
        let mut evidence = BTreeMap::new();
        evidence.insert("error_count".to_string(), errors.len().to_string());
        findings.push(Finding {
            failure_shape: SHAPE_UNKNOWN.to_string(),
            likely_cause: format!(
                "unclassified error: {} error record(s) match no known shape",
                errors.len()
            ),
            suggested_fix: format!(
                "inspect the error span/event at {component} (op `{op}`) and extend diagnose heuristics if this shape recurs"
            ),
            trace_id: trace_id.to_string(),
            span_id,
            op,
            component,
            evidence,
        });
    }

    findings
}

/// Diagnose every trace under loaded records.
pub fn diagnose_loaded(records: &[crate::trace::LoadedRecord]) -> DiagnoseReport {
    let values: Vec<&serde_json::Value> =
        records.iter().map(|loaded| &loaded.record).collect();
    diagnose_values(&values)
}

/// Diagnose already-borrowed record values (spans + events mixed).
pub fn diagnose_values(values: &[&serde_json::Value]) -> DiagnoseReport {
    let mut by_trace: BTreeMap<String, Vec<&serde_json::Value>> = BTreeMap::new();
    for value in values {
        if let Some(trace_id) = value
            .get("trace_id")
            .and_then(serde_json::Value::as_str)
        {
            by_trace
                .entry(trace_id.to_string())
                .or_default()
                .push(*value);
        }
    }
    let mut traces: Vec<TraceDiagnosis> = Vec::new();
    let mut spans_total: u64 = 0;
    let mut spans_complete: u64 = 0;
    let mut error_spans: u64 = 0;
    let mut error_events: u64 = 0;
    let mut traces_complete: u64 = 0;
    for (trace_id, trace_records) in &by_trace {
        let completeness = completeness_for_trace(trace_id, trace_records);
        spans_total = spans_total.saturating_add(completeness.spans_total);
        spans_complete = spans_complete.saturating_add(completeness.spans_complete);
        error_spans = error_spans.saturating_add(completeness.error_spans);
        error_events = error_events.saturating_add(completeness.error_events);
        if completeness.complete {
            traces_complete = traces_complete.saturating_add(1);
        }
        let findings = diagnose_trace_records(trace_id, trace_records);
        traces.push(TraceDiagnosis { trace_id: trace_id.clone(), completeness, findings });
    }
    let completeness_pct = if spans_total == 0 {
        100
    } else {
        spans_complete.saturating_mul(100).saturating_div(spans_total)
    };
    DiagnoseReport {
        traces_total: by_trace.len() as u64,
        spans_total,
        spans_complete,
        completeness_pct,
        error_spans,
        error_events,
        error_span_zero: error_spans == 0 && error_events == 0,
        traces_complete,
        traces,
    }
}

fn finding_json(finding: &Finding) -> serde_json::Value {
    let mut evidence = serde_json::Map::new();
    for (key, value) in &finding.evidence {
        evidence.insert(key.clone(), serde_json::Value::from(value.clone()));
    }
    serde_json::json!({
        "failure_shape": finding.failure_shape,
        "likely_cause": finding.likely_cause,
        "suggested_fix": finding.suggested_fix,
        "trace_id": finding.trace_id,
        "span_id": finding.span_id,
        "op": finding.op,
        "component": finding.component,
        "evidence": serde_json::Value::Object(evidence),
    })
}

fn completeness_json(completeness: &TraceCompleteness) -> serde_json::Value {
    serde_json::json!({
        "spans_total": completeness.spans_total,
        "spans_complete": completeness.spans_complete,
        "spans_incomplete": completeness.spans_incomplete,
        "completeness_pct": completeness.completeness_pct,
        "error_spans": completeness.error_spans,
        "error_events": completeness.error_events,
        "error_span_zero": completeness.error_span_zero,
        "roots": completeness.roots,
        "orphans": completeness.orphans,
        "complete": completeness.complete,
        "reasons": completeness.reasons,
    })
}

/// Stable snake_case `--json` envelope for one trace.
pub fn trace_diagnosis_json(diagnosis: &TraceDiagnosis) -> serde_json::Value {
    let findings: Vec<serde_json::Value> = diagnosis.findings.iter().map(finding_json).collect();
    let mut completeness = completeness_json(&diagnosis.completeness);
    if let Some(obj) = completeness.as_object_mut() {
        obj.insert("schema_version".to_string(), serde_json::Value::from(TRACE_JSON_SCHEMA_VERSION));
        obj.insert("trace_id".to_string(), serde_json::Value::from(diagnosis.trace_id.clone()));
        obj.insert("findings".to_string(), serde_json::Value::Array(findings));
    }
    completeness
}

/// Stable snake_case `--json` envelope for a whole telemetry root.
pub fn report_json(report: &DiagnoseReport) -> serde_json::Value {
    let mut all_findings: Vec<serde_json::Value> = Vec::new();
    let mut trace_objs: Vec<serde_json::Value> = Vec::new();
    for trace in &report.traces {
        for finding in &trace.findings {
            all_findings.push(finding_json(finding));
        }
        let mut obj = completeness_json(&trace.completeness);
        if let Some(map) = obj.as_object_mut() {
            map.insert("trace_id".to_string(), serde_json::Value::from(trace.trace_id.clone()));
            let trace_findings: Vec<serde_json::Value> =
                trace.findings.iter().map(finding_json).collect();
            map.insert("findings".to_string(), serde_json::Value::Array(trace_findings));
        }
        trace_objs.push(obj);
    }
    serde_json::json!({
        "schema_version": TRACE_JSON_SCHEMA_VERSION,
        "traces_total": report.traces_total,
        "spans_total": report.spans_total,
        "spans_complete": report.spans_complete,
        "spans_incomplete": report.spans_total.saturating_sub(report.spans_complete),
        "completeness_pct": report.completeness_pct,
        "error_spans": report.error_spans,
        "error_events": report.error_events,
        "error_span_zero": report.error_span_zero,
        "traces_complete": report.traces_complete,
        "findings": all_findings,
        "traces": trace_objs,
    })
}

fn resolve_root(dir: &Option<PathBuf>) -> PathBuf {
    dir.clone().unwrap_or_else(crate::trace::default_telemetry_root)
}

/// Dispatch `trace diagnose`: single trace (`--trace-id`) or whole root.
pub fn run_diagnose(
    trace_id: &Option<String>,
    json: bool,
    dir: &Option<PathBuf>,
) -> Result<Outcome, CliError> {
    let root = resolve_root(dir);
    let loaded = load_telemetry_dir(&root);
    if let Some(want) = trace_id {
        let matching = records_for_trace(&loaded.records, want);
        if matching.is_empty() {
            return Err(CliError::InvalidArg(format!("unknown trace_id: {want}")));
        }
        let redacted: Vec<serde_json::Value> =
            matching.iter().map(|loaded| redact_record(&loaded.record)).collect();
        let refs: Vec<&serde_json::Value> = redacted.iter().collect();
        let completeness = completeness_for_trace(want, &refs);
        let findings = diagnose_trace_records(want, &refs);
        let diagnosis = TraceDiagnosis { trace_id: want.clone(), completeness, findings };
        if json {
            println!("{}", trace_diagnosis_json(&diagnosis));
        } else {
            print_human_trace(&diagnosis);
        }
        return Ok(Outcome::Same);
    }
    let redacted: Vec<serde_json::Value> =
        loaded.records.iter().map(|loaded| redact_record(&loaded.record)).collect();
    let refs: Vec<&serde_json::Value> = redacted.iter().collect();
    let report = diagnose_values(&refs);
    if json {
        println!("{}", report_json(&report));
    } else {
        print_human_report(&report);
    }
    Ok(Outcome::Same)
}

fn print_human_trace(diagnosis: &TraceDiagnosis) {
    let completeness = &diagnosis.completeness;
    println!(
        "trace {} completeness {}% ({}/{} spans, roots={} orphans={} complete={} error_span_zero={})",
        diagnosis.trace_id,
        completeness.completeness_pct,
        completeness.spans_complete,
        completeness.spans_total,
        completeness.roots,
        completeness.orphans,
        completeness.complete,
        completeness.error_span_zero,
    );
    if diagnosis.findings.is_empty() {
        println!("diagnosis: ok (no findings)");
    } else {
        for finding in &diagnosis.findings {
            println!(
                "finding {} op={} component={} span={}",
                finding.failure_shape, finding.op, finding.component, finding.span_id
            );
            println!("  likely_cause: {}", finding.likely_cause);
            println!("  suggested_fix: {}", finding.suggested_fix);
        }
    }
}

fn print_human_report(report: &DiagnoseReport) {
    println!(
        "diagnose {} traces, {} spans, completeness {}% error_span_zero={}",
        report.traces_total, report.spans_total, report.completeness_pct, report.error_span_zero
    );
    println!(
        "complete traces {}/{} errors: {} spans, {} events",
        report.traces_complete, report.traces_total, report.error_spans, report.error_events
    );
    for trace in &report.traces {
        println!(
            "  trace {} completeness {}% complete={} findings={}",
            trace.trace_id,
            trace.completeness.completeness_pct,
            trace.completeness.complete,
            trace.findings.len()
        );
        for finding in &trace.findings {
            println!("    {}: {}", finding.failure_shape, finding.likely_cause);
        }
    }
}
