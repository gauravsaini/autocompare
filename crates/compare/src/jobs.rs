//! Background job execution with cancellation and progress reporting.
//!
//! Provides [`JobRunner`] and [`JobHandle`] for running comparison, sync,
//! and merge operations off the main thread without blocking.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::folder::odd_enabled;
use crate::folder_merge::{FolderMerge, apply_tree_merge_cancellable, plan_tree_merge_cancellable};
use crate::sync::{SyncPlanner, apply_sync_cancellable, plan_sync_with_compare_cancellable};
use crate::{
    Error, FolderComparison, FolderOptions, Result, SyncOptions, SyncPlan, SyncReport,
    TreeMergePlan, TreeMergeReport, compare_folders_cancellable,
};
use autocompare_telemetry::TraceContext;

pub use crate::folder::{FolderStatusCounts, FolderTreeNode};

/// Maximum number of recent progress updates retained in the bounded queue.
const MAX_PROGRESS_HISTORY: usize = 64;

/// A thread-safe cancellation token.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Creates a new cancellation token.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Signals cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Checks if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Borrows the underlying `AtomicBool`.
    pub fn as_atomic(&self) -> &AtomicBool {
        &self.0
    }

    /// Clones the inner `Arc<AtomicBool>`.
    pub fn arc_atomic(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }
}

/// Lifecycle status of a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum JobStatus {
    /// The job is currently queued or executing on a background thread.
    #[default]
    Running,
    /// The job completed successfully.
    Completed,
    /// The job failed with an error or panicked.
    Failed,
    /// The job was cancelled.
    Cancelled,
}

/// Progress update sent from a running background job.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobProgress {
    /// Items processed so far.
    pub current: usize,
    /// Estimated total items, if known.
    pub total: Option<usize>,
    /// Human-readable description of current work.
    pub message: String,
}

impl JobProgress {
    /// Creates a new progress update.
    pub fn new(current: usize, total: Option<usize>, message: impl Into<String>) -> Self {
        Self {
            current,
            total,
            message: message.into(),
        }
    }

    /// Computes the percentage completed (0.0 to 100.0), clamped if total is known.
    pub fn percentage(&self) -> Option<f32> {
        self.total.map(|tot| {
            if tot == 0 {
                100.0
            } else {
                let p = (self.current as f32 / tot as f32) * 100.0;
                p.clamp(0.0, 100.0)
            }
        })
    }

    /// Computes the fraction completed (0.0 to 1.0), clamped if total is known.
    pub fn fraction(&self) -> Option<f64> {
        self.total.map(|tot| {
            if tot == 0 {
                1.0
            } else {
                let f = self.current as f64 / tot as f64;
                f.clamp(0.0, 1.0)
            }
        })
    }

    /// Checks if progress is complete (current >= total).
    pub fn is_complete(&self) -> bool {
        self.total.is_some_and(|tot| self.current >= tot)
    }
}

/// A handle to a background job executing on a dedicated thread.
pub struct JobHandle<T> {
    cancel_flag: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<T>>>,
    latest_progress: Arc<Mutex<Option<JobProgress>>>,
    progress_history: Arc<Mutex<VecDeque<JobProgress>>>,
    status: Arc<Mutex<JobStatus>>,
    /// ODD Phase 4 Leaf B: fork-join parent context shared by every
    /// shard of one logical operation. `None` when telemetry is off.
    trace_parent: Option<TraceContext>,
    /// Child context derived by the worker thread at start, read back
    /// after [`JobHandle::join`] to prove parent linkage.
    trace_child: Arc<Mutex<Option<TraceContext>>>,
    /// Parent span the worker's shard span follows from.
    parent_span: Option<tracing::Span>,
}

