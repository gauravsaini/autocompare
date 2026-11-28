//! Library interface for the autocompare CLI.
//!
//! Exposes parsing, command dispatch, reporting, and exit code mappings
//! for integration testing and external consumers.

#![forbid(unsafe_code)]

pub mod args;
pub mod bcomp;
pub mod diagnose;
pub mod error;
pub mod outcome;
pub mod render_audio;
pub mod render_folder;
pub mod render_foldermerge;
pub mod render_hex;
pub mod render_picture;
pub mod render_sync;
pub mod render_table;
pub mod render_text;
pub mod render_text_unified;
pub mod report;
pub mod report_adapter;
pub mod report_adapter_media;
pub mod runner;
pub mod session_cmd;
pub mod term;
pub mod trace;

pub use args::{Cli, ColorWhen, Command, SyncDir};
pub use error::CliError;
pub use outcome::Outcome;
pub use report::{ReportFormat, ReportSpec};
