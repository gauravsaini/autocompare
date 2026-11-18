//! autocompare: compare text, folders and bytes, and merge three ways.
//!
//! Headless-first: this binary is a thin renderer over the
//! `autocompare-compare` and `autocompare-diff` libraries.

#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod render_folder;
mod render_hex;
mod render_text;
mod term;

/// How a comparison ended, mapped to the process exit code:
/// 0 = no differences, 1 = differences found, 2 = error.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Same,
    Different,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Parser)]
#[command(
    name = "autocompare",
    version,
    about = "Compare text, folders and bytes, and merge three ways",
    disable_help_subcommand = true
)]
struct Cli {
    /// When to color output.
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, global = true)]
    color: ColorWhen,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare two text files line by line, side by side.
    Text {
        /// Left file.
        left: String,
        /// Right file.
        right: String,
        /// Ignore differences in letter case.
        #[arg(long)]
        ignore_case: bool,
        /// Ignore whitespace differences inside lines.
        #[arg(long)]
        ignore_space_change: bool,
        /// Print a unified diff instead of side-by-side.
        #[arg(long)]
        unified: bool,
        /// Total width of the side-by-side view.
        #[arg(long, default_value_t = 80)]
        width: usize,
    },
    /// Compare two folder trees recursively by size and modified time.
    Folder {
        /// Left folder.
        left: String,
        /// Right folder.
        right: String,
        /// Also compare file bytes, not only size and modified time.
        #[arg(long)]
        content: bool,
        /// Match file names case-insensitively.
        #[arg(long)]
        ignore_case: bool,
        /// Only walk this many levels deep.
        #[arg(long, default_value_t = 32)]
        depth: usize,
        /// Only compare names matching this simple glob ('*' and '?').
        #[arg(long)]
        include: Option<String>,
        /// Skip names matching this simple glob ('*' and '?').
        #[arg(long)]
        exclude: Option<String>,
    },
    /// Compare two files byte by byte.
    Hex {
        /// Left file.
        left: String,
        /// Right file.
        right: String,
        /// Show at most this many differing runs.
        #[arg(long, default_value_t = 50)]
        max_hunks: usize,
    },
    /// Merge two files that share a common base version.
    Merge {
        /// The common ancestor version.
        base: String,
        /// The left (ours) version.
        left: String,
        /// The right (theirs) version.
        right: String,
        /// Write the merge result to this file instead of standard output.
        #[arg(long)]
        output: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let color = term::color_enabled(cli.color);
    match run(cli.command, color) {
        Ok(Outcome::Same) => ExitCode::SUCCESS,
        Ok(Outcome::Different) => ExitCode::from(1),
        Err(err) => {
            eprintln!("autocompare: {err}");
            ExitCode::from(2)
        }
    }
}

fn run(command: Command, color: bool) -> autocompare_compare::Result<Outcome> {
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
                ignore_case,
                ignore_space_change,
            };
            let cmp = autocompare_compare::compare_text_files(
                Path::new(&left),
                Path::new(&right),
                &opts,
            )?;
            if unified {
                render_text::unified(&cmp, color);
            } else {
                render_text::side_by_side(&cmp, width, color);
            }
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
                content_compare: content,
                depth_limit: depth,
                case_insensitive: ignore_case,
                include,
                exclude,
            };
            let cmp =
                autocompare_compare::compare_folders(Path::new(&left), Path::new(&right), &opts)?;
            render_folder::print(&cmp, color);
            Ok(render_folder::outcome(&cmp))
        }
        Command::Hex {
            left,
            right,
            max_hunks,
        } => {
            let cmp = autocompare_compare::compare_binary_files(
                Path::new(&left),
                Path::new(&right),
                max_hunks,
            )?;
            render_hex::print(&cmp, color);
            Ok(render_hex::outcome(&cmp))
        }
        Command::Merge {
            base,
            left,
            right,
            output,
        } => {
            let base_lines = autocompare_compare::load_lines(Path::new(&base))?;
            let left_lines = autocompare_compare::load_lines(Path::new(&left))?;
            let right_lines = autocompare_compare::load_lines(Path::new(&right))?;
            let merged = autocompare_diff::merge3(
                &to_refs(&base_lines),
                &to_refs(&left_lines),
                &to_refs(&right_lines),
            );
            let conflicted = merged.iter().any(|line| line.starts_with("<<<<<<<"));
            write_merged(&merged, output.as_deref())?;
            if conflicted {
                eprintln!("autocompare: the merge has conflicts");
            }
            Ok(if conflicted {
                Outcome::Different
            } else {
                Outcome::Same
            })
        }
    }
}

fn to_refs(lines: &[String]) -> Vec<&str> {
    lines.iter().map(|line| line.as_str()).collect()
}

fn write_merged(merged: &[String], output: Option<&str>) -> autocompare_compare::Result<()> {
    let mut text = merged.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    match output {
        Some(path) => match std::fs::write(path, text) {
            Ok(()) => Ok(()),
            Err(source) => Err(autocompare_compare::Error::Io {
                path: path.to_string(),
                source,
            }),
        },
        None => {
            if !text.is_empty() {
                print!("{text}");
            }
            Ok(())
        }
    }
}
