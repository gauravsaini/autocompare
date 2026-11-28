//! ODD Phase 9: developer inspection CLI (`autocompare trace ...`).
//!
//! Offline reader over the Phase 8 dated JSONL layout
//! (`<support-dir>/telemetry/YYYY-MM-DD/*.jsonl`, same files the
//! `DatedJsonlSink` and `apps/desktop/src/observability/query.ts` parse).
//! Default output is human-readable; `--json` emits stable snake_case
//! machine JSON reserved for Phase 10 agent use. Secrets are redacted via
//! `autocompare-telemetry`'s [`redact`](autocompare_telemetry::redact)
//! helpers before display or bundling. Nothing here touches the network.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::error::CliError;
use crate::outcome::Outcome;

/// Schema version echoed in every JSON envelope this module emits.
pub const TRACE_JSON_SCHEMA_VERSION: u64 = 1;

/// Default per-day file name (mirrors the Phase 8 sink layout).
pub const DAILY_FILE_NAME: &str = "events.jsonl";

/// Clap wrapper so `args.rs` can mount `autocompare trace <subcommand>`.
#[derive(clap::Args, Debug, Clone)]
pub struct TraceArgs {
    /// Trace inspection subcommand to run.
    #[command(subcommand)]
    pub action: TraceAction,
}

