//! ODD feed-in loop usage dashboard (`trace agents`).
//!
//! Loop iterations are recorded as ordinary telemetry `event` records in
//! the same JSONL store (dogfood: LLM usage is itself observable), so no
//! new infrastructure is needed. This module aggregates the five `agent.*`
//! event names into a per-agent table, stable `--json`, and a
//! self-contained offline `--html` dashboard. All three render paths share
//! [`compute_agent_report`], so the numbers cannot diverge.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::error::CliError;
use crate::outcome::Outcome;
use crate::trace::{LoadedRecord, TRACE_JSON_SCHEMA_VERSION, default_telemetry_root, load_telemetry_dir, record_timestamp};

/// Agent loop start: one per feed-in loop iteration.
pub const AGENT_LOOP_START: &str = "agent.loop_start";
/// Agent fetched a trace (`trace errors/list/show --json`).
pub const AGENT_TRACE_FETCH: &str = "agent.trace_fetch";
/// Agent read a `trace diagnose --json` payload.
pub const AGENT_DIAGNOSE_READ: &str = "agent.diagnose_read";
/// Agent applied a patch at the diagnosed component.
pub const AGENT_PATCH_APPLY: &str = "agent.patch_apply";
/// Agent reran validation (gate + diagnose compare).
pub const AGENT_VALIDATE_RERUN: &str = "agent.validate_rerun";
/// Display names for the human table header (order-stable).
pub const AGENT_KNOWN_EVENTS: &[&str] = &[
    AGENT_LOOP_START,
    AGENT_TRACE_FETCH,
    AGENT_DIAGNOSE_READ,
    AGENT_PATCH_APPLY,
    AGENT_VALIDATE_RERUN,
];

/// One dashboard row: a single `(agent_id, session_id)` cohort.
#[derive(Debug, Clone, Default)]
pub struct AgentRow {
    /// `attributes.agent_id` cohort key.
    pub agent_id: String,
    /// `attributes.session_id` cohort key.
    pub session_id: String,
    /// `agent.loop_start` events.
    pub iterations: u64,
    /// `agent.trace_fetch` events.
    pub traces_fetched: u64,
    /// `agent.diagnose_read` events.
    pub diagnoses_read: u64,
    /// `agent.patch_apply` events.
    pub patches_applied: u64,
    /// `agent.validate_rerun` events.
    pub validations: u64,
    /// Distinct iterations with a patch plus an `outcome: ok` validation.
    pub fixes: u64,
    /// `100 * fixes / iterations` (0 when no iterations).
    pub fix_rate_pct: u64,
    /// Earliest `at` timestamp in the cohort.
    pub first_at: String,
    /// Latest `at` timestamp in the cohort.
    pub last_at: String,
}

/// Whole-store aggregation shared by the table, `--json`, and `--html`.
#[derive(Debug, Clone, Default)]
pub struct AgentReport {
    /// Per-cohort rows, sorted by `(agent_id, session_id)`.
    pub groups: Vec<AgentRow>,
    /// `groups.len()` as a count.
    pub groups_total: u64,
    /// Total `agent.loop_start` events.
    pub iterations_total: u64,
    /// Total fixes across cohorts.
    pub fixes_total: u64,
    /// `loop_start` events per `YYYY-MM-DD` day (sorted).
    pub iterations_by_day: BTreeMap<String, u64>,
    /// `(failure_shape, count)` from `diagnose_read` events, count desc.
    pub top_failure_shapes: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Default)]
struct IterFlags {
    patched: bool,
    validated_ok: bool,
}

#[derive(Debug, Clone, Default)]
struct GroupAcc {
    iterations: u64,
    traces_fetched: u64,
    diagnoses_read: u64,
    patches_applied: u64,
    validations: u64,
    first_at: Option<String>,
    last_at: Option<String>,
    iters: BTreeMap<String, IterFlags>,
}

fn is_known_agent_event(record: &serde_json::Value) -> bool {
    if record.get("kind").and_then(serde_json::Value::as_str) != Some("event") {
        return false;
    }
    record
        .get("name")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|name| AGENT_KNOWN_EVENTS.contains(&name))
}

fn attr_str<'a>(record: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    record
        .get("attributes")
        .and_then(serde_json::Value::as_object)
        .and_then(|attrs| attrs.get(key))
        .and_then(serde_json::Value::as_str)
}

