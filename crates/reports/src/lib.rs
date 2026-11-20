//! Reporting engine for autocompare.
//!
//! Provides decoupled report generation in multiple formats with Beyond Compare parity:
//! - HTML (color, monochrome/no-color, unified, side-by-side, linked child reports, parent navigation)
//! - Plain text renderer (unified, line numbers, child reports, parent navigation)
//! - XML renderer (machine readable, full stats, child reports, parent navigation)
//! - Unified and context patch renderer

#![forbid(unsafe_code)]

pub mod html;
pub mod patch;
pub mod text;
pub mod xml;

pub use html::{
    HtmlColorBuilder, HtmlLayout, HtmlMonoBuilder, HtmlReportBuilder, HtmlTheme, render_html,
    render_html_bundle, render_html_color, render_html_mono, render_html_side_by_side,
    render_html_with_children,
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
    pub status: Option<String>,
    pub stats: Option<DiffStats>,
}

impl ChildReportAnchor {
    /// Creates a new child report anchor with default empty status and stats.
    pub fn new(title: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            href: href.into(),
            status: None,
            stats: None,
        }
    }

    /// Attaches an optional status badge (e.g. "Different", "Left Only", "Right Only", "Identical").
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Attaches optional diff statistics to this child anchor.
    pub fn with_stats(mut self, stats: DiffStats) -> Self {
        self.stats = Some(stats);
        self
    }
}

/// Anchor descriptor linking back to a parent or index report (for breadcrumb navigation in child reports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentReportAnchor {
    pub title: String,
    pub href: String,
}

impl ParentReportAnchor {
    /// Creates a new parent report anchor.
    pub fn new(title: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            href: href.into(),
        }
    }
}

/// Display filter for diff reports, matching Beyond Compare's display filter options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportFilter {
    /// Show all lines (context + differences).
    #[default]
    All,
    /// Show only lines with differences (deletions, insertions, modifications).
    DiffsOnly,
    /// Show differences plus up to `context` lines of surrounding unchanged context.
    Context(usize),
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

    /// Returns a filtered clone of this report input based on `ReportFilter`.
    pub fn filtered(&self, filter: ReportFilter) -> Self {
        match filter {
            ReportFilter::All => self.clone(),
            ReportFilter::DiffsOnly => {
                let filtered_hunks = self
                    .hunks
                    .iter()
                    .filter_map(|hunk| {
                        let changed_lines: Vec<HunkLine> = hunk
                            .lines
                            .iter()
                            .filter(|l| l.change != LineChange::Unchanged)
                            .cloned()
                            .collect();
                        if changed_lines.is_empty() {
                            None
                        } else {
                            Some(Hunk::new(
                                hunk.old_start,
                                hunk.old_lines,
                                hunk.new_start,
                                hunk.new_lines,
                                changed_lines,
                            ))
                        }
                    })
                    .collect();
                Self {
                    title: self.title.clone(),
                    hunks: filtered_hunks,
                    stats: self.stats.clone(),
                }
            }
            ReportFilter::Context(ctx) => {
                let filtered_hunks = self
                    .hunks
                    .iter()
                    .filter_map(|hunk| {
                        let total_lines = hunk.lines.len();
                        let mut include = vec![false; total_lines];
                        for (idx, line) in hunk.lines.iter().enumerate() {
                            if line.change != LineChange::Unchanged {
                                let start = idx.saturating_sub(ctx);
                                let end =
                                    (idx.saturating_add(ctx)).min(total_lines.saturating_sub(1));
                                for slot in
                                    include.iter_mut().take(end.saturating_add(1)).skip(start)
                                {
                                    *slot = true;
                                }
                            }
                        }

                        let selected: Vec<HunkLine> = hunk
                            .lines
                            .iter()
                            .zip(include.iter())
                            .filter_map(|(l, &inc)| if inc { Some(l.clone()) } else { None })
                            .collect();

                        if selected.is_empty() {
                            None
                        } else {
                            Some(Hunk::new(
                                hunk.old_start,
                                hunk.old_lines,
                                hunk.new_start,
                                hunk.new_lines,
                                selected,
                            ))
                        }
                    })
                    .collect();
                Self {
                    title: self.title.clone(),
                    hunks: filtered_hunks,
                    stats: self.stats.clone(),
                }
            }
        }
    }
}

/// A bundle representing a master parent report and multiple linked child reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedReportBundle {
    pub parent_title: String,
    pub parent_filename: String,
    pub parent_input: ReportInput,
    pub children: Vec<LinkedChildItem>,
}

/// A child report item within a linked report bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedChildItem {
    pub title: String,
    pub filename: String,
    pub status: Option<String>,
    pub input: ReportInput,
}

impl LinkedReportBundle {
    /// Creates a new linked report bundle with a parent report.
    pub fn new(
        parent_title: impl Into<String>,
        parent_filename: impl Into<String>,
        parent_input: ReportInput,
    ) -> Self {
        Self {
            parent_title: parent_title.into(),
            parent_filename: parent_filename.into(),
            parent_input,
            children: Vec::new(),
        }
    }

    /// Appends a child report item to this bundle.
    pub fn with_child(
        mut self,
        title: impl Into<String>,
        filename: impl Into<String>,
        status: Option<String>,
        input: ReportInput,
    ) -> Self {
        self.children.push(LinkedChildItem {
            title: title.into(),
            filename: filename.into(),
            status,
            input,
        });
        self
    }

    /// Adds a child report item to the bundle instance.
    pub fn add_child(
        &mut self,
        title: impl Into<String>,
        filename: impl Into<String>,
        status: Option<String>,
        input: ReportInput,
    ) -> &mut Self {
        self.children.push(LinkedChildItem {
            title: title.into(),
            filename: filename.into(),
            status,
            input,
        });
        self
    }

    /// Renders the parent HTML report and all child HTML reports with mutual navigation links.
    ///
    /// Returns `(parent_html, Vec<(child_filename, child_html)>)`.
    pub fn render_html(
        &self,
        theme: HtmlTheme,
        layout: HtmlLayout,
    ) -> (String, Vec<(String, String)>) {
        let mut child_anchors = Vec::with_capacity(self.children.len());
        for child in &self.children {
            let mut anchor = ChildReportAnchor::new(&child.title, &child.filename);
            if let Some(ref st) = child.status {
                anchor = anchor.with_status(st);
            }
            anchor = anchor.with_stats(child.input.stats.clone());
            child_anchors.push(anchor);
        }

        let parent_html = HtmlReportBuilder::new(&self.parent_input)
            .theme(theme)
            .layout(layout)
            .with_child_anchors(child_anchors)
            .render();

        let mut child_reports = Vec::with_capacity(self.children.len());
        for child in &self.children {
            let mut builder = HtmlReportBuilder::new(&child.input)
                .theme(theme)
                .layout(layout)
                .with_parent_anchor(&self.parent_title, &self.parent_filename);
            if let Some(ref st) = child.status {
                builder = builder.with_subtitle(format!("Status: {}", st));
            }
            let child_html = builder.render();
            child_reports.push((child.filename.clone(), child_html));
        }

        (parent_html, child_reports)
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
