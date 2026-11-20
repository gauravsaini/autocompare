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

use crate::folder_merge::{apply_tree_merge_cancellable, plan_tree_merge_cancellable};
use crate::sync::{apply_sync_cancellable, plan_sync_with_compare_cancellable};
use crate::{
    Error, FolderComparison, FolderOptions, Result, SyncOptions, SyncPlan, SyncReport,
    TreeMergePlan, TreeMergeReport, compare_folders_cancellable,
};

pub use crate::folder::{FolderStatusCounts, FolderTreeNode};

/// Maximum number of recent progress updates retained in the bounded queue.
const MAX_PROGRESS_HISTORY: usize = 64;

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

    /// Polls the next progress update from the bounded history queue.
    pub fn poll_progress(&self) -> Option<JobProgress> {
        match self.progress_history.lock() {
            Ok(mut guard) => guard.pop_front(),
            Err(_) => None,
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
        let progress_history = Arc::new(Mutex::new(VecDeque::new()));
        let history_state = Arc::clone(&progress_history);

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

        let thread_builder = thread::Builder::new().name(name.to_string());
        let thread_handle = thread_builder.spawn(move || work(cancel_for_thread, progress_fn));

        let thread = thread_handle.ok();

        JobHandle {
            cancel_flag,
            thread,
            latest_progress,
            progress_history,
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
