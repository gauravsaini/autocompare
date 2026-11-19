//! Plain-text diff report generator.

use crate::{ChildReportAnchor, Hunk, ReportInput};

/// Builder for plain-text diff reports.
#[derive(Debug, Clone)]
pub struct TextReportBuilder<'a> {
    input: &'a ReportInput,
    child_anchors: Vec<ChildReportAnchor>,
    subtitle: Option<String>,
    show_line_numbers: bool,
}

impl<'a> TextReportBuilder<'a> {
    /// Creates a new text report builder.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            input,
            child_anchors: Vec::new(),
            subtitle: None,
            show_line_numbers: false,
        }
    }

    /// Adds a child report anchor.
    pub fn with_child_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.child_anchors.push(ChildReportAnchor::new(title, href));
        self
    }

    /// Adds multiple child report anchors.
    pub fn with_child_anchors(mut self, anchors: Vec<ChildReportAnchor>) -> Self {
        self.child_anchors.extend(anchors);
        self
    }

    /// Enables or disables line numbers in output.
    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Sets an optional subtitle.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Renders the plain-text report.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(2048);

        // Header
        out.push_str(
            "================================================================================\n",
        );
        out.push_str("DIFF REPORT: ");
        out.push_str(&self.input.title);
        out.push('\n');

        if let Some(ref sub) = self.subtitle {
            out.push_str("Subtitle: ");
            out.push_str(sub);
            out.push('\n');
        }

        out.push_str(
            "================================================================================\n",
        );

        // Stats
        out.push_str("SUMMARY:\n");
        out.push_str(&format!(
            "  Additions:     {}\n",
            self.input.stats.additions
        ));
        out.push_str(&format!(
            "  Deletions:     {}\n",
            self.input.stats.deletions
        ));
        out.push_str(&format!(
            "  Modifications: {}\n",
            self.input.stats.modifications
        ));
        out.push_str(&format!(
            "  Unchanged:     {}\n",
            self.input.stats.unchanged
        ));
        out.push_str(&format!(
            "  Total changes: {}\n",
            self.input.stats.total_changes()
        ));
        out.push_str(
            "--------------------------------------------------------------------------------\n",
        );

        // Child reports
        if !self.child_anchors.is_empty() {
            out.push_str("LINKED CHILD REPORTS:\n");
            for anchor in &self.child_anchors {
                out.push_str(&format!("  * {} -> {}\n", anchor.title, anchor.href));
            }
            out.push_str("--------------------------------------------------------------------------------\n");
        }

        // Hunks
        if self.input.hunks.is_empty() {
            out.push_str("No differences detected.\n");
        } else {
            for hunk in &self.input.hunks {
                render_text_hunk(&mut out, hunk, self.show_line_numbers);
            }
        }

        out.push_str(
            "================================================================================\n",
        );
        out
    }
}

fn render_text_hunk(out: &mut String, hunk: &Hunk, show_line_numbers: bool) {
    out.push_str(&hunk.header());
    out.push('\n');

    for line in &hunk.lines {
        let prefix = line.change.prefix();
        if show_line_numbers {
            let old_str = match line.old_line_no {
                Some(n) => format!("{:4}", n),
                None => "    ".to_string(),
            };
            let new_str = match line.new_line_no {
                Some(n) => format!("{:4}", n),
                None => "    ".to_string(),
            };
            out.push_str(&format!(
                "{} {} {} {}\n",
                old_str, new_str, prefix, line.content
            ));
        } else {
            out.push(prefix);
            out.push(' ');
            out.push_str(&line.content);
            out.push('\n');
        }
    }
}

/// Convenience function to render a report input into plain text.
pub fn render_text(input: &ReportInput) -> String {
    TextReportBuilder::new(input).render()
}

/// Convenience function to render a report input into plain text with child reports.
pub fn render_text_with_children(input: &ReportInput, children: &[ChildReportAnchor]) -> String {
    let mut builder = TextReportBuilder::new(input);
    for child in children {
        builder = builder.with_child_anchor(child.title.clone(), child.href.clone());
    }
    builder.render()
}
