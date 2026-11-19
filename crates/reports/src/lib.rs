//! Reporting engine for autocompare.
//!
//! Provides decoupled report generation in multiple formats:
//! - HTML (color and monochrome builders with linked child-report anchors)
//! - Plain text renderer
//! - XML renderer
//! - Unified and context patch renderer

pub mod html;
pub mod patch;
pub mod text;
pub mod xml;

pub use html::{
    HtmlColorBuilder, HtmlMonoBuilder, HtmlReportBuilder, HtmlTheme, render_html,
    render_html_color, render_html_mono, render_html_with_children,
};
pub use patch::{
    PatchFormat, PatchReportBuilder, render_context_patch, render_patch, render_unified_patch,
};
pub use text::{TextReportBuilder, render_text, render_text_with_children};
pub use xml::{XmlReportBuilder, render_xml, render_xml_with_children};

/// Represents the classification of a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineChange {
    /// Line exists in both old and new versions without changes.
    Unchanged,
    /// Line was deleted from the old version.
    Delete,
    /// Line was inserted into the new version.
    Insert,
    /// Line was modified between old and new versions.
    Modify,
}

impl LineChange {
    /// Returns the standard single-character diff prefix.
    pub fn prefix(&self) -> char {
        match self {
            Self::Unchanged => ' ',
            Self::Delete => '-',
            Self::Insert => '+',
            Self::Modify => '!',
        }
    }

    /// Returns a human-readable identifier for CSS classes and XML attributes.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Delete => "delete",
            Self::Insert => "insert",
            Self::Modify => "modify",
        }
    }
}

/// A single formatted line within a diff hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkLine {
    pub change: LineChange,
    pub content: String,
    pub old_line_no: Option<usize>,
    pub new_line_no: Option<usize>,
}

impl HunkLine {
    /// Creates a new unchanged line.
    pub fn unchanged(content: impl Into<String>, old_no: usize, new_no: usize) -> Self {
        Self {
            change: LineChange::Unchanged,
            content: content.into(),
            old_line_no: Some(old_no),
            new_line_no: Some(new_no),
        }
    }

    /// Creates a new deleted line.
    pub fn delete(content: impl Into<String>, old_no: usize) -> Self {
        Self {
            change: LineChange::Delete,
            content: content.into(),
            old_line_no: Some(old_no),
            new_line_no: None,
        }
    }

    /// Creates a new inserted line.
    pub fn insert(content: impl Into<String>, new_no: usize) -> Self {
        Self {
            change: LineChange::Insert,
            content: content.into(),
            old_line_no: None,
            new_line_no: Some(new_no),
        }
    }

    /// Creates a new modified line.
    pub fn modify(content: impl Into<String>, old_no: usize, new_no: usize) -> Self {
        Self {
            change: LineChange::Modify,
            content: content.into(),
            old_line_no: Some(old_no),
            new_line_no: Some(new_no),
        }
    }
}

/// A unified diff hunk covering an edited region with context lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub lines: Vec<HunkLine>,
}

impl Hunk {
    /// Creates a new hunk with the specified line spans and lines.
    pub fn new(
        old_start: usize,
        old_lines: usize,
        new_start: usize,
        new_lines: usize,
        lines: Vec<HunkLine>,
    ) -> Self {
        Self {
            old_start,
            old_lines,
            new_start,
            new_lines,
            lines,
        }
    }

    /// Formats the standard unified diff header for this hunk, e.g. `@@ -1,5 +1,6 @@`.
    pub fn header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_lines, self.new_start, self.new_lines
        )
    }
}

/// Aggregate statistics for a diff report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
    pub unchanged: usize,
}

impl DiffStats {
    /// Creates a new diff statistics instance.
    pub fn new(additions: usize, deletions: usize, modifications: usize, unchanged: usize) -> Self {
        Self {
            additions,
            deletions,
            modifications,
            unchanged,
        }
    }

    /// Total number of changed lines (additions + deletions + modifications).
    pub fn total_changes(&self) -> usize {
        self.additions
            .saturating_add(self.deletions)
            .saturating_add(self.modifications)
    }
}

/// Anchor descriptor linking to a child report (e.g. for folder or sub-component diffs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildReportAnchor {
    pub title: String,
    pub href: String,
}

impl ChildReportAnchor {
    /// Creates a new child report anchor.
    pub fn new(title: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            href: href.into(),
        }
    }
}

/// Decoupled input for all report builders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportInput {
    pub title: String,
    pub hunks: Vec<Hunk>,
    pub stats: DiffStats,
}

impl ReportInput {
    /// Creates a new report input.
    pub fn new(title: impl Into<String>, hunks: Vec<Hunk>, stats: DiffStats) -> Self {
        Self {
            title: title.into(),
            hunks,
            stats,
        }
    }
}

/// Escapes HTML special characters: `&`, `<`, `>`, `"`, `'`.
///
/// Guaranteed never to panic on hostile input.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Escapes XML special characters: `&`, `<`, `>`, `"`, `'`.
///
/// Guaranteed never to panic on hostile input.
pub fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}
