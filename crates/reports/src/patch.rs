//! Unified and context patch diff report generator.

use crate::{LineChange, ReportFilter, ReportInput};

/// Patch output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PatchFormat {
    /// Standard Unified diff format (`--- a/...`, `+++ b/...`, `@@ -a,b +c,d @@`).
    #[default]
    Unified,
    /// Traditional Context diff format (`*** ...`, `--- ...`, `***************`).
    Context,
}

/// Builder for patch reports.
#[derive(Debug, Clone)]
pub struct PatchReportBuilder<'a> {
    input: &'a ReportInput,
    old_path: Option<String>,
    new_path: Option<String>,
    old_timestamp: Option<String>,
    new_timestamp: Option<String>,
    format: PatchFormat,
    filter: ReportFilter,
}

impl<'a> PatchReportBuilder<'a> {
    /// Creates a new patch report builder configured for Unified format.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            input,
            old_path: None,
            new_path: None,
            old_timestamp: None,
            new_timestamp: None,
            format: PatchFormat::Unified,
            filter: ReportFilter::All,
        }
    }

    /// Sets the old and new file paths for the patch header.
    pub fn with_paths(mut self, old_path: impl Into<String>, new_path: impl Into<String>) -> Self {
        self.old_path = Some(old_path.into());
        self.new_path = Some(new_path.into());
        self
    }

    /// Sets the old file path for the patch header.
    pub fn with_old_path(mut self, old_path: impl Into<String>) -> Self {
        self.old_path = Some(old_path.into());
        self
    }

    /// Sets the new file path for the patch header.
    pub fn with_new_path(mut self, new_path: impl Into<String>) -> Self {
        self.new_path = Some(new_path.into());
        self
    }

    /// Sets the old and new timestamps for the patch header.
    pub fn with_timestamps(
        mut self,
        old_timestamp: impl Into<String>,
        new_timestamp: impl Into<String>,
    ) -> Self {
        self.old_timestamp = Some(old_timestamp.into());
        self.new_timestamp = Some(new_timestamp.into());
        self
    }

    /// Sets the old timestamp for the patch header.
    pub fn with_old_timestamp(mut self, old_timestamp: impl Into<String>) -> Self {
        self.old_timestamp = Some(old_timestamp.into());
        self
    }

    /// Sets the new timestamp for the patch header.
    pub fn with_new_timestamp(mut self, new_timestamp: impl Into<String>) -> Self {
        self.new_timestamp = Some(new_timestamp.into());
        self
    }

    /// Configures the patch format (Unified or Context).
    pub fn format(mut self, format: PatchFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets the format to Unified diff.
    pub fn unified(self) -> Self {
        self.format(PatchFormat::Unified)
    }

    /// Sets the format to Context diff.
    pub fn context(self) -> Self {
        self.format(PatchFormat::Context)
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

    /// Renders the patch string.
    pub fn render(&self) -> String {
        let old_p = match self.old_path.as_deref() {
            Some(p) => p,
            None => self.input.title.as_str(),
        };
        let new_p = match self.new_path.as_deref() {
            Some(p) => p,
            None => self.input.title.as_str(),
        };

        let effective_input = self.input.filtered(self.filter);

        match self.format {
            PatchFormat::Unified => render_unified(
                &effective_input,
                old_p,
                new_p,
                self.old_timestamp.as_deref(),
                self.new_timestamp.as_deref(),
            ),
            PatchFormat::Context => render_context(
                &effective_input,
                old_p,
                new_p,
                self.old_timestamp.as_deref(),
                self.new_timestamp.as_deref(),
            ),
        }
    }
}

fn render_unified(
    input: &ReportInput,
    old_p: &str,
    new_p: &str,
    old_ts: Option<&str>,
    new_ts: Option<&str>,
) -> String {
    let mut out = String::with_capacity(2048);

    out.push_str("--- a/");
    out.push_str(old_p);
    if let Some(ts) = old_ts {
        out.push('\t');
        out.push_str(ts);
    }
    out.push('\n');

    out.push_str("+++ b/");
    out.push_str(new_p);
    if let Some(ts) = new_ts {
        out.push('\t');
        out.push_str(ts);
    }
    out.push('\n');

    for hunk in &input.hunks {
        out.push_str(&hunk.header());
        out.push('\n');

        for line in &hunk.lines {
            match line.change {
                LineChange::Unchanged => {
                    out.push(' ');
                    out.push_str(&line.content);
                    out.push('\n');
                }
                LineChange::Delete => {
                    out.push('-');
                    out.push_str(&line.content);
                    out.push('\n');
                }
                LineChange::Insert => {
                    out.push('+');
                    out.push_str(&line.content);
                    out.push('\n');
                }
                LineChange::Modify => {
                    out.push('!');
                    out.push_str(&line.content);
                    out.push('\n');
                }
            }
        }
    }

    out
}

fn render_context(
    input: &ReportInput,
    old_p: &str,
    new_p: &str,
    old_ts: Option<&str>,
    new_ts: Option<&str>,
) -> String {
    let mut out = String::with_capacity(2048);

    out.push_str("*** ");
    out.push_str(old_p);
    if let Some(ts) = old_ts {
        out.push('\t');
        out.push_str(ts);
    }
    out.push('\n');

    out.push_str("--- ");
    out.push_str(new_p);
    if let Some(ts) = new_ts {
        out.push('\t');
        out.push_str(ts);
    }
    out.push('\n');

    for hunk in &input.hunks {
        out.push_str("***************\n");

        let old_end = hunk
            .old_start
            .saturating_add(hunk.old_lines)
            .saturating_sub(1);
        let new_end = hunk
            .new_start
            .saturating_add(hunk.new_lines)
            .saturating_sub(1);

        out.push_str(&format!("*** {},{} ****\n", hunk.old_start, old_end));
        for line in &hunk.lines {
            if line.change != LineChange::Insert {
                let p = match line.change {
                    LineChange::Unchanged => ' ',
                    LineChange::Delete => '-',
                    LineChange::Modify => '!',
                    LineChange::Insert => '+',
                };
                out.push(p);
                out.push(' ');
                out.push_str(&line.content);
                out.push('\n');
            }
        }

        out.push_str(&format!("--- {},{} ----\n", hunk.new_start, new_end));
        for line in &hunk.lines {
            if line.change != LineChange::Delete {
                let p = match line.change {
                    LineChange::Unchanged => ' ',
                    LineChange::Insert => '+',
                    LineChange::Modify => '!',
                    LineChange::Delete => '-',
                };
                out.push(p);
                out.push(' ');
                out.push_str(&line.content);
                out.push('\n');
            }
        }
    }

    out
}

/// Convenience function to render a report input into unified patch format.
pub fn render_unified_patch(input: &ReportInput) -> String {
    PatchReportBuilder::new(input).unified().render()
}

/// Convenience alias for `render_unified_patch`.
pub fn render_patch(input: &ReportInput) -> String {
    render_unified_patch(input)
}

/// Convenience function to render a report input into context patch format.
pub fn render_context_patch(input: &ReportInput) -> String {
    PatchReportBuilder::new(input).context().render()
}

/// Convenience function to render a linked report bundle into unified or context patch files.
///
/// Returns `(parent_patch, Vec<(child_filename, child_patch)>)`.
pub fn render_patch_bundle(
    bundle: &crate::LinkedReportBundle,
    format: PatchFormat,
) -> (String, Vec<(String, String)>) {
    let parent_patch = PatchReportBuilder::new(&bundle.parent_input)
        .with_paths(&bundle.parent_filename, &bundle.parent_filename)
        .format(format)
        .render();

    let mut child_patches = Vec::with_capacity(bundle.children.len());
    for child in &bundle.children {
        let child_patch = PatchReportBuilder::new(&child.input)
            .with_paths(&child.filename, &child.filename)
            .format(format)
            .render();
        child_patches.push((child.filename.clone(), child_patch));
    }

    (parent_patch, child_patches)
}

/// Convenience function to render a linked report bundle combined into a single unified or context patch.
pub fn render_patch_bundle_combined(
    bundle: &crate::LinkedReportBundle,
    format: PatchFormat,
) -> String {
    let mut combined = String::with_capacity(4096);
    if !bundle.parent_input.hunks.is_empty() {
        let parent_patch = PatchReportBuilder::new(&bundle.parent_input)
            .with_paths(&bundle.parent_filename, &bundle.parent_filename)
            .format(format)
            .render();
        combined.push_str(&parent_patch);
    }

    for child in &bundle.children {
        if !child.input.hunks.is_empty() {
            let child_patch = PatchReportBuilder::new(&child.input)
                .with_paths(&child.filename, &child.filename)
                .format(format)
                .render();
            if !combined.is_empty() && !combined.ends_with('\n') {
                combined.push('\n');
            }
            combined.push_str(&child_patch);
        }
    }

    combined
}