/// Developer inspection subcommands (ODD Phase 9, §10.2 + §9).
#[derive(clap::Subcommand, Debug, Clone)]
pub enum TraceAction {
    /// Show every span and event for one trace id.
    Show {
        /// 128-bit trace id (32 lowercase hex chars).
        trace_id: String,
        /// Emit stable snake_case machine JSON instead of human text.
        #[arg(long)]
        json: bool,
        /// Telemetry root override (`<support-dir>/telemetry` by default).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// List spans, newest first, with optional exact-match filters.
    List {
        /// Exact `op` match (e.g. `text.compare`).
        #[arg(long)]
        op: Option<String>,
        /// Exact `status` match (`ok` or `error`).
        #[arg(long)]
        status: Option<String>,
        /// Exact `component` match (e.g. `rs.cmd/text`).
        #[arg(long)]
        component: Option<String>,
        /// Exact `trace_id` match.
        #[arg(long)]
        trace_id: Option<String>,
        /// Relative window (`30s`, `90m`, `24h`, `7d`) before now.
        #[arg(long)]
        since: Option<String>,
        /// Inclusive timestamp lower bound (ISO-8601 or `YYYY-MM-DD`).
        #[arg(long)]
        from: Option<String>,
        /// Inclusive timestamp upper bound (ISO-8601 or `YYYY-MM-DD`).
        #[arg(long)]
        to: Option<String>,
        /// Maximum rows to print.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit stable snake_case machine JSON instead of human text.
        #[arg(long)]
        json: bool,
        /// Telemetry root override (`<support-dir>/telemetry` by default).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Show error spans and error events, newest first.
    Errors {
        /// Exact `component` match.
        #[arg(long)]
        component: Option<String>,
        /// Exact `trace_id` match.
        #[arg(long)]
        trace_id: Option<String>,
        /// Relative window (`30s`, `90m`, `24h`, `7d`) before now.
        #[arg(long)]
        since: Option<String>,
        /// Maximum rows to print.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit stable snake_case machine JSON instead of human text.
        #[arg(long)]
        json: bool,
        /// Telemetry root override (`<support-dir>/telemetry` by default).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Aggregate spans per op with p50/p99 durations and sink volume.
    Stats {
        /// Relative window (`30s`, `90m`, `24h`, `7d`) before now.
        #[arg(long)]
        since: Option<String>,
        /// Emit stable snake_case machine JSON instead of human text.
        #[arg(long)]
        json: bool,
        /// Telemetry root override (`<support-dir>/telemetry` by default).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Zip one trace (redacted records + redacted inputs + manifest) for issues.
    Bundle {
        /// 128-bit trace id (32 lowercase hex chars).
        trace_id: String,
        /// Telemetry root override (`<support-dir>/telemetry` by default).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Output zip path (defaults to `./trace-<short-id>.zip`).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Diagnose likely causes + completeness for one trace or a whole root (ODD Phase 11).
    Diagnose {
        /// Diagnose a single trace id (omit to diagnose every trace under the root).
        #[arg(long)]
        trace_id: Option<String>,
        /// Emit stable snake_case machine JSON instead of human text.
        #[arg(long)]
        json: bool,
        /// Telemetry root override (`<support-dir>/telemetry` by default).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Summarize agent feed-in loop usage (ODD loop dogfood dashboard).
    Agents {
        /// Telemetry root override (`<support-dir>/telemetry` by default).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Emit stable snake_case machine JSON instead of human text.
        #[arg(long)]
        json: bool,
        /// Write a self-contained offline HTML dashboard to this path.
        #[arg(long)]
        html: Option<PathBuf>,
    },
}

/// One parsed JSONL line with its provenance.
#[derive(Debug, Clone)]
pub struct LoadedRecord {
    /// Parsed record body.
    pub record: serde_json::Value,
    /// File the line came from.
    pub source_file: PathBuf,
    /// 1-based line number within `source_file`.
    pub line_no: u64,
}

/// Result of loading every JSONL record under a telemetry root.
#[derive(Debug, Default)]
pub struct LoadReport {
    /// Parsed records in file order (oldest first).
    pub records: Vec<LoadedRecord>,
    /// JSONL files successfully read.
    pub files_scanned: u64,
    /// Total lines read across those files.
    pub lines_scanned: u64,
    /// Malformed or non-object lines skipped.
    pub skipped_lines: u64,
    /// Files that could not be read at all.
    pub unreadable_files: u64,
}

/// Resolve the telemetry root: `$AUTOCOMPARE_TELEMETRY_DIR` wins, otherwise
/// the OS support directory plus `telemetry`, else a temp-dir fallback.
/// Never fails; offline-only (no network, no registry lookups).
pub fn default_telemetry_root() -> PathBuf {
    if let Ok(dir) = std::env::var("AUTOCOMPARE_TELEMETRY_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Some(home) = home_dir() {
        #[cfg(target_os = "macos")]
        {
            return home
                .join("Library")
                .join("Application Support")
                .join("autocompare")
                .join("telemetry");
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                if !local.trim().is_empty() {
                    return PathBuf::from(local).join("autocompare").join("telemetry");
                }
            }
            return home.join("autocompare").join("telemetry");
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            return home.join(".local").join("share").join("autocompare").join("telemetry");
        }
    }
    std::env::temp_dir().join("autocompare-telemetry")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

/// Parse JSONL text into records. Malformed lines are skipped and counted —
/// parsing never fails, mirroring the sink's never-throw rule and the
/// `parseJsonlRecords` semantics in `query.ts`.
pub fn parse_jsonl_text(text: &str) -> (Vec<serde_json::Value>, u64) {
    let mut records = Vec::new();
    let mut skipped: u64 = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(value) if value.is_object() => records.push(value),
            _ => {
                skipped = skipped.saturating_add(1);
            }
        }
    }
    (records, skipped)
}

/// Validate one record against `telemetry/schema/v1/{span,event}.json`
/// (structural mirror, no extra dependencies). Returns the list of
/// violations; empty means schema-valid.
pub fn validate_record(record: &serde_json::Value) -> Vec<String> {
    let Some(obj) = record.as_object() else {
        return vec!["record_must_be_object".to_string()];
    };
    if obj.get("schema_version").and_then(serde_json::Value::as_u64) != Some(1) {
        return vec!["bad_schema_version".to_string()];
    }
    match obj.get("kind").and_then(serde_json::Value::as_str) {
        Some("span") => validate_span(obj),
        Some("event") => validate_event(obj),
        _ => vec!["bad_kind".to_string()],
    }
}

const SPAN_ALLOWED_KEYS: &[&str] = &[
    "schema_version",
    "kind",
    "trace_id",
    "span_id",
    "parent_span_id",
    "op",
    "component",
    "session_id",
    "view_id",
    "started_at",
    "duration_ns",
    "status",
    "attributes",
    "redaction",
];

const EVENT_ALLOWED_KEYS: &[&str] = &[
    "schema_version",
    "kind",
    "name",
    "trace_id",
    "span_id",
    "at",
    "severity",
    "attributes",
    "repro",
];

fn unknown_keys(obj: &serde_json::Map<String, serde_json::Value>, allowed: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            out.push(format!("additional_property:{key}"));
        }
    }
    out
}

fn is_lower_hex(text: &str, len: usize) -> bool {
    text.len() == len
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

fn valid_component(text: &str) -> bool {
    let Some((prefix, rest)) = text.split_once('.') else {
        return false;
    };
    if prefix.len() != 2 || !prefix.bytes().all(|b| b.is_ascii_lowercase()) {
        return false;
    }
    !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'/' ))
}

fn valid_event_name(text: &str) -> bool {
    let Some((head, tail)) = text.split_once('.') else {
        return false;
    };
    let part_ok = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };
    part_ok(head) && part_ok(tail) && !tail.contains('.')
}

fn ymd_prefix_ok(timestamp: &str) -> bool {
    timestamp
        .get(..10)
        .is_some_and(|prefix| autocompare_telemetry::sink::parse_ymd(prefix).is_some())
}

