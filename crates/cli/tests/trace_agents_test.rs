#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! ODD feed-in loop dashboard tests: `trace agents` aggregates the five
//! `agent.*` loop events (recorded as ordinary telemetry events in the
//! same JSONL store) into per-agent counts. Seeds two agents/sessions plus
//! edge cases (validate without patch, events with missing fields, and
//! interleaved non-agent records that must be ignored).

use std::path::PathBuf;
use std::process::Command;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_autocompare")
}

fn agent_event(
    name: &str,
    trace: &str,
    span: &str,
    at: &str,
    severity: &str,
    attrs: &str,
) -> String {
    format!(
        "{{\"schema_version\":1,\"kind\":\"event\",\"name\":\"{name}\",\"trace_id\":\"{trace}\",\"span_id\":\"{span}\",\"at\":\"{at}\",\"severity\":\"{severity}\",\"attributes\":{attrs}}}"
    )
}

fn app_span(trace: &str, span: &str, status: &str, at: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"kind\":\"span\",\"trace_id\":\"{trace}\",\"span_id\":\"{span}\",\"parent_span_id\":null,\"op\":\"text.compare\",\"component\":\"rs.cmd/text\",\"started_at\":\"{at}\",\"duration_ns\":1000,\"status\":\"{status}\",\"attributes\":{{\"transport\":\"tauri\"}},\"redaction\":\"v1-default\"}}"
    )
}

const TRACE_A1: &str = "a1111111111111111111111111111111";
const TRACE_A2: &str = "a2222222222222222222222222222222";
const TRACE_B1: &str = "b1111111111111111111111111111111";
const SPAN_A: &str = "a000000000000001";
const SPAN_B: &str = "b000000000000001";

fn seed_body() -> String {
    vec![
        // agent_alpha / sess_1, iteration 1: full loop ending in an ok validation.
        agent_event(
            "agent.loop_start", TRACE_A1, SPAN_A, "2026-11-28T09:00:00Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":1,\"trace_id\":\"a1111111111111111111111111111111\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.trace_fetch", TRACE_A1, SPAN_A, "2026-11-28T09:00:05Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":1,\"trace_id\":\"a1111111111111111111111111111111\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.diagnose_read", TRACE_A1, SPAN_A, "2026-11-28T09:00:09Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":1,\"trace_id\":\"a1111111111111111111111111111111\",\"failure_shape\":\"contract_drift\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.patch_apply", TRACE_A1, SPAN_A, "2026-11-28T09:01:00Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":1,\"trace_id\":\"a1111111111111111111111111111111\",\"failure_shape\":\"contract_drift\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.validate_rerun", TRACE_A1, SPAN_A, "2026-11-28T09:02:00Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":1,\"trace_id\":\"a1111111111111111111111111111111\",\"outcome\":\"ok\"}",
        ),
        // agent_alpha / sess_1, iteration 2: patch then a failing validation.
        agent_event(
            "agent.loop_start", TRACE_A2, SPAN_A, "2026-11-28T10:00:00Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":2,\"trace_id\":\"a2222222222222222222222222222222\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.trace_fetch", TRACE_A2, SPAN_A, "2026-11-28T10:00:05Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":2,\"trace_id\":\"a2222222222222222222222222222222\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.diagnose_read", TRACE_A2, SPAN_A, "2026-11-28T10:00:09Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":2,\"trace_id\":\"a2222222222222222222222222222222\",\"failure_shape\":\"serde_mismatch\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.patch_apply", TRACE_A2, SPAN_A, "2026-11-28T10:01:00Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":2,\"trace_id\":\"a2222222222222222222222222222222\",\"failure_shape\":\"serde_mismatch\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.validate_rerun", TRACE_A2, SPAN_A, "2026-11-28T10:02:00Z", "info",
            "{\"agent_id\":\"agent_alpha\",\"session_id\":\"sess_1\",\"iteration\":2,\"trace_id\":\"a2222222222222222222222222222222\",\"outcome\":\"fail\"}",
        ),
        // agent_beta / sess_2, iteration 1: validate WITHOUT any patch (must not count as a fix).
        agent_event(
            "agent.loop_start", TRACE_B1, SPAN_B, "2026-11-28T11:00:00Z", "info",
            "{\"agent_id\":\"agent_beta\",\"session_id\":\"sess_2\",\"iteration\":1,\"trace_id\":\"b1111111111111111111111111111111\",\"outcome\":\"ok\"}",
        ),
        agent_event(
            "agent.validate_rerun", TRACE_B1, SPAN_B, "2026-11-28T11:02:00Z", "info",
            "{\"agent_id\":\"agent_beta\",\"session_id\":\"sess_2\",\"iteration\":1,\"trace_id\":\"b1111111111111111111111111111111\",\"outcome\":\"ok\"}",
        ),
        // Edge: agent event missing agent_id/session_id must be tolerated, not counted.
        agent_event(
            "agent.loop_start", TRACE_B1, SPAN_B, "2026-11-28T11:03:00Z", "info",
            "{\"iteration\":9,\"trace_id\":\"b1111111111111111111111111111111\",\"outcome\":\"ok\"}",
        ),
        // Edge: non-agent records interleaved (ordinary app span + error event) must be ignored.
        app_span(TRACE_B1, SPAN_B, "ok", "2026-11-28T11:04:00Z"),
        agent_event(
            "ipc.serde_error", TRACE_B1, SPAN_B, "2026-11-28T11:05:00Z", "error",
            "{\"component\":\"rs.cmd/sync\",\"field\":\"dry_run\"}",
        ),
    ]
    .join("\n")
        + "\n"
}

