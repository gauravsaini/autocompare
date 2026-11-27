//! CLI argument models and subcommands for autocompare.
//!
//! Defines the clap-derived command hierarchy supporting all 9 comparison
//! subcommands, VCS flags, and reporting parameters.

use clap::{Parser, Subcommand};

pub use crate::outcome::ExitCodeMode;

/// When to emit ANSI color sequences in output.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorWhen {
    /// Detect terminal capability automatically.
    #[default]
    Auto,
    /// Force ANSI color sequences on.
    Always,
    /// Suppress all ANSI color sequences.
    Never,
}

/// Direction for folder synchronization.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SyncDir {
    /// Copy left-side content over the right side.
    #[default]
    L2r,
    /// Copy right-side content over the left side.
    R2l,
    /// Copy whichever side is newer over the older one.
    Newer,
}

/// Command line options for autocompare and bcomp compat mode.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "autocompare",
    version,
    about = "Compare text, folders and bytes, and merge three ways",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// When to color output.
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, global = true)]
    pub color: ColorWhen,

    /// Generate comparison report: format:path (html, text, xml, patch).
    #[arg(long, global = true)]
    pub report: Option<String>,

    /// Read-only mode for VCS tools and sessions.
    #[arg(long, global = true)]
    pub ro: bool,

    /// Read-only mode for left input.
    #[arg(long, global = true)]
    pub ro1: bool,

    /// Read-only mode for right input.
    #[arg(long, global = true)]
    pub ro2: bool,

    /// Read-only mode for base / ancestor / center input.
    #[arg(long, global = true)]
    pub ro3: bool,

    /// Read-only mode for output input.
    #[arg(long, global = true)]
    pub ro4: bool,

    /// Automatically merge when there are no conflicts.
    #[arg(long, global = true)]
    pub automerge: bool,

    /// Output destination path for 3-way merge results.
    #[arg(long, global = true)]
    pub mergeoutput: Option<String>,

    /// VCS input 1 alias (typically left/local).
    #[arg(long, global = true)]
    pub vcs1: Option<String>,

    /// VCS input 2 alias (typically right/remote).
    #[arg(long, global = true)]
    pub vcs2: Option<String>,

    /// VCS input 3 alias (typically base/ancestor).
    #[arg(long, global = true)]
    pub vcs3: Option<String>,

    /// VCS input 4 alias (typically output/merged).
    #[arg(long, global = true)]
    pub vcs4: Option<String>,

    /// VCS title/label for left / first input.
    #[arg(long, global = true)]
    pub title1: Option<String>,

    /// VCS title/label for right / second input.
    #[arg(long, global = true)]
    pub title2: Option<String>,

    /// VCS title/label for center / base input.
    #[arg(long, global = true)]
    pub title3: Option<String>,

    /// VCS title/label for output input.
    #[arg(long, global = true)]
    pub title4: Option<String>,

    /// Path to a session document to load and execute.
    #[arg(long, global = true)]
    pub session: Option<String>,

    /// Save the current comparison session configuration to path.
    #[arg(long, global = true)]
    pub save_session: Option<String>,

    /// Exit code convention mode: diff (0/1/2) or bc (Beyond Compare return codes).
    #[arg(long, value_enum, default_value_t = ExitCodeMode::Diff, global = true)]
    pub exit_code_mode: ExitCodeMode,

    /// Explicit git-diff compatible exit code flag (0=equal, 1=diff, 2=error).
    #[arg(long, global = true)]
    pub exit_code: bool,

    /// Write CI-friendly JUnit XML report to the given path.
    #[arg(long, global = true)]
    pub junit: Option<String>,

    /// Force Beyond Compare return codes (0=success, 11=binary diff, 13=rules diff, 14=conflicts).
    #[arg(long, global = true)]
    pub bc_exit_code: bool,

    /// Title override for comparison reports.
    #[arg(long, global = true)]
    pub report_title: Option<String>,

    /// Run in a separate solo process without reusing background instances.
    #[arg(long, global = true)]
    pub solo: bool,

    /// Suppress backup file creation during merge/save operations.
    #[arg(long, global = true)]
    pub nobackups: bool,

    /// Automatically review conflicts in interactive mode if present.
    #[arg(long, global = true)]
    pub reviewconflicts: bool,

    /// Comparison subcommand to run.
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    /// Determines active exit code convention mode.
    pub fn effective_exit_code_mode(&self) -> ExitCodeMode {
        if self.bc_exit_code {
            ExitCodeMode::BeyondCompare
        } else {
            self.exit_code_mode
        }
    }

    /// Whether any read-only flag is active.
    pub fn is_read_only(&self) -> bool {
        self.ro || self.ro1 || self.ro2 || self.ro3 || self.ro4
    }

    /// Whether left/first input is marked read-only.
    pub fn is_left_read_only(&self) -> bool {
        self.ro || self.ro1
    }

    /// Whether right/second input is marked read-only.
    pub fn is_right_read_only(&self) -> bool {
        self.ro || self.ro2
    }

    /// Whether center/base input is marked read-only.
    pub fn is_base_read_only(&self) -> bool {
        self.ro || self.ro3
    }

    /// Whether output input is marked read-only.
    pub fn is_output_read_only(&self) -> bool {
        self.ro || self.ro4
    }
}