fn validate_span(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let mut errors = unknown_keys(obj, SPAN_ALLOWED_KEYS);
    let str_field = |key: &str| obj.get(key).and_then(serde_json::Value::as_str);
    if !str_field("trace_id").is_some_and(|s| is_lower_hex(s, 32)) {
        errors.push("bad_trace_id".to_string());
    }
    if !str_field("span_id").is_some_and(|s| is_lower_hex(s, 16)) {
        errors.push("bad_span_id".to_string());
    }
    if let Some(parent) = obj.get("parent_span_id") {
        let ok = parent.is_null()
            || parent.as_str().is_some_and(|s| is_lower_hex(s, 16));
        if !ok {
            errors.push("bad_parent_span_id".to_string());
        }
    }
    if !str_field("op").is_some_and(|s| !s.is_empty()) {
        errors.push("bad_op".to_string());
    }
    if !str_field("component").is_some_and(valid_component) {
        errors.push("bad_component".to_string());
    }
    if !str_field("started_at").is_some_and(ymd_prefix_ok) {
        errors.push("bad_started_at".to_string());
    }
    if let Some(duration) = obj.get("duration_ns")
        && duration.as_u64().is_none() {
            errors.push("bad_duration_ns".to_string());
        }
    if !matches!(str_field("status"), Some("ok") | Some("error")) {
        errors.push("bad_status".to_string());
    }
    if let Some(attrs) = obj.get("attributes")
        && !attrs.is_object() {
            errors.push("bad_attributes".to_string());
        }
    if let Some(redaction) = obj.get("redaction")
        && redaction.as_str() != Some("v1-default") {
            errors.push("bad_redaction".to_string());
        }
    for key in ["session_id", "view_id"] {
        if let Some(value) = obj.get(key)
            && value.as_str().is_none_or(|s| s.is_empty()) {
                errors.push(format!("bad_{key}"));
            }
    }
    errors
}

fn validate_event(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let mut errors = unknown_keys(obj, EVENT_ALLOWED_KEYS);
    let str_field = |key: &str| obj.get(key).and_then(serde_json::Value::as_str);
    if !str_field("name").is_some_and(valid_event_name) {
        errors.push("bad_name".to_string());
    }
    if !str_field("trace_id").is_some_and(|s| is_lower_hex(s, 32)) {
        errors.push("bad_trace_id".to_string());
    }
    if !str_field("span_id").is_some_and(|s| is_lower_hex(s, 16)) {
        errors.push("bad_span_id".to_string());
    }
    if !str_field("at").is_some_and(ymd_prefix_ok) {
        errors.push("bad_at".to_string());
    }
    if !matches!(
        str_field("severity"),
        Some("debug") | Some("info") | Some("warn") | Some("error")
    ) {
        errors.push("bad_severity".to_string());
    }
    if let Some(attrs) = obj.get("attributes")
        && !attrs.is_object() {
            errors.push("bad_attributes".to_string());
        }
    if let Some(repro) = obj.get("repro") {
        let repro_ok = repro.as_object().is_some_and(|map| {
            map.iter().all(|(key, value)| {
                (key == "e2e" || key == "fixture") && value.as_str().is_some()
            })
        });
        if !repro_ok {
            errors.push("bad_repro".to_string());
        }
    }
    errors
}

/// Record timestamp: `started_at` (span) or `at` (event), when present.
pub fn record_timestamp(record: &serde_json::Value) -> Option<&str> {
    record
        .get("started_at")
        .and_then(serde_json::Value::as_str)
        .or_else(|| record.get("at").and_then(serde_json::Value::as_str))
}

/// True when the record is a span.
pub fn is_span(record: &serde_json::Value) -> bool {
    record.get("kind").and_then(serde_json::Value::as_str) == Some("span")
}

/// True when the record carries an error (`status: error` span or
/// `severity: error` event).
pub fn is_error_record(record: &serde_json::Value) -> bool {
    record.get("status").and_then(serde_json::Value::as_str) == Some("error")
        || (record.get("kind").and_then(serde_json::Value::as_str) == Some("event")
            && record.get("severity").and_then(serde_json::Value::as_str) == Some("error"))
}

/// Load every JSONL record under `root`, covering both the Phase 8 dated
/// layout (`<root>/YYYY-MM-DD/*.jsonl`) and flat `*.jsonl` files directly in
/// `root` (handy for seeded fixtures). A `root` pointing at a single file
/// reads just that file. Never fails outright: per-file problems are
/// counted in the report.
pub fn load_telemetry_dir(root: &Path) -> LoadReport {
    let mut report = LoadReport::default();
    if root.is_file() {
        read_jsonl_file(root, &mut report);
        return report;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return report;
    };
    let mut dated_dirs: Vec<PathBuf> = Vec::new();
    let mut flat_files: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dated = path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(autocompare_telemetry::sink::is_dated_dir_name);
            if dated {
                dated_dirs.push(path);
            }
        } else if path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
        {
            flat_files.push(path);
        }
    }
    dated_dirs.sort();
    flat_files.sort();
    for dir in &dated_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            report.unreadable_files = report.unreadable_files.saturating_add(1);
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(std::ffi::OsStr::to_str)
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
            })
            .collect();
        files.sort();
        for file in &files {
            read_jsonl_file(file, &mut report);
        }
    }
    for file in &flat_files {
        read_jsonl_file(file, &mut report);
    }
    report
}

fn read_jsonl_file(path: &Path, report: &mut LoadReport) {
    let Ok(text) = std::fs::read_to_string(path) else {
        report.unreadable_files = report.unreadable_files.saturating_add(1);
        return;
    };
    report.files_scanned = report.files_scanned.saturating_add(1);
    let mut line_no: u64 = 0;
    for line in text.lines() {
        line_no = line_no.saturating_add(1);
        report.lines_scanned = report.lines_scanned.saturating_add(1);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(record) if record.is_object() => report.records.push(LoadedRecord {
                record,
                source_file: path.to_path_buf(),
                line_no,
            }),
            _ => {
                report.skipped_lines = report.skipped_lines.saturating_add(1);
            }
        }
    }
}

