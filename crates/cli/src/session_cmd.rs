//! Session loading and execution for autocompare CLI.
//!
//! Loads persisted compare session documents and dispatches comparison
//! runs based on tab configuration.

use std::path::Path;

use autocompare_sessions::{SessionDocument, SessionTab};

use crate::error::CliError;
use crate::outcome::{ExitCodeMode, Outcome};
use crate::report::{ReportSpec, write_report};
use crate::{
    render_audio, render_folder, render_foldermerge, render_hex, render_picture, render_sync,
    render_table, render_text,
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
                let mut input = crate::report::text_report_input(left, right, &cmp);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
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
                let mut input = crate::report::folder_report_input(left, right, &cmp);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
                write_report(spec, &input)?;
            }
            Ok(render_folder::outcome(&cmp))
        }
        "hex" => {
            let cmp =
                autocompare_compare::compare_binary_files(Path::new(left), Path::new(right), 50)?;
            render_hex::print(&cmp, color);
            if let Some(spec) = report {
                let mut input = crate::report::hex_report_input(left, right, &cmp);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
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
                let mut input = crate::report::table_report_input(left, right, &cmp);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
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
                let mut input = crate::report::picture_report_input(left, right, &cmp);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
                write_report(spec, &input)?;
            }
            Ok(render_picture::outcome(&cmp))
        }
        "audio" => {
            let cmp = autocompare_compare::compare_audio_files(Path::new(left), Path::new(right))?;
            render_audio::print(&cmp, color);
            if let Some(spec) = report {
                let mut input = crate::report::audio_report_input(left, right, &cmp);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
                write_report(spec, &input)?;
            }
            Ok(render_audio::outcome(&cmp))
        }
        "sync" => {
            let opts = autocompare_compare::SyncOptions::default();
            let plan = autocompare_compare::plan_sync(Path::new(left), Path::new(right), &opts)?;
            render_sync::print_plan(&plan, color);
            if let Some(spec) = report {
                let mut input = crate::report::sync_report_input(left, right, &plan);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
                write_report(spec, &input)?;
            }
            let diff = !plan.steps.is_empty();
            Ok(Outcome::for_diff(diff, ExitCodeMode::Diff))
        }
        "merge" => {
            let base = tab.base_path.as_deref().unwrap_or("");
            if base.is_empty() {
                return Err(CliError::InvalidArg(format!(
                    "merge session tab '{}' is missing base path",
                    tab.title
                )));
            }
            let out_dest = tab.output_path.as_deref();
            if tab.read_only && out_dest.is_some() {
                return Err(CliError::ReadOnly(
                    "cannot write merge in read-only session tab".into(),
                ));
            }
            let b_lines = autocompare_compare::load_lines(Path::new(base))?;
            let l_lines = autocompare_compare::load_lines(Path::new(left))?;
            let r_lines = autocompare_compare::load_lines(Path::new(right))?;
            let b_refs: Vec<&str> = b_lines.iter().map(String::as_str).collect();
            let l_refs: Vec<&str> = l_lines.iter().map(String::as_str).collect();
            let r_refs: Vec<&str> = r_lines.iter().map(String::as_str).collect();
            let merged = autocompare_diff::merge3(&b_refs, &l_refs, &r_refs);
            let conflicted = merged.iter().any(|line| line.starts_with("<<<<<<<"));
            crate::runner::write_merged(&merged, out_dest)?;
            if conflicted {
                eprintln!("autocompare: the merge has conflicts");
            }
            if let Some(spec) = report {
                let mut input = crate::report::merge_report_input(base, left, right, &merged);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
                write_report(spec, &input)?;
            }
            Ok(Outcome::for_merge(conflicted, ExitCodeMode::Diff))
        }
        "folder-merge" | "folder_merge" => {
            let base = tab.base_path.as_deref().unwrap_or("");
            if base.is_empty() {
                return Err(CliError::InvalidArg(format!(
                    "folder merge session tab '{}' is missing base path",
                    tab.title
                )));
            }
            let out_dest = tab.output_path.as_deref();
            if tab.read_only && out_dest.is_some() {
                return Err(CliError::ReadOnly(
                    "cannot apply folder merge in read-only session tab".into(),
                ));
            }
            let plan = autocompare_compare::plan_tree_merge(
                Path::new(base),
                Path::new(left),
                Path::new(right),
            )?;
            render_foldermerge::print_plan(&plan, color);
            if let Some(spec) = report {
                let mut input = crate::report::foldermerge_report_input(base, left, right, &plan);
                if !tab.title.is_empty() {
                    input.title = tab.title.clone();
                }
                write_report(spec, &input)?;
            }
            let diff = !plan.steps.is_empty() || !plan.skipped.is_empty();
            if let Some(out) = out_dest {
                let rep = autocompare_compare::apply_tree_merge(
                    Path::new(left),
                    Path::new(right),
                    &plan,
                    Path::new(out),
                )?;
                render_foldermerge::print_report(&rep, color);
                let failed_diff = !rep.failed.is_empty() || !plan.conflicts.is_empty();
                Ok(Outcome::for_merge(failed_diff, ExitCodeMode::Diff))
            } else {
                Ok(Outcome::for_merge(
                    !plan.conflicts.is_empty() || diff,
                    ExitCodeMode::Diff,
                ))
            }
        }
        other => Err(CliError::InvalidArg(format!(
            "unsupported session tab type '{other}'"
        ))),
    }
}

