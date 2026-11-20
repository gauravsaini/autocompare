//! Session loading and execution for autocompare CLI.
//!
//! Loads persisted compare session documents and dispatches comparison
//! runs based on tab configuration.

use std::path::Path;

use autocompare_sessions::{SessionDocument, SessionTab};

use crate::error::CliError;
use crate::outcome::Outcome;
use crate::report::{write_report, ReportSpec};
use crate::{
    render_audio, render_folder, render_hex, render_picture, render_sync, render_table, render_text,
};

/// Loads a session document from disk and runs comparison for the active tab.
pub fn load_and_run(
    session_path: &str,
    color: bool,
    report: Option<&ReportSpec>,
) -> Result<Outcome, CliError> {
    let doc = SessionDocument::load_from_file(session_path)?;
    let tab = doc
        .tabs
        .get(doc.active_tab_index)
        .or_else(|| doc.tabs.first())
        .ok_or_else(|| {
            CliError::InvalidArg(format!(
                "session '{session_path}' does not contain any tabs"
            ))
        })?;

    let left = tab.left_path.as_deref().unwrap_or("");
    let right = tab.right_path.as_deref().unwrap_or("");

    if left.is_empty() || right.is_empty() {
        return Err(CliError::InvalidArg(format!(
            "session tab '{}' is missing left or right path",
            tab.title
        )));
    }

    match tab.session_type.as_str() {
        "text" => {
            let opts = autocompare_compare::TextOptions::default();
            let cmp =
                autocompare_compare::compare_text_files(Path::new(left), Path::new(right), &opts)?;
            render_text::side_by_side(&cmp, 80, color);
            if let Some(spec) = report {
                let input = crate::report::text_report_input(left, right, &cmp);
                write_report(spec, &input)?;
            }
            Ok(render_text::outcome(&cmp))
        }
        "folder" => {
            let opts = autocompare_compare::FolderOptions::default();
            let cmp =
                autocompare_compare::compare_folders(Path::new(left), Path::new(right), &opts)?;
            render_folder::print(&cmp, color);
            if let Some(spec) = report {
                let input = crate::report::folder_report_input(left, right, &cmp);
                write_report(spec, &input)?;
            }
            Ok(render_folder::outcome(&cmp))
        }
        "hex" => {
            let cmp =
                autocompare_compare::compare_binary_files(Path::new(left), Path::new(right), 50)?;
            render_hex::print(&cmp, color);
            if let Some(spec) = report {
                let input = crate::report::hex_report_input(left, right, &cmp);
                write_report(spec, &input)?;
            }
            Ok(render_hex::outcome(&cmp))
        }
        "table" => {
            let opts = autocompare_compare::TableOptions::default();
            let cmp =
                autocompare_compare::compare_table_files(Path::new(left), Path::new(right), &opts)?;
            render_table::print(&cmp, color);
            if let Some(spec) = report {
                let input = crate::report::table_report_input(left, right, &cmp);
                write_report(spec, &input)?;
            }
            Ok(render_table::outcome(&cmp))
        }
        "picture" => {
            let opts = autocompare_compare::PictureOptions::default();
            let cmp = autocompare_compare::compare_picture_files(
                Path::new(left),
                Path::new(right),
                &opts,
            )?;
            render_picture::print(&cmp, color, None);
            if let Some(spec) = report {
                let input = crate::report::picture_report_input(left, right, &cmp);
                write_report(spec, &input)?;
            }
            Ok(render_picture::outcome(&cmp))
        }
        "audio" => {
            let cmp = autocompare_compare::compare_audio_files(Path::new(left), Path::new(right))?;
            render_audio::print(&cmp, color);
            if let Some(spec) = report {
                let input = crate::report::audio_report_input(left, right, &cmp);
                write_report(spec, &input)?;
            }
            Ok(render_audio::outcome(&cmp))
        }
        "sync" => {
            let opts = autocompare_compare::SyncOptions::default();
            let plan = autocompare_compare::plan_sync(Path::new(left), Path::new(right), &opts)?;
            render_sync::print_plan(&plan, color);
            if let Some(spec) = report {
                let input = crate::report::sync_report_input(left, right, &plan);
                write_report(spec, &input)?;
            }
            let diff = !plan.steps.is_empty();
            Ok(Outcome::for_diff(diff, crate::outcome::ExitCodeMode::Diff))
        }
        other => Err(CliError::InvalidArg(format!(
            "unsupported session tab type '{other}'"
        ))),
    }
}

/// Saves current CLI parameters into a session file.
pub fn save_session(
    path: &str,
    session_type: &str,
    left: &str,
    right: &str,
    read_only: bool,
) -> Result<(), CliError> {
    let mut tab = SessionTab::new("tab-1", format!("{left} vs {right}"))
        .with_paths(left, right)
        .with_session_type(session_type);
    tab.read_only = read_only;

    let doc = SessionDocument::with_tabs(vec![tab]);
    doc.save_to_file(path)?;
    Ok(())
}