/// Exact-match list filters shared by `list` and `errors`.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    /// Exact `op` match.
    pub op: Option<String>,
    /// Exact `status` match (`ok` | `error`).
    pub status: Option<String>,
    /// Exact `component` match.
    pub component: Option<String>,
    /// Exact `trace_id` match.
    pub trace_id: Option<String>,
    /// Inclusive timestamp lower bound (lexicographic, like `query.ts`).
    pub from: Option<String>,
    /// Inclusive timestamp upper bound (lexicographic, like `query.ts`).
    pub to: Option<String>,
    /// Relative window in seconds before now (`since` flag).
    pub since_secs: Option<i64>,
    /// Errors only (the `errors` subcommand).
    pub errors_only: bool,
    /// Maximum rows to return.
    pub limit: usize,
}

fn str_eq(record: &serde_json::Value, key: &str, want: &Option<String>) -> bool {
    want.as_ref().is_none_or(|expected| {
        record
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|actual| actual == expected)
    })
}

fn in_bounds(record: &serde_json::Value, from: &Option<String>, to: &Option<String>) -> bool {
    if from.is_none() && to.is_none() {
        return true;
    }
    let Some(timestamp) = record_timestamp(record) else {
        return false;
    };
    if from.as_ref().is_some_and(|lower| timestamp < lower) {
        return false;
    }
    if to.as_ref().is_some_and(|upper| timestamp > upper) {
        return false;
    }
    true
}

fn now_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}

fn within_since(record: &serde_json::Value, since_secs: Option<i64>) -> bool {
    let Some(window) = since_secs else {
        return true;
    };
    let Some(timestamp) = record_timestamp(record) else {
        return false;
    };
    let Some(epoch) = parse_rfc3339_epoch_secs(timestamp) else {
        return false;
    };
    epoch >= now_epoch_secs().saturating_sub(window)
}

/// Filter records newest-first (reversed file order), mirroring
/// `filterSpanRecords` in `query.ts`. Non-span records never match `list`.
pub fn filter_list<'a>(records: &'a [LoadedRecord], query: &ListQuery) -> Vec<&'a LoadedRecord> {
    let mut out = Vec::new();
    for loaded in records.iter().rev() {
        let record = &loaded.record;
        if !is_span(record) {
            continue;
        }
        if query.errors_only && !is_error_record(record) {
            // `errors` also surfaces error *events* below; spans filter here.
            continue;
        }
        if !str_eq(record, "op", &query.op) {
            continue;
        }
        if !str_eq(record, "status", &query.status) {
            continue;
        }
        if !str_eq(record, "component", &query.component) {
            continue;
        }
        if !str_eq(record, "trace_id", &query.trace_id) {
            continue;
        }
        if !in_bounds(record, &query.from, &query.to) {
            continue;
        }
        if !within_since(record, query.since_secs) {
            continue;
        }
        out.push(loaded);
        if query.limit > 0 && out.len() >= query.limit {
            break;
        }
    }
    out
}

/// Error spans plus error events for the `errors` subcommand, newest first.
pub fn filter_errors<'a>(records: &'a [LoadedRecord], query: &ListQuery) -> Vec<&'a LoadedRecord> {
    let mut out = Vec::new();
    for loaded in records.iter().rev() {
        let record = &loaded.record;
        if !is_error_record(record) {
            continue;
        }
        if !is_span(record)
            && record.get("kind").and_then(serde_json::Value::as_str) != Some("event")
        {
            continue;
        }
        if !str_eq(record, "component", &query.component) {
            continue;
        }
        if !str_eq(record, "trace_id", &query.trace_id) {
            continue;
        }
        if !in_bounds(record, &query.from, &query.to) {
            continue;
        }
        if !within_since(record, query.since_secs) {
            continue;
        }
        // `component` on error events lives under `attributes.component`.
        if let Some(want) = query.component.as_ref() {
            let event_component = record
                .get("attributes")
                .and_then(|attrs| attrs.get("component"))
                .and_then(serde_json::Value::as_str);
            let span_component = record
                .get("component")
                .and_then(serde_json::Value::as_str);
            if span_component != Some(want.as_str()) && event_component != Some(want.as_str()) {
                continue;
            }
        }
        out.push(loaded);
        if query.limit > 0 && out.len() >= query.limit {
            break;
        }
    }
    out
}

/// Every record (spans and events) for one trace, oldest first.
pub fn records_for_trace<'a>(records: &'a [LoadedRecord], trace_id: &str) -> Vec<&'a LoadedRecord> {
    records
        .iter()
        .filter(|loaded| {
            loaded
                .record
                .get("trace_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|actual| actual == trace_id)
        })
        .collect()
}

