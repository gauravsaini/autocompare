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
            "html-color" | "color-html" => Some(Self::HtmlColor),
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

/// Destination and format specification for generating a diff report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportSpec {
    /// Chosen report output format.
    pub format: ReportFormat,
    /// Destination filesystem path.
    pub path: PathBuf,
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
