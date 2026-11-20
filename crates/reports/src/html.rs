//! HTML report builder and renderers (color and monochrome with linked child reports and side-by-side layout).

use crate::{
    ChildReportAnchor, Hunk, HunkLine, LineChange, LinkedReportBundle, ParentReportAnchor,
    ReportFilter, ReportInput, escape_html,
};

/// Theme selection for HTML diff reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HtmlTheme {
    /// Full-color syntax and diff highlighting (green/red/yellow).
    #[default]
    Color,
    /// Monochrome high-contrast styling (black/white/grayscale) suitable for print and accessibility.
    Mono,
}

/// Layout mode for HTML diff reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HtmlLayout {
    /// Standard unified / interleaved single-table layout (default).
    #[default]
    Unified,
    /// Side-by-side two-pane comparison layout.
    SideBySide,
}

/// Builder for customizable HTML diff reports.
#[derive(Debug, Clone)]
pub struct HtmlReportBuilder<'a> {
    input: &'a ReportInput,
    theme: HtmlTheme,
    layout: HtmlLayout,
    child_anchors: Vec<ChildReportAnchor>,
    parent_anchor: Option<ParentReportAnchor>,
    subtitle: Option<String>,
    filter: ReportFilter,
}

impl<'a> HtmlReportBuilder<'a> {
    /// Creates a new HTML report builder with default (Color) theme and Unified layout.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            input,
            theme: HtmlTheme::Color,
            layout: HtmlLayout::Unified,
            child_anchors: Vec::new(),
            parent_anchor: None,
            subtitle: None,
            filter: ReportFilter::All,
        }
    }

    /// Creates an HTML report builder configured with the Color theme.
    pub fn color(input: &'a ReportInput) -> Self {
        Self::new(input).theme(HtmlTheme::Color)
    }

    /// Creates an HTML report builder configured with the Mono theme.
    pub fn mono(input: &'a ReportInput) -> Self {
        Self::new(input).theme(HtmlTheme::Mono)
    }

    /// Sets the theme for this report.
    pub fn theme(mut self, theme: HtmlTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the layout mode for this report (Unified or SideBySide).
    pub fn layout(mut self, layout: HtmlLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Configures the report to use Side-by-Side layout.
    pub fn side_by_side(self) -> Self {
        self.layout(HtmlLayout::SideBySide)
    }

    /// Configures the report to use Unified layout.
    pub fn unified(self) -> Self {
        self.layout(HtmlLayout::Unified)
    }

    /// Appends a single child report anchor.
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

    /// Appends multiple child report anchors.
    pub fn with_child_anchors(mut self, anchors: Vec<ChildReportAnchor>) -> Self {
        self.child_anchors.extend(anchors);
        self
    }

    /// Adds a child anchor to the current builder instance.
    pub fn add_child_anchor(&mut self, anchor: ChildReportAnchor) -> &mut Self {
        self.child_anchors.push(anchor);
        self
    }

    /// Sets a parent report anchor for back-link navigation (e.g. from child report back to folder index).
    pub fn with_parent_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.parent_anchor = Some(ParentReportAnchor::new(title, href));
        self
    }

    /// Sets an optional subtitle.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Sets a display filter for the report lines.
    pub fn with_filter(mut self, filter: ReportFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Filters the report to show only differences (hides unchanged context lines).
    pub fn diffs_only(self) -> Self {
        self.with_filter(ReportFilter::DiffsOnly)
    }

    /// Renders the complete HTML document string.
    pub fn render(&self) -> String {
        let mut html = String::with_capacity(8192);
        let escaped_title = escape_html(&self.input.title);

        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        html.push_str("  <meta charset=\"UTF-8\">\n");
        html.push_str(
            "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
        );
        html.push_str("  <title>");
        html.push_str(&escaped_title);
        html.push_str(" - Diff Report</title>\n");
        html.push_str("  <style>\n");
        html.push_str(match self.theme {
            HtmlTheme::Color => COLOR_CSS,
            HtmlTheme::Mono => MONO_CSS,
        });
        html.push_str("  </style>\n</head>\n<body>\n");

        // Parent navigation breadcrumb if present
        if let Some(ref parent) = self.parent_anchor {
            html.push_str("  <nav class=\"parent-nav\" id=\"parent-nav\">\n");
            html.push_str("    <a class=\"parent-report-link\" href=\"");
            html.push_str(&escape_html(&parent.href));
            html.push_str("\">&larr; Back to Parent Report: ");
            html.push_str(&escape_html(&parent.title));
            html.push_str("</a>\n");
            html.push_str("  </nav>\n\n");
        }

        // Header section
        html.push_str("  <header class=\"report-header\">\n");
        html.push_str("    <h1 class=\"report-title\">");
        html.push_str(&escaped_title);
        html.push_str("</h1>\n");

        if let Some(ref sub) = self.subtitle {
            html.push_str("    <p class=\"report-subtitle\">");
            html.push_str(&escape_html(sub));
            html.push_str("</p>\n");
        }

        // Summary stats bar
        html.push_str("    <div class=\"diff-stats\">\n");
        html.push_str("      <span class=\"stat-item stat-additions\">+");
        html.push_str(&self.input.stats.additions.to_string());
        html.push_str(" additions</span>\n");
        html.push_str("      <span class=\"stat-item stat-deletions\">-");
        html.push_str(&self.input.stats.deletions.to_string());
        html.push_str(" deletions</span>\n");
        html.push_str("      <span class=\"stat-item stat-modifications\">~");
        html.push_str(&self.input.stats.modifications.to_string());
        html.push_str(" modifications</span>\n");
        html.push_str("      <span class=\"stat-item stat-unchanged\">=");
        html.push_str(&self.input.stats.unchanged.to_string());
        html.push_str(" unchanged</span>\n");
        html.push_str("    </div>\n");
        html.push_str("  </header>\n\n");

        // Linked child reports navigation if present
        if !self.child_anchors.is_empty() {
            html.push_str("  <nav class=\"child-reports\" id=\"child-reports\">\n");
            html.push_str("    <h2 class=\"section-title\">Linked Child Reports</h2>\n");
            html.push_str("    <ul class=\"child-report-list\">\n");
            for anchor in &self.child_anchors {
                html.push_str("      <li class=\"child-report-item\">");
                html.push_str("<a class=\"child-report-anchor\" href=\"");
                html.push_str(&escape_html(&anchor.href));
                html.push_str("\">");
                html.push_str(&escape_html(&anchor.title));
                html.push_str("</a>");

                if let Some(ref st) = anchor.status {
                    html.push_str(" <span class=\"child-status-badge\">");
                    html.push_str(&escape_html(st));
                    html.push_str("</span>");
                }

                if let Some(ref stats) = anchor.stats {
                    html.push_str(" <span class=\"child-stats-badge\">(+");
                    html.push_str(&stats.additions.to_string());
                    html.push_str(", -");
                    html.push_str(&stats.deletions.to_string());
                    html.push_str(", ~");
                    html.push_str(&stats.modifications.to_string());
                    html.push_str(")</span>");
                }

                html.push_str("</li>\n");
            }
            html.push_str("    </ul>\n");
            html.push_str("  </nav>\n\n");
        }

        // Apply filter if specified
        let effective_input = self.input.filtered(self.filter);

        // Hunks container
        html.push_str("  <main class=\"diff-container\">\n");
        if effective_input.hunks.is_empty() {
            html.push_str("    <div class=\"diff-empty\">No differences detected.</div>\n");
        } else {
            for hunk in &effective_input.hunks {
                match self.layout {
                    HtmlLayout::Unified => render_html_unified_hunk(&mut html, hunk),
                    HtmlLayout::SideBySide => render_html_side_by_side_hunk(&mut html, hunk),
                }
            }
        }
        html.push_str("  </main>\n\n");

        // Footer
        html.push_str("  <footer class=\"report-footer\">\n");
        html.push_str("    <p>Generated by autocompare-reports</p>\n");
        html.push_str("  </footer>\n");
        html.push_str("</body>\n</html>\n");

        html
    }
}

/// Dedicated builder for color HTML diff reports.
#[derive(Debug, Clone)]
pub struct HtmlColorBuilder<'a> {
    inner: HtmlReportBuilder<'a>,
}