/// Parse a relative window (`30s`, `90m`, `24h`, `7d`, case-insensitive)
/// into seconds. Returns `None` for malformed input.
pub fn parse_since_secs(raw: &str) -> Option<i64> {
    let text = raw.trim().to_ascii_lowercase();
    let (number, factor) = if let Some(stripped) = text.strip_suffix('s') {
        (stripped, 1_i64)
    } else if let Some(stripped) = text.strip_suffix('m') {
        (stripped, 60_i64)
    } else if let Some(stripped) = text.strip_suffix('h') {
        (stripped, 3_600_i64)
    } else if let Some(stripped) = text.strip_suffix('d') {
        (stripped, 86_400_i64)
    } else {
        (text.as_str(), 1_i64)
    };
    let amount: f64 = number.trim().parse().ok()?;
    if !amount.is_finite() || amount < 0.0 {
        return None;
    }
    let secs = amount * factor as f64;
    if secs > i64::MAX as f64 {
        return None;
    }
    Some(secs as i64)
}

/// Minimal RFC-3339 (`YYYY-MM-DDTHH:MM:SS[.frac][Z|±HH:MM]`) to epoch
/// seconds (UTC). Returns `None` for malformed input. Dependency-free so
/// the CLI stays offline-lean.
pub fn parse_rfc3339_epoch_secs(timestamp: &str) -> Option<i64> {
    let (date, time) = timestamp.split_once('T')?;
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() {
        return None;
    }
    let (clock, offset_secs) = split_rfc3339_offset(time)?;
    let mut clock_parts = clock.split(':');
    let hour: i64 = clock_parts.next()?.parse().ok()?;
    let minute: i64 = clock_parts.next()?.parse().ok()?;
    let second: i64 = clock_parts
        .next()?
        .split('.')
        .next()?
        .parse()
        .ok()?;
    if clock_parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second - offset_secs)
}

fn split_rfc3339_offset(time: &str) -> Option<(&str, i64)> {
    if let Some(clock) = time.strip_suffix('Z').or_else(|| time.strip_suffix('z')) {
        return Some((clock, 0));
    }
    let bytes = time.as_bytes();
    let mut at: Option<usize> = None;
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && (*byte == b'+' || *byte == b'-') {
            at = Some(index);
        }
    }
    let split_at = at?;
    let clock = time.get(..split_at)?;
    let zone = time.get(split_at..)?;
    let sign = if zone.starts_with('+') { 1_i64 } else { -1_i64 };
    let digits = zone.get(1..)?;
    let (hour_str, min_str) = digits.split_once(':')?;
    let hour: i64 = hour_str.parse().ok()?;
    let minute: i64 = min_str.parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some((clock, sign * (hour * 3_600 + minute * 60)))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    if !(1..=31).contains(&day) {
        return None;
    }
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year.rem_euclid(400);
    let month_prime = (month + 9) % 12;
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146097 + day_of_era - 719468)
}

/// Redact secret-looking content via `autocompare-telemetry`'s redaction
/// gate: attribute keys (or string values) matching secret markers become
/// `[redacted]`. Structure is preserved; only the secrets are dropped.
pub fn redact_record(record: &serde_json::Value) -> serde_json::Value {
    redact_value(record)
}

fn redact_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, item) in map {
                if autocompare_telemetry::redact::looks_like_secret(key) {
                    out.insert(key.clone(), serde_json::Value::from("[redacted]"));
                } else {
                    out.insert(key.clone(), redact_value(item));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(redact_value).collect())
        }
        serde_json::Value::String(text) => {
            if autocompare_telemetry::redact::looks_like_secret(text) {
                serde_json::Value::from("[redacted]")
            } else {
                serde_json::Value::String(text.clone())
            }
        }
        other => other.clone(),
    }
}

fn short_id(trace_id: &str) -> &str {
    trace_id.get(..8).unwrap_or(trace_id)
}

