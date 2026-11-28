#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! ODD Phase 10 agent-query tests: the Phase 9 `trace --json` surface is
//! the agent API. These tests prove (1) every `--json` envelope parses and
//! conforms to the checked-in `telemetry/schema/v1/*.json` contracts
//! (read-only: schemas are asserted against, never written), with stable
//! snake_case keys; and (2) a scripted agent loop localizes a seeded
//! `sync.apply` snake_case fault from trace data alone — no source hints.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use autocompare_cli::trace::{load_telemetry_dir, records_for_trace, validate_record};

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_autocompare")
}

/// Checked-in schema dir. Read-only: tests only ever read these files.
fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../telemetry/schema/v1")
}

fn schema_json(name: &str) -> serde_json::Value {
    let path = schema_dir().join(name);
    let text = std::fs::read_to_string(&path).expect("read schema");
    serde_json::from_str(&text).expect("parse schema")
}

fn required_keys(schema: &serde_json::Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn allowed_keys(schema: &serde_json::Value) -> BTreeSet<String> {
    schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default()
}

fn is_snake_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

// Seeded bug: sync.apply snake_case drift (engine contract `dry_run`,
// one layer emits `dryRun`), modelled after the live mock drift in
// ODD section 3.2. Traceable with no source access.
const BUG_TRACE: &str = "b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0";
const BUG_ROOT: &str = "1111111111111111";
const BUG_CHILD: &str = "2222222222222222";
const BUG_LEAF: &str = "3333333333333333";
const FIX_TRACE: &str = "f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1";
const OK_TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";

fn bug_trace_body() -> String {
    [
        // Root span (ui layer, ok).
        format!(
            "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{BUG_TRACE}\",\"span_id\":\"{BUG_ROOT}\",\"parent_span_id\":null,\"op\":\"sync.plan\",\"component\":\"ts.ui/app\",\"started_at\":\"2026-11-28T09:14:02Z\",\"duration_ns\":18420311,\"status\":\"ok\",\"attributes\":{{\"transport\":\"tauri\",\"api_token\":\"hunter2 token\"}},\"redaction\":\"v1-default\"}}"
        ),
        // Child span (ipc layer, ok).
        format!(
            "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{BUG_TRACE}\",\"span_id\":\"{BUG_CHILD}\",\"parent_span_id\":\"{BUG_ROOT}\",\"op\":\"sync.plan\",\"component\":\"ts.ipc/client\",\"started_at\":\"2026-11-28T09:14:02Z\",\"duration_ns\":5211077,\"status\":\"ok\",\"attributes\":{{\"transport\":\"tauri\"}},\"redaction\":\"v1-default\"}}"
        ),
        // Failing leaf (rs layer): the seeded fault.
        format!(
            "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{BUG_TRACE}\",\"span_id\":\"{BUG_LEAF}\",\"parent_span_id\":\"{BUG_CHILD}\",\"op\":\"sync.apply\",\"component\":\"rs.cmd/sync\",\"started_at\":\"2026-11-28T09:14:03Z\",\"duration_ns\":903112,\"status\":\"error\",\"attributes\":{{\"transport\":\"tauri\",\"field\":\"dry_run\",\"expected\":\"u64\",\"got\":\"dryRun\"}},\"redaction\":\"v1-default\"}}"
        ),
        // Error event with owner, cause hint, and repro pointer.
        format!(
            "{{\"schema_version\":1,\"kind\":\"event\",\"name\":\"ipc.serde_error\",\"trace_id\":\"{BUG_TRACE}\",\"span_id\":\"{BUG_LEAF}\",\"at\":\"2026-11-28T09:14:03Z\",\"severity\":\"error\",\"attributes\":{{\"component\":\"rs.cmd/sync\",\"field\":\"dry_run\",\"expected\":\"u64\",\"got\":\"dryRun\",\"likely_cause\":\"snake_case drift: response carries dryRun, contract wants dry_run\"}},\"repro\":{{\"e2e\":\"e2e/v2/v2-workflows.spec.ts#sync\",\"fixture\":\"sessions:seed\"}}}}"
        ),
    ]
    .join("\n")
        + "\n"
}

fn fixed_trace_body() -> String {
    // Same shape as the bug, but the drift is fixed: all ok, no errors.
    [
        format!(
            "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{FIX_TRACE}\",\"span_id\":\"4444444444444444\",\"parent_span_id\":null,\"op\":\"sync.plan\",\"component\":\"ts.ui/app\",\"started_at\":\"2026-11-28T09:15:02Z\",\"duration_ns\":18420311,\"status\":\"ok\",\"attributes\":{{\"transport\":\"tauri\"}},\"redaction\":\"v1-default\"}}"
        ),
        format!(
            "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{FIX_TRACE}\",\"span_id\":\"5555555555555555\",\"parent_span_id\":\"4444444444444444\",\"op\":\"sync.plan\",\"component\":\"ts.ipc/client\",\"started_at\":\"2026-11-28T09:15:02Z\",\"duration_ns\":5211077,\"status\":\"ok\",\"attributes\":{{\"transport\":\"tauri\"}},\"redaction\":\"v1-default\"}}"
        ),
        format!(
            "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{FIX_TRACE}\",\"span_id\":\"6666666666666666\",\"parent_span_id\":\"5555555555555555\",\"op\":\"sync.apply\",\"component\":\"rs.cmd/sync\",\"started_at\":\"2026-11-28T09:15:03Z\",\"duration_ns\":903112,\"status\":\"ok\",\"attributes\":{{\"transport\":\"tauri\",\"field\":\"dry_run\"}},\"redaction\":\"v1-default\"}}"
        ),
    ]
    .join("\n")
        + "\n"
}

fn ok_trace_body() -> String {
    format!(
        "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{OK_TRACE}\",\"span_id\":\"00f067aa0ba902b7\",\"parent_span_id\":null,\"op\":\"text.compare\",\"component\":\"rs.cmd/text\",\"started_at\":\"2026-11-28T09:14:02Z\",\"duration_ns\":18420311,\"status\":\"ok\",\"attributes\":{{\"transport\":\"tauri\"}},\"redaction\":\"v1-default\"}}\n"
    )
}

/// Seed `<root>/2026-11-28/events.jsonl` under a fresh /tmp dir.
fn seed_root(body: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::Builder::new()
        .prefix("odd-agent-")
        .tempdir_in("/tmp")
        .expect("tempdir");
    assert!(
        tmp.path().to_str().unwrap_or_default().starts_with("/tmp"),
        "seed must live in /tmp"
    );
    let root = tmp.path().join("telemetry");
    let day = root.join("2026-11-28");
    std::fs::create_dir_all(&day).expect("mkdir day");
    std::fs::write(day.join("events.jsonl"), body).expect("seed jsonl");
    (tmp, root)
}

fn run_trace(args: &[&str]) -> std::process::Output {
    Command::new(bin_path())
        .args(["trace"])
        .args(args)
        .output()
        .expect("run trace cli")
}

fn stdout_json(output: &std::process::Output, context: &str) -> serde_json::Value {
    assert!(output.status.success(), "{context} failed: {output:?}");
    serde_json::from_slice(&output.stdout).expect("cli --json must parse")
}

/// Assert one `--json` envelope: version, snake_case keys, schema shape.
fn assert_envelope(
    payload: &serde_json::Value,
    record_keys: &[&str],
    span_schema: &serde_json::Value,
    event_schema: &serde_json::Value,
    context: &str,
) {
    assert_eq!(
        payload.get("schema_version").and_then(serde_json::Value::as_u64),
        Some(1),
        "{context}: envelope schema_version"
    );
    let obj = payload.as_object().expect("envelope must be an object");
    for key in obj.keys() {
        assert!(is_snake_key(key), "{context}: envelope key not snake_case: {key}");
    }
    let span_required = required_keys(span_schema);
    let span_allowed = allowed_keys(span_schema);
    let event_required = required_keys(event_schema);
    let event_allowed = allowed_keys(event_schema);
    for key in record_keys {
        let records = payload
            .get(key)
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("{context}: missing array field {key}"));
        for record in records {
            let map = record.as_object().expect("record must be an object");
            for record_key in map.keys() {
                assert!(
                    is_snake_key(record_key),
                    "{context}: record key not snake_case: {record_key}"
                );
            }
            match map.get("kind").and_then(serde_json::Value::as_str) {
                Some("span") => {
                    for required in &span_required {
                        assert!(
                            map.contains_key(required),
                            "{context}: span missing {required}"
                        );
                    }
                    for record_key in map.keys() {
                        assert!(
                            span_allowed.contains(record_key),
                            "{context}: span has extra key {record_key}"
                        );
                    }
                    assert!(
                        validate_record(record).is_empty(),
                        "{context}: span fails validation: {:?}",
                        validate_record(record)
                    );
                }
                Some("event") => {
                    for required in &event_required {
                        assert!(
                            map.contains_key(required),
                            "{context}: event missing {required}"
                        );
                    }
                    for record_key in map.keys() {
                        assert!(
                            event_allowed.contains(record_key),
                            "{context}: event has extra key {record_key}"
                        );
                    }
                    assert!(
                        validate_record(record).is_empty(),
                        "{context}: event fails validation: {:?}",
                        validate_record(record)
                    );
                }
                other => panic!("{context}: unknown kind {other:?}"),
            }
        }
    }
}