impl<'a> HtmlColorBuilder<'a> {
    /// Creates a new Color HTML builder.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            inner: HtmlReportBuilder::color(input),
        }
    }

    /// Sets the layout mode for this report (Unified or SideBySide).
    pub fn layout(mut self, layout: HtmlLayout) -> Self {
        self.inner = self.inner.layout(layout);
        self
    }

    /// Configures the report to use Side-by-Side layout.
    pub fn side_by_side(self) -> Self {
        self.layout(HtmlLayout::SideBySide)
    }

    /// Configures the report to use Unified layout.
    pub fn unified(self) -> Self {
        self.layout(HtmlLayout::Unified)
    }

    /// Adds a child report anchor.
    pub fn with_child_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.inner = self.inner.with_child_anchor(title, href);
        self
    }

    /// Adds multiple child report anchors.
    pub fn with_child_anchors(mut self, anchors: Vec<ChildReportAnchor>) -> Self {
        self.inner = self.inner.with_child_anchors(anchors);
        self
    }

    /// Adds a child anchor to the current builder instance.
    pub fn add_child_anchor(&mut self, anchor: ChildReportAnchor) -> &mut Self {
        self.inner.add_child_anchor(anchor);
        self
    }

    /// Sets a parent report anchor for back-link navigation.
    pub fn with_parent_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.inner = self.inner.with_parent_anchor(title, href);
        self
    }

    /// Sets an optional subtitle.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.inner = self.inner.with_subtitle(subtitle);
        self
    }

    /// Sets a display filter for the report lines.
    pub fn with_filter(mut self, filter: ReportFilter) -> Self {
        self.inner = self.inner.with_filter(filter);
        self
    }

    /// Filters the report to show only differences.
    pub fn diffs_only(self) -> Self {
        self.with_filter(ReportFilter::DiffsOnly)
    }

    /// Renders the color HTML report.
    pub fn render(&self) -> String {
        self.inner.render()
    }
}

