//! Plain-text diff report generator.

use crate::{ChildReportAnchor, Hunk, ParentReportAnchor, ReportFilter, ReportInput};

/// Builder for plain-text diff reports.
#[derive(Debug, Clone)]
pub struct TextReportBuilder<'a> {
    input: &'a ReportInput,
    child_anchors: Vec<ChildReportAnchor>,
    parent_anchor: Option<ParentReportAnchor>,
    breadcrumbs: Vec<ParentReportAnchor>,
    subtitle: Option<String>,
    show_line_numbers: bool,
    filter: ReportFilter,
}

impl<'a> TextReportBuilder<'a> {
    /// Creates a new text report builder.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            input,
            child_anchors: Vec::new(),
            parent_anchor: None,
            breadcrumbs: Vec::new(),
            subtitle: None,
            show_line_numbers: false,
            filter: ReportFilter::All,
        }
    }

    /// Adds a child report anchor.
    pub fn with_child_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.child_anchors.push(ChildReportAnchor::new(title, href));
        self
    }

    /// Appends a child report anchor with status and diff stats.
    pub fn with_child_anchor_detailed(
        mut self,
        title: impl Into<String>,
        href: impl Into<String>,
        status: Option<String>,
        stats: Option<crate::DiffStats>,
    ) -> Self {
        let mut anchor = ChildReportAnchor::new(title, href);
        if let Some(st) = status {
            anchor = anchor.with_status(st);
        }
        if let Some(st) = stats {
            anchor = anchor.with_stats(st);
        }
        self.child_anchors.push(anchor);
        self
    }

    /// Adds multiple child report anchors.
    pub fn with_child_anchors(mut self, anchors: Vec<ChildReportAnchor>) -> Self {
        self.child_anchors.extend(anchors);
        self
    }

    /// Adds a child anchor to the current builder instance.
    pub fn add_child_anchor(&mut self, anchor: ChildReportAnchor) -> &mut Self {
        self.child_anchors.push(anchor);
        self
    }

    /// Sets a parent report anchor for back-link navigation.
    pub fn with_parent_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.parent_anchor = Some(ParentReportAnchor::new(title, href));
        self
    }

    /// Appends a breadcrumb link to the parent navigation trail.
    pub fn with_breadcrumb(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.breadcrumbs.push(ParentReportAnchor::new(title, href));
        self
    }

    /// Appends multiple breadcrumb links to the parent navigation trail.
    pub fn with_breadcrumbs(mut self, crumbs: Vec<ParentReportAnchor>) -> Self {
        self.breadcrumbs.extend(crumbs);
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

    /// Sets a display filter for diff lines.
    pub fn with_filter(mut self, filter: ReportFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Filters the report to show differences only.
    pub fn diffs_only(self) -> Self {
        self.with_filter(ReportFilter::DiffsOnly)
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

        if !self.breadcrumbs.is_empty() {
            out.push_str("BREADCRUMBS:\n");
            for (i, crumb) in self.breadcrumbs.iter().enumerate() {
                out.push_str("  ");
                out.push_str(&crumb.title);
                out.push_str(" (");
                out.push_str(&crumb.href);
                out.push(')');
                if i.saturating_add(1) < self.breadcrumbs.len() {
                    out.push_str(" >\n");
                } else {
                    out.push('\n');
                }
            }
        } else if let Some(ref parent) = self.parent_anchor {
            out.push_str("PARENT REPORT: ");
            out.push_str(&parent.title);
            out.push_str(" (");
            out.push_str(&parent.href);
            out.push_str(")\n");
        }

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
                out.push_str(&format!("  * {} -> {}", anchor.title, anchor.href));
                if let Some(ref status) = anchor.status {
                    out.push_str(&format!(" [{}]", status));
                }
                if let Some(ref stats) = anchor.stats {
                    out.push_str(&format!(
                        " (+{}, -{}, ~{})",
                        stats.additions, stats.deletions, stats.modifications
                    ));
                }
                out.push('\n');
            }
            out.push_str("--------------------------------------------------------------------------------\n");
        }

        // Apply filter
        let effective_input = self.input.filtered(self.filter);

        // Hunks
        if effective_input.hunks.is_empty() {
            out.push_str("No differences detected.\n");
        } else {
            for hunk in &effective_input.hunks {
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

/// Convenience function to render a linked report bundle (parent index + child reports) into plain text.
///
/// Returns `(parent_text, Vec<(child_filename, child_text)>)`.
pub fn render_text_bundle(
    bundle: &crate::LinkedReportBundle,
    show_line_numbers: bool,
) -> (String, Vec<(String, String)>) {
    let mut child_anchors = Vec::with_capacity(bundle.children.len());
    for child in &bundle.children {
        let mut anchor = ChildReportAnchor::new(&child.title, &child.filename);
        if let Some(ref st) = child.status {
            anchor = anchor.with_status(st);
        }
        anchor = anchor.with_stats(child.input.stats.clone());
        child_anchors.push(anchor);
    }

    let parent_text = TextReportBuilder::new(&bundle.parent_input)
        .with_child_anchors(child_anchors)
        .with_line_numbers(show_line_numbers)
        .render();

    let mut child_reports = Vec::with_capacity(bundle.children.len());
    for child in &bundle.children {
        let mut builder = TextReportBuilder::new(&child.input)
            .with_parent_anchor(&bundle.parent_title, &bundle.parent_filename)
            .with_line_numbers(show_line_numbers);
        if let Some(ref st) = child.status {
            builder = builder.with_subtitle(format!("Status: {}", st));
        }
        let child_text = builder.render();
        child_reports.push((child.filename.clone(), child_text));
    }

    (parent_text, child_reports)
}