/// Saves current CLI parameters into a session file, supporting 3-way merge paths.
pub fn save_session_with_merge_paths(
    path: &str,
    session_type: &str,
    base: Option<&str>,
    left: &str,
    right: &str,
    output: Option<&str>,
    read_only: bool,
) -> Result<(), CliError> {
    let title = if let Some(b) = base {
        format!("{b} vs {left} vs {right}")
    } else {
        format!("{left} vs {right}")
    };
    let mut tab = SessionTab::new("tab-1", title)
        .with_paths(left, right)
        .with_session_type(session_type);
    tab.base_path = base.map(ToString::to_string);
    tab.output_path = output.map(ToString::to_string);
    tab.read_only = read_only;

    let doc = SessionDocument::with_tabs(vec![tab]);
    doc.save_to_file(path)?;
    Ok(())
}

/// Saves current CLI parameters into a session file.
pub fn save_session(
    path: &str,
    session_type: &str,
    left: &str,
    right: &str,
    read_only: bool,
) -> Result<(), CliError> {
    save_session_with_merge_paths(path, session_type, None, left, right, None, read_only)
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
    use tempfile::tempdir;

    #[test]
    fn test_session_save_and_reload_all_types() {
        let tmp = tempdir().expect("tempdir");
        let sess_file = tmp.path().join("all_types_session.json");
        let path_str = sess_file.to_str().expect("path");

        // 1. Text session
        let f1 = tmp.path().join("t1.txt");
        let f2 = tmp.path().join("t2.txt");
        std::fs::write(&f1, "hello\nworld\n").expect("write");
        std::fs::write(&f2, "hello\nearth\n").expect("write");

        save_session(
            path_str,
            "text",
            f1.to_str().expect("p"),
            f2.to_str().expect("p"),
            false,
        )
        .expect("save text");

        let outcome = load_and_run(path_str, false, None).expect("load text");
        assert_eq!(outcome.code(), 1);

        // 2. Merge session (clean 3-way merge: disjoint modifications)
        let base = tmp.path().join("base.txt");
        let left = tmp.path().join("left.txt");
        let right = tmp.path().join("right.txt");
        let out = tmp.path().join("merged.txt");
        std::fs::write(&base, "line 1\nline 2\nline 3\n").expect("write");
        std::fs::write(&left, "ours 1\nline 2\nline 3\n").expect("write");
        std::fs::write(&right, "line 1\nline 2\ntheirs 3\n").expect("write");

        save_session_with_merge_paths(
            path_str,
            "merge",
            Some(base.to_str().expect("p")),
            left.to_str().expect("p"),
            right.to_str().expect("p"),
            Some(out.to_str().expect("p")),
            false,
        )
        .expect("save merge");

        let m_outcome = load_and_run(path_str, false, None).expect("load merge");
        assert_eq!(m_outcome.code(), 0);
        let merged_content = std::fs::read_to_string(&out).expect("read");
        assert!(merged_content.contains("ours"));
        assert!(merged_content.contains("theirs"));

        // 3. Hex session
        let h1 = tmp.path().join("h1.bin");
        let h2 = tmp.path().join("h2.bin");
        std::fs::write(&h1, [0x01, 0x02, 0x03]).expect("write");
        std::fs::write(&h2, [0x01, 0x02, 0x03]).expect("write");
        save_session(
            path_str,
            "hex",
            h1.to_str().expect("p"),
            h2.to_str().expect("p"),
            false,
        )
        .expect("save hex");
        let hex_outcome = load_and_run(path_str, false, None).expect("load hex");
        assert_eq!(hex_outcome.code(), 0);

        // 4. Folder session
        let d1 = tmp.path().join("d1");
        let d2 = tmp.path().join("d2");
        std::fs::create_dir_all(&d1).expect("mkdir");
        std::fs::create_dir_all(&d2).expect("mkdir");
        save_session(
            path_str,
            "folder",
            d1.to_str().expect("p"),
            d2.to_str().expect("p"),
            false,
        )
        .expect("save folder");
        let folder_outcome = load_and_run(path_str, false, None).expect("load folder");
        assert_eq!(folder_outcome.code(), 0);
    }

    #[test]
    fn test_session_report_generation_with_tab_title() {
        let tmp = tempdir().expect("tempdir");
        let sess_file = tmp.path().join("report_sess.json");
        let rep_file = tmp.path().join("report.html");

        let f1 = tmp.path().join("r1.txt");
        let f2 = tmp.path().join("r2.txt");
        std::fs::write(&f1, "sample text line\n").expect("write");
        std::fs::write(&f2, "sample text line\n").expect("write");

        let mut tab = SessionTab::new("tab-custom", "Custom Session Tab Title")
            .with_paths(f1.to_str().expect("p"), f2.to_str().expect("p"))
            .with_session_type("text");
        tab.title = "Custom Session Tab Title".to_string();
        let doc = SessionDocument::with_tabs(vec![tab]);
        doc.save_to_file(sess_file.to_str().expect("p"))
            .expect("save");

        let spec_str = format!("html:{}", rep_file.display());
        let spec = ReportSpec::parse(&spec_str).expect("parse spec");
        let outcome =
            load_and_run(sess_file.to_str().expect("p"), false, Some(&spec)).expect("load and run");
        assert_eq!(outcome.code(), 0);

        let report_html = std::fs::read_to_string(&rep_file).expect("read rep");
        assert!(report_html.contains("Custom Session Tab Title"));
    }
}