/// Dedicated builder for monochrome HTML diff reports.
#[derive(Debug, Clone)]
pub struct HtmlMonoBuilder<'a> {
    inner: HtmlReportBuilder<'a>,
}

impl<'a> HtmlMonoBuilder<'a> {
    /// Creates a new Monochrome HTML builder.
    pub fn new(input: &'a ReportInput) -> Self {
        Self {
            inner: HtmlReportBuilder::mono(input),
        }
    }

    /// Sets the layout mode for this report (Unified or SideBySide).
    pub fn layout(mut self, layout: HtmlLayout) -> Self {
        self.inner = self.inner.layout(layout);
        self
    }

    /// Configures the report to use Side-by-Side layout.
    pub fn side_by_side(self) -> Self {
        self.layout(HtmlLayout::SideBySide)
    }

    /// Configures the report to use Unified layout.
    pub fn unified(self) -> Self {
        self.layout(HtmlLayout::Unified)
    }

    /// Adds a child report anchor.
    pub fn with_child_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.inner = self.inner.with_child_anchor(title, href);
        self
    }

    /// Adds multiple child report anchors.
    pub fn with_child_anchors(mut self, anchors: Vec<ChildReportAnchor>) -> Self {
        self.inner = self.inner.with_child_anchors(anchors);
        self
    }

    /// Adds a child anchor to the current builder instance.
    pub fn add_child_anchor(&mut self, anchor: ChildReportAnchor) -> &mut Self {
        self.inner.add_child_anchor(anchor);
        self
    }

    /// Sets a parent report anchor for back-link navigation.
    pub fn with_parent_anchor(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.inner = self.inner.with_parent_anchor(title, href);
        self
    }

    /// Sets an optional subtitle.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.inner = self.inner.with_subtitle(subtitle);
        self
    }

    /// Sets a display filter for the report lines.
    pub fn with_filter(mut self, filter: ReportFilter) -> Self {
        self.inner = self.inner.with_filter(filter);
        self
    }

    /// Filters the report to show only differences.
    pub fn diffs_only(self) -> Self {
        self.with_filter(ReportFilter::DiffsOnly)
    }

    /// Renders the monochrome HTML report.
    pub fn render(&self) -> String {
        self.inner.render()
    }
}