/// Comparison and merge subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Compare two text files line by line, side by side (alias: `diff`).
    #[command(alias = "diff")]
    Text {
        /// Left file path.
        left: String,
        /// Right file path.
        right: String,
        /// Ignore differences in letter case.
        #[arg(long, short = 'i')]
        ignore_case: bool,
        /// Ignore whitespace differences inside lines.
        #[arg(long, short = 'b')]
        ignore_space_change: bool,
        /// Print a unified diff instead of side-by-side.
        #[arg(long, short = 'u')]
        unified: bool,
        /// Total width of the side-by-side view.
        #[arg(long, short = 'w', default_value_t = 80)]
        width: usize,
    },
    /// Compare two folder trees recursively by size and modified time.
    Folder {
        /// Left folder path.
        left: String,
        /// Right folder path.
        right: String,
        /// Also compare file bytes, not only size and modified time.
        #[arg(long)]
        content: bool,
        /// Match file names case-insensitively.
        #[arg(long, short = 'i')]
        ignore_case: bool,
        /// Only walk this many levels deep.
        #[arg(long, short = 'd', default_value_t = 32)]
        depth: usize,
        /// Only compare names matching this simple glob ('*' and '?').
        #[arg(long)]
        include: Option<String>,
        /// Skip names matching this simple glob ('*' and '?').
        #[arg(long)]
        exclude: Option<String>,
        /// Explicit recursive walk flag (folder comparison is recursive by default).
        #[arg(long, short = 'r')]
        recursive: bool,
    },
    /// Compare two files byte by byte.
    Hex {
        /// Left file path.
        left: String,
        /// Right file path.
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
    /// Compare two delimited (CSV-style) tables cell by cell.
    Table {
        /// Left file path.
        left: String,
        /// Right file path.
        right: String,
        /// Field delimiter.
        #[arg(long, default_value_t = ',')]
        delimiter: char,
        /// Treat the first row as data, not a header.
        #[arg(long)]
        no_header: bool,
    },
    /// Plan (and optionally apply) a folder sync.
    Sync {
        /// Left folder path.
        left: String,
        /// Right folder path.
        right: String,
        /// Which side wins.
        #[arg(long, value_enum, default_value_t = SyncDir::L2r)]
        direction: SyncDir,
        /// Delete one-sided entries on the losing side.
        #[arg(long)]
        prune: bool,
        /// Execute the plan (default is a dry run).
        #[arg(long)]
        apply: bool,
    },
    /// Compare two PNG images pixel by pixel.
    Picture {
        /// Left file path.
        left: String,
        /// Right file path.
        right: String,
        /// Write a heatmap PNG to this path.
        #[arg(long)]
        heatmap: Option<String>,
    },
    /// Plan (and optionally apply) a three-way folder merge.
    #[command(name = "folder-merge")]
    FolderMerge {
        /// The common ancestor tree.
        base: String,
        /// The left (ours) tree.
        left: String,
        /// The right (theirs) tree.
        right: String,
        /// Directory to write the merged tree to (required with --apply).
        #[arg(long)]
        output: Option<String>,
        /// Execute the plan into --output (default is a dry run).
        #[arg(long)]
        apply: bool,
    },
    /// Compare two audio files (WAV, MP3) sample by sample.
    Audio {
        /// Left file path.
        left: String,
        /// Right file path.
        right: String,
    },
    /// Load and run a saved comparison session.
    Session {
        /// Path to the session document.
        path: String,
    },
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

    #[test]
    fn test_cli_read_only_flags() {
        let mut cli = Cli::try_parse_from(["autocompare"]).expect("parse");
        assert!(!cli.is_read_only());
        assert!(!cli.is_left_read_only());
        assert!(!cli.is_right_read_only());
        assert!(!cli.is_base_read_only());
        assert!(!cli.is_output_read_only());

        cli.ro = true;
        assert!(cli.is_read_only());
        assert!(cli.is_left_read_only());
        assert!(cli.is_right_read_only());
        assert!(cli.is_base_read_only());
        assert!(cli.is_output_read_only());

        cli.ro = false;
        cli.ro1 = true;
        assert!(cli.is_read_only());
        assert!(cli.is_left_read_only());
        assert!(!cli.is_right_read_only());

        cli.ro1 = false;
        cli.ro3 = true;
        assert!(cli.is_read_only());
        assert!(cli.is_base_read_only());

        cli.ro3 = false;
        cli.ro4 = true;
        assert!(cli.is_read_only());
        assert!(cli.is_output_read_only());
    }

    #[test]
    fn test_cli_effective_exit_code_mode() {
        let mut cli = Cli::try_parse_from(["autocompare"]).expect("parse");
        assert_eq!(cli.effective_exit_code_mode(), ExitCodeMode::Diff);

        cli.bc_exit_code = true;
        assert_eq!(cli.effective_exit_code_mode(), ExitCodeMode::BeyondCompare);

        cli.bc_exit_code = false;
        cli.exit_code_mode = ExitCodeMode::BeyondCompare;
        assert_eq!(cli.effective_exit_code_mode(), ExitCodeMode::BeyondCompare);
    }

    #[test]
    fn test_cli_subcommands_clap_parse() {
        let cli_text =
            Cli::try_parse_from(["autocompare", "text", "a.txt", "b.txt"]).expect("text");
        assert!(matches!(cli_text.command, Some(Command::Text { .. })));

        let cli_hex = Cli::try_parse_from(["autocompare", "hex", "a.bin", "b.bin"]).expect("hex");
        assert!(matches!(cli_hex.command, Some(Command::Hex { .. })));

        let cli_merge =
            Cli::try_parse_from(["autocompare", "merge", "base.txt", "left.txt", "right.txt"])
                .expect("merge");
        assert!(matches!(cli_merge.command, Some(Command::Merge { .. })));

        let cli_fmerge = Cli::try_parse_from([
            "autocompare",
            "folder-merge",
            "base_dir",
            "left_dir",
            "right_dir",
        ])
        .expect("fmerge");
        assert!(matches!(
            cli_fmerge.command,
            Some(Command::FolderMerge { .. })
        ));

        let cli_table =
            Cli::try_parse_from(["autocompare", "table", "a.csv", "b.csv"]).expect("table");
        assert!(matches!(cli_table.command, Some(Command::Table { .. })));

        let cli_sync =
            Cli::try_parse_from(["autocompare", "sync", "dir_a", "dir_b"]).expect("sync");
        assert!(matches!(cli_sync.command, Some(Command::Sync { .. })));

        let cli_picture =
            Cli::try_parse_from(["autocompare", "picture", "a.png", "b.png"]).expect("pic");
        assert!(matches!(cli_picture.command, Some(Command::Picture { .. })));

        let cli_audio =
            Cli::try_parse_from(["autocompare", "audio", "a.wav", "b.wav"]).expect("audio");
        assert!(matches!(cli_audio.command, Some(Command::Audio { .. })));

        let cli_session =
            Cli::try_parse_from(["autocompare", "session", "my_session.json"]).expect("session");
        assert!(matches!(cli_session.command, Some(Command::Session { .. })));
    }
}
