//! Background job execution with cancellation and progress reporting.
//!
//! Provides [`JobRunner`] and [`JobHandle`] for running comparison, sync,
//! and merge operations off the main thread without blocking.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::{
    Error, FolderComparison, FolderOptions, Result, SyncOptions, SyncPlan, SyncReport,
    TreeMergePlan, TreeMergeReport, apply_sync, apply_tree_merge, compare_folders_cancellable,
    plan_sync, plan_tree_merge,
};

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
}

/// A handle to a background job executing on a dedicated thread.
pub struct JobHandle<T> {
    cancel_flag: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<T>>>,
    latest_progress: Arc<Mutex<Option<JobProgress>>>,
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

    /// Reads the latest reported progress, if any.
    pub fn progress(&self) -> Option<JobProgress> {
        match self.latest_progress.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => None,
        }
    }

    /// Waits for the job to complete and returns its result.
    ///
    /// # Errors
    ///
    /// Returns the underlying operation error, or an [`Error::Io`] if the
    /// worker thread panicked or could not be joined.
    pub fn join(mut self) -> Result<T> {
        let Some(thread) = self.thread.take() else {
            return Err(Error::Cancelled);
        };
        match thread.join() {
            Ok(res) => res,
            Err(_) => Err(Error::Io {
                path: "background-job".to_string(),
                source: std::io::Error::other("worker thread panicked"),
            }),
        }
    }
}

/// A runner that spawns background tasks with cancellation and progress reporting.
#[derive(Debug, Default, Clone)]
pub struct JobRunner;

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
        let cancel_for_thread = Arc::clone(&cancel_flag);
        let latest_progress = Arc::new(Mutex::new(None));
        let progress_state = Arc::clone(&latest_progress);

        let progress_fn: Arc<dyn Fn(JobProgress) + Send + Sync> =
            Arc::new(move |p: JobProgress| {
                if let Ok(mut lock) = progress_state.lock() {
                    *lock = Some(p);
                }
            });

        let thread_builder = thread::Builder::new().name(name.to_string());
        let thread_handle = thread_builder.spawn(move || work(cancel_for_thread, progress_fn));

        let thread = thread_handle.ok();

        JobHandle {
            cancel_flag,
            thread,
            latest_progress,
        }
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

    /// Spawns a background sync planning job.
    pub fn spawn_sync_plan(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        opts: SyncOptions,
    ) -> JobHandle<SyncPlan> {
        let left = left.as_ref().to_path_buf();
        let right = right.as_ref().to_path_buf();
        Self::spawn(move |_cancel, progress| {
            progress(JobProgress::new(0, None, "Planning sync..."));
            let plan = plan_sync(&left, &right, &opts)?;
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
        Self::spawn(move |_cancel, progress| {
            let total = plan.steps.len();
            progress(JobProgress::new(0, Some(total), "Applying sync..."));
            let report = apply_sync(&left, &right, &plan)?;
            progress(JobProgress::new(
                report.copied + report.deleted,
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
        Self::spawn(move |_cancel, progress| {
            progress(JobProgress::new(0, None, "Planning folder merge..."));
            let plan = plan_tree_merge(&base, &left, &right)?;
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
        Self::spawn(move |_cancel, progress| {
            let total = plan.steps.len();
            progress(JobProgress::new(0, Some(total), "Applying folder merge..."));
            let report = apply_tree_merge(&left, &right, &plan, &output)?;
            progress(JobProgress::new(
                report.written + report.dirs_created,
                Some(total),
                "Folder merge apply complete",
            ));
            Ok(report)
        })
    }
}
