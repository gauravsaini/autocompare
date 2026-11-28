#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! ODD Phase 9 inspection CLI tests: seed JSONL in /tmp, assert parse +
//! schema validation, and prove `bundle` emits a readable zip.
//!
//! Covers the same JSONL layout the Phase 8 `DatedJsonlSink` writes and
//! `apps/desktop/src/observability/query.ts` parses.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::Command;

use autocompare_cli::trace::{
    ListQuery, bundle_trace, compute_stats, filter_errors, filter_list, is_error_record,
    is_span, load_telemetry_dir, parse_jsonl_text, parse_rfc3339_epoch_secs, parse_since_secs,
    records_for_trace, redact_record, validate_record,
};
use tempfile::tempdir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_autocompare")
}

const TRACE_OK: &str = "4bf92f3577b34da6a3ce929d0e0e4736";
const TRACE_ERR: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn span_line(trace: &str, span: &str, op: &str, status: &str, at: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{trace}\",\"span_id\":\"{span}\",\"op\":\"{op}\",\"component\":\"rs.cmd/text\",\"started_at\":\"{at}\",\"duration_ns\":18420311,\"status\":\"{status}\",\"attributes\":{{\"transport\":\"tauri\",\"api_token\":\"hunter2 token\"}},\"redaction\":\"v1-default\"}}"
    )
}

fn event_line(trace: &str, span: &str, at: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"kind\":\"event\",\"name\":\"ipc.serde_error\",\"trace_id\":\"{trace}\",\"span_id\":\"{span}\",\"at\":\"{at}\",\"severity\":\"error\",\"attributes\":{{\"component\":\"rs.compare/text\"}}}}"
    )
}

/// Seed a dated-layout fixture: `<root>/2026-11-28/events.jsonl`.
fn seed_telemetry_root() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("telemetry");
    let day = root.join("2026-11-28");
    std::fs::create_dir_all(&day).expect("mkdir day");
    let body = [
        span_line(TRACE_OK, "00f067aa0ba902b7", "text.compare", "ok", "2026-11-28T09:14:02Z"),
        span_line(TRACE_ERR, "b7ad6b7169203331", "sync.apply", "error", "2026-11-28T10:00:00Z"),
        event_line(TRACE_ERR, "b7ad6b7169203331", "2026-11-28T10:00:01Z"),
        "not json at all".to_string(),
    ]
    .join("\n")
        + "\n";
    std::fs::write(day.join("events.jsonl"), body).expect("seed jsonl");
    (tmp, root)
}

#[test]
fn test_trace_parse_and_schema_validate_seeded_jsonl() {
    let (_tmp, root) = seed_telemetry_root();
    let report = load_telemetry_dir(&root);
    assert_eq!(report.files_scanned, 1);
    assert_eq!(report.lines_scanned, 4);
    // The malformed line is skipped, never fatal.
    assert_eq!(report.skipped_lines, 1);
    assert_eq!(report.records.len(), 3);

    // Same text-level parse semantics as query.ts `parseJsonlRecords`.
    let raw = std::fs::read_to_string(root.join("2026-11-28/events.jsonl")).expect("read seed");
    let (parsed, skipped) = parse_jsonl_text(&raw);
    assert_eq!(parsed.len(), 3);
    assert_eq!(skipped, 1);

    // Every seeded record is schema-valid (span.json + event.json shapes).
    for loaded in &report.records {
        assert!(
            validate_record(&loaded.record).is_empty(),
            "seed must validate: {:?}",
            validate_record(&loaded.record)
        );
    }

    // Malformed ids fail validation with field-level errors.
    let bad = serde_json::json!({
        "schema_version": 1, "kind": "span",
        "trace_id": "ZZZ", "span_id": "00f067aa0ba902b7",
        "op": "text.compare", "component": "rs.cmd/text",
        "started_at": "2026-11-28T09:14:02Z", "status": "ok",
    });
    let errors = validate_record(&bad);
    assert!(errors.contains(&"bad_trace_id".to_string()), "errors: {errors:?}");

    // Trace lookup, filters, and error surfacing.
    assert_eq!(records_for_trace(&report.records, TRACE_OK).len(), 1);
    assert_eq!(records_for_trace(&report.records, TRACE_ERR).len(), 2);
    let query = ListQuery {
        op: Some("text.compare".to_string()),
        status: Some("ok".to_string()),
        limit: 50,
        ..ListQuery::default()
    };
    let rows = filter_list(&report.records, &query);
    assert_eq!(rows.len(), 1);
    assert!(is_span(&rows[0].record));
    let errors_only = ListQuery { limit: 50, ..ListQuery::default() };
    let error_rows = filter_errors(&report.records, &errors_only);
    assert_eq!(error_rows.len(), 2);
    assert!(error_rows.iter().all(|loaded| is_error_record(&loaded.record)));

    // Stats + helpers used by `trace stats --json`.
    let stats = compute_stats(&report.records, None);
    assert_eq!(stats.spans_total, 2);
    assert_eq!(stats.events_total, 1);
    assert_eq!(stats.traces_total, 2);
    assert_eq!(stats.error_spans, 1);
    assert_eq!(stats.by_op.get("text.compare"), Some(&1));
    assert!(stats.duration_ns_p50 > 0);
    assert_eq!(parse_since_secs("24h"), Some(86_400));
    assert_eq!(parse_since_secs("bogus"), None);
    assert!(parse_rfc3339_epoch_secs("2026-11-28T09:14:02Z").is_some());
    assert_eq!(parse_rfc3339_epoch_secs("not-a-date"), None);

    // Secrets are redacted through the telemetry redact gate.
    let redacted = redact_record(&report.records[0].record);
    let token = redacted
        .get("attributes")
        .and_then(|attrs| attrs.get("api_token"))
        .and_then(|value| value.as_str())
        .expect("redacted token");
    assert_eq!(token, "[redacted]");
}