fn seed_root(body: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::Builder::new()
        .prefix("odd-agents-")
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

fn run_agents(dir: &str, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["trace", "agents", "--dir", dir];
    args.extend_from_slice(extra);
    Command::new(bin_path())
        .args(&args)
        .output()
        .expect("run trace agents")
}

fn is_snake_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

#[test]
fn test_agents_human_table_counts() {
    let (_tmp, root) = seed_root(&seed_body());
    let dir = root.to_str().expect("utf8 path").to_string();
    let out = run_agents(&dir, &[]);
    assert!(out.status.success(), "agents failed: {out:?}");
    let text = String::from_utf8_lossy(&out.stdout);

    // Both cohorts appear; the malformed event contributes no phantom cohort.
    assert!(text.contains("agent_alpha"), "table must name agent_alpha");
    assert!(text.contains("agent_beta"), "table must name agent_beta");
    assert!(text.contains("sess_1") && text.contains("sess_2"));
    assert!(!text.contains("iteration\":9"), "missing-field event must not leak");

    // Alpha row: 2 iterations, 2 fetched, 2 diagnosed, 2 patched,
    // 2 validations, 1 fix (iteration 2 failed validation), 50%.
    let alpha = text
        .lines()
        .find(|line| line.contains("agent_alpha"))
        .expect("alpha row")
        .to_string();
    for want in ["2", "2", "2", "2", "1", "50%"] {
        assert!(alpha.contains(want), "alpha row missing {want}: {alpha}");
    }
    // Beta row: 1 iteration, 0 fetched/diagnosed/patched, 1 validation,
    // 0 fixes (validate without patch is not a fix), 0%.
    let beta = text
        .lines()
        .find(|line| line.contains("agent_beta"))
        .expect("beta row")
        .to_string();
    assert!(beta.contains("0%"), "beta fix rate must be 0%: {beta}");

    // Non-agent records never surface in the dashboard.
    assert!(!text.contains("serde_error"), "non-agent events must be ignored");
    assert!(!text.contains("text.compare"), "app spans must be ignored");
}

#[test]
fn test_agents_json_envelope_keys() {
    let (_tmp, root) = seed_root(&seed_body());
    let dir = root.to_str().expect("utf8 path").to_string();
    let out = run_agents(&dir, &["--json"]);
    assert!(out.status.success(), "agents --json failed: {out:?}");
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("agents --json must parse");

    assert_eq!(
        payload.get("schema_version").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    for key in payload.as_object().expect("object").keys() {
        assert!(is_snake_key(key), "envelope key not snake_case: {key}");
    }
    assert_eq!(
        payload.get("groups_total").and_then(serde_json::Value::as_u64),
        Some(2)
    );
    assert_eq!(
        payload.get("iterations_total").and_then(serde_json::Value::as_u64),
        Some(3),
        "2 alpha + 1 beta iterations; the missing-field event is skipped"
    );
    assert_eq!(
        payload.get("fixes_total").and_then(serde_json::Value::as_u64),
        Some(1)
    );

    let groups = payload
        .get("groups")
        .and_then(serde_json::Value::as_array)
        .expect("groups array");
    assert_eq!(groups.len(), 2);
    for group in groups {
        let map = group.as_object().expect("group object");
        for required in [
            "agent_id",
            "session_id",
            "iterations",
            "traces_fetched",
            "diagnoses_read",
            "patches_applied",
            "validations",
            "fixes",
            "fix_rate_pct",
            "first_at",
            "last_at",
        ] {
            assert!(map.contains_key(required), "group missing {required}");
        }
        for key in map.keys() {
            assert!(is_snake_key(key), "group key not snake_case: {key}");
        }
    }
    let by_id: std::collections::BTreeMap<&str, &serde_json::Value> = groups
        .iter()
        .map(|group| {
            (
                group
                    .get("agent_id")
                    .and_then(serde_json::Value::as_str)
                    .expect("agent_id"),
                group,
            )
        })
        .collect();
    let alpha = by_id["agent_alpha"];
    assert_eq!(
        alpha.get("fixes").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    assert_eq!(
        alpha
            .get("fix_rate_pct")
            .and_then(serde_json::Value::as_u64),
        Some(50)
    );
    assert_eq!(
        alpha.get("first_at").and_then(serde_json::Value::as_str),
        Some("2026-11-28T09:00:00Z")
    );
    let beta = by_id["agent_beta"];
    assert_eq!(
        beta.get("validations").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    assert_eq!(
        beta.get("fixes").and_then(serde_json::Value::as_u64),
        Some(0),
        "validate without patch must not count as a fix"
    );

    let days = payload
        .get("iterations_by_day")
        .and_then(serde_json::Value::as_object)
        .expect("iterations_by_day");
    assert_eq!(
        days.get("2026-11-28").and_then(serde_json::Value::as_u64),
        Some(3)
    );
    let shapes = payload
        .get("top_failure_shapes")
        .and_then(serde_json::Value::as_array)
        .expect("top_failure_shapes");
    let names: Vec<&str> = shapes
        .iter()
        .filter_map(|shape| {
            shape
                .get("failure_shape")
                .and_then(serde_json::Value::as_str)
        })
        .collect();
    assert!(names.contains(&"contract_drift"), "shapes: {names:?}");
    assert!(names.contains(&"serde_mismatch"), "shapes: {names:?}");
}

#[test]
fn test_agents_html_contains_agents_and_svg() {
    let (_tmp, root) = seed_root(&seed_body());
    let dir = root.to_str().expect("utf8 path").to_string();
    let html_path = root.join("agents.html");
    let html_str = html_path.to_str().expect("utf8 path").to_string();
    let out = run_agents(&dir, &["--html", &html_str]);
    assert!(out.status.success(), "agents --html failed: {out:?}");
    let html = std::fs::read_to_string(&html_path).expect("read html");
    assert!(html.contains("agent_alpha"), "html must name agent_alpha");
    assert!(html.contains("agent_beta"), "html must name agent_beta");
    assert!(html.contains("<svg"), "html must embed inline svg bars");
    assert!(
        html.contains("contract_drift"),
        "html must list top failure shapes"
    );
    assert!(
        !html.contains("<script src")
            && !html.contains("<link ")
            && !html.contains("<img "),
        "html must be self-contained with no external deps"
    );
}

#[test]
fn test_agents_empty_store() {
    let (_tmp, root) = seed_root(&app_span(TRACE_B1, SPAN_B, "ok", "2026-11-28T11:04:00Z"));
    let dir = root.to_str().expect("utf8 path").to_string();
    let out = run_agents(&dir, &[]);
    assert!(out.status.success(), "agents on app-only store failed: {out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("no agent loop events"), "table: {text}");
    let json_out = run_agents(&dir, &["--json"]);
    let payload: serde_json::Value =
        serde_json::from_slice(&json_out.stdout).expect("empty --json must parse");
    assert_eq!(
        payload.get("groups_total").and_then(serde_json::Value::as_u64),
        Some(0)
    );
    assert_eq!(
        payload.get("schema_version").and_then(serde_json::Value::as_u64),
        Some(1)
    );
}
