//! Report generation and format dispatch for autocompare CLI.
//!
//! Converts session diff outcomes into structured reports (HTML, Plain Text,
//! XML, Unified Patch, Context Patch) and writes them to disk.

use std::path::{Path, PathBuf};

use autocompare_reports::ReportInput;

use crate::error::CliError;

pub use crate::report_adapter::*;
pub use crate::report_adapter_media::*;

/// Supported output formats for comparison reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Full-featured HTML report with color styling.
    Html,
    /// Full-featured HTML report with color styling (explicit alias).
    HtmlColor,
    /// Monochrome / printer-friendly HTML report without color highlights.
    HtmlMono,
    /// Human-readable plain text report.
    Text,
    /// Machine-readable XML report.
    Xml,
    /// Standard unified diff patch format.
    Patch,
    /// Standard unified diff patch format (explicit alias).
    UnifiedPatch,
    /// Context diff patch format.
    ContextPatch,
}

impl ReportFormat {
    /// Parses format string identifier.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "html" | "htm" => Some(Self::Html),
            "html-color" | "color-html" | "color" => Some(Self::HtmlColor),
            "html-mono" | "mono-html" | "mono" => Some(Self::HtmlMono),
            "text" | "txt" => Some(Self::Text),
            "xml" => Some(Self::Xml),
            "patch" | "diff" | "unified" | "unified-patch" => Some(Self::Patch),
            "context" | "context-patch" => Some(Self::ContextPatch),
            _ => None,
        }
    }

    /// Infers format from file extension.
    pub fn infer_from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(e) if e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm") => {
                Self::Html
            }
            Some(e) if e.eq_ignore_ascii_case("xml") => Self::Xml,
            Some(e) if e.eq_ignore_ascii_case("patch") || e.eq_ignore_ascii_case("diff") => {
                Self::Patch
            }
            _ => Self::Text,
        }
    }
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Html | Self::HtmlColor => write!(f, "html"),
            Self::HtmlMono => write!(f, "html-mono"),
            Self::Text => write!(f, "text"),
            Self::Xml => write!(f, "xml"),
            Self::Patch | Self::UnifiedPatch => write!(f, "patch"),
            Self::ContextPatch => write!(f, "context"),
        }
    }
}

/// Destination and format specification for generating a diff report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportSpec {
    /// Chosen report output format.
    pub format: ReportFormat,
    /// Destination filesystem path.
    pub path: PathBuf,
}

impl std::fmt::Display for ReportSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.format, self.path.display())
    }
}

impl ReportSpec {
    /// Parses a report argument of the form `<fmt:path>` or `<path>`.
    pub fn parse(s: &str) -> Result<Self, CliError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(CliError::InvalidArg(
                "--report argument cannot be empty".to_string(),
            ));
        }

        if let Some((fmt_str, path_str)) = trimmed.split_once(':')
            && let Some(format) = ReportFormat::parse_str(fmt_str)
        {
            if path_str.trim().is_empty() {
                return Err(CliError::InvalidArg(format!(
                    "missing file path in --report spec '{trimmed}'"
                )));
            }
            return Ok(Self {
                format,
                path: PathBuf::from(path_str.trim()),
            });
        }

        let path = PathBuf::from(trimmed);
        let format = ReportFormat::infer_from_path(&path);
        Ok(Self { format, path })
    }
}

/// Renders a diff report into a formatted string according to the specification.
pub fn render_report(spec: &ReportSpec, input: &ReportInput) -> String {
    match spec.format {
        ReportFormat::Html | ReportFormat::HtmlColor => {
            autocompare_reports::render_html(input, autocompare_reports::HtmlTheme::Color)
        }
        ReportFormat::HtmlMono => {
            autocompare_reports::render_html(input, autocompare_reports::HtmlTheme::Mono)
        }
        ReportFormat::Text => autocompare_reports::render_text(input),
        ReportFormat::Xml => autocompare_reports::render_xml(input),
        ReportFormat::Patch | ReportFormat::UnifiedPatch => {
            autocompare_reports::render_unified_patch(input)
        }
        ReportFormat::ContextPatch => autocompare_reports::render_context_patch(input),
    }
}