fn span_line(record: &serde_json::Value) -> String {
    let span_id = record
        .get("span_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let op = record
        .get("op")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            record
                .get("attributes")
                .and_then(|a| a.get("op"))
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("?");
    let component = record
        .get("component")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            record
                .get("attributes")
                .and_then(|a| a.get("component"))
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("?");
    let status = record
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let at = record_timestamp(record).unwrap_or("?");
    let duration = record
        .get("duration_ns")
        .and_then(serde_json::Value::as_u64)
        .map(|ns| format!(" {:.2}ms", ns as f64 / 1_000_000.0))
        .unwrap_or_default();
    format!("  span {span_id} {op} {component} {status}{duration} {at}")
}

fn event_line(record: &serde_json::Value) -> String {
    let name = record
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let severity = record
        .get("severity")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let component = record
        .get("component")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            record
                .get("attributes")
                .and_then(|attrs| attrs.get("component"))
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("?");
    let at = record_timestamp(record).unwrap_or("?");
    format!("  event {name} {severity} {component} {at}")
}

/// Aggregate statistics over loaded records (the `stats` subcommand).
#[derive(Debug, Clone)]
pub struct TraceStats {
    /// Closed spans seen.
    pub spans_total: u64,
    /// Events seen.
    pub events_total: u64,
    /// Distinct trace ids.
    pub traces_total: u64,
    /// Span count per `op`.
    pub by_op: BTreeMap<String, u64>,
    /// Span count per `status`.
    pub by_status: BTreeMap<String, u64>,
    /// Spans with `status: error`.
    pub error_spans: u64,
    /// Error-severity events.
    pub error_events: u64,
    /// p50 of `duration_ns` over spans that carry it.
    pub duration_ns_p50: u64,
    /// p99 of `duration_ns` over spans that carry it.
    pub duration_ns_p99: u64,
}

/// Compute [`TraceStats`] over records, honouring an optional `since_secs`
/// relative window (same semantics as list filtering).
pub fn compute_stats(records: &[LoadedRecord], since_secs: Option<i64>) -> TraceStats {
    let mut stats = TraceStats {
        spans_total: 0,
        events_total: 0,
        traces_total: 0,
        by_op: BTreeMap::new(),
        by_status: BTreeMap::new(),
        error_spans: 0,
        error_events: 0,
        duration_ns_p50: 0,
        duration_ns_p99: 0,
    };
    let mut durations: Vec<u64> = Vec::new();
    let mut trace_ids: Vec<&str> = Vec::new();
    for loaded in records {
        let record = &loaded.record;
        if !within_since(record, since_secs) {
            continue;
        }
        if let Some(trace_id) = record
            .get("trace_id")
            .and_then(serde_json::Value::as_str)
            && !trace_ids.contains(&trace_id) {
                trace_ids.push(trace_id);
            }
        if is_span(record) {
            stats.spans_total = stats.spans_total.saturating_add(1);
            if let Some(op) = record.get("op").and_then(serde_json::Value::as_str) {
                *stats.by_op.entry(op.to_string()).or_insert(0) =
                    stats.by_op.get(op).copied().unwrap_or(0).saturating_add(1);
            }
            if let Some(status) = record.get("status").and_then(serde_json::Value::as_str) {
                *stats.by_status.entry(status.to_string()).or_insert(0) =
                    stats.by_status.get(status).copied().unwrap_or(0).saturating_add(1);
            }
            if is_error_record(record) {
                stats.error_spans = stats.error_spans.saturating_add(1);
            }
            if let Some(duration) = record.get("duration_ns").and_then(serde_json::Value::as_u64)
            {
                durations.push(duration);
            }
        } else if record.get("kind").and_then(serde_json::Value::as_str) == Some("event") {
            stats.events_total = stats.events_total.saturating_add(1);
            if is_error_record(record) {
                stats.error_events = stats.error_events.saturating_add(1);
            }
        }
    }
    stats.traces_total = trace_ids.len() as u64;
    durations.sort_unstable();
    if !durations.is_empty() {
        stats.duration_ns_p50 = percentile(&durations, 0.50);
        stats.duration_ns_p99 = percentile(&durations, 0.99);
    }
    stats
}

fn percentile(sorted: &[u64], fraction: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (fraction * sorted.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len().saturating_sub(1));
    sorted.get(index).copied().unwrap_or(0)
}

fn stats_json(
    stats: &TraceStats,
    report: &LoadReport,
    since_secs: Option<i64>,
) -> serde_json::Value {
    let by_op: serde_json::Map<String, serde_json::Value> = stats
        .by_op
        .iter()
        .map(|(op, count)| (op.clone(), serde_json::Value::from(*count)))
        .collect();
    let by_status: serde_json::Map<String, serde_json::Value> = stats
        .by_status
        .iter()
        .map(|(status, count)| (status.clone(), serde_json::Value::from(*count)))
        .collect();
    let mut out = serde_json::Map::new();
    out.insert("schema_version".to_string(), serde_json::Value::from(TRACE_JSON_SCHEMA_VERSION));
    out.insert("spans_total".to_string(), serde_json::Value::from(stats.spans_total));
    out.insert("events_total".to_string(), serde_json::Value::from(stats.events_total));
    out.insert(
        "records_total".to_string(),
        serde_json::Value::from(stats.spans_total.saturating_add(stats.events_total)),
    );
    out.insert("traces_total".to_string(), serde_json::Value::from(stats.traces_total));
    out.insert("by_op".to_string(), serde_json::Value::Object(by_op));
    out.insert("by_status".to_string(), serde_json::Value::Object(by_status));
    out.insert("error_spans".to_string(), serde_json::Value::from(stats.error_spans));
    out.insert("error_events".to_string(), serde_json::Value::from(stats.error_events));
    out.insert(
        "duration_ns_p50".to_string(),
        serde_json::Value::from(stats.duration_ns_p50),
    );
    out.insert(
        "duration_ns_p99".to_string(),
        serde_json::Value::from(stats.duration_ns_p99),
    );
    out.insert("files_scanned".to_string(), serde_json::Value::from(report.files_scanned));
    out.insert("lines_scanned".to_string(), serde_json::Value::from(report.lines_scanned));
    out.insert("skipped_lines".to_string(), serde_json::Value::from(report.skipped_lines));
    out.insert(
        "unreadable_files".to_string(),
        serde_json::Value::from(report.unreadable_files),
    );
    if let Some(window) = since_secs {
        out.insert("since_secs".to_string(), serde_json::Value::from(window));
    }
    serde_json::Value::Object(out)
}

/// Report from [`bundle_trace`].
#[derive(Debug, Clone)]
pub struct BundleReport {
    /// Zip file that was written.
    pub out_path: PathBuf,
    /// Redacted spans included.
    pub spans: u64,
    /// Redacted events included.
    pub events: u64,
}

/// Zip one trace for issue attachment: redacted `trace.json`, redacted
/// per-span `inputs.redacted.json`, and a `manifest.json` carrying the
/// schema version. Returns the bundle report. Fails when the trace is
/// unknown or the zip cannot be written.
pub fn bundle_trace(
    records: &[LoadedRecord],
    trace_id: &str,
    out_path: &Path,
) -> Result<BundleReport, CliError> {
    let matching = records_for_trace(records, trace_id);
    if matching.is_empty() {
        return Err(CliError::InvalidArg(format!("unknown trace_id: {trace_id}")));
    }
    let redacted: Vec<serde_json::Value> =
        matching.iter().map(|loaded| redact_record(&loaded.record)).collect();
    let (spans, events) = redacted.iter().fold((0_u64, 0_u64), |(spans, events), record| {
        if is_span(record) {
            (spans.saturating_add(1), events)
        } else {
            (spans, events.saturating_add(1))
        }
    });
    let trace_json = serde_json::to_string_pretty(&redacted)
        .map_err(|err| CliError::InvalidArg(format!("bundle encode failed: {err}")))?;
    let inputs: Vec<serde_json::Value> = redacted
        .iter()
        .filter(|record| is_span(record))
        .map(|record| {
            serde_json::json!({
                "span_id": record.get("span_id"),
                "op": record.get("op"),
                "attributes": record.get("attributes").cloned().unwrap_or(serde_json::Value::Null),
            })
        })
        .collect();
    let inputs_json = serde_json::to_string_pretty(&inputs)
        .map_err(|err| CliError::InvalidArg(format!("bundle encode failed: {err}")))?;
    let manifest = serde_json::json!({
        "schema_version": TRACE_JSON_SCHEMA_VERSION,
        "trace_id": trace_id,
        "spans": spans,
        "events": events,
        "redacted": true,
        "redaction": "v1-default",
        "generator": "autocompare trace bundle (ODD Phase 9)",
    });
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|err| CliError::InvalidArg(format!("bundle encode failed: {err}")))?;

    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| CliError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
    let file = std::fs::File::create(out_path).map_err(|source| CliError::Io {
        path: out_path.display().to_string(),
        source,
    })?;
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut writer = zip::ZipWriter::new(file);
    for (name, contents) in [
        ("trace.json", trace_json.as_str()),
        ("inputs.redacted.json", inputs_json.as_str()),
        ("manifest.json", manifest_json.as_str()),
    ] {
        writer
            .start_file(name, options)
            .map_err(|err| CliError::InvalidArg(format!("bundle zip failed: {err}")))?;
        writer
            .write_all(contents.as_bytes())
            .map_err(|source| CliError::Io {
                path: out_path.display().to_string(),
                source,
            })?;
    }
    writer
        .finish()
        .map_err(|err| CliError::InvalidArg(format!("bundle zip failed: {err}")))?;
    Ok(BundleReport { out_path: out_path.to_path_buf(), spans, events })
}

