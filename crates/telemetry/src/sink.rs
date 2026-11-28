//! Sinks: in-memory ring (tests/dev) and append-only JSONL files.

use serde_json::Value;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

/// Destination for telemetry records. Implementations must never block
/// the calling operation; overflow drops and counts.
pub trait Sink: Send + Sync {
    fn emit(&self, record: Value);
    fn dropped(&self) -> u64;
}

/// Bounded in-memory ring, newest records retained.
pub struct InMemorySink {
    inner: Mutex<Inner>,
    capacity: usize,
}

struct Inner {
    records: VecDeque<Value>,
    dropped: u64,
}

impl InMemorySink {
    pub fn new(capacity: usize) -> Self {
        Self { inner: Mutex::new(Inner { records: VecDeque::new(), dropped: 0 }), capacity }
    }

    pub fn snapshot(&self) -> Vec<Value> {
        self.inner
            .lock()
            .map(|i| i.records.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Sink for InMemorySink {
    fn emit(&self, record: Value) {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.records.len() >= self.capacity {
                inner.records.pop_front();
                inner.dropped += 1;
            }
            inner.records.push_back(record);
        }
    }

    fn dropped(&self) -> u64 {
        self.inner.lock().map(|i| i.dropped).unwrap_or(0)
    }
}

/// Append-only JSONL file sink. Write errors are counted, never raised.
pub struct JsonlFileSink {
    path: std::path::PathBuf,
    dropped: std::sync::atomic::AtomicU64,
}

impl JsonlFileSink {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path, dropped: std::sync::atomic::AtomicU64::new(0) }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Sink for JsonlFileSink {
    fn emit(&self, record: Value) {
        let line = match serde_json::to_string(&record) {
            Ok(l) => l,
            Err(_) => {
                self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
        };
        match std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            Ok(mut f) => {
                if writeln!(f, "{line}").is_err() {
                    self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            Err(_) => {
                self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// ODD Phase 8: dated JSONL layout + retention (additive extension).
//
// The `Sink`, `InMemorySink` and `JsonlFileSink` API above is untouched.
// `DatedJsonlSink` routes each record to `<root>/YYYY-MM-DD/<file>` (date
// taken from the record's `started_at`/`at` field, falling back to today)
// and prunes dated directories older than the retention window (default 7
// days). JSONL remains the default local sink; OTLP export (see
// `telemetry/collector-config.*`) is opt-in only and changes nothing here.
// ---------------------------------------------------------------------------

/// Default retention window: dated directories older than this are pruned.
pub const DEFAULT_RETENTION_DAYS: u64 = 7;

/// File name used inside each `YYYY-MM-DD` directory.
pub const DAILY_FILE_NAME: &str = "events.jsonl";

/// Append-only JSONL sink with per-day rotation under `<root>/YYYY-MM-DD/`.
///
/// `root` is `<support-dir>/telemetry`. Routing never blocks the calling
/// operation: unserializable records and I/O errors degrade to a counted
/// drop, mirroring [`JsonlFileSink`].
pub struct DatedJsonlSink {
    root: std::path::PathBuf,
    file_name: String,
    retention_days: u64,
    dropped: std::sync::atomic::AtomicU64,
}

impl DatedJsonlSink {
    /// Sink rooted at `root` (i.e. `<support-dir>/telemetry`), with the
    /// default file name and retention window.
    pub fn new(root: std::path::PathBuf) -> Self {
        Self {
            root,
            file_name: DAILY_FILE_NAME.to_string(),
            retention_days: DEFAULT_RETENTION_DAYS,
            dropped: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Override the per-day file name (default [`DAILY_FILE_NAME`]).
    pub fn with_file_name(mut self, file_name: &str) -> Self {
        self.file_name = file_name.to_string();
        self
    }

    /// Override the retention window in days (default [`DEFAULT_RETENTION_DAYS`]).
    pub fn with_retention_days(mut self, days: u64) -> Self {
        self.retention_days = days;
        self
    }

    /// Root directory (`<support-dir>/telemetry`).
    pub fn root(&self) -> &std::path::Path {
        self.root.as_path()
    }

    /// Per-day file name.
    pub fn file_name(&self) -> &str {
        self.file_name.as_str()
    }

    /// Retention window in days.
    pub fn retention_days(&self) -> u64 {
        self.retention_days
    }

    /// Directory for a `YYYY-MM-DD` date.
    pub fn dir_for_ymd(&self, ymd: &str) -> std::path::PathBuf {
        self.root.join(ymd)
    }

    /// File path for a `YYYY-MM-DD` date.
    pub fn path_for_ymd(&self, ymd: &str) -> std::path::PathBuf {
        self.root.join(ymd).join(self.file_name.as_str())
    }

    /// File path for today (UTC).
    pub fn path_for_today(&self) -> std::path::PathBuf {
        self.path_for_ymd(today_ymd().as_str())
    }

    /// Prune dated directories older than the retention window, relative to
    /// today (UTC). Returns the number of directories removed. Never fails:
    /// an unreadable root or I/O errors remove nothing.
    pub fn prune(&self) -> u64 {
        let cutoff = cutoff_ymd(today_ymd().as_str(), self.retention_days);
        self.prune_older_than(cutoff.as_str())
    }

    /// Prune dated directories strictly older than `keep_from_ymd`
    /// (`YYYY-MM-DD`, inclusive bound). Returns the number of directories
    /// removed. Non-dated entries (files, `latest`, …) are never touched,
    /// and only directories are candidates for removal.
    pub fn prune_older_than(&self, keep_from_ymd: &str) -> u64 {
        let mut removed: u64 = 0;
        let entries = match std::fs::read_dir(self.root.as_path()) {
            Ok(entries) => entries,
            Err(_) => return 0,
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !is_dated_dir_name(name) {
                continue;
            }
            if name < keep_from_ymd && std::fs::remove_dir_all(entry.path()).is_ok() {
                removed = removed.saturating_add(1);
            }
        }
        removed
    }

    fn note_dropped(&self) {
        self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Sink for DatedJsonlSink {
    fn emit(&self, record: Value) {
        let line = match serde_json::to_string(&record) {
            Ok(line) => line,
            Err(_) => {
                self.note_dropped();
                return;
            }
        };
        let ymd = ymd_of_record(&record).unwrap_or_else(today_ymd);
        let path = self.path_for_ymd(ymd.as_str());
        let dir_ready = path
            .parent()
            .is_none_or(|parent| std::fs::create_dir_all(parent).is_ok());
        if !dir_ready {
            self.note_dropped();
            return;
        }
        match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => {
                if writeln!(file, "{line}").is_err() {
                    self.note_dropped();
                }
            }
            Err(_) => self.note_dropped(),
        }
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Today (UTC) as `YYYY-MM-DD`.
pub fn today_ymd() -> String {
    ymd_from_system_time(std::time::SystemTime::now())
}

/// Format a [`std::time::SystemTime`] as `YYYY-MM-DD` (UTC).
/// Pre-epoch times saturate to the epoch day.
pub fn ymd_from_system_time(time: std::time::SystemTime) -> String {
    let days = time
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Extract the `YYYY-MM-DD` date from a record's `started_at` (span) or
/// `at` (event) field. Returns `None` when absent or malformed, in which
/// case the sink falls back to today.
pub fn ymd_of_record(record: &Value) -> Option<String> {
    for key in ["started_at", "at"] {
        if let Some(date) = record
            .get(key)
            .and_then(serde_json::Value::as_str)
            .and_then(|s| s.get(..10))
            .filter(|d| parse_ymd(d).is_some())
        {
            return Some(date.to_string());
        }
    }
    None
}

/// True when `name` is a valid `YYYY-MM-DD` directory name.
pub fn is_dated_dir_name(name: &str) -> bool {
    parse_ymd(name).is_some()
}

/// Parse `YYYY-MM-DD` into (year, month, day). Rejects malformed input and
/// impossible dates (month range, day range incl. leap years).
pub fn parse_ymd(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split('-');
    let year = parts.next()?;
    let month = parts.next()?;
    let day = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    let (Some(y), Some(m), Some(d)) = (
        year.parse::<i64>().ok(),
        month.parse::<i64>().ok(),
        day.parse::<i64>().ok(),
    ) else {
        return None;
    };
    if !(1..=12).contains(&m) {
        return None;
    }
    if d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

/// Inclusive retention cutoff: dated directories strictly older than the
/// returned `YYYY-MM-DD` fall outside a `retention_days` window ending at
/// `today`. Unparseable `today` saturates to the epoch day.
pub fn cutoff_ymd(today: &str, retention_days: u64) -> String {
    let days = parse_ymd(today).map_or(0, |(y, m, d)| days_from_civil(y, m, d));
    let retention = retention_days.min(i64::MAX as u64) as i64;
    let (year, month, day) = civil_from_days(days.saturating_sub(retention));
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
    }
}

// Howard Hinnant's civil-day algorithms (no date/time dependency).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_prime = (month + 9) % 12;
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146096
    } / 146097;
    let day_of_era = shifted - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (
        if month <= 2 { year + 1 } else { year },
        month,
        day,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod phase8_tests {
    use super::super::{emit, sample};
    use super::super::{TraceContext, context};
    use super::*;

    fn fresh_tempdir(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "ac-tel-phase8-{}-{}-{}",
            std::process::id(),
            tag,
            nanos
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn read_lines(path: &std::path::Path) -> Vec<String> {
        let content = std::fs::read_to_string(path).expect("read jsonl day file");
        content.lines().map(str::to_string).collect()
    }

    #[test]
    fn rotation_boundary_routes_records_by_record_date() {
        let root = fresh_tempdir("rotation");
        let sink = DatedJsonlSink::new(root.clone());
        assert_eq!(sink.retention_days(), DEFAULT_RETENTION_DAYS);
        assert_eq!(sink.file_name(), DAILY_FILE_NAME);

        // Spans route by `started_at`, events by `at`, dateless records by today.
        sink.emit(serde_json::json!({
            "schema_version": 1, "kind": "span", "op": "text.compare",
            "started_at": "2026-11-20T10:00:00Z", "status": "ok",
        }));
        sink.emit(serde_json::json!({
            "schema_version": 1, "kind": "event", "name": "app.started",
            "at": "2026-11-20T11:00:00Z", "severity": "info",
        }));
        sink.emit(serde_json::json!({
            "schema_version": 1, "kind": "span", "op": "sync.apply",
            "started_at": "2026-11-21T00:00:01Z", "status": "error",
        }));
        sink.emit(serde_json::json!({"schema_version": 1, "kind": "span", "op": "ui.tab_switch"}));
        assert_eq!(sink.dropped(), 0);

        let day20 = read_lines(&root.join("2026-11-20").join(DAILY_FILE_NAME));
        assert_eq!(day20.len(), 2);
        assert!(day20.iter().all(|l| {
            serde_json::from_str::<Value>(l)
                .ok()
                .and_then(|v| ymd_of_record(&v))
                .as_deref()
                == Some("2026-11-20")
        }));

        let day21 = read_lines(&root.join("2026-11-21").join(DAILY_FILE_NAME));
        assert_eq!(day21.len(), 1);
        assert!(
            day21
                .first()
                .is_some_and(|line| line.contains("sync.apply"))
        );

        let today = read_lines(&sink.path_for_today());
        assert_eq!(today.len(), 1);
        assert!(
            today
                .first()
                .is_some_and(|line| line.contains("ui.tab_switch"))
        );

        // Malformed dates never create junk directories — they fall back to today.
        sink.emit(serde_json::json!({
            "schema_version": 1, "kind": "span", "started_at": "not-a-date",
        }));
        assert!(!root.join("not-a-date").exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn retention_pruning_keeps_window_and_ignores_other_entries() {
        // 7-day default: window ending 2026-11-28 keeps 2026-11-21 onwards.
        assert_eq!(cutoff_ymd("2026-11-28", 7), "2026-11-21");
        assert_eq!(cutoff_ymd("2026-11-28", DEFAULT_RETENTION_DAYS), "2026-11-21");
        // Leap-year boundary: 2024-03-01 minus 1 day is 2024-02-29.
        assert_eq!(cutoff_ymd("2024-03-01", 1), "2024-02-29");
        assert_eq!(cutoff_ymd("2026-11-28", 0), "2026-11-28");

        let root = fresh_tempdir("retention");
        for dir in ["2026-11-01", "2026-11-20", "2026-11-21", "2026-11-27", "latest"] {
            std::fs::create_dir_all(root.join(dir)).expect("seed dir");
            std::fs::write(root.join(dir).join(DAILY_FILE_NAME), "seed\n").expect("seed file");
        }
        std::fs::write(root.join("notes.txt"), "keep me\n").expect("seed file");

        let sink = DatedJsonlSink::new(root.clone()).with_retention_days(7);
        let removed = sink.prune_older_than("2026-11-21");
        assert_eq!(removed, 2);
        assert!(!root.join("2026-11-01").exists());
        assert!(!root.join("2026-11-20").exists());
        assert!(root.join("2026-11-21").exists());
        assert!(root.join("2026-11-27").exists());
        assert!(root.join("latest").exists());
        assert!(root.join("notes.txt").exists());

        // Pruning is idempotent and never fails on a missing root.
        assert_eq!(sink.prune_older_than("2026-11-21"), 0);
        let missing = DatedJsonlSink::new(root.join("does-not-exist"));
        assert_eq!(missing.prune_older_than("2026-11-21"), 0);
        assert_eq!(missing.prune(), 0);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn schema_validation_of_sampled_records() {
        fn is_hex(value: &str, len: usize) -> bool {
            value.len() == len && value.bytes().all(|b| b.is_ascii_hexdigit())
        }

        fn assert_span_valid(record: &Value) {
            assert_eq!(
                record.get("schema_version").and_then(Value::as_u64),
                Some(1)
            );
            assert_eq!(record.get("kind").and_then(Value::as_str), Some("span"));
            assert!(
                record
                    .get("trace_id")
                    .and_then(Value::as_str)
                    .is_some_and(|s| is_hex(s, 32)),
                "trace_id is 32 lowercase hex"
            );
            assert!(
                record
                    .get("span_id")
                    .and_then(Value::as_str)
                    .is_some_and(|s| is_hex(s, 16)),
                "span_id is 16 lowercase hex"
            );
            assert!(
                record.get("op").and_then(Value::as_str).is_some_and(|s| !s.is_empty())
            );
            assert!(
                record
                    .get("component")
                    .and_then(Value::as_str)
                    .is_some_and(|s| s.contains('.')),
                "component matches dotted-layer shape"
            );
            assert!(
                record
                    .get("started_at")
                    .and_then(Value::as_str)
                    .and_then(|s| s.get(..10))
                    .is_some_and(|d| parse_ymd(d).is_some()),
                "started_at carries a routable date"
            );
            assert!(record.get("duration_ns").and_then(Value::as_u64).is_some());
            assert!(matches!(
                record.get("status").and_then(Value::as_str),
                Some("ok") | Some("error")
            ));
            assert!(record.get("attributes").and_then(Value::as_object).is_some());
            assert_eq!(
                record.get("redaction").and_then(Value::as_str),
                Some("v1-default")
            );
        }

        // Head-based sampling: at rate 0.0 only errors survive; at 1.0 all do.
        let sampling = sample::Sampling::new(0.0);
        assert!(sampling.keep("text.compare", true, 0.99));
        assert!(!sampling.keep("text.compare", false, 0.99));
        let full = sample::Sampling::new(1.0);
        assert!(full.keep("text.compare", false, 0.0));

        // Every record the sampler keeps must be schema-shaped AND routable
        // to a dated directory (round-trip through the Phase 8 sink).
        let root = fresh_tempdir("schema");
        let sink = DatedJsonlSink::new(root.clone());
        let mut kept = 0u32;
        for (i, (is_error, dice, expect_keep)) in
            [(false, 0.99, false), (true, 0.99, true), (true, 0.0, true)].iter().enumerate()
        {
            assert_eq!(
                sampling.keep("text.compare", *is_error, *dice),
                *expect_keep,
                "rate 0.0 keeps errors only"
            );
            if !*expect_keep {
                continue;
            }
            let ctx = TraceContext::fresh(true);
            let mut span = emit::Span::begin(
                &ctx,
                None,
                "text.compare",
                "rs.cmd/text",
                Some("s1"),
                Some("text"),
                "2026-11-28T00:00:00Z",
            );
            span.set_attr("iter", serde_json::json!(i));
            let record = span.finish(*is_error);
            assert_span_valid(&record);
            assert_eq!(ymd_of_record(&record).as_deref(), Some("2026-11-28"));
            sink.emit(record);
            kept += 1;
        }
        assert!(kept >= 2, "errors bypass sampling, so at least 2 records kept");
        assert_eq!(sink.dropped(), 0);
        let lines = read_lines(&root.join("2026-11-28").join(DAILY_FILE_NAME));
        assert_eq!(lines.len(), kept as usize);
        for line in &lines {
            let record: Value = serde_json::from_str(line).expect("valid jsonl line");
            assert_span_valid(&record);
        }

        // Error events keep the event shape (telemetry/schema/v1/event.json).
        let ctx = context::TraceContext::fresh(true);
        let event = emit::error_event(
            &ctx,
            "compare.failed",
            "rs.compare/text",
            "plain failure",
            "2026-11-28T00:00:00Z",
        );
        assert_eq!(event.get("kind").and_then(Value::as_str), Some("event"));
        assert_eq!(
            event.get("severity").and_then(Value::as_str),
            Some("error")
        );
        assert_eq!(ymd_of_record(&event).as_deref(), Some("2026-11-28"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