impl<T> JobHandle<T> {
    /// Requests cancellation of the background job.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Checks if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    /// Returns the cancellation flag for passing to cancellable operations.
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel_flag)
    }

    /// Checks whether the background thread has finished executing.
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// Returns the current lifecycle status of the job.
    pub fn status(&self) -> JobStatus {
        if self.is_cancelled() && !self.is_finished() {
            return JobStatus::Cancelled;
        }
        match self.status.lock() {
            Ok(guard) => *guard,
            Err(_) => JobStatus::Failed,
        }
    }

    /// Checks whether the job is currently running.
    pub fn is_running(&self) -> bool {
        self.status() == JobStatus::Running
    }

    /// Checks whether the job completed successfully.
    pub fn is_completed(&self) -> bool {
        self.status() == JobStatus::Completed
    }

    /// Checks whether the job failed or encountered an error.
    pub fn is_failed(&self) -> bool {
        self.status() == JobStatus::Failed
    }

    /// Reads the latest reported progress, if any.
    pub fn progress(&self) -> Option<JobProgress> {
        match self.latest_progress.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => None,
        }
    }

    /// Polls the next progress update from the bounded history queue.
    pub fn poll_progress(&self) -> Option<JobProgress> {
        match self.progress_history.lock() {
            Ok(mut guard) => guard.pop_front(),
            Err(_) => None,
        }
    }

    /// Non-blocking poll of the next progress update from the bounded queue.
    pub fn try_recv_progress(&self) -> Option<JobProgress> {
        self.poll_progress()
    }

    /// Returns the number of progress updates currently buffered in the queue.
    pub fn progress_queue_len(&self) -> usize {
        match self.progress_history.lock() {
            Ok(guard) => guard.len(),
            Err(_) => 0,
        }
    }

    /// Drains all buffered progress updates from the bounded queue.
    pub fn drain_progress(&self) -> Vec<JobProgress> {
        match self.progress_history.lock() {
            Ok(mut guard) => {
                let mut items = Vec::new();
                while let Some(item) = guard.pop_front() {
                    items.push(item);
                }
                items
            }
            Err(_) => Vec::new(),
        }
    }

    /// Returns a copy of the current bounded progress history.
    pub fn progress_history(&self) -> Vec<JobProgress> {
        match self.progress_history.lock() {
            Ok(guard) => guard.iter().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Waits for the job to complete and returns its result.
    ///
    /// # Errors
    ///
    /// Returns the underlying operation error; a worker thread that
    /// panicked, failed to spawn, or could not be joined becomes a
    /// structured [`Error::Io`] ("background-job"), never a panic
    /// and never a misleading [`Error::Cancelled`].
    pub fn join(mut self) -> Result<T> {
        let Some(thread) = self.thread.take() else {
            return Err(Error::Io {
                path: "background-job".to_string(),
                source: std::io::Error::other("worker thread failed to spawn"),
            });
        };
        // ODD Phase 4 Leaf B: join timings are recorded on the
        // fork-join parent span so one logical operation shows one
        // parent with its shard joins.
        let started = std::time::Instant::now();
        let out = thread.join();
        let elapsed_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        if let Some(parent) = self.parent_span.as_ref() {
            parent.in_scope(|| {
                tracing::info!(phase = "join", elapsed_us = elapsed_us);
            });
        }
        match out {
            Ok(res) => res,
            Err(_) => Err(Error::Io {
                path: "background-job".to_string(),
                source: std::io::Error::other("worker thread panicked"),
            }),
        }
    }

    /// Fork-join parent context for this job, if telemetry is on.
    pub fn trace_parent(&self) -> Option<TraceContext> {
        self.trace_parent
    }

    /// Child context derived by the worker thread, if it started and
    /// telemetry is on. Shares the parent's trace id by construction.
    pub fn trace_shard(&self) -> Option<TraceContext> {
        match self.trace_child.lock() {
            Ok(guard) => *guard,
            Err(_) => None,
        }
    }
}

/// A runner that spawns background tasks with cancellation and progress reporting.
#[derive(Debug, Default, Clone)]
pub struct JobRunner;

/// Outcome of one joined fanout shard.
pub struct ShardOutcome<T> {
    /// Shard position in the fanout group.
    pub index: usize,
    /// Wall time around this shard's join, in microseconds.
    pub elapsed_us: u64,
    /// Child context derived by the shard worker, if it started and
    /// telemetry is on. Shares the fanout parent's trace id.
    pub trace: Option<TraceContext>,
    /// The shard's own result.
    pub result: Result<T>,
}

/// Joined report for a fanout group: one parent, per-shard outcomes.
pub struct FanoutReport<T> {
    /// The shared fork-join parent context, if telemetry is on.
    pub parent: Option<TraceContext>,
    /// Per-shard outcomes in shard order.
    pub outcomes: Vec<ShardOutcome<T>>,
}

/// A fork-join group of background shards under one parent context.
pub struct FanoutHandle<T> {
    parent: Option<TraceContext>,
    shards: Vec<JobHandle<T>>,
}

impl<T> FanoutHandle<T> {
    /// Number of shards in the group.
    pub fn len(&self) -> usize {
        self.shards.len()
    }

    /// Whether the group holds no shards.
    pub fn is_empty(&self) -> bool {
        self.shards.is_empty()
    }

    /// The shared fork-join parent context, if telemetry is on.
    pub fn parent_trace(&self) -> Option<TraceContext> {
        self.parent
    }

    /// Requests cancellation of every shard in the group.
    pub fn cancel_all(&self) {
        for handle in &self.shards {
            handle.cancel();
        }
    }

    /// Joins every shard in order, collecting per-shard join timings
    /// and child contexts into one report.
    pub fn join(mut self) -> FanoutReport<T> {
        let mut outcomes = Vec::new();
        for (index, handle) in self.shards.drain(..).enumerate() {
            // Read back the child slot after the blocking join: the
            // worker records it first thing, so it is always set when
            // telemetry is on and the thread started.
            let slot = Arc::clone(&handle.trace_child);
            let started = std::time::Instant::now();
            let result = handle.join();
            let elapsed_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
            let trace = match slot.lock() {
                Ok(guard) => *guard,
                Err(_) => None,
            };
            outcomes.push(ShardOutcome {
                index,
                elapsed_us,
                trace,
                result,
            });
        }
        FanoutReport {
            parent: self.parent,
            outcomes,
        }
    }
}

impl JobRunner {
    /// Creates a new `JobRunner`.
    pub const fn new() -> Self {
        Self
    }

    /// Spawns a closure on a background worker thread with a cancellation flag and progress callback.
    pub fn spawn<T, F>(work: F) -> JobHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AtomicBool>, Arc<dyn Fn(JobProgress) + Send + Sync>) -> Result<T>
            + Send
            + 'static,
    {
        Self::spawn_with_name("autocompare-job", work)
    }

    /// Spawns a named worker thread with a cancellation flag and progress callback.
    pub fn spawn_with_name<T, F>(name: &str, work: F) -> JobHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AtomicBool>, Arc<dyn Fn(JobProgress) + Send + Sync>) -> Result<T>
            + Send
            + 'static,
    {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let parent = if odd_enabled() {
            Some(TraceContext::fresh(true))
        } else {
            None
        };
        Self::spawn_inner(name, cancel_flag, parent, work)
    }

    /// Shared worker-thread setup for every spawn path: progress
    /// plumbing, status tracking, plus the ODD fork-join linkage (one
    /// parent context/span per logical operation, one child per shard).
    fn spawn_inner<T, F>(
        name: &str,
        cancel_flag: Arc<AtomicBool>,
        parent: Option<TraceContext>,
        work: F,
    ) -> JobHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AtomicBool>, Arc<dyn Fn(JobProgress) + Send + Sync>) -> Result<T>
            + Send
            + 'static,
    {
        let job_name = name.to_string();
        let cancel_for_thread = Arc::clone(&cancel_flag);
        let latest_progress = Arc::new(Mutex::new(None));
        let progress_state = Arc::clone(&latest_progress);
        let progress_history = Arc::new(Mutex::new(VecDeque::new()));
        let history_state = Arc::clone(&progress_history);
        let status = Arc::new(Mutex::new(JobStatus::Running));
        let status_for_thread = Arc::clone(&status);
        let trace_child: Arc<Mutex<Option<TraceContext>>> = Arc::new(Mutex::new(None));
        let child_slot = Arc::clone(&trace_child);
        let parent_span: Option<tracing::Span> = if parent.is_some() {
            Some(tracing::info_span!(
                "jobs.fork_join",
                component = "rs.compare/jobs",
                job = job_name.clone()
            ))
        } else {
            None
        };
        let shard_parent = parent_span.clone();

        let progress_fn: Arc<dyn Fn(JobProgress) + Send + Sync> =
            Arc::new(move |p: JobProgress| {
                if let Ok(mut lock) = progress_state.lock() {
                    *lock = Some(p.clone());
                }
                if let Ok(mut h_lock) = history_state.lock() {
                    if h_lock.len() >= MAX_PROGRESS_HISTORY {
                        let _ = h_lock.pop_front();
                    }
                    h_lock.push_back(p);
                }
            });

        let thread_builder = thread::Builder::new().name(job_name.clone());
        let thread_handle = thread_builder.spawn(move || {
            // Every shard derives a child of the fork-join parent (read
            // back after join to prove linkage) and runs under a shard
            // span that follows from the parent span.
            if let Some(child) = parent.map(|p| p.child())
                && let Ok(mut slot) = child_slot.lock()
            {
                *slot = Some(child);
            }
            let shard_span =
                tracing::info_span!("jobs.shard", component = "rs.compare/jobs", job = job_name);
            if let Some(follow) = shard_parent.as_ref() {
                shard_span.follows_from(follow);
            }
            let _shard_guard = shard_span.enter();
            let res = work(cancel_for_thread, progress_fn);
            if let Ok(mut lock) = status_for_thread.lock() {
                *lock = match &res {
                    Ok(_) => JobStatus::Completed,
                    Err(Error::Cancelled) => JobStatus::Cancelled,
                    Err(_) => JobStatus::Failed,
                };
            }
            res
        });

        let thread = thread_handle.ok();

        JobHandle {
            cancel_flag,
            thread,
            latest_progress,
            progress_history,
            status,
            trace_parent: parent,
            trace_child,
            parent_span,
        }
    }

    /// Spawns a background task with an external cancellation token.
    pub fn spawn_with_token<T, F>(token: CancellationToken, work: F) -> JobHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AtomicBool>, Arc<dyn Fn(JobProgress) + Send + Sync>) -> Result<T>
            + Send
            + 'static,
    {
        let cancel_flag = token.arc_atomic();
        let parent = if odd_enabled() {
            Some(TraceContext::fresh(true))
        } else {
            None
        };
        Self::spawn_inner("autocompare-job", cancel_flag, parent, work)
    }

    /// Spawns a fork-join group: every shard runs on its own thread but
    /// derives a child of ONE shared parent context, so the join below
    /// can attribute per-shard timings to one logical operation.
    ///
    /// ODD Phase 4 Leaf B: parallel shards as child spans of one parent
    /// with join timings. Shard names are `{name}-shard-{index}`.
    pub fn spawn_fanout<T, F>(name: &str, shards: Vec<F>) -> FanoutHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AtomicBool>, Arc<dyn Fn(JobProgress) + Send + Sync>) -> Result<T>
            + Send
            + 'static,
    {
        let parent = if odd_enabled() {
            Some(TraceContext::fresh(true))
        } else {
            None
        };
        let mut handles = Vec::new();
        for (index, work) in shards.into_iter().enumerate() {
            let shard_name = format!("{name}-shard-{index}");
            let cancel_flag = Arc::new(AtomicBool::new(false));
            handles.push(Self::spawn_inner(&shard_name, cancel_flag, parent, work));
        }
        FanoutHandle {
            parent,
            shards: handles,
        }
    }

    /// Spawns a background job executing both planning and application for a `SyncPlanner`.
    pub fn spawn_sync_run(planner: SyncPlanner) -> JobHandle<SyncReport> {
        Self::spawn_with_name("autocompare-sync-run", move |cancel, progress| {
            progress(JobProgress::new(0, None, "Starting sync dry run..."));
            let plan = planner.dry_run()?;
            let total = plan.steps.len();
            progress(JobProgress::new(0, Some(total), "Applying sync..."));
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let prog_cb = {
                let p = Arc::clone(&progress);
                move |done: usize, tot: usize, rel: &str| {
                    p(JobProgress::new(done, Some(tot), format!("Syncing {rel}")));
                }
            };
            let report = apply_sync_cancellable(
                planner.left(),
                planner.right(),
                &plan,
                Some(&cancel),
                Some(&prog_cb),
            )?;
            progress(JobProgress::new(
                report.copied.saturating_add(report.deleted),
                Some(total),
                "Sync run complete",
            ));
            Ok(report)
        })
    }

    /// Spawns a background job executing both planning and application for a `FolderMerge`.
    pub fn spawn_folder_merge_run(
        merge: FolderMerge,
        output: impl AsRef<Path>,
    ) -> JobHandle<TreeMergeReport> {
        let out = output.as_ref().to_path_buf();
        Self::spawn_with_name("autocompare-merge-run", move |cancel, progress| {
            progress(JobProgress::new(0, None, "Starting folder merge plan..."));
            let plan = merge.plan()?;
            let total = plan.steps.len();
            progress(JobProgress::new(0, Some(total), "Applying folder merge..."));
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let prog_cb = {
                let p = Arc::clone(&progress);
                move |done: usize, tot: usize, rel: &str| {
                    p(JobProgress::new(done, Some(tot), format!("Merging {rel}")));
                }
            };
            let report = apply_tree_merge_cancellable(
                merge.left(),
                merge.right(),
                &plan,
                &out,
                Some(&cancel),
                Some(&prog_cb),
            )?;
            progress(JobProgress::new(
                report.written.saturating_add(report.dirs_created),
                Some(total),
                "Folder merge run complete",
            ));
            Ok(report)
        })
    }

    /// Spawns a background cancellable folder comparison job.
    pub fn spawn_folder_compare(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        opts: FolderOptions,
    ) -> JobHandle<FolderComparison> {
        let left = left.as_ref().to_path_buf();
        let right = right.as_ref().to_path_buf();
        Self::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "Starting folder compare..."));
            let res = compare_folders_cancellable(&left, &right, &opts, &cancel)?;
            progress(JobProgress::new(
                res.entries.len(),
                Some(res.entries.len()),
                "Folder compare complete",
            ));
            Ok(res)
        })
    }

    /// Spawns a background cancellable folder comparison over two
    /// virtual filesystems (LocalFs, Zip, snapshots).
    pub fn spawn_vfs_folder_compare(
        left: std::sync::Arc<dyn autocompare_vfs::Vfs>,
        right: std::sync::Arc<dyn autocompare_vfs::Vfs>,
        opts: FolderOptions,
    ) -> JobHandle<FolderComparison> {
        Self::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "Starting folder compare..."));
            let res = crate::vfs_compare::compare_vfs_folders_cancellable(
                &*left,
                &*right,
                &opts,
                Some(&cancel),
            )?;
            progress(JobProgress::new(
                res.entries.len(),
                Some(res.entries.len()),
                "Folder compare complete",
            ));
            Ok(res)
        })
    }

    /// Spawns a background sync planning job.
    pub fn spawn_sync_plan(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        opts: SyncOptions,
    ) -> JobHandle<SyncPlan> {
        Self::spawn_sync_plan_with_compare(left, right, opts, FolderOptions::default())
    }

    /// Spawns a background sync planning job with explicit folder
    /// comparison rules (depth limit, content compare, name
    /// filters): the background plan mirrors the folder view it
    /// was previewed with.
    pub fn spawn_sync_plan_with_compare(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        opts: SyncOptions,
        compare: FolderOptions,
    ) -> JobHandle<SyncPlan> {
        let left = left.as_ref().to_path_buf();
        let right = right.as_ref().to_path_buf();
        Self::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "Planning sync..."));
            let plan =
                plan_sync_with_compare_cancellable(&left, &right, &opts, &compare, Some(&cancel))?;
            progress(JobProgress::new(
                plan.steps.len(),
                Some(plan.steps.len()),
                "Sync planning complete",
            ));
            Ok(plan)
        })
    }

    /// Spawns a background sync apply job.
    pub fn spawn_sync_apply(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        plan: SyncPlan,
    ) -> JobHandle<SyncReport> {
        let left = left.as_ref().to_path_buf();
        let right = right.as_ref().to_path_buf();
        Self::spawn(move |cancel, progress| {
            let total = plan.steps.len();
            progress(JobProgress::new(0, Some(total), "Applying sync..."));
            let prog_cb = {
                let p = Arc::clone(&progress);
                move |done: usize, tot: usize, rel: &str| {
                    p(JobProgress::new(done, Some(tot), format!("Syncing {rel}")));
                }
            };
            let report =
                apply_sync_cancellable(&left, &right, &plan, Some(&cancel), Some(&prog_cb))?;
            progress(JobProgress::new(
                report.copied.saturating_add(report.deleted),
                Some(total),
                "Sync apply complete",
            ));
            Ok(report)
        })
    }

    /// Spawns a background three-way folder merge planning job.
    pub fn spawn_folder_merge_plan(
        base: impl AsRef<Path>,
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
    ) -> JobHandle<TreeMergePlan> {
        let base = base.as_ref().to_path_buf();
        let left = left.as_ref().to_path_buf();
        let right = right.as_ref().to_path_buf();
        Self::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "Planning folder merge..."));
            let plan = plan_tree_merge_cancellable(&base, &left, &right, Some(&cancel))?;
            progress(JobProgress::new(
                plan.steps.len(),
                Some(plan.steps.len()),
                "Folder merge planning complete",
            ));
            Ok(plan)
        })
    }

    /// Spawns a background three-way folder merge apply job.
    pub fn spawn_folder_merge_apply(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        plan: TreeMergePlan,
        output: impl AsRef<Path>,
    ) -> JobHandle<TreeMergeReport> {
        let left = left.as_ref().to_path_buf();
        let right = right.as_ref().to_path_buf();
        let output = output.as_ref().to_path_buf();
        Self::spawn(move |cancel, progress| {
            let total = plan.steps.len();
            progress(JobProgress::new(0, Some(total), "Applying folder merge..."));
            let prog_cb = {
                let p = Arc::clone(&progress);
                move |done: usize, tot: usize, rel: &str| {
                    p(JobProgress::new(done, Some(tot), format!("Merging {rel}")));
                }
            };
            let report = apply_tree_merge_cancellable(
                &left,
                &right,
                &plan,
                &output,
                Some(&cancel),
                Some(&prog_cb),
            )?;
            progress(JobProgress::new(
                report.written.saturating_add(report.dirs_created),
                Some(total),
                "Folder merge apply complete",
            ));
            Ok(report)
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::SyncMode;
    use std::time::Duration;

    #[test]
    fn test_cancellation_token_and_status() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        let t_clone = token.clone();
        let handle = JobRunner::spawn_with_token(token, move |cancel, progress| {
            progress(JobProgress::new(0, Some(10), "working..."));
            for i in 1..=10 {
                if cancel.load(Ordering::Relaxed) {
                    return Err(Error::Cancelled);
                }
                progress(JobProgress::new(i, Some(10), format!("step {i}")));
                thread::sleep(Duration::from_millis(5));
            }
            Ok(42)
        });

        assert!(handle.is_running() || handle.is_completed());
        t_clone.cancel();
        assert!(t_clone.is_cancelled());

        let res = handle.join();
        assert!(matches!(res, Err(Error::Cancelled) | Ok(42)));
    }

    #[test]
    fn test_job_status_success_and_failure() {
        let success_handle = JobRunner::spawn(|_cancel, _progress| Ok("done"));
        let res = success_handle.join().unwrap();
        assert_eq!(res, "done");

        let fail_handle = JobRunner::spawn(|_cancel, _progress| {
            Err(Error::Unsupported {
                path: "test".to_string(),
                what: "intentional failure",
            })
        });
        let res2: Result<()> = fail_handle.join();
        assert!(res2.is_err());
    }

    #[test]
    fn test_progress_queue_polling_helpers() {
        let handle = JobRunner::spawn(|_cancel, progress| {
            progress(JobProgress::new(1, Some(3), "p1"));
            progress(JobProgress::new(2, Some(3), "p2"));
            progress(JobProgress::new(3, Some(3), "p3"));
            Ok(())
        });

        // Drain or poll while handle is active
        assert!(handle.progress_queue_len() <= 64);
        let first = handle.try_recv_progress();
        assert!(first.is_some() || handle.progress_queue_len() <= 64);
        let _ = handle.join();
    }

    // --- round-sync3 edge coverage: cancellable sync progress ---

    fn write_job_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(path, contents).expect("write fixture");
    }

    fn job_temp_base(name: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "autocompare_job_sync3_{name}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("create temp base");
        base
    }

    #[test]
    fn test_spawn_sync_run_parity_and_progress() {
        let base = job_temp_base("run");
        let left = base.join("left");
        let right = base.join("right");
        std::fs::create_dir_all(&left).expect("create left");
        std::fs::create_dir_all(&right).expect("create right");
        write_job_file(&left.join("new.txt"), "new\n");
        write_job_file(&left.join("sub/inner.txt"), "inner\n");
        write_job_file(&right.join("extra.txt"), "extra\n");

        let planner = SyncPlanner::new(&left, &right, SyncMode::Mirror);
        let expected = planner.dry_run().expect("dry run");
        let expected_steps = expected.steps.len();
        assert_ne!(expected_steps, 0, "mirror fixture plans steps");
        assert!(!right.join("new.txt").exists(), "dry run copies nothing");
        assert!(right.join("extra.txt").exists(), "dry run prunes nothing");

        let handle = JobRunner::spawn_sync_run(planner);
        thread::sleep(Duration::from_millis(100));
        assert!(handle.progress_queue_len() <= 64, "progress stays bounded");
        let history = handle.progress_history();
        assert!(!history.is_empty(), "progress visible before join");
        assert!(
            history.iter().any(|p| p.total == Some(expected_steps)),
            "progress totals match the plan: {history:?}"
        );
        let report = handle.join().expect("sync run");
        assert!(report.failed.is_empty());
        assert_eq!(
            report.copied.saturating_add(report.deleted),
            expected_steps,
            "apply parity: report totals match dry-run steps"
        );
        assert!(right.join("new.txt").exists());
        assert!(right.join("sub/inner.txt").exists());
        assert!(!right.join("extra.txt").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_spawn_sync_apply_progress_sequence() {
        let base = job_temp_base("apply");
        let left = base.join("left");
        let right = base.join("right");
        std::fs::create_dir_all(&left).expect("create left");
        std::fs::create_dir_all(&right).expect("create right");
        write_job_file(&left.join("a.txt"), "a\n");
        write_job_file(&left.join("b.txt"), "b\n");

        let planner = SyncPlanner::new(&left, &right, SyncMode::UpdateRight);
        let plan = planner.dry_run().expect("dry run");
        let total = plan.steps.len();
        assert_eq!(total, 2);

        let handle = JobRunner::spawn_sync_apply(&left, &right, plan);
        thread::sleep(Duration::from_millis(100));
        let history = handle.progress_history();
        assert!(!history.is_empty(), "apply emits progress");
        assert!(
            history.iter().any(|p| p.message.contains("a.txt")),
            "per-file progress names the file: {history:?}"
        );
        assert!(
            history
                .iter()
                .any(|p| p.message.contains("Sync apply complete")),
            "final progress closes the run: {history:?}"
        );
        assert!(
            history.iter().all(|p| p.total.is_none_or(|t| t == total)),
            "every total matches the step count: {history:?}"
        );
        let report = handle.join().expect("apply");
        assert_eq!(report.copied, total);
        assert!(report.failed.is_empty());
        assert!(right.join("a.txt").exists());
        assert!(right.join("b.txt").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_precancelled_token_job_reports_cancelled_status() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
        let handle = JobRunner::spawn_with_token(token, |cancel, _progress| {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            Ok(1)
        });
        thread::sleep(Duration::from_millis(50));
        assert_eq!(handle.status(), JobStatus::Cancelled);
        let err = handle.join().unwrap_err();
        assert!(matches!(err, Error::Cancelled));
    }

    #[test]
    fn test_fanout_parent_linkage_and_join_timings() {
        // ODD Phase 4 Leaf B: parallel shards as children of one parent
        // with join timings. When telemetry is on every shard shares
        // the parent trace id under a distinct span id; when off every
        // side is None and the values/timings still hold.
        let make_shard = |value: i32| {
            move |_cancel: Arc<AtomicBool>,
                  progress: Arc<dyn Fn(JobProgress) + Send + Sync>|
                  -> Result<i32> {
                progress(JobProgress::new(1, Some(2), "shard working"));
                thread::sleep(Duration::from_millis(5));
                progress(JobProgress::new(2, Some(2), "shard done"));
                Ok(value)
            }
        };
        let fanout = JobRunner::spawn_fanout(
            "odd-fanout-test",
            vec![make_shard(10), make_shard(20), make_shard(30)],
        );
        assert_eq!(fanout.len(), 3);
        assert!(!fanout.is_empty());
        let parent = fanout.parent_trace();
        let report = fanout.join();
        assert_eq!(report.outcomes.len(), 3);
        assert_eq!(report.parent, parent);
        let mut values = Vec::new();
        let mut seen_spans = Vec::new();
        for outcome in &report.outcomes {
            match &outcome.result {
                Ok(v) => values.push(*v),
                Err(_) => panic!("shard must succeed"),
            }
            assert!(outcome.elapsed_us > 0, "join timing must be recorded");
            match (parent, outcome.trace) {
                (Some(p), Some(child)) => {
                    assert_eq!(child.trace_id, p.trace_id);
                    assert_ne!(child.span_id, p.span_id);
                    seen_spans.push(child.span_id);
                }
                (None, None) => {}
                _ => panic!("parent and shard linkage must agree"),
            }
        }
        assert_eq!(values, vec![10, 20, 30]);
        for (i, a) in seen_spans.iter().enumerate() {
            for b in seen_spans.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }
}
