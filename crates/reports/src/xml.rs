//! XML diff report generator.

use crate::{ChildReportAnchor, Hunk, ReportInput, escape_xml};

/// Builder for XML diff reports.
#[derive(Debug, Clone)]
pub struct XmlReportBuilder<'a> {
    input: &'a ReportInput,
    child_anchors: Vec<ChildReportAnchor>,
    subtitle: Option<String>,
}

impl<'a> XmlReportBuilder<'a> {
    /// Creates a new XML report builder.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            input,
            child_anchors: Vec::new(),
            subtitle: None,
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

    /// Sets an optional subtitle.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Renders the XML report document string.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(4096);

        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<diff-report xmlns=\"urn:autocompare:report:v1\">\n");

        out.push_str("  <title>");
        out.push_str(&escape_xml(&self.input.title));
        out.push_str("</title>\n");

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
                out.push_str("\">");
                out.push_str(&escape_xml(&anchor.title));
                out.push_str("</child-report>\n");
            }
            out.push_str("  </child-reports>\n");
        }

        // Hunks
        out.push_str("  <hunks>\n");
        for hunk in &self.input.hunks {
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