fn resolve_root(dir: &Option<PathBuf>) -> PathBuf {
    dir.clone().unwrap_or_else(default_telemetry_root)
}

fn invalid_count(records: &[LoadedRecord]) -> u64 {
    records
        .iter()
        .filter(|loaded| !validate_record(&loaded.record).is_empty())
        .count() as u64
}

/// Dispatch a trace subcommand. Human-readable output by default; `--json`
/// switches to stable snake_case machine JSON.
pub fn run(action: &TraceAction, _color: bool) -> Result<Outcome, CliError> {
    match action {
        TraceAction::Show { trace_id, json, dir } => {
            let report = load_telemetry_dir(&resolve_root(dir));
            let matching = records_for_trace(&report.records, trace_id);
            if matching.is_empty() {
                return Err(CliError::InvalidArg(format!("unknown trace_id: {trace_id}")));
            }
            if *json {
                let redacted: Vec<serde_json::Value> = matching
                    .iter()
                    .map(|loaded| redact_record(&loaded.record))
                    .collect();
                let (spans, events) =
                    redacted.iter().fold((0_u64, 0_u64), |(spans, events), record| {
                        if is_span(record) {
                            (spans.saturating_add(1), events)
                        } else {
                            (spans, events.saturating_add(1))
                        }
                    });
                let payload = serde_json::json!({
                    "schema_version": TRACE_JSON_SCHEMA_VERSION,
                    "trace_id": trace_id,
                    "span_count": spans,
                    "event_count": events,
                    "spans": redacted.iter().filter(|record| is_span(record)).collect::<Vec<_>>(),
                    "events": redacted.iter().filter(|record| !is_span(record)).collect::<Vec<_>>(),
                });
                println!("{payload}");
            } else {
                let spans = matching.iter().filter(|loaded| is_span(&loaded.record)).count();
                let events = matching.len().saturating_sub(spans);
                println!("trace {trace_id} ({spans} spans, {events} events)");
                for loaded in &matching {
                    let redacted = redact_record(&loaded.record);
                    if is_span(&redacted) {
                        println!("{}", span_line(&redacted));
                    } else {
                        println!("{}", event_line(&redacted));
                    }
                }
            }
            Ok(Outcome::Same)
        }
        TraceAction::List {
            op,
            status,
            component,
            trace_id,
            since,
            from,
            to,
            limit,
            json,
            dir,
        } => {
            if let Some(status) = status
                && status != "ok" && status != "error" {
                    return Err(CliError::InvalidArg(format!("bad status filter: {status}")));
                }
            let since_secs: Option<i64> = match since.as_deref() {
                None => None,
                Some(raw) => Some(parse_since_secs(raw).ok_or_else(|| {
                    CliError::InvalidArg(format!("bad --since: {raw} (use 30s, 90m, 24h, 7d)"))
                })?),
            };
            let report = load_telemetry_dir(&resolve_root(dir));
            let query = ListQuery {
                op: op.clone(),
                status: status.clone(),
                component: component.clone(),
                trace_id: trace_id.clone(),
                from: from.clone(),
                to: to.clone(),
                since_secs,
                errors_only: false,
                limit: *limit,
            };
            let rows = filter_list(&report.records, &query);
            if *json {
                let redacted: Vec<serde_json::Value> =
                    rows.iter().map(|loaded| redact_record(&loaded.record)).collect();
                let payload = serde_json::json!({
                    "schema_version": TRACE_JSON_SCHEMA_VERSION,
                    "count": redacted.len(),
                    "limit": *limit,
                    "records": redacted,
                });
                println!("{payload}");
            } else if rows.is_empty() {
                println!("no matching spans");
            } else {
                for loaded in &rows {
                    println!("{}", span_line(&redact_record(&loaded.record)));
                }
            }
            Ok(Outcome::Same)
        }
        TraceAction::Errors { component, trace_id, since, limit, json, dir } => {
            let since_secs: Option<i64> = match since.as_deref() {
                None => None,
                Some(raw) => Some(parse_since_secs(raw).ok_or_else(|| {
                    CliError::InvalidArg(format!("bad --since: {raw} (use 30s, 90m, 24h, 7d)"))
                })?),
            };
            let report = load_telemetry_dir(&resolve_root(dir));
            let query = ListQuery {
                component: component.clone(),
                trace_id: trace_id.clone(),
                since_secs,
                errors_only: true,
                limit: *limit,
                ..ListQuery::default()
            };
            let rows = filter_errors(&report.records, &query);
            if *json {
                let redacted: Vec<serde_json::Value> =
                    rows.iter().map(|loaded| redact_record(&loaded.record)).collect();
                let payload = serde_json::json!({
                    "schema_version": TRACE_JSON_SCHEMA_VERSION,
                    "count": redacted.len(),
                    "limit": *limit,
                    "errors": redacted,
                });
                println!("{payload}");
            } else if rows.is_empty() {
                println!("no errors");
            } else {
                for loaded in &rows {
                    let redacted = redact_record(&loaded.record);
                    if is_span(&redacted) {
                        println!("{}", span_line(&redacted));
                    } else {
                        println!("{}", event_line(&redacted));
                    }
                }
            }
            Ok(Outcome::Same)
        }
        TraceAction::Stats { since, json, dir } => {
            let since_secs: Option<i64> = match since.as_deref() {
                None => None,
                Some(raw) => Some(parse_since_secs(raw).ok_or_else(|| {
                    CliError::InvalidArg(format!("bad --since: {raw} (use 30s, 90m, 24h, 7d)"))
                })?),
            };
            let report = load_telemetry_dir(&resolve_root(dir));
            let stats = compute_stats(&report.records, since_secs);
            if *json {
                println!("{}", stats_json(&stats, &report, since_secs));
            } else {
                println!(
                    "spans: {}  events: {}  traces: {}",
                    stats.spans_total, stats.events_total, stats.traces_total
                );
                println!(
                    "errors: {} spans, {} events",
                    stats.error_spans, stats.error_events
                );
                println!(
                    "duration_ns p50={} p99={}",
                    stats.duration_ns_p50, stats.duration_ns_p99
                );
                println!(
                    "sink: {} files, {} lines, {} skipped, {} invalid, {} unreadable",
                    report.files_scanned,
                    report.lines_scanned,
                    report.skipped_lines,
                    invalid_count(&report.records),
                    report.unreadable_files
                );
                let mut ops: Vec<(&String, &u64)> = stats.by_op.iter().collect();
                ops.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
                for (op, count) in ops {
                    println!("  op {op}: {count}");
                }
            }
            Ok(Outcome::Same)
        }
        TraceAction::Bundle { trace_id, dir, out } => {
            let report = load_telemetry_dir(&resolve_root(dir));
            let out_path = out.clone().unwrap_or_else(|| {
                PathBuf::from(format!("trace-{}.zip", short_id(trace_id)))
            });
            let bundle = bundle_trace(&report.records, trace_id, &out_path)?;
            println!(
                "bundle {} ({} spans, {} events) -> {}",
                trace_id,
                bundle.spans,
                bundle.events,
                bundle.out_path.display()
            );
            Ok(Outcome::Same)
        }
        TraceAction::Diagnose { trace_id, json, dir } => {
            crate::diagnose::run_diagnose(trace_id, *json, dir)
        }
        TraceAction::Agents { dir, json, html } => crate::agents::run_agents(dir, *json, html),
    }
}
