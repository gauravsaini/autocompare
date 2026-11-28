#![forbid(unsafe_code)]

//! Folder sync IPC commands (Wave 1c).
//!
//! Thin Tauri wrappers over the headless engine:
//! [`autocompare_compare::plan_sync_with_compare`] for planning and
//! [`autocompare_compare::apply_sync`] for applying.
//!
//! Registration in `commands/mod.rs` and the Tauri `invoke_handler`
//! table is owned by Wave-1R; this module only defines the handlers.
//!
//! `sync_plan` is read-only. `sync_apply` with `dry_run = true`
//! inspects the plan and reports what *would* happen without touching
//! the filesystem.

use std::path::Path;

use autocompare_compare::{FolderOptions, SyncAction, SyncOptions, SyncPlan, SyncReport};
use autocompare_telemetry::TraceContext;
use serde::{Deserialize, Serialize};

/// Outcome of [`sync_apply`].
///
/// In a dry run nothing is copied or deleted: `copied`/`deleted` stay
/// zero and `planned_copies`/`planned_deletes` describe the plan. In
/// a real run the planned counters are zero and `copied`/`deleted`
/// come from the engine report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplyReport {
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Files successfully copied (real runs only).
    pub copied: usize,
    /// Files or directories successfully deleted (real runs only).
    pub deleted: usize,
    /// Copy steps in the plan (dry runs only).
    pub planned_copies: usize,
    /// Delete steps in the plan (dry runs only).
    pub planned_deletes: usize,
    /// Steps that failed: (relative path, plain-English error).
    pub failed: Vec<(String, String)>,
    /// ODD Phase 4 Leaf B: echoed `__trace` envelope on the wire.
    /// Present only when the caller supplied a valid `__trace`;
    /// omitted otherwise so uninstrumented callers see a
    /// byte-identical shape.
    #[serde(rename = "__trace", default, skip_serializing_if = "Option::is_none")]
    pub trace_echo: Option<serde_json::Value>,
}

fn count_plan(plan: &SyncPlan) -> (usize, usize) {
    let mut copies = 0usize;
    let mut deletes = 0usize;
    for step in &plan.steps {
        match step.action {
            SyncAction::CopyToRight | SyncAction::CopyToLeft => {
                copies = copies.saturating_add(1);
            }
            SyncAction::DeleteRight | SyncAction::DeleteLeft => {
                deletes = deletes.saturating_add(1);
            }
        }
    }
    (copies, deletes)
}