/// Helper to render an individual hunk into unified HTML.
fn render_html_unified_hunk(html: &mut String, hunk: &Hunk) {
    html.push_str("    <section class=\"diff-hunk\">\n");
    html.push_str("      <div class=\"hunk-header\"><code>");
    html.push_str(&escape_html(&hunk.header()));
    html.push_str("</code></div>\n");
    html.push_str("      <table class=\"diff-table\">\n");

    for line in &hunk.lines {
        html.push_str("        <tr class=\"diff-line line-");
        html.push_str(line.change.as_str());
        html.push_str("\">\n");

        // Old line number
        html.push_str("          <td class=\"line-no line-no-old\">");
        if let Some(old_no) = line.old_line_no {
            html.push_str(&old_no.to_string());
        }
        html.push_str("</td>\n");

        // New line number
        html.push_str("          <td class=\"line-no line-no-new\">");
        if let Some(new_no) = line.new_line_no {
            html.push_str(&new_no.to_string());
        }
        html.push_str("</td>\n");

        // Change marker
        html.push_str("          <td class=\"line-marker\">");
        let prefix = line.change.prefix();
        if prefix != ' ' {
            html.push(prefix);
        } else {
            html.push_str("&nbsp;");
        }
        html.push_str("</td>\n");

        // Content
        html.push_str("          <td class=\"line-content\"><pre>");
        html.push_str(&escape_html(&line.content));
        html.push_str("</pre></td>\n");

        html.push_str("        </tr>\n");
    }

    html.push_str("      </table>\n");
    html.push_str("    </section>\n");
}

#[derive(Debug, Clone, Copy)]
struct SbsSide<'b> {
    line_no: Option<usize>,
    marker: char,
    content: &'b str,
    change: LineChange,
}

#[derive(Debug, Clone, Copy)]
struct SbsRow<'b> {
    left: SbsSide<'b>,
    right: SbsSide<'b>,
}