#[test]
fn test_trace_bundle_emits_zip() {
    let (_tmp, root) = seed_telemetry_root();
    let report = load_telemetry_dir(&root);
    let out = root.join("trace-bundle.zip");
    let bundle = bundle_trace(&report.records, TRACE_ERR, &out).expect("bundle");
    assert_eq!(bundle.spans, 1);
    assert_eq!(bundle.events, 1);
    assert!(out.exists());

    // The bundle is a real zip with the three expected members.
    let file = std::fs::File::open(&out).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("read zip");
    assert_eq!(archive.len(), 3);
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("zip entry");
        names.push(entry.name().to_string());
        let mut contents = String::new();
        entry.read_to_string(&mut contents).expect("zip read");
        assert!(!contents.is_empty());
    }
    assert!(names.contains(&"trace.json".to_string()));
    assert!(names.contains(&"inputs.redacted.json".to_string()));
    assert!(names.contains(&"manifest.json".to_string()));

    // Unknown trace ids fail instead of writing an empty zip.
    let missing = root.join("missing.zip");
    assert!(bundle_trace(&report.records, &"0".repeat(32), &missing).is_err());
    assert!(!missing.exists());
}

#[test]
fn test_trace_cli_end_to_end_show_list_errors_stats_bundle() {
    let (_tmp, root) = seed_telemetry_root();
    let dir = root.to_str().expect("path").to_string();

    let show = Command::new(bin_path())
        .args(["trace", "show", TRACE_OK, "--dir", &dir])
        .output()
        .expect("run show");
    assert!(show.status.success());
    let show_text = String::from_utf8_lossy(&show.stdout);
    assert!(show_text.contains("text.compare"), "show: {show_text}");

    let show_json = Command::new(bin_path())
        .args(["trace", "show", TRACE_OK, "--dir", &dir, "--json"])
        .output()
        .expect("run show json");
    assert!(show_json.status.success());
    let payload: serde_json::Value =
        serde_json::from_slice(&show_json.stdout).expect("show json parses");
    assert_eq!(
        payload.get("trace_id").and_then(|value| value.as_str()),
        Some(TRACE_OK)
    );
    assert_eq!(
        payload.get("span_count").and_then(|value| value.as_u64()),
        Some(1)
    );
    // `--json` carries full (redacted) records: the seeded secret is gone.
    let payload_text = String::from_utf8_lossy(&show_json.stdout);
    assert!(payload_text.contains("[redacted]"), "show redacts: {payload_text}");
    assert!(!payload_text.contains("hunter2"), "secret leaked: {payload_text}");

    let list = Command::new(bin_path())
        .args(["trace", "list", "--op", "sync.apply", "--dir", &dir, "--json"])
        .output()
        .expect("run list");
    assert!(list.status.success());
    let list_payload: serde_json::Value =
        serde_json::from_slice(&list.stdout).expect("list json parses");
    assert_eq!(
        list_payload.get("count").and_then(|value| value.as_u64()),
        Some(1)
    );

    let errors = Command::new(bin_path())
        .args(["trace", "errors", "--dir", &dir])
        .output()
        .expect("run errors");
    assert!(errors.status.success());
    assert!(String::from_utf8_lossy(&errors.stdout).contains("sync.apply"));

    let stats = Command::new(bin_path())
        .args(["trace", "stats", "--dir", &dir, "--json"])
        .output()
        .expect("run stats");
    assert!(stats.status.success());
    let stats_payload: serde_json::Value =
        serde_json::from_slice(&stats.stdout).expect("stats json parses");
    assert_eq!(
        stats_payload.get("spans_total").and_then(|value| value.as_u64()),
        Some(2)
    );

    let bundle_out = root.join("cli-bundle.zip");
    let bundle = Command::new(bin_path())
        .args([
            "trace",
            "bundle",
            TRACE_ERR,
            "--dir",
            &dir,
            "--out",
            bundle_out.to_str().expect("path"),
        ])
        .output()
        .expect("run bundle");
    assert!(bundle.status.success(), "bundle failed");
    assert!(bundle_out.exists());
}
