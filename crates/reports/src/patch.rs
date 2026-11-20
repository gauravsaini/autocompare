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
        let old_p = self
            .old_path
            .as_deref()
            .unwrap_or(self.input.title.as_str());
        let new_p = self
            .new_path
            .as_deref()
            .unwrap_or(self.input.title.as_str());

        let effective_input = self.input.filtered(self.filter);

        match self.format {
            PatchFormat::Unified => render_unified(&effective_input, old_p, new_p),
            PatchFormat::Context => render_context(&effective_input, old_p, new_p),
        }
    }
}

fn render_unified(input: &ReportInput, old_p: &str, new_p: &str) -> String {
    let mut out = String::with_capacity(2048);

    out.push_str("--- a/");
    out.push_str(old_p);
    out.push('\n');

    out.push_str("+++ b/");
    out.push_str(new_p);
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

fn render_context(input: &ReportInput, old_p: &str, new_p: &str) -> String {
    let mut out = String::with_capacity(2048);

    out.push_str("*** ");
    out.push_str(old_p);
    out.push('\n');

    out.push_str("--- ");
    out.push_str(new_p);
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
