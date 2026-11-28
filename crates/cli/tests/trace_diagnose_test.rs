#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::vec_init_then_push,
    clippy::too_many_arguments
)]

//! ODD Phase 11 diagnostics tests: seeded traces per failure shape assert
//! the correct `failure_shape` (+ cause/fix), and completeness scoring
//! (both-endpoints percent + error-span-zero) via `trace diagnose --json`.

use std::path::PathBuf;
use std::process::Command;

use autocompare_cli::diagnose::{
    SHAPE_CONTRACT_DRIFT, SHAPE_MISSING_ECHO, SHAPE_OK, SHAPE_PANIC, SHAPE_SERDE_MISMATCH,
    SHAPE_TIMEOUT, completeness_for_trace, diagnose_loaded,
};
use autocompare_cli::trace::load_telemetry_dir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_autocompare")
}

const TRACE_DRIFT: &str = "d1111111111111111111111111111111";
const TRACE_SERDE: &str = "d2222222222222222222222222222222";
const TRACE_ECHO: &str = "d3333333333333333333333333333333";
const TRACE_TIMEOUT: &str = "d4444444444444444444444444444444";
const TRACE_PANIC: &str = "d5555555555555555555555555555555";
const TRACE_OK: &str = "d6666666666666666666666666666666";

fn span(
    trace: &str,
    span_id: &str,
    parent: Option<&str>,
    op: &str,
    component: &str,
    status: &str,
    duration: Option<u64>,
    attrs: &str,
) -> String {    let parent_json = parent.map_or("null".to_string(), |parent| format!("\"{parent}\""));
    let duration_json = duration.map_or("".to_string(), |ns| format!(",\"duration_ns\":{ns}"));
    format!(
        "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{trace}\",\"span_id\":\"{span_id}\",\"parent_span_id\":{parent_json},\"op\":\"{op}\",\"component\":\"{component}\",\"started_at\":\"2026-11-28T09:14:02Z\"{duration_json},\"status\":\"{status}\",\"attributes\":{attrs},\"redaction\":\"v1-default\"}}"
    )
}

fn event(trace: &str, span_id: &str, name: &str, severity: &str, attrs: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"kind\":\"event\",\"name\":\"{name}\",\"trace_id\":\"{trace}\",\"span_id\":\"{span_id}\",\"at\":\"2026-11-28T09:14:03Z\",\"severity\":\"{severity}\",\"attributes\":{attrs}}}"
    )
}

fn seed_body() -> String {
    let mut lines: Vec<String> = Vec::new();
    // 1. contract drift: field dry_run vs got dryRun (+ serde envelope).
    lines.push(span(
        TRACE_DRIFT, "a111111111111111", None, "sync.plan", "ts.ui/app", "ok",
        Some(1000), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_DRIFT, "a222222222222222", Some("a111111111111111"), "sync.plan",
        "ts.ipc/client", "ok", Some(900), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_DRIFT, "a333333333333333", Some("a222222222222222"), "sync.apply",
        "rs.cmd/sync", "error", Some(800),
        "{\"transport\":\"tauri\",\"field\":\"dry_run\",\"expected\":\"u64\",\"got\":\"dryRun\"}",
    ));
    lines.push(event(
        TRACE_DRIFT, "a333333333333333", "ipc.serde_error", "error",
        "{\"component\":\"rs.cmd/sync\",\"field\":\"dry_run\",\"expected\":\"u64\",\"got\":\"dryRun\"}",
    ));
    // 2. serde mismatch (non-drift type break): count u64 vs missing.
    lines.push(span(
        TRACE_SERDE, "b111111111111111", None, "text.compare", "ts.ui/app", "ok",
        Some(1000), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_SERDE, "b222222222222222", Some("b111111111111111"), "text.compare",
        "rs.cmd/text", "error", Some(700),
        "{\"field\":\"count\",\"expected\":\"u64\",\"got\":\"missing\"}",
    ));
    lines.push(event(
        TRACE_SERDE, "b222222222222222", "ipc.serde_error", "error",
        "{\"component\":\"rs.cmd/text\",\"field\":\"count\",\"expected\":\"u64\",\"got\":\"missing\"}",
    ));
    // 3. missing echo: ts spans only, no rs.* layer.
    lines.push(span(
        TRACE_ECHO, "c111111111111111", None, "sync.plan", "ts.ui/app", "ok",
        Some(1000), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_ECHO, "c222222222222222", Some("c111111111111111"), "sync.plan",
        "ts.ipc/client", "ok", Some(900), "{\"transport\":\"tauri\"}",
    ));
    // 4. timeout: huge duration + timeout markers.
    lines.push(span(
        TRACE_TIMEOUT, "e111111111111111", None, "folder.walk", "ts.ui/app", "ok",
        Some(1000), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_TIMEOUT, "e222222222222222", Some("e111111111111111"), "folder.walk",
        "rs.cmd/folder", "error", Some(9_000_000_000),
        "{\"timeout_ms\":\"5000\",\"error\":\"deadline_exceeded\"}",
    ));
    lines.push(event(
        TRACE_TIMEOUT, "e222222222222222", "op.timeout", "error",
        "{\"component\":\"rs.cmd/folder\",\"timeout_ms\":\"5000\"}",
    ));
    // 5. panic: unwind markers.
    lines.push(span(
        TRACE_PANIC, "f111111111111111", None, "merge.apply", "ts.ui/app", "ok",
        Some(1000), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_PANIC, "f222222222222222", Some("f111111111111111"), "merge.apply",
        "rs.cmd/merge", "error", Some(600),
        "{\"panic_message\":\"index out of bounds\",\"error\":\"catch_unwind\"}",
    ));
    lines.push(event(
        TRACE_PANIC, "f222222222222222", "cmd.panic", "error",
        "{\"component\":\"rs.cmd/merge\",\"panic\":\"index out of bounds\"}",
    ));
    // 6. ok trace: complete, zero errors.
    lines.push(span(
        TRACE_OK, "0111111111111111", None, "text.compare", "ts.ui/app", "ok",
        Some(1000), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_OK, "0222222222222222", Some("0111111111111111"), "text.compare",
        "ts.ipc/client", "ok", Some(900), "{\"transport\":\"tauri\"}",
    ));
    lines.push(span(
        TRACE_OK, "0333333333333333", Some("0222222222222222"), "text.compare",
        "rs.cmd/text", "ok", Some(800), "{\"transport\":\"tauri\"}",
    ));
    lines.join("\n") + "\n"
}