fn iteration_key(record: &serde_json::Value) -> Option<String> {
    let attrs = record
        .get("attributes")
        .and_then(serde_json::Value::as_object)?;
    match attrs.get("iteration") {
        Some(serde_json::Value::Number(number)) => Some(number.to_string()),
        Some(serde_json::Value::String(text)) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

fn day_bucket(at: &str) -> Option<String> {
    let day = at.get(..10)?;
    let bytes = day.as_bytes();
    let is_dash = |index: usize| bytes.get(index).is_some_and(|byte| *byte == b'-');
    if !is_dash(4) || !is_dash(7) {
        return None;
    }
    let digits = |range: std::ops::Range<usize>| {
        day.get(range)
            .is_some_and(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    };
    if !digits(0..4) || !digits(5..7) || !digits(8..10) {
        return None;
    }
    Some(day.to_string())
}

/// Aggregate agent loop events over loaded JSONL records.
///
/// Tolerates the documented edge cases: non-agent records are ignored,
/// events missing `agent_id`/`session_id` are skipped, and events missing
/// `iteration` still count toward their counter but cannot form a fix.
pub fn compute_agent_report(records: &[LoadedRecord]) -> AgentReport {
    let mut groups: BTreeMap<(String, String), GroupAcc> = BTreeMap::new();
    let mut days: BTreeMap<String, u64> = BTreeMap::new();
    let mut shapes: BTreeMap<String, u64> = BTreeMap::new();

    for loaded in records {
        let record = &loaded.record;
        if !is_known_agent_event(record) {
            continue;
        }
        let (Some(agent_id), Some(session_id)) =
            (attr_str(record, "agent_id"), attr_str(record, "session_id"))
        else {
            continue;
        };
        if agent_id.is_empty() || session_id.is_empty() {
            continue;
        }
        let key = (agent_id.to_string(), session_id.to_string());
        let acc = groups.entry(key).or_default();
        let name = record
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        match name {
            AGENT_LOOP_START => {
                acc.iterations = acc.iterations.saturating_add(1);
            }
            AGENT_TRACE_FETCH => {
                acc.traces_fetched = acc.traces_fetched.saturating_add(1);
            }
            AGENT_DIAGNOSE_READ => {
                acc.diagnoses_read = acc.diagnoses_read.saturating_add(1);
                if let Some(shape) = attr_str(record, "failure_shape")
                    && !shape.is_empty()
                {
                    *shapes.entry(shape.to_string()).or_insert(0) =
                        shapes.get(shape).copied().unwrap_or(0).saturating_add(1);
                }
            }
            AGENT_PATCH_APPLY => {
                acc.patches_applied = acc.patches_applied.saturating_add(1);
                if let Some(iter) = iteration_key(record) {
                    acc.iters.entry(iter).or_default().patched = true;
                }
            }
            AGENT_VALIDATE_RERUN => {
                acc.validations = acc.validations.saturating_add(1);
                if let Some(iter) = iteration_key(record) {
                    let outcome_ok = record
                        .get("attributes")
                        .and_then(serde_json::Value::as_object)
                        .and_then(|attrs| attrs.get("outcome"))
                        .and_then(serde_json::Value::as_str)
                        == Some("ok");
                    if outcome_ok {
                        acc.iters.entry(iter).or_default().validated_ok = true;
                    } else {
                        acc.iters.entry(iter).or_default();
                    }
                }
            }
            _ => {}
        }
        if name == AGENT_LOOP_START
            && let Some(at) = record_timestamp(record)
            && let Some(day) = day_bucket(at)
        {
            *days.entry(day).or_insert(0) =
                days.get(&day).copied().unwrap_or(0).saturating_add(1);
        }
        if let Some(at) = record_timestamp(record) {
            let newer = |known: &Option<String>| {
                known.as_ref().is_none_or(|prev| at > prev.as_str())
            };
            let older = |known: &Option<String>| {
                known.as_ref().is_none_or(|prev| at < prev.as_str())
            };
            if older(&acc.first_at) {
                acc.first_at = Some(at.to_string());
            }
            if newer(&acc.last_at) {
                acc.last_at = Some(at.to_string());
            }
        }
    }

    let mut rows: Vec<AgentRow> = Vec::new();
    let mut iterations_total: u64 = 0;
    let mut fixes_total: u64 = 0;
    for ((agent_id, session_id), acc) in &groups {
        let fixes = acc
            .iters
            .values()
            .filter(|flags| flags.patched && flags.validated_ok)
            .count() as u64;
        let fix_rate_pct = if acc.iterations == 0 {
            0
        } else {
            fixes.saturating_mul(100).saturating_div(acc.iterations)
        };
        iterations_total = iterations_total.saturating_add(acc.iterations);
        fixes_total = fixes_total.saturating_add(fixes);
        rows.push(AgentRow {
            agent_id: agent_id.clone(),
            session_id: session_id.clone(),
            iterations: acc.iterations,
            traces_fetched: acc.traces_fetched,
            diagnoses_read: acc.diagnoses_read,
            patches_applied: acc.patches_applied,
            validations: acc.validations,
            fixes,
            fix_rate_pct,
            first_at: acc.first_at.clone().unwrap_or_default(),
            last_at: acc.last_at.clone().unwrap_or_default(),
        });
    }

    let mut top_shapes: Vec<(String, u64)> = shapes.into_iter().collect();
    top_shapes.sort_by(|left, right| {
        right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0))
    });

    AgentReport {
        groups_total: rows.len() as u64,
        groups: rows,
        iterations_total,
        fixes_total,
        iterations_by_day: days,
        top_failure_shapes: top_shapes,
    }
}

/// Stable snake_case `--json` envelope (same `schema_version: 1` convention
/// as `trace.rs`).
pub fn report_json(report: &AgentReport) -> serde_json::Value {
    let groups: Vec<serde_json::Value> = report
        .groups
        .iter()
        .map(|row| {
            serde_json::json!({
                "agent_id": row.agent_id,
                "session_id": row.session_id,
                "iterations": row.iterations,
                "traces_fetched": row.traces_fetched,
                "diagnoses_read": row.diagnoses_read,
                "patches_applied": row.patches_applied,
                "validations": row.validations,
                "fixes": row.fixes,
                "fix_rate_pct": row.fix_rate_pct,
                "first_at": row.first_at,
                "last_at": row.last_at,
            })
        })
        .collect();
    let by_day: serde_json::Map<String, serde_json::Value> = report
        .iterations_by_day
        .iter()
        .map(|(day, count)| (day.clone(), serde_json::Value::from(*count)))
        .collect();
    let shapes: Vec<serde_json::Value> = report
        .top_failure_shapes
        .iter()
        .map(|(shape, count)| {
            serde_json::json!({ "failure_shape": shape, "count": count })
        })
        .collect();
    serde_json::json!({
        "schema_version": TRACE_JSON_SCHEMA_VERSION,
        "groups_total": report.groups_total,
        "iterations_total": report.iterations_total,
        "fixes_total": report.fixes_total,
        "groups": groups,
        "iterations_by_day": serde_json::Value::Object(by_day),
        "top_failure_shapes": shapes,
    })
}

/// Human table rendered from the same [`AgentReport`] as `--json`.
pub fn render_table(report: &AgentReport) -> String {
    if report.groups.is_empty() {
        return "no agent loop events\n".to_string();
    }
    let headers = [
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
    ];
    let rows: Vec<Vec<String>> = report
        .groups
        .iter()
        .map(|row| {
            vec![
                row.agent_id.clone(),
                row.session_id.clone(),
                row.iterations.to_string(),
                row.traces_fetched.to_string(),
                row.diagnoses_read.to_string(),
                row.patches_applied.to_string(),
                row.validations.to_string(),
                row.fixes.to_string(),
                format!("{}%", row.fix_rate_pct),
                row.first_at.clone(),
                row.last_at.clone(),
            ]
        })
        .collect();
    let mut widths: Vec<usize> = headers.iter().map(|head| head.len()).collect();
    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index)
                && cell.len() > *width
            {
                *width = cell.len();
            }
        }
    }
    let mut out = format!(
        "agents: {} groups, {} iterations, {} fixes\n",
        report.groups_total, report.iterations_total, report.fixes_total
    );
    out.push_str(&pad_row(&headers, &widths));
    out.push('\n');
    for row in &rows {
        let cells: Vec<&str> = row.iter().map(String::as_str).collect();
        out.push_str(&pad_row(&cells, &widths));
        out.push('\n');
    }
    out
}

