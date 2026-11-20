//! XML diff report generator.

use crate::{ChildReportAnchor, Hunk, ParentReportAnchor, ReportFilter, ReportInput, escape_xml};

/// Builder for XML diff reports.
#[derive(Debug, Clone)]
pub struct XmlReportBuilder<'a> {
    input: &'a ReportInput,
    child_anchors: Vec<ChildReportAnchor>,
    parent_anchor: Option<ParentReportAnchor>,
    breadcrumbs: Vec<ParentReportAnchor>,
    subtitle: Option<String>,
    filter: ReportFilter,
}

impl<'a> XmlReportBuilder<'a> {
    /// Creates a new XML report builder.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            input,
            child_anchors: Vec::new(),
            parent_anchor: None,
            breadcrumbs: Vec::new(),
            subtitle: None,
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

    /// Renders the XML report document string.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(4096);

        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<diff-report xmlns=\"urn:autocompare:report:v1\">\n");

        out.push_str("  <title>");
        out.push_str(&escape_xml(&self.input.title));
        out.push_str("</title>\n");

        if !self.breadcrumbs.is_empty() {
            out.push_str("  <breadcrumbs>\n");
            for crumb in &self.breadcrumbs {
                out.push_str("    <breadcrumb href=\"");
                out.push_str(&escape_xml(&crumb.href));
                out.push_str("\">");
                out.push_str(&escape_xml(&crumb.title));
                out.push_str("</breadcrumb>\n");
            }
            out.push_str("  </breadcrumbs>\n");
        } else if let Some(ref parent) = self.parent_anchor {
            out.push_str("  <parent-report href=\"");
            out.push_str(&escape_xml(&parent.href));
            out.push_str("\">");
            out.push_str(&escape_xml(&parent.title));
            out.push_str("</parent-report>\n");
        }

        if let Some(ref sub) = self.subtitle {
            out.push_str("  <subtitle>");
            out.push_str(&escape_xml(sub));
            out.push_str("</subtitle>\n");
        }

        // Stats element
        out.push_str("  <stats additions=\"");
        out.push_str(&self.input.stats.additions.to_string());
        out.push_str("\" deletions=\"");
        out.push_str(&self.input.stats.deletions.to_string());
        out.push_str("\" modifications=\"");
        out.push_str(&self.input.stats.modifications.to_string());
        out.push_str("\" unchanged=\"");
        out.push_str(&self.input.stats.unchanged.to_string());
        out.push_str("\" total-changes=\"");
        out.push_str(&self.input.stats.total_changes().to_string());
        out.push_str("\" />\n");

        // Child reports
        if !self.child_anchors.is_empty() {
            out.push_str("  <child-reports>\n");
            for anchor in &self.child_anchors {
                out.push_str("    <child-report href=\"");
                out.push_str(&escape_xml(&anchor.href));
                out.push('"');

                if let Some(ref status) = anchor.status {
                    out.push_str(" status=\"");
                    out.push_str(&escape_xml(status));
                    out.push('"');
                }

                if let Some(ref stats) = anchor.stats {
                    out.push_str(" additions=\"");
                    out.push_str(&stats.additions.to_string());
                    out.push_str("\" deletions=\"");
                    out.push_str(&stats.deletions.to_string());
                    out.push_str("\" modifications=\"");
                    out.push_str(&stats.modifications.to_string());
                    out.push('"');
                }

                out.push('>');
                out.push_str(&escape_xml(&anchor.title));
                out.push_str("</child-report>\n");
            }
            out.push_str("  </child-reports>\n");
        }

        // Apply filter
        let effective_input = self.input.filtered(self.filter);

        // Hunks
        out.push_str("  <hunks>\n");
        for hunk in &effective_input.hunks {
            render_xml_hunk(&mut out, hunk);
        }
        out.push_str("  </hunks>\n");

        out.push_str("</diff-report>\n");
        out
    }
}

fn render_xml_hunk(out: &mut String, hunk: &Hunk) {
    out.push_str("    <hunk old-start=\"");
    out.push_str(&hunk.old_start.to_string());
    out.push_str("\" old-lines=\"");
    out.push_str(&hunk.old_lines.to_string());
    out.push_str("\" new-start=\"");
    out.push_str(&hunk.new_start.to_string());
    out.push_str("\" new-lines=\"");
    out.push_str(&hunk.new_lines.to_string());
    out.push_str("\" header=\"");
    out.push_str(&escape_xml(&hunk.header()));
    out.push_str("\">\n");

    for line in &hunk.lines {
        out.push_str("      <line change=\"");
        out.push_str(line.change.as_str());
        out.push('"');

        if let Some(old_no) = line.old_line_no {
            out.push_str(" old-line=\"");
            out.push_str(&old_no.to_string());
            out.push('"');
        }

        if let Some(new_no) = line.new_line_no {
            out.push_str(" new-line=\"");
            out.push_str(&new_no.to_string());
            out.push('"');
        }

        out.push('>');
        out.push_str(&escape_xml(&line.content));
        out.push_str("</line>\n");
    }

    out.push_str("    </hunk>\n");
}

/// Convenience function to render a report input into XML.
pub fn render_xml(input: &ReportInput) -> String {
    XmlReportBuilder::new(input).render()
}

/// Convenience function to render a report input into XML with child reports.
pub fn render_xml_with_children(input: &ReportInput, children: &[ChildReportAnchor]) -> String {
    let mut builder = XmlReportBuilder::new(input);
    for child in children {
        builder = builder.with_child_anchor(child.title.clone(), child.href.clone());
    }
    builder.render()
}

/// Convenience function to render a linked report bundle (parent index + child reports) into XML.
///
/// Returns `(parent_xml, Vec<(child_filename, child_xml)>)`.
pub fn render_xml_bundle(bundle: &crate::LinkedReportBundle) -> (String, Vec<(String, String)>) {
    let mut child_anchors = Vec::with_capacity(bundle.children.len());
    for child in &bundle.children {
        let mut anchor = ChildReportAnchor::new(&child.title, &child.filename);
        if let Some(ref st) = child.status {
            anchor = anchor.with_status(st);
        }
        anchor = anchor.with_stats(child.input.stats.clone());
        child_anchors.push(anchor);
    }

    let parent_xml = XmlReportBuilder::new(&bundle.parent_input)
        .with_child_anchors(child_anchors)
        .render();

    let mut child_reports = Vec::with_capacity(bundle.children.len());
    for child in &bundle.children {
        let mut builder = XmlReportBuilder::new(&child.input)
            .with_parent_anchor(&bundle.parent_title, &bundle.parent_filename);
        if let Some(ref st) = child.status {
            builder = builder.with_subtitle(format!("Status: {}", st));
        }
        let child_xml = builder.render();
        child_reports.push((child.filename.clone(), child_xml));
    }

    (parent_xml, child_reports)
}