fn pair_sbs_lines<'b>(lines: &'b [HunkLine]) -> Vec<SbsRow<'b>> {
    let mut rows = Vec::with_capacity(lines.len());
    let mut i = 0usize;

    while i < lines.len() {
        let line = match lines.get(i) {
            Some(l) => l,
            None => break,
        };

        match line.change {
            LineChange::Unchanged => {
                rows.push(SbsRow {
                    left: SbsSide {
                        line_no: line.old_line_no,
                        marker: ' ',
                        content: line.content.as_str(),
                        change: LineChange::Unchanged,
                    },
                    right: SbsSide {
                        line_no: line.new_line_no,
                        marker: ' ',
                        content: line.content.as_str(),
                        change: LineChange::Unchanged,
                    },
                });
                i = i.saturating_add(1);
            }
            LineChange::Modify => {
                rows.push(SbsRow {
                    left: SbsSide {
                        line_no: line.old_line_no,
                        marker: '!',
                        content: line.content.as_str(),
                        change: LineChange::Modify,
                    },
                    right: SbsSide {
                        line_no: line.new_line_no,
                        marker: '!',
                        content: line.content.as_str(),
                        change: LineChange::Modify,
                    },
                });
                i = i.saturating_add(1);
            }
            LineChange::Delete | LineChange::Insert => {
                let mut deletes: Vec<&'b HunkLine> = Vec::new();
                let mut inserts: Vec<&'b HunkLine> = Vec::new();

                while let Some(cur) = lines.get(i) {
                    if cur.change == LineChange::Delete {
                        deletes.push(cur);
                        i = i.saturating_add(1);
                    } else if cur.change == LineChange::Insert {
                        inserts.push(cur);
                        i = i.saturating_add(1);
                    } else {
                        break;
                    }
                }

                let max_len = deletes.len().max(inserts.len());
                for k in 0..max_len {
                    let left = match deletes.get(k) {
                        Some(del) => SbsSide {
                            line_no: del.old_line_no,
                            marker: '-',
                            content: del.content.as_str(),
                            change: LineChange::Delete,
                        },
                        None => SbsSide {
                            line_no: None,
                            marker: ' ',
                            content: "",
                            change: LineChange::Unchanged,
                        },
                    };
                    let right = match inserts.get(k) {
                        Some(ins) => SbsSide {
                            line_no: ins.new_line_no,
                            marker: '+',
                            content: ins.content.as_str(),
                            change: LineChange::Insert,
                        },
                        None => SbsSide {
                            line_no: None,
                            marker: ' ',
                            content: "",
                            change: LineChange::Unchanged,
                        },
                    };
                    rows.push(SbsRow { left, right });
                }
            }
        }
    }

    rows
}

/// Helper to render an individual hunk into side-by-side HTML.
fn render_html_side_by_side_hunk(html: &mut String, hunk: &Hunk) {
    html.push_str("    <section class=\"diff-hunk\">\n");
    html.push_str("      <div class=\"hunk-header\"><code>");
    html.push_str(&escape_html(&hunk.header()));
    html.push_str("</code></div>\n");
    html.push_str("      <table class=\"diff-table diff-table-sbs\">\n");
    html.push_str("        <thead>\n");
    html.push_str("          <tr class=\"sbs-header-row\">\n");
    html.push_str(
        "            <th colspan=\"3\" class=\"sbs-side-title sbs-left-title\">Left (Old)</th>\n",
    );
    html.push_str(
        "            <th colspan=\"3\" class=\"sbs-side-title sbs-right-title\">Right (New)</th>\n",
    );
    html.push_str("          </tr>\n");
    html.push_str("        </thead>\n");
    html.push_str("        <tbody>\n");

    let rows = pair_sbs_lines(&hunk.lines);
    for row in rows {
        html.push_str("          <tr class=\"diff-line sbs-row\">\n");

        // Left side
        let left_cls = if row.left.line_no.is_none() && row.left.content.is_empty() {
            "sbs-empty"
        } else {
            match row.left.change {
                LineChange::Unchanged => "line-unchanged",
                LineChange::Delete => "line-delete",
                LineChange::Insert => "line-insert",
                LineChange::Modify => "line-modify",
            }
        };
        html.push_str("            <td class=\"line-no line-no-old ");
        html.push_str(left_cls);
        html.push_str("\">");
        if let Some(no) = row.left.line_no {
            html.push_str(&no.to_string());
        }
        html.push_str("</td>\n");

        html.push_str("            <td class=\"line-marker ");
        html.push_str(left_cls);
        html.push_str("\">");
        if row.left.marker != ' ' {
            html.push(row.left.marker);
        } else {
            html.push_str("&nbsp;");
        }
        html.push_str("</td>\n");

        html.push_str("            <td class=\"line-content sbs-cell-left-content ");
        html.push_str(left_cls);
        html.push_str("\"><pre>");
        html.push_str(&escape_html(row.left.content));
        html.push_str("</pre></td>\n");

        // Right side
        let right_cls = if row.right.line_no.is_none() && row.right.content.is_empty() {
            "sbs-empty"
        } else {
            match row.right.change {
                LineChange::Unchanged => "line-unchanged",
                LineChange::Delete => "line-delete",
                LineChange::Insert => "line-insert",
                LineChange::Modify => "line-modify",
            }
        };
        html.push_str("            <td class=\"line-no line-no-new ");
        html.push_str(right_cls);
        html.push_str("\">");
        if let Some(no) = row.right.line_no {
            html.push_str(&no.to_string());
        }
        html.push_str("</td>\n");

        html.push_str("            <td class=\"line-marker ");
        html.push_str(right_cls);
        html.push_str("\">");
        if row.right.marker != ' ' {
            html.push(row.right.marker);
        } else {
            html.push_str("&nbsp;");
        }
        html.push_str("</td>\n");

        html.push_str("            <td class=\"line-content ");
        html.push_str(right_cls);
        html.push_str("\"><pre>");
        html.push_str(&escape_html(row.right.content));
        html.push_str("</pre></td>\n");

        html.push_str("          </tr>\n");
    }

    html.push_str("        </tbody>\n");
    html.push_str("      </table>\n");
    html.push_str("    </section>\n");
}

