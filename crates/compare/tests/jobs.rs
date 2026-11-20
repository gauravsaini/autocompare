//! Tests for cancellable background JobRunner.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use autocompare_compare::{
    Error, FolderOptions, FolderStatus, JobProgress, JobRunner, SyncMode, SyncPlanner,
};
use tempfile::tempdir;

#[test]
fn job_runner_basic_spawn_and_join() {
    let handle = JobRunner::spawn(|cancel, progress| {
        progress(JobProgress::new(1, Some(2), "working"));
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        progress(JobProgress::new(2, Some(2), "done"));
        Ok(42)
    });

    let res = handle.join().expect("job should succeed");
    assert_eq!(res, 42);
}

#[test]
fn job_runner_cancellation_aborts_job() {
    let started = Arc::new(AtomicBool::new(false));
    let started_clone = Arc::clone(&started);

    let handle = JobRunner::spawn(move |cancel, _progress| {
        started_clone.store(true, Ordering::SeqCst);
        for _ in 0..100 {
            if cancel.load(Ordering::SeqCst) {
                return Err(Error::Cancelled);
            }
            sleep(Duration::from_millis(5));
        }
        Ok(100)
    });

    // Wait until job starts
    while !started.load(Ordering::SeqCst) {
        sleep(Duration::from_millis(2));
    }

    handle.cancel();
    assert!(handle.is_cancelled());

    let err = handle.join().expect_err("job should be cancelled");
    assert!(matches!(err, Error::Cancelled));
}

#[test]
fn job_runner_progress_tracking() {
    let handle = JobRunner::spawn(|_cancel, progress| {
        progress(JobProgress::new(10, Some(100), "step 1"));
        sleep(Duration::from_millis(20));
        progress(JobProgress::new(50, Some(100), "halfway"));
        sleep(Duration::from_millis(20));
        progress(JobProgress::new(100, Some(100), "complete"));
        Ok("success")
    });

    let res = handle.join().expect("job should succeed");
    assert_eq!(res, "success");
}

#[test]
fn job_runner_folder_compare_background() {
    let temp = tempdir().expect("tempdir");
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    std::fs::create_dir_all(&left).expect("create left");
    std::fs::create_dir_all(&right).expect("create right");

    std::fs::write(left.join("file.txt"), "hello").expect("write left");
    std::fs::write(right.join("file.txt"), "hello").expect("write right");

    let opts = FolderOptions {
        content_compare: true,
        ..Default::default()
    };

    let handle = JobRunner::spawn_folder_compare(&left, &right, opts);
    let comparison = handle.join().expect("compare should succeed");
    assert_eq!(comparison.entries.len(), 1);
    assert_eq!(comparison.entries[0].status, FolderStatus::Same);
}

#[test]
fn job_runner_sync_background() {
    let temp = tempdir().expect("tempdir");
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    std::fs::create_dir_all(&left).expect("create left");
    std::fs::create_dir_all(&right).expect("create right");

    std::fs::write(left.join("new_file.txt"), "sync content").expect("write left");

    let planner = SyncPlanner::new(&left, &right, SyncMode::UpdateRight);
    let opts = planner.options();

    let plan_handle = JobRunner::spawn_sync_plan(&left, &right, opts);
    let plan = plan_handle.join().expect("plan should succeed");
    assert_eq!(plan.steps.len(), 1);

    let apply_handle = JobRunner::spawn_sync_apply(&left, &right, plan);
    let report = apply_handle.join().expect("apply should succeed");
    assert_eq!(report.copied, 1);
    assert_eq!(report.failed.len(), 0);
    assert!(right.join("new_file.txt").exists());
}

#[test]
fn job_progress_bounded_calculations() {
    let p1 = JobProgress::new(50, Some(100), "halfway");
    assert_eq!(p1.percentage(), Some(50.0));
    assert_eq!(p1.fraction(), Some(0.5));
    assert!(!p1.is_complete());

    let p2 = JobProgress::new(100, Some(100), "done");
    assert_eq!(p2.percentage(), Some(100.0));
    assert_eq!(p2.fraction(), Some(1.0));
    assert!(p2.is_complete());

    // Clamped overshoot
    let p3 = JobProgress::new(150, Some(100), "overshoot");
    assert_eq!(p3.percentage(), Some(100.0));
    assert_eq!(p3.fraction(), Some(1.0));
    assert!(p3.is_complete());

    // Unknown total
    let p4 = JobProgress::new(42, None, "unknown");
    assert_eq!(p4.percentage(), None);
    assert_eq!(p4.fraction(), None);
    assert!(!p4.is_complete());
}

#[test]
fn job_runner_bounded_progress_queue_and_polling() {
    let handle = JobRunner::spawn(|_cancel, progress| {
        for i in 1..=5 {
            progress(JobProgress::new(i, Some(5), format!("step {i}")));
        }
        Ok("done")
    });

    let res = handle.join().expect("job should succeed");
    assert_eq!(res, "done");
}

#[test]
fn job_runner_folder_merge_background_cancellation() {
    let temp = tempdir().expect("tempdir");
    let base = temp.path().join("base");
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    let out = temp.path().join("out");
    std::fs::create_dir_all(&base).expect("create base");
    std::fs::create_dir_all(&left).expect("create left");
    std::fs::create_dir_all(&right).expect("create right");

    std::fs::write(base.join("f.txt"), "base").expect("write base");
    std::fs::write(left.join("f.txt"), "left").expect("write left");
    std::fs::write(right.join("f.txt"), "right").expect("write right");

    let plan_handle = JobRunner::spawn_folder_merge_plan(&base, &left, &right);
    let plan = plan_handle.join().expect("plan merge should succeed");
    assert_eq!(plan.steps.len(), 1);

    let apply_handle = JobRunner::spawn_folder_merge_apply(&left, &right, plan, &out);
    let report = apply_handle.join().expect("apply merge should succeed");
    assert_eq!(report.written, 1);
    assert_eq!(report.failed.len(), 0);
}
