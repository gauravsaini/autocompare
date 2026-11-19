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
                write_report(spec, &crate::report::text_report_input(left, right, &cmp))?;
            }
            save_tab(cli.save_session.as_deref(), "text", left, right, cli.ro)?;
            Ok(render_text::outcome(&cmp))
        }
        Command::Folder {
            left,
            right,
            content,
            ignore_case,
            depth,
            include,
            exclude,
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
                write_report(spec, &crate::report::folder_report_input(left, right, &cmp))?;
            }
            save_tab(cli.save_session.as_deref(), "folder", left, right, cli.ro)?;
            Ok(render_folder::outcome(&cmp))
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
            render_hex::print(&cmp, color);
            if let Some(spec) = report {
                write_report(spec, &crate::report::hex_report_input(left, right, &cmp))?;
            }
            save_tab(cli.save_session.as_deref(), "hex", left, right, cli.ro)?;
            Ok(render_hex::outcome(&cmp))
        }
        Command::Merge {
            base,
            left,
            right,
            output,
        } => {
            let out_dest = cli.mergeoutput.as_ref().or(output.as_ref());
            if cli.ro && out_dest.is_some() {
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
                write_report(
                    spec,
                    &crate::report::merge_report_input(base, left, right, &merged),
                )?;
            }
            Ok(if conflicted {
                Outcome::Different
            } else {
                Outcome::Same
            })
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
                write_report(spec, &crate::report::table_report_input(left, right, &cmp))?;
            }
            save_tab(cli.save_session.as_deref(), "table", left, right, cli.ro)?;
            Ok(render_table::outcome(&cmp))
        }
        Command::Sync {
            left,
            right,
            direction,
            prune,
            apply,
        } => {
            if cli.ro && *apply {
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
                write_report(spec, &crate::report::sync_report_input(left, right, &plan))?;
            }
            if !apply {
                println!(
                    "{}",
                    term::paint(
                        color,
                        term::DIM,
                        "dry run: nothing changed (pass --apply to run)"
                    )
                );
                Ok(if plan.steps.is_empty() {
                    Outcome::Same
                } else {
                    Outcome::Different
                })
            } else {
                let rep =
                    autocompare_compare::apply_sync(Path::new(left), Path::new(right), &plan)?;
                render_sync::print_report(&rep, color);
                Ok(if rep.failed.is_empty() {
                    Outcome::Same
                } else {
                    Outcome::Different
                })
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
                write_report(
                    spec,
                    &crate::report::picture_report_input(left, right, &cmp),
                )?;
            }
            save_tab(cli.save_session.as_deref(), "picture", left, right, cli.ro)?;
            Ok(render_picture::outcome(&cmp))
        }
        Command::FolderMerge {
            base,
            left,
            right,
            output,
            apply,
        } => {
            let out_dest = cli.mergeoutput.as_ref().or(output.as_ref());
            if cli.ro && *apply {
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
                write_report(
                    spec,
                    &crate::report::foldermerge_report_input(base, left, right, &plan),
                )?;
            }
            if !apply {
                println!(
                    "{}",
                    term::paint(
                        color,
                        term::DIM,
                        "dry run: nothing written (pass --apply and --output to run)"
                    )
                );
                Ok(if plan.steps.is_empty() && plan.skipped.is_empty() {
                    Outcome::Same
                } else {
                    Outcome::Different
                })
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
                Ok(if rep.failed.is_empty() && plan.conflicts.is_empty() {
                    Outcome::Same
                } else {
                    Outcome::Different
                })
            }
        }
        Command::Audio { left, right } => {
            let cmp = autocompare_compare::compare_audio_files(Path::new(left), Path::new(right))?;
            render_audio::print(&cmp, color);
            if let Some(spec) = report {
                write_report(spec, &crate::report::audio_report_input(left, right, &cmp))?;
            }
            save_tab(cli.save_session.as_deref(), "audio", left, right, cli.ro)?;
            Ok(render_audio::outcome(&cmp))
        }
    }
}

fn save_tab(
    path: Option<&str>,
    stype: &str,
    left: &str,
    right: &str,
    ro: bool,
) -> Result<(), CliError> {
    if let Some(p) = path {
        session_cmd::save_session(p, stype, left, right, ro)?;
    }
    Ok(())
}

fn write_merged(merged: &[String], output: Option<&str>) -> Result<(), CliError> {
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