/// Formats and writes report document according to specification.
pub fn write_report(spec: &ReportSpec, input: &ReportInput) -> Result<(), CliError> {
    let rendered = render_report(spec, input);

    if let Some(parent) = spec.path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| CliError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }

    std::fs::write(&spec.path, rendered).map_err(|source| CliError::Io {
        path: spec.path.display().to_string(),
        source,
    })?;

    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use autocompare_reports::{DiffStats, Hunk, HunkLine};
    use tempfile::tempdir;

    fn sample_report_input() -> ReportInput {
        let lines = vec![
            HunkLine::unchanged("alpha", 1, 1),
            HunkLine::delete("beta_old", 2),
            HunkLine::insert("beta_new", 2),
            HunkLine::unchanged("gamma", 3, 3),
        ];
        let hunk = Hunk::new(1, 3, 1, 3, lines);
        let stats = DiffStats::new(1, 1, 0, 2);
        ReportInput::new("test_report".to_string(), vec![hunk], stats)
    }

    #[test]
    fn test_report_format_parse_str() {
        assert_eq!(ReportFormat::parse_str("html"), Some(ReportFormat::Html));
        assert_eq!(ReportFormat::parse_str("HTM"), Some(ReportFormat::Html));
        assert_eq!(
            ReportFormat::parse_str("html-color"),
            Some(ReportFormat::HtmlColor)
        );
        assert_eq!(
            ReportFormat::parse_str("color-html"),
            Some(ReportFormat::HtmlColor)
        );
        assert_eq!(
            ReportFormat::parse_str("color"),
            Some(ReportFormat::HtmlColor)
        );
        assert_eq!(
            ReportFormat::parse_str("html-mono"),
            Some(ReportFormat::HtmlMono)
        );
        assert_eq!(
            ReportFormat::parse_str("mono-html"),
            Some(ReportFormat::HtmlMono)
        );
        assert_eq!(
            ReportFormat::parse_str("mono"),
            Some(ReportFormat::HtmlMono)
        );
        assert_eq!(ReportFormat::parse_str("text"), Some(ReportFormat::Text));
        assert_eq!(ReportFormat::parse_str("txt"), Some(ReportFormat::Text));
        assert_eq!(ReportFormat::parse_str("xml"), Some(ReportFormat::Xml));
        assert_eq!(ReportFormat::parse_str("patch"), Some(ReportFormat::Patch));
        assert_eq!(ReportFormat::parse_str("diff"), Some(ReportFormat::Patch));
        assert_eq!(
            ReportFormat::parse_str("unified"),
            Some(ReportFormat::Patch)
        );
        assert_eq!(
            ReportFormat::parse_str("unified-patch"),
            Some(ReportFormat::Patch)
        );
        assert_eq!(
            ReportFormat::parse_str("context"),
            Some(ReportFormat::ContextPatch)
        );
        assert_eq!(
            ReportFormat::parse_str("context-patch"),
            Some(ReportFormat::ContextPatch)
        );
        assert_eq!(ReportFormat::parse_str("unknown_fmt"), None);
    }

    #[test]
    fn test_report_format_infer_from_path() {
        assert_eq!(
            ReportFormat::infer_from_path(Path::new("out.html")),
            ReportFormat::Html
        );
        assert_eq!(
            ReportFormat::infer_from_path(Path::new("out.htm")),
            ReportFormat::Html
        );
        assert_eq!(
            ReportFormat::infer_from_path(Path::new("out.xml")),
            ReportFormat::Xml
        );
        assert_eq!(
            ReportFormat::infer_from_path(Path::new("out.patch")),
            ReportFormat::Patch
        );
        assert_eq!(
            ReportFormat::infer_from_path(Path::new("out.diff")),
            ReportFormat::Patch
        );
        assert_eq!(
            ReportFormat::infer_from_path(Path::new("out.txt")),
            ReportFormat::Text
        );
        assert_eq!(
            ReportFormat::infer_from_path(Path::new("out")),
            ReportFormat::Text
        );
    }

    #[test]
    fn test_report_spec_display_and_parse() {
        let spec = ReportSpec::parse("html:out/report.html").expect("parse");
        assert_eq!(spec.format, ReportFormat::Html);
        assert_eq!(spec.path, PathBuf::from("out/report.html"));
        assert_eq!(format!("{spec}"), "html:out/report.html");

        let inferred = ReportSpec::parse("out/diff.patch").expect("parse");
        assert_eq!(inferred.format, ReportFormat::Patch);
        assert_eq!(inferred.path, PathBuf::from("out/diff.patch"));

        assert!(ReportSpec::parse("").is_err());
        assert!(ReportSpec::parse("html:").is_err());
    }

    #[test]
    fn test_render_and_write_all_report_formats() {
        let tmp = tempdir().expect("tempdir");
        let input = sample_report_input();

        let formats = [
            (ReportFormat::Html, "out.html"),
            (ReportFormat::HtmlColor, "out_color.html"),
            (ReportFormat::HtmlMono, "out_mono.html"),
            (ReportFormat::Text, "out.txt"),
            (ReportFormat::Xml, "out.xml"),
            (ReportFormat::Patch, "out.patch"),
            (ReportFormat::UnifiedPatch, "out_unified.patch"),
            (ReportFormat::ContextPatch, "out_context.patch"),
        ];

        for (fmt, filename) in formats {
            let nested_path = tmp.path().join("nested").join("sub").join(filename);
            let spec = ReportSpec {
                format: fmt,
                path: nested_path.clone(),
            };

            let rendered = render_report(&spec, &input);
            assert!(
                !rendered.is_empty(),
                "rendered string should not be empty for {fmt:?}"
            );

            write_report(&spec, &input).expect("write_report");
            assert!(
                nested_path.exists(),
                "report file should be written to {nested_path:?}"
            );
            let disk_content = std::fs::read_to_string(&nested_path).expect("read");
            assert_eq!(disk_content, rendered);
        }
    }
}
