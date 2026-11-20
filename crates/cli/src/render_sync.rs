//! Plan and report rendering for folder sync.

use autocompare_compare::{SyncAction, SyncPlan, SyncReport};

use crate::term;

/// Formats a plan as the exact lines `print_plan` writes to stdout
/// (one per step, then skipped entries and a summary; an empty plan
/// yields the "nothing to do" line). The depth-limit note stays on
/// stderr in `print_plan` and is not part of the returned text.
pub fn format_plan(plan: &SyncPlan, color: bool) -> String {
    let mut out = String::new();
    if plan.steps.is_empty() && plan.skipped.is_empty() {
        out.push_str("nothing to do: the trees already match the plan\n");
        return out;
    }
    for step in &plan.steps {
        let (glyph, code) = match step.action {
            SyncAction::CopyToRight => ("\u{2192}", term::GREEN),
            SyncAction::CopyToLeft => ("\u{2190}", term::GREEN),
            SyncAction::DeleteRight => ("\u{00d7}", term::RED),
            SyncAction::DeleteLeft => ("\u{00d7}", term::RED),
        };
        let dir_mark = if step.is_dir { "/" } else { "" };
        out.push_str(&format!(
            "{} {}{}\n",
            term::paint(color, code, glyph),
            step.rel_path,
            dir_mark
        ));
    }
    for (path, reason) in &plan.skipped {
        out.push_str(&format!(
            "{}\n",
            term::paint(color, term::DIM, &format!("~ {path} ({reason})"))
        ));
    }
    out.push_str(&format!(
        "{} step(s) planned, {} skipped\n",
        plan.steps.len(),
        plan.skipped.len()
    ));
    out
}

/// Prints a plan: one line per step, then skipped entries and a
/// summary.
pub fn print_plan(plan: &SyncPlan, color: bool) {
    if plan.truncated {
        eprintln!(
            "autocompare: note: the walk stopped at the depth limit; deeper entries are not planned"
        );
    }
    print!("{}", format_plan(plan, color));
}

/// Formats a report as the exact lines `print_report` writes to
/// stdout (copied/deleted counts, then one line per failed step).
pub fn format_report(report: &SyncReport, color: bool) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}\n",
        term::paint(color, term::GREEN, &format!("copied: {}", report.copied))
    ));
    out.push_str(&format!(
        "{}\n",
        term::paint(color, term::RED, &format!("deleted: {}", report.deleted))
    ));
    for (path, reason) in &report.failed {
        out.push_str(&format!(
            "{}\n",
            term::paint(color, term::RED, &format!("failed: {path} ({reason})"))
        ));
    }
    out
}

/// Prints the outcome of applying a plan.
pub fn print_report(report: &SyncReport, color: bool) {
    print!("{}", format_report(report, color));
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
    use autocompare_compare::SyncStep;

    fn step(rel: &str, is_dir: bool, action: SyncAction) -> SyncStep {
        SyncStep {
            rel_path: rel.to_owned(),
            is_dir,
            action,
        }
    }

    #[test]
    fn test_format_plan_empty_reports_nothing_to_do() {
        let plan = SyncPlan::default();
        assert_eq!(
            format_plan(&plan, false),
            "nothing to do: the trees already match the plan\n"
        );
    }

    #[test]
    fn test_format_plan_steps_skipped_and_summary() {
        let plan = SyncPlan {
            steps: vec![
                step("new.txt", false, SyncAction::CopyToRight),
                step("gone", true, SyncAction::DeleteRight),
                step("back.txt", false, SyncAction::CopyToLeft),
            ],
            skipped: vec![("same.txt".to_owned(), "destination is newer".to_owned())],
            truncated: false,
        };
        let out = format_plan(&plan, false);
        assert!(
            out.contains("\u{2192} new.txt\n"),
            "copy-right glyph: {out}"
        );
        assert!(out.contains("\u{00d7} gone/\n"), "dir delete mark: {out}");
        assert!(
            out.contains("\u{2190} back.txt\n"),
            "copy-left glyph: {out}"
        );
        assert!(
            out.contains("~ same.txt (destination is newer)"),
            "skipped line: {out}"
        );
        assert!(
            out.contains("3 step(s) planned, 1 skipped"),
            "summary: {out}"
        );
        assert!(!out.contains('\x1b'), "no ANSI when color is off");
    }

    #[test]
    fn test_format_plan_truncated_flag_stays_on_stderr() {
        let plan = SyncPlan {
            steps: vec![step("a.txt", false, SyncAction::CopyToRight)],
            skipped: Vec::new(),
            truncated: true,
        };
        let out = format_plan(&plan, false);
        assert!(out.contains("a.txt"), "stdout keeps the step: {out}");
        assert!(
            !out.contains("depth limit"),
            "stderr note is not in stdout text: {out}"
        );
    }

    #[test]
    fn test_format_report_counts_failures_and_color() {
        let report = SyncReport {
            copied: 2,
            deleted: 1,
            failed: vec![("gone.txt".to_owned(), "source vanished".to_owned())],
        };
        let plain = format_report(&report, false);
        assert!(plain.contains("copied: 2"), "copied: {plain}");
        assert!(plain.contains("deleted: 1"), "deleted: {plain}");
        assert!(
            plain.contains("failed: gone.txt (source vanished)"),
            "failed: {plain}"
        );
        assert!(!plain.contains('\x1b'), "no ANSI when color is off");
        let colored = format_report(&report, true);
        assert!(
            colored.contains('\x1b'),
            "color on wraps codes: {colored:?}"
        );
    }
}
