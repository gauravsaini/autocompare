//! Plan and report rendering for three-way folder merges.

use autocompare_compare::{TreeMergeAction, TreeMergePlan, TreeMergeReport};

use crate::term;

/// Prints a merge plan: one line per step, then omissions, conflicts
/// and a summary.
pub fn print_plan(plan: &TreeMergePlan, color: bool) {
    if plan.steps.is_empty() && plan.skipped.is_empty() {
        println!("nothing to do: the merge plan is empty");
        return;
    }
    for step in &plan.steps {
        let (glyph, code) = match step.action {
            TreeMergeAction::TakeLeft => ("L", term::BLUE),
            TreeMergeAction::TakeRight => ("R", term::BLUE),
            TreeMergeAction::MergedText => ("M", term::GREEN),
            TreeMergeAction::Conflict => ("C", term::RED),
            TreeMergeAction::MakeDir => ("+", term::DIM),
        };
        let dir_mark = if matches!(step.action, TreeMergeAction::MakeDir) {
            "/"
        } else {
            ""
        };
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
    for path in &plan.conflicts {
        println!(
            "{}",
            term::paint(color, term::RED, &format!("! {path} (conflict)"))
        );
    }
    println!(
        "{} step(s) planned, {} omitted, {} conflict(s)",
        plan.steps.len(),
        plan.skipped.len(),
        plan.conflicts.len()
    );
}

/// Prints the outcome of applying a merge plan.
pub fn print_report(report: &TreeMergeReport, color: bool) {
    println!(
        "{}",
        term::paint(color, term::GREEN, &format!("written: {}", report.written))
    );
    println!(
        "{}",
        term::paint(
            color,
            term::DIM,
            &format!("directories created: {}", report.dirs_created)
        )
    );
    for (path, reason) in &report.failed {
        println!(
            "{}",
            term::paint(color, term::RED, &format!("failed: {path} ({reason})"))
        );
    }
}