/// Convenience function to render a report input into HTML with a given theme.
pub fn render_html(input: &ReportInput, theme: HtmlTheme) -> String {
    HtmlReportBuilder::new(input).theme(theme).render()
}

/// Convenience function to render a report input into full-color HTML.
pub fn render_html_color(input: &ReportInput) -> String {
    HtmlReportBuilder::color(input).render()
}

/// Convenience function to render a report input into monochrome HTML.
pub fn render_html_mono(input: &ReportInput) -> String {
    HtmlReportBuilder::mono(input).render()
}

/// Convenience function to render a report input into side-by-side HTML.
pub fn render_html_side_by_side(input: &ReportInput, theme: HtmlTheme) -> String {
    HtmlReportBuilder::new(input)
        .theme(theme)
        .side_by_side()
        .render()
}

/// Convenience function to render an HTML report with linked child reports.
pub fn render_html_with_children(
    input: &ReportInput,
    theme: HtmlTheme,
    children: &[ChildReportAnchor],
) -> String {
    let mut builder = HtmlReportBuilder::new(input).theme(theme);
    for child in children {
        builder = builder.with_child_anchor(child.title.clone(), child.href.clone());
    }
    builder.render()
}

/// Convenience function to render a linked report bundle (parent index + child reports).
pub fn render_html_bundle(
    bundle: &LinkedReportBundle,
    theme: HtmlTheme,
    layout: HtmlLayout,
) -> (String, Vec<(String, String)>) {
    bundle.render_html(theme, layout)
}