fn pad_row(cells: &[&str], widths: &[usize]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(cells.len());
    for (index, cell) in cells.iter().enumerate() {
        let width = widths.get(index).copied().unwrap_or(cell.len());
        let mut padded = (*cell).to_string();
        while padded.len() < width {
            padded.push(' ');
        }
        parts.push(padded);
    }
    parts.join("  ")
}

fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// One inline-SVG bar chart (no external deps, offline-safe).
fn svg_bars(title: &str, pairs: &[(String, u64)], unit: &str) -> String {
    let max = pairs.iter().map(|(_, value)| *value).max().unwrap_or(0);
    let row_h: u64 = 30;
    let height = 44 + pairs.len() as u64 * row_h + 12;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"640\" height=\"{height}\" role=\"img\" aria-label=\"{label}\">",
        label = esc(title)
    );
    out.push_str(&format!(
        "<text x=\"12\" y=\"24\" font-size=\"14\" font-family=\"sans-serif\">{}</text>",
        esc(title)
    ));
    for (index, (label, value)) in pairs.iter().enumerate() {
        let y = 44 + index as u64 * row_h;
        let width = if max == 0 {
            0
        } else {
            value.saturating_mul(340).saturating_div(max)
        };
        out.push_str(&format!(
            "<text x=\"12\" y=\"{ty}\" font-size=\"12\" font-family=\"monospace\">{lab}</text><rect x=\"250\" y=\"{y}\" width=\"{width}\" height=\"18\" fill=\"#2563eb\"/><text x=\"{vx}\" y=\"{ty}\" font-size=\"12\" font-family=\"monospace\">{val}{unit}</text>",
            ty = y.saturating_add(14),
            lab = esc(label),
            vx = 250 + width + 8,
            val = value,
            unit = esc(unit),
        ));
    }
    out.push_str("</svg>");
    out
}

