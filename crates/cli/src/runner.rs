//! Command dispatcher and executor for autocompare CLI.
//!
//! Executes the requested comparison or merge subcommand and dispatches
//! report generation and session persistence.

use std::path::{Path, PathBuf};

use crate::args::{Cli, Command, SyncDir};
use crate::error::CliError;
use crate::outcome::Outcome;
use crate::report::{ReportSpec, write_report};
use crate::{
    render_audio, render_folder, render_foldermerge, render_hex, render_picture, render_sync,
    render_table, render_text, render_text_unified, session_cmd, term,
};

/// Runs the parsed subcommand with active flags.
pub fn run(cli: &Cli, color: bool, report: Option<&ReportSpec>) -> Result<Outcome, CliError> {
    let Some(ref command) = cli.command else {
        return Err(CliError::InvalidArg(
            "no comparison command or input files specified".to_string(),
        ));
    };

    let mode = cli.effective_exit_code_mode();
    let is_ro = cli.is_read_only();

    match command {
        Command::Text {
            left,
            right,
            ignore_case,
            ignore_space_change,
            unified,
            width,
        } => {
            let opts = autocompare_compare::TextOptions {
                ignore_case: *ignore_case,
                ignore_space_change: *ignore_space_change,
            };
            let cmp =
                autocompare_compare::compare_text_files(Path::new(left), Path::new(right), &opts)?;
            if *unified {
                render_text_unified::unified(&cmp, color);
            } else {
                render_text::side_by_side(&cmp, *width, color);
            }
            if let Some(spec) = report {
                let mut input = crate::report::text_report_input(left, right, &cmp);
                if let Some(title) = resolve_report_title(cli, left, right, None) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab(cli.save_session.as_deref(), "text", left, right, is_ro)?;
            let raw_outcome = render_text::outcome(&cmp);
            Ok(raw_outcome.to_mode(mode))
        }
        Command::Folder {
            left,
            right,
            content,
            ignore_case,
            depth,
            include,
            exclude,
            recursive: _,
        } => {
            let opts = autocompare_compare::FolderOptions {
                content_compare: *content,
                depth_limit: *depth,
                case_insensitive: *ignore_case,
                include: include.clone(),
                exclude: exclude.clone(),
            };
            let cmp =
                autocompare_compare::compare_folders(Path::new(left), Path::new(right), &opts)?;
            render_folder::print(&cmp, color);
            if let Some(spec) = report {
                let mut input = crate::report::folder_report_input(left, right, &cmp);
                if let Some(title) = resolve_report_title(cli, left, right, None) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab(cli.save_session.as_deref(), "folder", left, right, is_ro)?;
            let raw_outcome = render_folder::outcome(&cmp);
            Ok(raw_outcome.to_mode(mode))
        }
        Command::Hex {
            left,
            right,
            max_hunks,
        } => {
            let cmp = autocompare_compare::compare_binary_files(
                Path::new(left),
                Path::new(right),
                *max_hunks,
            )?;
            let is_diff = render_hex::outcome(&cmp).is_diff();
            render_hex::print(&cmp, color);
            if let Some(spec) = report {
                let mut input = crate::report::hex_report_input(left, right, &cmp);
                if let Some(title) = resolve_report_title(cli, left, right, None) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab(cli.save_session.as_deref(), "hex", left, right, is_ro)?;
            Ok(Outcome::for_binary(is_diff, mode))
        }
        Command::Merge {
            base,
            left,
            right,
            output,
        } => {
            let out_dest = cli.mergeoutput.as_ref().or(output.as_ref());
            if (is_ro || cli.ro4) && out_dest.is_some() {
                return Err(CliError::ReadOnly(
                    "cannot write merge in read-only mode".into(),
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
            write_merged(&merged, out_dest.map(|s| s.as_str()))?;
            if conflicted {
                eprintln!("autocompare: the merge has conflicts");
            }
            if let Some(spec) = report {
                let mut input = crate::report::merge_report_input(base, left, right, &merged);
                if let Some(title) = resolve_report_title(cli, left, right, Some(base)) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab_with_merge(
                cli.save_session.as_deref(),
                "merge",
                Some(base),
                left,
                right,
                out_dest.map(|s| s.as_str()),
                is_ro,
            )?;
            Ok(Outcome::for_merge(conflicted, mode))
        }
        Command::Table {
            left,
            right,
            delimiter,
            no_header,
        } => {
            let opts = autocompare_compare::TableOptions {
                delimiter: *delimiter,
                header: !no_header,
            };
            let cmp =
                autocompare_compare::compare_table_files(Path::new(left), Path::new(right), &opts)?;
            render_table::print(&cmp, color);
            if let Some(spec) = report {
                let mut input = crate::report::table_report_input(left, right, &cmp);
                if let Some(title) = resolve_report_title(cli, left, right, None) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab(cli.save_session.as_deref(), "table", left, right, is_ro)?;
            let raw_outcome = render_table::outcome(&cmp);
            Ok(raw_outcome.to_mode(mode))
        }
        Command::Sync {
            left,
            right,
            direction,
            prune,
            apply,
        } => {
            if is_ro && *apply {
                return Err(CliError::ReadOnly(
                    "cannot apply sync in read-only mode".into(),
                ));
            }
            let dir = match direction {
                SyncDir::L2r => autocompare_compare::SyncDirection::LeftToRight,
                SyncDir::R2l => autocompare_compare::SyncDirection::RightToLeft,
                SyncDir::Newer => autocompare_compare::SyncDirection::NewerWins,
            };
            let opts = autocompare_compare::SyncOptions {
                direction: dir,
                prune: *prune,
            };
            let plan = autocompare_compare::plan_sync(Path::new(left), Path::new(right), &opts)?;
            render_sync::print_plan(&plan, color);
            if let Some(spec) = report {
                let mut input = crate::report::sync_report_input(left, right, &plan);
                if let Some(title) = resolve_report_title(cli, left, right, None) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab(cli.save_session.as_deref(), "sync", left, right, is_ro)?;
            if !apply {
                println!(
                    "{}",
                    term::paint(
                        color,
                        term::DIM,
                        "dry run: nothing changed (pass --apply to run)"
                    )
                );
                let diff = !plan.steps.is_empty();
                Ok(Outcome::for_diff(diff, mode))
            } else {
                let rep =
                    autocompare_compare::apply_sync(Path::new(left), Path::new(right), &plan)?;
                render_sync::print_report(&rep, color);
                let diff = !rep.failed.is_empty();
                Ok(Outcome::for_diff(diff, mode))
            }
        }
        Command::Picture {
            left,
            right,
            heatmap,
        } => {
            let opts = autocompare_compare::PictureOptions {
                heatmap: heatmap.as_deref().map(PathBuf::from),
            };
            let cmp = autocompare_compare::compare_picture_files(
                Path::new(left),
                Path::new(right),
                &opts,
            )?;
            render_picture::print(&cmp, color, heatmap.as_deref());
            if let Some(spec) = report {
                let mut input = crate::report::picture_report_input(left, right, &cmp);
                if let Some(title) = resolve_report_title(cli, left, right, None) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab(cli.save_session.as_deref(), "picture", left, right, is_ro)?;
            let raw_outcome = render_picture::outcome(&cmp);
            Ok(raw_outcome.to_mode(mode))
        }
        Command::FolderMerge {
            base,
            left,
            right,
            output,
            apply,
        } => {
            let out_dest = cli.mergeoutput.as_ref().or(output.as_ref());
            let should_apply = *apply || (cli.automerge && out_dest.is_some());
            if (is_ro || cli.ro4) && should_apply {
                return Err(CliError::ReadOnly(
                    "cannot apply folder merge in read-only mode".into(),
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
                if let Some(title) = resolve_report_title(cli, left, right, Some(base)) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab_with_merge(
                cli.save_session.as_deref(),
                "folder-merge",
                Some(base),
                left,
                right,
                out_dest.map(|s| s.as_str()),
                is_ro,
            )?;
            if !should_apply {
                println!(
                    "{}",
                    term::paint(
                        color,
                        term::DIM,
                        "dry run: nothing written (pass --apply and --output to run)"
                    )
                );
                let diff = !plan.steps.is_empty() || !plan.skipped.is_empty();
                Ok(Outcome::for_merge(!plan.conflicts.is_empty() || diff, mode))
            } else {
                let Some(out) = out_dest else {
                    return Err(CliError::InvalidArg(
                        "--apply needs an output directory".into(),
                    ));
                };
                let rep = autocompare_compare::apply_tree_merge(
                    Path::new(left),
                    Path::new(right),
                    &plan,
                    Path::new(out),
                )?;
                render_foldermerge::print_report(&rep, color);
                let diff = !rep.failed.is_empty() || !plan.conflicts.is_empty();
                Ok(Outcome::for_merge(diff, mode))
            }
        }
        Command::Audio { left, right } => {
            let cmp = autocompare_compare::compare_audio_files(Path::new(left), Path::new(right))?;
            render_audio::print(&cmp, color);
            if let Some(spec) = report {
                let mut input = crate::report::audio_report_input(left, right, &cmp);
                if let Some(title) = resolve_report_title(cli, left, right, None) {
                    input.title = title;
                }
                write_report(spec, &input)?;
            }
            save_tab(cli.save_session.as_deref(), "audio", left, right, is_ro)?;
            let raw_outcome = render_audio::outcome(&cmp);
            Ok(raw_outcome.to_mode(mode))
        }
        Command::Session { path } => {
            let outcome = session_cmd::load_and_run(path, color, report)?;
            save_tab(cli.save_session.as_deref(), "session", path, path, is_ro)?;
            Ok(outcome.to_mode(mode))
        }
    }
}

/// Resolves effective report title from explicit report_title, VCS titles (title1..4), or default.
fn resolve_report_title(
    cli: &Cli,
    default_left: &str,
    default_right: &str,
    base: Option<&str>,
) -> Option<String> {
    if let Some(ref t) = cli.report_title {
        return Some(t.clone());
    }
    match (base, &cli.title3) {
        (Some(b_def), _)
            if cli.title1.is_some() || cli.title2.is_some() || cli.title3.is_some() =>
        {
            let b_title = cli.title3.as_deref().unwrap_or(b_def);
            let l_title = cli.title1.as_deref().unwrap_or(default_left);
            let r_title = cli.title2.as_deref().unwrap_or(default_right);
            Some(format!("{b_title} <-> {l_title} <-> {r_title}"))
        }
        _ => match (&cli.title1, &cli.title2) {
            (Some(t1), Some(t2)) => Some(format!("{t1} <-> {t2}")),
            (Some(t1), None) => Some(format!("{t1} <-> {default_right}")),
            (None, Some(t2)) => Some(format!("{default_left} <-> {t2}")),
            (None, None) => None,
        },
    }
}

fn save_tab(
    path: Option<&str>,
    stype: &str,
    left: &str,
    right: &str,
    ro: bool,
) -> Result<(), CliError> {
    save_tab_with_merge(path, stype, None, left, right, None, ro)
}

fn save_tab_with_merge(
    path: Option<&str>,
    stype: &str,
    base: Option<&str>,
    left: &str,
    right: &str,
    output: Option<&str>,
    ro: bool,
) -> Result<(), CliError> {
    if let Some(p) = path {
        session_cmd::save_session_with_merge_paths(p, stype, base, left, right, output, ro)?;
    }
    Ok(())
}

pub(crate) fn write_merged(merged: &[String], output: Option<&str>) -> Result<(), CliError> {
    let mut text = merged.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    match output {
        Some(path) => std::fs::write(path, text).map_err(|source| CliError::Io {
            path: path.to_string(),
            source,
        }),
        None => {
            if !text.is_empty() {
                print!("{text}");
            }
            Ok(())
        }
    }
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
    use crate::args::ExitCodeMode;
    use tempfile::tempdir;

    fn make_test_cli(cmd: Command) -> Cli {
        Cli {
            color: crate::args::ColorWhen::Never,
            report: None,
            ro: false,
            ro1: false,
            ro2: false,
            ro3: false,
            ro4: false,
            automerge: false,
            mergeoutput: None,
            vcs1: None,
            vcs2: None,
            vcs3: None,
            vcs4: None,
            title1: None,
            title2: None,
            title3: None,
            title4: None,
            session: None,
            save_session: None,
            exit_code_mode: ExitCodeMode::Diff,
            bc_exit_code: false,
            report_title: None,
            solo: false,
            nobackups: false,
            reviewconflicts: false,
            command: Some(cmd),
        }
    }

    #[test]
    fn test_runner_text_exit_codes_both_modes() {
        let tmp = tempdir().expect("tempdir");
        let same1 = tmp.path().join("same1.txt");
        let same2 = tmp.path().join("same2.txt");
        let diff = tmp.path().join("diff.txt");

        std::fs::write(&same1, "line 1\nline 2\n").expect("write");
        std::fs::write(&same2, "line 1\nline 2\n").expect("write");
        std::fs::write(&diff, "line 1\nchanged 2\n").expect("write");

        // Identical in Diff mode -> 0
        let cli_same = make_test_cli(Command::Text {
            left: same1.to_str().expect("p").to_string(),
            right: same2.to_str().expect("p").to_string(),
            ignore_case: false,
            ignore_space_change: false,
            unified: false,
            width: 80,
        });
        let res_same = run(&cli_same, false, None).expect("run");
        assert_eq!(res_same.code(), 0);

        // Differing in Diff mode -> 1
        let cli_diff = make_test_cli(Command::Text {
            left: same1.to_str().expect("p").to_string(),
            right: diff.to_str().expect("p").to_string(),
            ignore_case: false,
            ignore_space_change: false,
            unified: false,
            width: 80,
        });
        let res_diff = run(&cli_diff, false, None).expect("run");
        assert_eq!(res_diff.code(), 1);

        // Identical in BC mode -> 0
        let mut cli_same_bc = cli_same;
        cli_same_bc.bc_exit_code = true;
        let res_same_bc = run(&cli_same_bc, false, None).expect("run");
        assert_eq!(res_same_bc.code(), 0);

        // Differing in BC mode -> 13 (Rules diff)
        let mut cli_diff_bc = cli_diff;
        cli_diff_bc.bc_exit_code = true;
        let res_diff_bc = run(&cli_diff_bc, false, None).expect("run");
        assert_eq!(res_diff_bc.code(), 13);
    }

    #[test]
    fn test_runner_hex_exit_codes_both_modes() {
        let tmp = tempdir().expect("tempdir");
        let h1 = tmp.path().join("h1.bin");
        let h2 = tmp.path().join("h2.bin");
        let h3 = tmp.path().join("h3.bin");

        std::fs::write(&h1, [0xAA, 0xBB, 0xCC]).expect("write");
        std::fs::write(&h2, [0xAA, 0xBB, 0xCC]).expect("write");
        std::fs::write(&h3, [0xAA, 0xFF, 0xCC]).expect("write");

        // Identical -> 0
        let cli_same = make_test_cli(Command::Hex {
            left: h1.to_str().expect("p").to_string(),
            right: h2.to_str().expect("p").to_string(),
            max_hunks: 50,
        });
        let res_same = run(&cli_same, false, None).expect("run");
        assert_eq!(res_same.code(), 0);

        // Differing in Diff mode -> 1
        let cli_diff = make_test_cli(Command::Hex {
            left: h1.to_str().expect("p").to_string(),
            right: h3.to_str().expect("p").to_string(),
            max_hunks: 50,
        });
        let res_diff = run(&cli_diff, false, None).expect("run");
        assert_eq!(res_diff.code(), 1);

        // Differing in BC mode -> 11 (Binary diff)
        let mut cli_diff_bc = cli_diff;
        cli_diff_bc.bc_exit_code = true;
        let res_diff_bc = run(&cli_diff_bc, false, None).expect("run");
        assert_eq!(res_diff_bc.code(), 11);
    }

    #[test]
    fn test_runner_merge_clean_conflicted_and_readonly_guard() {
        let tmp = tempdir().expect("tempdir");
        let base = tmp.path().join("base.txt");
        let left = tmp.path().join("left.txt");
        let right_clean = tmp.path().join("right_clean.txt");
        let right_conflict = tmp.path().join("right_conflict.txt");
        let out = tmp.path().join("out.txt");

        std::fs::write(&base, "base line\n").expect("write");
        std::fs::write(&left, "left line\n").expect("write");
        std::fs::write(&right_clean, "base line\n").expect("write");
        std::fs::write(&right_conflict, "conflict line\n").expect("write");

        // Clean merge -> 0
        let cli_clean = make_test_cli(Command::Merge {
            base: base.to_str().expect("p").to_string(),
            left: left.to_str().expect("p").to_string(),
            right: right_clean.to_str().expect("p").to_string(),
            output: Some(out.to_str().expect("p").to_string()),
        });
        let res_clean = run(&cli_clean, false, None).expect("run");
        assert_eq!(res_clean.code(), 0);

        // Conflicted merge in Diff mode -> 1
        let cli_conflict = make_test_cli(Command::Merge {
            base: base.to_str().expect("p").to_string(),
            left: left.to_str().expect("p").to_string(),
            right: right_conflict.to_str().expect("p").to_string(),
            output: Some(out.to_str().expect("p").to_string()),
        });
        let res_conflict = run(&cli_conflict, false, None).expect("run");
        assert_eq!(res_conflict.code(), 1);

        // Conflicted merge in BC mode -> 14
        let mut cli_conflict_bc = cli_conflict.clone();
        cli_conflict_bc.bc_exit_code = true;
        let res_conflict_bc = run(&cli_conflict_bc, false, None).expect("run");
        assert_eq!(res_conflict_bc.code(), 14);

        // Read-only guard -> error (exit code 2)
        let mut cli_ro = cli_clean;
        cli_ro.ro = true;
        let err = run(&cli_ro, false, None).unwrap_err();
        assert!(matches!(err, CliError::ReadOnly(_)));
        assert_eq!(err.exit_code(), std::process::ExitCode::from(2));

        // Read-only ro4 guard -> error (exit code 2)
        let mut cli_ro4 = cli_conflict;
        cli_ro4.ro4 = true;
        let err4 = run(&cli_ro4, false, None).unwrap_err();
        assert!(matches!(err4, CliError::ReadOnly(_)));
        assert_eq!(err4.exit_code(), std::process::ExitCode::from(2));
    }

    #[test]
    fn test_runner_report_wiring_with_title_override_and_vcs_titles() {
        let tmp = tempdir().expect("tempdir");
        let left = tmp.path().join("left.txt");
        let right = tmp.path().join("right.txt");
        let rep_html = tmp.path().join("report.html");

        std::fs::write(&left, "alpha\n").expect("write");
        std::fs::write(&right, "beta\n").expect("write");

        let mut cli = make_test_cli(Command::Text {
            left: left.to_str().expect("p").to_string(),
            right: right.to_str().expect("p").to_string(),
            ignore_case: false,
            ignore_space_change: false,
            unified: false,
            width: 80,
        });

        // 1. VCS title1 and title2 propagation
        cli.title1 = Some("Branch-A (v1.0)".to_string());
        cli.title2 = Some("Branch-B (v2.0)".to_string());
        let spec = ReportSpec::parse(&format!("html:{}", rep_html.display())).expect("parse");
        run(&cli, false, Some(&spec)).expect("run");
        let html_content = std::fs::read_to_string(&rep_html).expect("read");
        assert!(html_content.contains("Branch-A (v1.0)"));
        assert!(html_content.contains("Branch-B (v2.0)"));

        // 2. Explicit report_title override takes precedence
        cli.report_title = Some("Overridden Unified Audit Report".to_string());
        run(&cli, false, Some(&spec)).expect("run");
        let html_content_override = std::fs::read_to_string(&rep_html).expect("read");
        assert!(html_content_override.contains("Overridden Unified Audit Report"));
    }
}