const COLOR_CSS: &str = r#"
    :root {
      --bg: #ffffff;
      --fg: #24292f;
      --border: #d0d7de;
      --ins-bg: #dafbe1;
      --ins-fg: #1a7f37;
      --ins-border: #aceebb;
      --del-bg: #ffebe9;
      --del-fg: #cf222e;
      --del-border: #ffcecb;
      --mod-bg: #fff8c5;
      --mod-fg: #9a6700;
      --mod-border: #fae17d;
      --hunk-header-bg: #f6f8fa;
      --hunk-header-fg: #57606a;
      --line-no-fg: #8c959f;
      --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
    }
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif; margin: 0; padding: 24px; background: var(--bg); color: var(--fg); line-height: 1.5; }
    .parent-nav { margin-bottom: 16px; }
    .parent-report-link { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 500; color: #0969da; text-decoration: none; padding: 4px 10px; border-radius: 6px; background: #f6f8fa; border: 1px solid var(--border); }
    .parent-report-link:hover { text-decoration: underline; background: #eaeef2; }
    .report-header { margin-bottom: 24px; padding-bottom: 16px; border-bottom: 1px solid var(--border); }
    .report-title { margin: 0 0 8px 0; font-size: 24px; font-weight: 600; }
    .report-subtitle { margin: 0 0 12px 0; color: #57606a; font-size: 14px; }
    .diff-stats { display: flex; gap: 12px; font-size: 13px; font-family: var(--font-mono); }
    .stat-item { padding: 2px 8px; border-radius: 4px; font-weight: 500; }
    .stat-additions { background: var(--ins-bg); color: var(--ins-fg); }
    .stat-deletions { background: var(--del-bg); color: var(--del-fg); }
    .stat-modifications { background: var(--mod-bg); color: var(--mod-fg); }
    .stat-unchanged { background: #f6f8fa; color: #57606a; }
    .child-reports { margin-bottom: 24px; padding: 16px; background: #f6f8fa; border: 1px solid var(--border); border-radius: 6px; }
    .child-reports h2 { margin: 0 0 12px 0; font-size: 16px; }
    .child-report-list { margin: 0; padding-left: 20px; }
    .child-report-item { margin-bottom: 8px; font-size: 13px; display: flex; align-items: center; gap: 8px; }
    .child-report-anchor { color: #0969da; text-decoration: none; font-weight: 500; }
    .child-report-anchor:hover { text-decoration: underline; }
    .child-status-badge { padding: 1px 6px; border-radius: 3px; font-size: 11px; font-weight: 600; background: #ddf4ff; color: #0969da; border: 1px solid #54aeff; }
    .child-stats-badge { font-family: var(--font-mono); font-size: 11px; color: #57606a; }
    .diff-container { border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
    .diff-hunk { border-bottom: 1px solid var(--border); }
    .diff-hunk:last-child { border-bottom: none; }
    .hunk-header { background: var(--hunk-header-bg); color: var(--hunk-header-fg); padding: 6px 12px; font-family: var(--font-mono); font-size: 12px; border-bottom: 1px solid var(--border); }
    .diff-table { width: 100%; border-collapse: collapse; font-family: var(--font-mono); font-size: 12px; }
    .diff-table-sbs { table-layout: fixed; }
    .sbs-header-row th { background: var(--hunk-header-bg); color: var(--hunk-header-fg); padding: 4px 8px; font-size: 11px; font-weight: 600; border-bottom: 1px solid var(--border); text-align: left; }
    .sbs-left-title { border-right: 2px solid var(--border); width: 50%; }
    .sbs-right-title { width: 50%; }
    .sbs-cell-left-content { border-right: 2px solid var(--border); }
    .sbs-empty { background-color: #fafbfc; }
    .diff-line td { padding: 1px 4px; vertical-align: top; }
    .line-no { width: 44px; text-align: right; color: var(--line-no-fg); user-select: none; font-size: 11px; padding-right: 8px !important; }
    .line-marker { width: 16px; text-align: center; user-select: none; font-weight: 600; }
    .line-content pre { margin: 0; font-family: inherit; font-size: inherit; white-space: pre-wrap; word-break: break-all; }
    .line-insert { background-color: var(--ins-bg); color: var(--ins-fg); }
    .line-insert .line-marker { color: var(--ins-fg); }
    .line-delete { background-color: var(--del-bg); color: var(--del-fg); }
    .line-delete .line-marker { color: var(--del-fg); }
    .line-modify { background-color: var(--mod-bg); color: var(--mod-fg); }
    .line-modify .line-marker { color: var(--mod-fg); }
    .report-footer { margin-top: 32px; font-size: 12px; color: #8c959f; text-align: center; }
    @media print {
      body { margin: 0; padding: 12mm; font-size: 9pt; background: #fff !important; color: #000 !important; }
      .diff-container, .diff-hunk, .child-reports { break-inside: avoid; page-break-inside: avoid; }
      a { color: #000 !important; text-decoration: underline !important; }
    }
"#;

const MONO_CSS: &str = r#"
    :root {
      --bg: #ffffff;
      --fg: #000000;
      --border: #333333;
      --line-no-fg: #555555;
      --hunk-header-bg: #e6e6e6;
      --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
    }
    body { font-family: var(--font-mono); margin: 0; padding: 24px; background: var(--bg); color: var(--fg); line-height: 1.4; }
    .parent-nav { margin-bottom: 16px; }
    .parent-report-link { display: inline-block; font-size: 12px; font-weight: bold; color: #000000; text-decoration: none; padding: 3px 8px; border: 1px solid var(--border); }
    .parent-report-link:hover { text-decoration: underline; }
    .report-header { margin-bottom: 24px; padding-bottom: 16px; border-bottom: 2px solid var(--border); }
    .report-title { margin: 0 0 8px 0; font-size: 22px; font-weight: bold; }
    .report-subtitle { margin: 0 0 12px 0; font-size: 13px; font-style: italic; }
    .diff-stats { display: flex; gap: 16px; font-size: 12px; margin-top: 8px; }
    .stat-item { border: 1px solid var(--border); padding: 2px 6px; }
    .child-reports { margin-bottom: 24px; padding: 12px; border: 1px dashed var(--border); }
    .child-reports h2 { margin: 0 0 8px 0; font-size: 14px; text-transform: uppercase; }
    .child-report-list { margin: 0; padding-left: 20px; }
    .child-report-item { margin-bottom: 6px; font-size: 12px; display: flex; align-items: center; gap: 8px; }
    .child-report-anchor { color: #000000; font-weight: bold; }
    .child-status-badge { border: 1px solid #000000; padding: 1px 4px; font-size: 10px; font-weight: bold; text-transform: uppercase; }
    .child-stats-badge { font-size: 11px; }
    .diff-container { border: 2px solid var(--border); }
    .diff-hunk { border-bottom: 1px solid var(--border); }
    .diff-hunk:last-child { border-bottom: none; }
    .hunk-header { background: var(--hunk-header-bg); font-weight: bold; padding: 4px 8px; border-bottom: 1px solid var(--border); }
    .diff-table { width: 100%; border-collapse: collapse; font-size: 12px; }
    .diff-table-sbs { table-layout: fixed; font-size: 11px; }
    .sbs-header-row th { background: var(--hunk-header-bg); font-weight: bold; padding: 4px 6px; font-size: 11px; border-bottom: 1px solid var(--border); text-align: left; }
    .sbs-left-title { border-right: 2px solid var(--border); width: 50%; }
    .sbs-right-title { width: 50%; }
    .sbs-cell-left-content { border-right: 2px solid var(--border); }
    .sbs-empty { background-color: #f2f2f2; }
    .diff-line td { padding: 1px 4px; }
    .line-no { width: 40px; text-align: right; color: var(--line-no-fg); user-select: none; }
    .line-marker { width: 16px; text-align: center; font-weight: bold; }
    .line-content pre { margin: 0; font-family: inherit; font-size: inherit; white-space: pre-wrap; }
    .line-insert { font-weight: bold; }
    .line-delete { text-decoration: line-through; }
    .line-modify { font-style: italic; }
    .report-footer { margin-top: 32px; font-size: 11px; text-align: center; border-top: 1px solid var(--border); padding-top: 8px; }
    @media print {
      body { margin: 0; padding: 10mm; font-size: 9pt; background: #fff !important; color: #000 !important; }
      .report-header { margin-bottom: 12px; padding-bottom: 8px; border-bottom: 2px solid #000; }
      .report-title { font-size: 16pt; }
      .diff-container { border: 1px solid #000; break-inside: avoid; page-break-inside: avoid; }
      .diff-hunk { break-inside: avoid; page-break-inside: avoid; }
      .child-reports { border: 1px solid #000; break-inside: avoid; }
      .report-footer { margin-top: 16px; border-top: 1px solid #000; }
      a { color: #000 !important; text-decoration: underline !important; }
    }
"#;