/// Self-contained offline HTML dashboard rendered from the same
/// [`AgentReport`] as the table (no divergent logic, no external deps).
pub fn render_html(report: &AgentReport) -> String {
    let day_pairs: Vec<(String, u64)> = report
        .iterations_by_day
        .iter()
        .map(|(day, count)| (day.clone(), *count))
        .collect();
    let rate_pairs: Vec<(String, u64)> = report
        .groups
        .iter()
        .map(|row| (format!("{}/{}", row.agent_id, row.session_id), row.fix_rate_pct))
        .collect();
    let shape_pairs: Vec<(String, u64)> = report
        .top_failure_shapes
        .iter()
        .take(8)
        .map(|(shape, count)| (shape.clone(), *count))
        .collect();

    let mut rows_html = String::new();
    for row in &report.groups {
        rows_html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}%</td></tr>",
            esc(&row.agent_id),
            esc(&row.session_id),
            row.iterations,
            row.traces_fetched,
            row.diagnoses_read,
            row.patches_applied,
            row.validations,
            row.fixes,
            row.fix_rate_pct,
        ));
    }
    if rows_html.is_empty() {
        rows_html.push_str("<tr><td colspan=\"9\">no agent loop events</td></tr>");
    }

    format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>ODD agent loop dashboard</title><style>body{{font-family:sans-serif;margin:24px;color:#111}}table{{border-collapse:collapse;margin-top:12px}}td,th{{border:1px solid #ccc;padding:4px 8px;font-size:13px}}section{{margin:24px 0}}h1{{font-size:20px}}h2{{font-size:16px}}</style></head><body><h1>ODD agent loop dashboard</h1><p>{groups} agent cohorts, {iters} iterations, {fixes} fixes</p><section><h2>iterations over time</h2>{days}</section><section><h2>fix rate per agent (%)</h2>{rates}</section><section><h2>top failure shapes diagnosed</h2>{shapes}</section><section><h2>cohorts</h2><table><tr><th>agent_id</th><th>session_id</th><th>iterations</th><th>traces_fetched</th><th>diagnoses_read</th><th>patches_applied</th><th>validations</th><th>fixes</th><th>fix_rate_pct</th></tr>{rows}</table></section></body></html>",
        groups = report.groups_total,
        iters = report.iterations_total,
        fixes = report.fixes_total,
        days = svg_bars("iterations over time", &day_pairs, ""),
        rates = svg_bars("fix rate per agent (%)", &rate_pairs, "%"),
        shapes = svg_bars("top failure shapes diagnosed", &shape_pairs, ""),
        rows = rows_html,
    )
}

fn resolve_root(dir: &Option<PathBuf>) -> PathBuf {
    dir.clone().unwrap_or_else(default_telemetry_root)
}

/// Dispatch `trace agents`: human table by default, `--json` for machines,
/// `--html OUT` additionally writes the offline dashboard file.
pub fn run_agents(
    dir: &Option<PathBuf>,
    json: bool,
    html: &Option<PathBuf>,
) -> Result<Outcome, CliError> {
    let report = compute_agent_report(&load_telemetry_dir(&resolve_root(dir)).records);
    if json {
        println!("{}", report_json(&report));
    } else {
        print!("{}", render_table(&report));
    }
    if let Some(out) = html {
        std::fs::write(out, render_html(&report)).map_err(|source| CliError::Io {
            path: out.display().to_string(),
            source,
        })?;
        println!("agents dashboard -> {}", out.display());
    }
    Ok(Outcome::Same)
}