/// Builds a sync plan for `left`/`right` without touching the
/// filesystem. `opts` mirrors the engine [`SyncOptions`] serde names
/// (`direction`: `LeftToRight` | `RightToLeft` | `NewerWins`;
/// `prune`: bool); when omitted the engine default applies
/// (left-to-right, no pruning). Missing roots error as
/// `path not found: ...`.
#[tauri::command]
pub fn sync_plan(
    left: String,
    right: String,
    opts: Option<SyncOptions>,
    // ODD Phase 4 Leaf B: optional `__trace` envelope; span-only here
    // because the engine `SyncPlan` shape cannot carry an echo field.
    __trace: Option<serde_json::Value>,
) -> Result<SyncPlan, String> {
    // ODD Phase 4 Leaf B vertical slice: extract the incoming trace (if
    // any) and derive a child span for this command. Absent, malformed,
    // or telemetry-off input yields no span and changes nothing.
    let trace_child: Option<TraceContext> = if autocompare_telemetry::Sampling::enabled() {
        __trace.as_ref().and_then(|v| {
            TraceContext::from_envelope(&serde_json::json!({ "__trace": v })).map(|ctx| ctx.child())
        })
    } else {
        None
    };
    let _odd_span_guard = trace_child.as_ref().map(|child| {
        let envelope = child.to_envelope();
        let trace_id = envelope
            .get("trace_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let span_id = envelope
            .get("span_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        tracing::info_span!(
            "sync.plan",
            trace_id = ?trace_id,
            span_id = ?span_id,
            component = "rs.cmd/sync"
        )
        .entered()
    });
    let options = opts.unwrap_or_default();
    autocompare_compare::plan_sync_with_compare(
        Path::new(&left),
        Path::new(&right),
        &options,
        &FolderOptions::default(),
    )
    .map_err(|e| e.to_string())
}

/// Applies a sync `plan` between `left` and `right`.
///
/// With `dry_run = true` the plan is only inspected: no copy, delete
/// or directory creation happens. With `dry_run = false` the engine
/// applies each step, recording per-step failures without aborting.
#[tauri::command]
pub fn sync_apply(
    left: String,
    right: String,
    plan: SyncPlan,
    dry_run: bool,
    // ODD Phase 4 Leaf B: optional `__trace` envelope. Tauri fills a
    // missing `Option` param with `None`; unknown/absent stays `None`.
    __trace: Option<serde_json::Value>,
) -> Result<SyncApplyReport, String> {
    // ODD Phase 4 Leaf B: extract the incoming trace (if any), derive a
    // child span for this command, and echo it in the response. Absent,
    // malformed, or telemetry-off input yields no span and no echo
    // field — the response shape is then byte-identical to before.
    let trace_child: Option<TraceContext> = if autocompare_telemetry::Sampling::enabled() {
        __trace.as_ref().and_then(|v| {
            TraceContext::from_envelope(&serde_json::json!({ "__trace": v })).map(|ctx| ctx.child())
        })
    } else {
        None
    };
    let _odd_span_guard = trace_child.as_ref().map(|child| {
        let envelope = child.to_envelope();
        let trace_id = envelope
            .get("trace_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let span_id = envelope
            .get("span_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        tracing::info_span!(
            "sync.apply",
            trace_id = ?trace_id,
            span_id = ?span_id,
            component = "rs.cmd/sync"
        )
        .entered()
    });
    let trace_echo: Option<serde_json::Value> =
        trace_child.as_ref().map(|child| child.to_envelope());
    if dry_run {
        let (planned_copies, planned_deletes) = count_plan(&plan);
        return Ok(SyncApplyReport {
            dry_run: true,
            copied: 0,
            deleted: 0,
            planned_copies,
            planned_deletes,
            failed: Vec::new(),
            trace_echo,
        });
    }
    let report: SyncReport =
        autocompare_compare::apply_sync(Path::new(&left), Path::new(&right), &plan)
            .map_err(|e| e.to_string())?;
    Ok(SyncApplyReport {
        dry_run: false,
        copied: report.copied,
        deleted: report.deleted,
        planned_copies: 0,
        planned_deletes: 0,
        failed: report.failed,
        trace_echo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_dir(tag: &str) -> PathBuf {
        let n = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut dir = std::env::temp_dir();
        dir.push(format!("wave1c-sync-{}-{}-{n}", std::process::id(), tag));
        dir
    }

    fn write_file(path: &Path, content: &str) -> bool {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && std::fs::create_dir_all(parent).is_err()
        {
            return false;
        }
        std::fs::write(path, content).is_ok()
    }

    fn read_file(path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn plan_or_fail(left: &str, right: &str, opts: Option<SyncOptions>) -> SyncPlan {
        match sync_plan(left.to_string(), right.to_string(), opts, None) {
            Ok(plan) => plan,
            Err(e) => {
                assert!(e.is_empty(), "sync_plan failed: {e}");
                SyncPlan::default()
            }
        }
    }

    #[test]
    fn plan_lists_left_only_file_as_copy_to_right() {
        let dir = unique_dir("plan");
        let left = dir.join("left");
        let right = dir.join("right");
        assert!(write_file(&left.join("only.txt"), "hello\n"));
        assert!(write_file(&right.join("marker.txt"), "x\n"));
        let plan = plan_or_fail(
            &left.display().to_string(),
            &right.display().to_string(),
            None,
        );
        let copy = plan.steps.iter().find(|step| step.rel_path == "only.txt");
        match copy {
            Some(step) => assert_eq!(step.action, SyncAction::CopyToRight),
            None => assert!(!plan.steps.is_empty(), "expected a copy step for only.txt"),
        }
        assert!(!plan.truncated);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_defaults_when_opts_omitted() {
        let dir = unique_dir("defaults");
        let left = dir.join("left");
        let right = dir.join("right");
        assert!(write_file(&left.join("a.txt"), "a\n"));
        assert!(write_file(&right.join("a.txt"), "a\n"));
        let plan = plan_or_fail(
            &left.display().to_string(),
            &right.display().to_string(),
            None,
        );
        assert!(plan.steps.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_missing_root_errors() {
        let dir = unique_dir("missing");
        let result = sync_plan(
            dir.join("left").display().to_string(),
            dir.join("right").display().to_string(),
            None,
            None,
        );
        assert!(result.is_err_and(|e| e.contains("path not found")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_touches_nothing() {
        let dir = unique_dir("dryrun");
        let left = dir.join("left");
        let right = dir.join("right");
        assert!(write_file(&left.join("new.txt"), "payload\n"));
        assert!(write_file(&right.join("stale.txt"), "stale\n"));
        let (l, r) = (left.display().to_string(), right.display().to_string());
        let plan = plan_or_fail(&l, &r, None);
        assert!(!plan.steps.is_empty());
        let report = match sync_apply(l.clone(), r.clone(), plan, true, None) {
            Ok(report) => report,
            Err(e) => {
                assert!(e.is_empty(), "dry run failed: {e}");
                return;
            }
        };
        assert!(report.dry_run);
        assert_eq!(report.copied, 0);
        assert_eq!(report.deleted, 0);
        assert!(report.failed.is_empty());
        assert!(report.planned_copies >= 1);
        // Disk is untouched: no copy arrived, no file vanished.
        assert!(!right.join("new.txt").exists());
        assert_eq!(
            read_file(&right.join("stale.txt")),
            Some("stale\n".to_string())
        );
        assert_eq!(
            read_file(&left.join("new.txt")),
            Some("payload\n".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn real_apply_copies_left_only_file() {
        let dir = unique_dir("apply");
        let left = dir.join("left");
        let right = dir.join("right");
        assert!(write_file(&left.join("new.txt"), "payload\n"));
        assert!(write_file(&right.join("marker.txt"), "x\n"));
        let (l, r) = (left.display().to_string(), right.display().to_string());
        let plan = plan_or_fail(&l, &r, None);
        let report = match sync_apply(l, r, plan, false, None) {
            Ok(report) => report,
            Err(e) => {
                assert!(e.is_empty(), "apply failed: {e}");
                return;
            }
        };
        assert!(!report.dry_run);
        assert!(report.copied >= 1);
        assert!(report.failed.is_empty());
        assert_eq!(
            read_file(&right.join("new.txt")),
            Some("payload\n".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn trace_echo_round_trip_on_sync_apply() {
        // ODD Phase 4 Leaf B: a valid `__trace` envelope yields a child
        // echo on the dry-run receipt (same trace, fresh span); absent
        // input leaves the wire shape byte-identical (no `__trace` key).
        let dir = unique_dir("trace");
        let left = dir.join("left");
        let right = dir.join("right");
        assert!(write_file(&left.join("new.txt"), "payload\n"));
        assert!(write_file(&right.join("marker.txt"), "x\n"));
        let (l, r) = (left.display().to_string(), right.display().to_string());
        let plan = plan_or_fail(&l, &r, None);
        assert!(!plan.steps.is_empty());

        let plain = match sync_apply(l.clone(), r.clone(), plan.clone(), true, None) {
            Ok(report) => report,
            Err(e) => {
                assert!(e.is_empty(), "plain apply failed: {e}");
                return;
            }
        };
        assert!(plain.dry_run);
        assert_eq!(plain.trace_echo, None);
        let plain_json = serde_json::to_string(&plain).unwrap_or_default();
        assert!(!plain_json.contains("__trace"));

        let parent = TraceContext::fresh(true);
        let parent_env = parent.to_envelope();
        let traced = match sync_apply(l, r, plan, true, Some(parent_env.clone())) {
            Ok(report) => report,
            Err(e) => {
                assert!(e.is_empty(), "traced apply failed: {e}");
                return;
            }
        };
        assert!(traced.trace_echo.is_some(), "echo must be present");
        let echo = traced.trace_echo.clone().unwrap_or(serde_json::Value::Null);
        assert_eq!(echo.get("trace_id"), parent_env.get("trace_id"));
        assert_ne!(echo.get("span_id"), parent_env.get("span_id"));
        assert_eq!(echo.get("sampled"), parent_env.get("sampled"));
        let traced_json = serde_json::to_string(&traced).unwrap_or_default();
        assert!(traced_json.contains("__trace"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_runs_on_parity_sync_fixtures_when_present() {
        let left = "../../../tools/parity-harness/fixtures/sync/left";
        let right = "../../../tools/parity-harness/fixtures/sync/right";
        if !Path::new(left).exists() || !Path::new(right).exists() {
            return;
        }
        match sync_plan(left.to_string(), right.to_string(), None, None) {
            Ok(plan) => {
                let (copies, deletes) = count_plan(&plan);
                assert_eq!(copies + deletes, plan.steps.len());
            }
            Err(e) => assert!(e.is_empty(), "fixture plan failed: {e}"),
        }
    }
}