#[test]
fn test_agent_json_parses_against_checked_in_schemas_read_only() {
    let span_schema = schema_json("span.json");
    let event_schema = schema_json("event.json");
    assert!(
        !required_keys(&span_schema).is_empty(),
        "span schema must declare required keys"
    );
    assert!(
        !required_keys(&event_schema).is_empty(),
        "event schema must declare required keys"
    );
    // Read-only proof: schemas are only read; mtimes must not move.
    let span_path = schema_dir().join("span.json");
    let event_path = schema_dir().join("event.json");
    let ops_path = schema_dir().join("ops.json");
    let before = [
        std::fs::metadata(&span_path).expect("stat span").modified().expect("mtime span"),
        std::fs::metadata(&event_path).expect("stat event").modified().expect("mtime event"),
        std::fs::metadata(&ops_path).expect("stat ops").modified().expect("mtime ops"),
    ];

    let body = bug_trace_body() + &fixed_trace_body() + &ok_trace_body();
    let (_tmp, root) = seed_root(&body);
    let dir = root.to_str().expect("utf8 path").to_string();

    // Library-level: every seeded record validates.
    let report = load_telemetry_dir(&root);
    assert_eq!(report.records.len(), 8);
    for loaded in &report.records {
        assert!(
            validate_record(&loaded.record).is_empty(),
            "seed must validate: {:?}",
            validate_record(&loaded.record)
        );
    }

    // CLI-level: every --json envelope parses and matches the schemas.
    let show = run_trace(&["show", BUG_TRACE, "--dir", &dir, "--json"]);
    let show_payload = stdout_json(&show, "show --json");
    assert_envelope(
        &show_payload,
        &["spans", "events"],
        &span_schema,
        &event_schema,
        "show",
    );
    assert_eq!(
        show_payload.get("trace_id").and_then(serde_json::Value::as_str),
        Some(BUG_TRACE)
    );

    let list = run_trace(&["list", "--op", "sync.apply", "--dir", &dir, "--json"]);
    let list_payload = stdout_json(&list, "list --json");
    assert_envelope(
        &list_payload,
        &["records"],
        &span_schema,
        &event_schema,
        "list",
    );
    assert_eq!(
        list_payload.get("count").and_then(serde_json::Value::as_u64),
        Some(2),
        "two sync.apply spans (bug error + fixed ok)"
    );

    let errors = run_trace(&["errors", "--dir", &dir, "--json"]);
    let errors_payload = stdout_json(&errors, "errors --json");
    assert_envelope(
        &errors_payload,
        &["errors"],
        &span_schema,
        &event_schema,
        "errors",
    );
    assert_eq!(
        errors_payload.get("count").and_then(serde_json::Value::as_u64),
        Some(2),
        "one error span + one error event"
    );

    let stats = run_trace(&["stats", "--dir", &dir, "--json"]);
    let stats_payload = stdout_json(&stats, "stats --json");
    assert_eq!(
        stats_payload.get("schema_version").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    assert_eq!(
        stats_payload.get("spans_total").and_then(serde_json::Value::as_u64),
        Some(7)
    );
    assert_eq!(
        stats_payload.get("error_spans").and_then(serde_json::Value::as_u64),
        Some(1)
    );

    // Redaction: the seeded secret never survives --json output, and the
    // record carrying it (visible in `show`) is masked, not dropped.
    let show_text = String::from_utf8_lossy(&show.stdout);
    assert!(show_text.contains("[redacted]"), "show must redact");
    for (name, output) in [("show", &show), ("list", &list), ("errors", &errors)] {
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(!text.contains("hunter2"), "{name} leaked secret");
    }

    // Schemas untouched by the whole run.
    let after = [
        std::fs::metadata(&span_path).expect("stat span").modified().expect("mtime span"),
        std::fs::metadata(&event_path).expect("stat event").modified().expect("mtime event"),
        std::fs::metadata(&ops_path).expect("stat ops").modified().expect("mtime ops"),
    ];
    assert_eq!(before, after, "schema files must stay read-only");
}

#[test]
fn test_agent_loop_localizes_seeded_snake_case_fault_from_trace_alone() {
    // The agent under test sees ONLY trace --json output. The expected
    // answers below are the seed oracle (what the agent must derive);
    // no source file is opened anywhere in this test besides the
    // telemetry schemas read above (contracts, not hints).
    let (_tmp, root) = seed_root(&bug_trace_body());
    let dir = root.to_str().expect("utf8 path").to_string();

    // Step 1 — fetch: newest errors first, derive the suspect trace id.
    let errors_out = run_trace(&["errors", "--dir", &dir, "--json"]);
    let errors_payload = stdout_json(&errors_out, "agent errors");
    let error_records = errors_payload
        .get("errors")
        .and_then(serde_json::Value::as_array)
        .expect("errors array");
    assert!(!error_records.is_empty(), "seeded bug must surface errors");
    let suspect_trace = error_records[0]
        .get("trace_id")
        .and_then(serde_json::Value::as_str)
        .expect("error carries trace_id")
        .to_string();
    assert_ne!(suspect_trace, "0".repeat(32), "suspect must be a real trace");

    // Step 2 — fetch full trace for the suspect only.
    let show_out = run_trace(&["show", &suspect_trace, "--dir", &dir, "--json"]);
    let case = stdout_json(&show_out, "agent show");
    let spans = case
        .get("spans")
        .and_then(serde_json::Value::as_array)
        .expect("spans array");
    let events = case
        .get("events")
        .and_then(serde_json::Value::as_array)
        .expect("events array");

    // Step 3 — locate: the single error span names op + component.
    let error_spans: Vec<&serde_json::Value> = spans
        .iter()
        .filter(|span| span.get("status").and_then(serde_json::Value::as_str) == Some("error"))
        .collect();
    assert_eq!(error_spans.len(), 1, "exactly one failing span");
    let failing_op = error_spans[0]
        .get("op")
        .and_then(serde_json::Value::as_str)
        .expect("error span op");
    let failing_component = error_spans[0]
        .get("component")
        .and_then(serde_json::Value::as_str)
        .expect("error span component");

    // Step 3b — locate: the error event names field/got/cause + repro.
    let error_events: Vec<&serde_json::Value> = events
        .iter()
        .filter(|event| {
            event.get("severity").and_then(serde_json::Value::as_str) == Some("error")
        })
        .collect();
    assert_eq!(error_events.len(), 1, "exactly one error event");
    let attrs = error_events[0]
        .get("attributes")
        .expect("error event attributes");
    let str_attr = |key: &str| attrs.get(key).and_then(serde_json::Value::as_str);
    let repro = error_events[0].get("repro").expect("error event repro");

    // Step 3c — verdict from trace data alone.
    assert_eq!(failing_op, "sync.apply");
    assert_eq!(failing_component, "rs.cmd/sync");
    assert_eq!(str_attr("component"), Some("rs.cmd/sync"));
    assert_eq!(str_attr("field"), Some("dry_run"));
    assert_eq!(str_attr("got"), Some("dryRun"));
    assert!(
        str_attr("likely_cause").unwrap_or_default().contains("dry_run"),
        "likely_cause must name the contract field"
    );
    assert_eq!(
        repro.get("e2e").and_then(serde_json::Value::as_str),
        Some("e2e/v2/v2-workflows.spec.ts#sync")
    );
    assert_eq!(
        repro.get("fixture").and_then(serde_json::Value::as_str),
        Some("sessions:seed")
    );

    // Completeness of the fetched trace (ODD section 7): one root,
    // every parent resolves, every span ok|error.
    let report = load_telemetry_dir(&root);
    let trace_records = records_for_trace(&report.records, &suspect_trace);
    let trace_spans: Vec<&serde_json::Value> =
        trace_records.iter().map(|loaded| &loaded.record).filter(|record| {
            record.get("kind").and_then(serde_json::Value::as_str) == Some("span")
        }).collect();
    let ids: BTreeSet<&str> = trace_spans
        .iter()
        .filter_map(|span| span.get("span_id").and_then(serde_json::Value::as_str))
        .collect();
    let roots = trace_spans
        .iter()
        .filter(|span| span.get("parent_span_id").is_none_or(serde_json::Value::is_null))
        .count();
    assert_eq!(roots, 1, "complete trace has exactly one root");
    for span in &trace_spans {
        let status = span.get("status").and_then(serde_json::Value::as_str);
        assert!(matches!(status, Some("ok") | Some("error")));
        if let Some(parent) = span.get("parent_span_id").and_then(serde_json::Value::as_str) {
            assert!(ids.contains(parent), "no orphan spans");
        }
    }

    // Step 4 — validate the fix: rerun would emit this all-ok trace.
    // The agent asserts zero new errors and a complete ok trace.
    let (_fixed_tmp, fixed_root) = seed_root(&fixed_trace_body());
    let fixed_dir = fixed_root.to_str().expect("utf8 path").to_string();
    let fixed_errors = run_trace(&[
        "list",
        "--op",
        "sync.apply",
        "--status",
        "error",
        "--dir",
        &fixed_dir,
        "--json",
    ]);
    let fixed_payload = stdout_json(&fixed_errors, "fixed errors");
    assert_eq!(
        fixed_payload.get("count").and_then(serde_json::Value::as_u64),
        Some(0),
        "fixed trace carries zero error spans"
    );
    let fixed_show = run_trace(&["show", FIX_TRACE, "--dir", &fixed_dir, "--json"]);
    let fixed_case = stdout_json(&fixed_show, "fixed show");
    let fixed_spans = fixed_case
        .get("spans")
        .and_then(serde_json::Value::as_array)
        .expect("fixed spans");
    assert_eq!(fixed_spans.len(), 3);
    assert!(
        fixed_spans.iter().all(|span| span
            .get("status")
            .and_then(serde_json::Value::as_str)
            == Some("ok")),
        "fixed trace is all-ok"
    );
    assert!(
        fixed_case
            .get("events")
            .and_then(serde_json::Value::as_array)
            .expect("fixed events")
            .is_empty(),
        "fixed trace carries zero events"
    );
}
