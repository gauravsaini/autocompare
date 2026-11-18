//! Plan and report rendering for folder sync.

use autocompare_compare::{SyncAction, SyncPlan, SyncReport};

use crate::term;

/// Prints a plan: one line per step, then skipped entries and a
/// summary.
pub fn print_plan(plan: &SyncPlan, color: bool) {
    if plan.truncated {
        eprintln!(
            "autocompare: note: the walk stopped at the depth limit; deeper entries are not planned"
        );
    }
    if plan.steps.is_empty() && plan.skipped.is_empty() {
        println!("nothing to do: the trees already match the plan");
        return;
    }
    for step in &plan.steps {
        let (glyph, code) = match step.action {
            SyncAction::CopyToRight => ("\u{2192}", term::GREEN),
            SyncAction::CopyToLeft => ("\u{2190}", term::GREEN),
            SyncAction::DeleteRight => ("\u{00d7}", term::RED),
            SyncAction::DeleteLeft => ("\u{00d7}", term::RED),
        };
        let dir_mark = if step.is_dir { "/" } else { "" };
        println!(
            "{} {}{}",
            term::paint(color, code, glyph),
            step.rel_path,
            dir_mark
        );
    }
    for (path, reason) in &plan.skipped {
        println!(
            "{}",
            term::paint(color, term::DIM, &format!("~ {path} ({reason})"))
        );
    }
    println!(
        "{} step(s) planned, {} skipped",
        plan.steps.len(),
        plan.skipped.len()
    );
}

/// Prints the outcome of applying a plan.
pub fn print_report(report: &SyncReport, color: bool) {
    println!(
        "{}",
        term::paint(color, term::GREEN, &format!("copied: {}", report.copied))
    );
    println!(
        "{}",
        term::paint(color, term::RED, &format!("deleted: {}", report.deleted))
    );
    for (path, reason) in &report.failed {
        println!(
            "{}",
            term::paint(color, term::RED, &format!("failed: {path} ({reason})"))
        );
    }
}
