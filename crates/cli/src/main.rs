//! autocompare binary entry point.
//!
//! Headless-first: thin CLI renderer over comparison libraries,
//! supporting standard subcommands, bcomp VCS compatibility, and reporting.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use autocompare_cli::args::Cli;
use autocompare_cli::bcomp;
use autocompare_cli::report::ReportSpec;
use autocompare_cli::runner;
use autocompare_cli::session_cmd;
use autocompare_cli::term;
use clap::Parser;

fn main() -> ExitCode {
    let raw_args: Vec<String> = std::env::args().collect();
    let resolved_args = bcomp::normalize_and_resolve_args(&raw_args);

    let cli = match Cli::try_parse_from(&resolved_args) {
        Ok(c) => c,
        Err(err) => {
            if err.use_stderr() {
                eprint!("{err}");
                return ExitCode::from(2);
            } else {
                print!("{err}");
                return ExitCode::SUCCESS;
            }
        }
    };

    let report_spec = match cli.report.as_deref() {
        Some(spec_str) => match ReportSpec::parse(spec_str) {
            Ok(spec) => Some(spec),
            Err(err) => {
                eprintln!("autocompare: {err}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    let color = term::color_enabled(cli.color);

    if let Some(ref session_path) = cli.session {
        return match session_cmd::load_and_run(session_path, color, report_spec.as_ref()) {
            Ok(outcome) => outcome.exit_code(),
            Err(err) => {
                eprintln!("autocompare: {err}");
                err.exit_code()
            }
        };
    }

    match runner::run(&cli, color, report_spec.as_ref()) {
        Ok(outcome) => outcome.exit_code(),
        Err(err) => {
            eprintln!("autocompare: {err}");
            err.exit_code()
        }
    }
}