fn seed_root(body: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::Builder::new()
        .prefix("odd-diagnose-")
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

fn is_snake_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn diagnose_json(dir: &str, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["trace", "diagnose", "--dir", dir, "--json"];
    args.extend_from_slice(extra);
    let output = Command::new(bin_path())
        .args(&args)
        .output()
        .expect("run diagnose");
    assert!(output.status.success(), "diagnose failed: {output:?}");
    serde_json::from_slice(&output.stdout).expect("diagnose --json must parse")
}

#[test]
fn test_diagnose_library_shapes_and_completeness() {
    let (_tmp, root) = seed_root(&seed_body());
    let report = load_telemetry_dir(&root);
    assert_eq!(report.records.len(), 18);
    let diagnosis = diagnose_loaded(&report.records);
    assert_eq!(diagnosis.traces_total, 6);
    // Aggregate: one span lacks no endpoints here (all seeded complete),
    // so completeness is 100; errors exist so error_span_zero is false.
    assert_eq!(diagnosis.completeness_pct, 100);
    assert!(!diagnosis.error_span_zero);
    assert_eq!(diagnosis.error_spans, 4);

    let by_trace: std::collections::BTreeMap<String, Vec<String>> = diagnosis
        .traces
        .iter()
        .map(|trace| {
            (
                trace.trace_id.clone(),
                trace
                    .findings
                    .iter()
                    .map(|finding| finding.failure_shape.clone())
                    .collect(),
            )
        })
        .collect();
    assert!(
        by_trace[TRACE_DRIFT].contains(&SHAPE_CONTRACT_DRIFT.to_string()),
        "drift: {by_trace:?}"
    );
    assert!(
        by_trace[TRACE_SERDE].contains(&SHAPE_SERDE_MISMATCH.to_string()),
        "serde: {by_trace:?}"
    );
    assert!(
        by_trace[TRACE_ECHO].contains(&SHAPE_MISSING_ECHO.to_string()),
        "echo: {by_trace:?}"
    );
    assert!(
        by_trace[TRACE_TIMEOUT].contains(&SHAPE_TIMEOUT.to_string()),
        "timeout: {by_trace:?}"
    );
    assert!(
        by_trace[TRACE_PANIC].contains(&SHAPE_PANIC.to_string()),
        "panic: {by_trace:?}"
    );
    // Ok trace: no findings, shape ok by absence, complete + error-free.
    assert!(by_trace[TRACE_OK].is_empty(), "ok trace must be clean");
    let ok = diagnosis
        .traces
        .iter()
        .find(|trace| trace.trace_id == TRACE_OK)
        .expect("ok trace");
    assert_eq!(ok.completeness.completeness_pct, 100);
    assert!(ok.completeness.error_span_zero);
    assert!(ok.completeness.complete);

    // Completeness endpoint check: a span missing duration_ns scores <100.
    let incomplete = serde_json::json!({
        "schema_version": 1, "kind": "span",
        "trace_id": TRACE_OK, "span_id": "0999999999999999",
        "parent_span_id": "0111111111111111",
        "op": "text.compare", "component": "rs.cmd/text",
        "started_at": "2026-11-28T09:14:02Z", "status": "ok",
        "attributes": {}, "redaction": "v1-default"
    });
    let refs = vec![&incomplete];
    let partial = completeness_for_trace(TRACE_OK, &refs);
    assert_eq!(partial.completeness_pct, 0);
    assert!(!partial.complete);
    assert!(partial.reasons.iter().any(|reason| reason.starts_with("missing_endpoints")));
    assert_eq!(SHAPE_OK, "ok");
}

#[test]
fn test_diagnose_cli_json_snake_case_per_shape() {
    let (_tmp, root) = seed_root(&seed_body());
    let dir = root.to_str().expect("utf8 path").to_string();

    let payload = diagnose_json(&dir, &[]);
    assert_eq!(
        payload.get("schema_version").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    for key in payload.as_object().expect("object").keys() {
        assert!(is_snake_key(key), "envelope key not snake_case: {key}");
    }
    assert_eq!(
        payload.get("traces_total").and_then(serde_json::Value::as_u64),
        Some(6)
    );
    assert_eq!(
        payload.get("completeness_pct").and_then(serde_json::Value::as_u64),
        Some(100)
    );
    assert_eq!(
        payload.get("error_span_zero").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    let findings = payload
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .expect("findings array");
    assert!(!findings.is_empty());
    for finding in findings {
        let map = finding.as_object().expect("finding object");
        for key in map.keys() {
            assert!(is_snake_key(key), "finding key not snake_case: {key}");
        }
        for required in [
            "failure_shape",
            "likely_cause",
            "suggested_fix",
            "trace_id",
            "span_id",
            "op",
            "component",
        ] {
            assert!(map.contains_key(required), "finding missing {required}");
        }
        let shape = map
            .get("failure_shape")
            .and_then(serde_json::Value::as_str)
            .expect("shape");
        assert!(
            [
                SHAPE_CONTRACT_DRIFT,
                SHAPE_SERDE_MISMATCH,
                SHAPE_MISSING_ECHO,
                SHAPE_TIMEOUT,
                SHAPE_PANIC
            ]
            .contains(&shape)
                || shape == "unknown",
            "unexpected shape {shape}"
        );
        assert!(
            !map
                .get("likely_cause")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .is_empty()
        );
        assert!(
            !map
                .get("suggested_fix")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .is_empty()
        );
    }
    let shapes: Vec<&str> = findings
        .iter()
        .filter_map(|finding| finding.get("failure_shape").and_then(serde_json::Value::as_str))
        .collect();
    for want in [
        SHAPE_CONTRACT_DRIFT,
        SHAPE_SERDE_MISMATCH,
        SHAPE_MISSING_ECHO,
        SHAPE_TIMEOUT,
        SHAPE_PANIC,
    ] {
        assert!(shapes.contains(&want), "missing shape {want}: {shapes:?}");
    }

    // Single-trace mode: drift trace names the contract field + fix.
    let one = diagnose_json(&dir, &["--trace-id", TRACE_DRIFT]);
    assert_eq!(
        one.get("trace_id").and_then(serde_json::Value::as_str),
        Some(TRACE_DRIFT)
    );
    let one_findings = one
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .expect("trace findings");
    // Drift trace carries the same field/got pair on both the error span
    // and the error event, so both records classify as contract_drift.
    assert_eq!(one_findings.len(), 2);
    for finding in one_findings {
        assert_eq!(
            finding.get("failure_shape").and_then(serde_json::Value::as_str),
            Some(SHAPE_CONTRACT_DRIFT)
        );
    }
    let cause = one_findings[0]
        .get("likely_cause")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let fix = one_findings[0]
        .get("suggested_fix")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    assert!(cause.contains("dry_run") && cause.contains("dryRun"), "cause: {cause}");
    assert!(fix.contains("dry_run"), "fix: {fix}");

    // Ok trace: zero findings, complete, error_span_zero true.
    let ok = diagnose_json(&dir, &["--trace-id", TRACE_OK]);
    assert_eq!(
        ok.get("completeness_pct").and_then(serde_json::Value::as_u64),
        Some(100)
    );
    assert_eq!(
        ok.get("error_span_zero").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        ok.get("complete").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert!(
        ok.get("findings")
            .and_then(serde_json::Value::as_array)
            .expect("ok findings")
            .is_empty()
    );

    // Unknown trace id fails instead of emitting an empty envelope.
    let missing = Command::new(bin_path())
        .args([
            "trace",
            "diagnose",
            "--trace-id",
            &"0".repeat(32),
            "--dir",
            &dir,
            "--json",
        ])
        .output()
        .expect("run diagnose missing");
    assert!(!missing.status.success());
}
