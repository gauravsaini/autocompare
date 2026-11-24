//! Workspace build and verification task runner (`xtask`).
//!
//! Subcommands:
//! - `layers`: verifies strict layering rules (lower crates never depend on higher crates)
//! - `secrets`: scans the repository tree for accidentally committed tokens and private keys
//! - `wasm`: verifies L0 crates compile to wasm32-unknown-unknown
//! - `check-parity`: runs the deterministic parity harness round
//! - `ci`: runs all verification checks

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = match args.next() {
        Some(c) => c,
        None => {
            eprintln!("usage: cargo run -p xtask -- <layers|secrets|wasm|check-parity|ci>");
            return ExitCode::from(1);
        }
    };

    match cmd.as_str() {
        "layers" => cmd_layers(),
        "secrets" => cmd_secrets(),
        "wasm" => cmd_wasm(),
        "check-parity" => cmd_check_parity(),
        "ci" => cmd_ci(),
        other => {
            eprintln!("unknown subcommand: {other}");
            eprintln!("available subcommands: layers, secrets, wasm, check-parity, ci");
            ExitCode::from(1)
        }
    }
}

/// Identifies the layer of a crate according to PLAN.md section 3.1:
/// - L0: autocompare-evidence, autocompare-diff
/// - L1: autocompare-vfs, autocompare-sessions, autocompare-reports
/// - L2: autocompare-compare
/// - L3: autocompare-cli, autocompare-gui, autocompare-desktop
fn crate_layer(name: &str) -> Option<u8> {
    match name {
        "autocompare-evidence" | "autocompare-diff" => Some(0),
        "autocompare-vfs" | "autocompare-sessions" | "autocompare-reports" => Some(1),
        "autocompare-compare" => Some(2),
        "autocompare-cli" | "autocompare-gui" | "autocompare-desktop" => Some(3),
        _ => None,
    }
}

fn cmd_layers() -> ExitCode {
    let crates_dir = Path::new("crates");
    if !crates_dir.exists() {
        eprintln!("error: crates/ directory not found from current working directory");
        return ExitCode::from(1);
    }

    let entries = match fs::read_dir(crates_dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("error reading crates/ directory: {err}");
            return ExitCode::from(1);
        }
    };

    let mut violations = Vec::new();

    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };
        let cargo_toml_path = entry.path().join("Cargo.toml");
        if !cargo_toml_path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&cargo_toml_path) {
            Ok(c) => c,
            Err(err) => {
                eprintln!("error reading {}: {err}", cargo_toml_path.display());
                return ExitCode::from(1);
            }
        };

        let this_crate = match extract_package_name(&content) {
            Some(name) => name,
            None => continue,
        };

        let this_layer = match crate_layer(&this_crate) {
            Some(l) => l,
            None => continue, // third-party or utility crate
        };

        let deps = extract_dependencies(&content);
        for dep in deps {
            // Check UI library restriction: only autocompare-gui may depend on egui/eframe/rfd
            if (dep == "egui" || dep == "eframe" || dep == "rfd") && this_crate != "autocompare-gui"
            {
                violations.push(format!(
                    "UI dependency '{dep}' forbidden in non-GUI crate '{this_crate}' (layer {this_layer})"
                ));
            }

            if let Some(dep_layer) = crate_layer(&dep)
                && this_layer < dep_layer
            {
                violations.push(format!(
                    "layer violation: '{this_crate}' (L{this_layer}) depends on higher crate '{dep}' (L{dep_layer})"
                ));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("layers check failed with {} violations:", violations.len());
        for v in violations {
            eprintln!("  - {v}");
        }
        ExitCode::from(1)
    } else {
        println!("layers: ok");
        ExitCode::SUCCESS
    }
}

fn extract_package_name(cargo_toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if in_package && trimmed.starts_with('[') {
            break;
        }
        if in_package
            && trimmed.starts_with("name")
            && let Some((_, val)) = trimmed.split_once('=')
        {
            let clean = val.trim().trim_matches('"');
            return Some(clean.to_string());
        }
    }
    None
}

fn extract_dependencies(cargo_toml: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;

    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_deps = trimmed == "[dependencies]"
                || trimmed.starts_with("[dependencies.")
                || trimmed == "[dev-dependencies]"
                || trimmed.starts_with("[dev-dependencies.");
            continue;
        }

        if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some((name, _)) = trimmed.split_once('=') {
                deps.push(name.trim().to_string());
            } else if !trimmed.contains('=') && !trimmed.starts_with('[') {
                deps.push(trimmed.to_string());
            }
        }
    }

    deps
}

fn cmd_secrets() -> ExitCode {
    let mut findings = Vec::new();
    let root = Path::new(".");
    scan_dir_for_secrets(root, &mut findings);

    if !findings.is_empty() {
        eprintln!("secrets check failed with {} findings:", findings.len());
        for (file, line_num, reason) in findings {
            eprintln!("  - {}:{}: {}", file.display(), line_num, reason);
        }
        ExitCode::from(1)
    } else {
        println!("secrets: ok");
        ExitCode::SUCCESS
    }
}

fn scan_dir_for_secrets(dir: &Path, findings: &mut Vec<(PathBuf, usize, String)>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.')
            || name_str == "target"
            || name_str == "node_modules"
            || name_str.ends_with(".bak")
            || name_str.ends_with(".png")
            || name_str.ends_with(".wav")
            || name_str.ends_with(".bin")
        {
            continue;
        }

        if path.is_dir() {
            scan_dir_for_secrets(&path, findings);
        } else if path.is_file() {
            scan_file_for_secrets(&path, findings);
        }
    }
}

fn scan_file_for_secrets(path: &Path, findings: &mut Vec<(PathBuf, usize, String)>) {
    if path.ends_with("xtask/src/main.rs") || path.ends_with("xtask/src/main.rs.bak") {
        return;
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return, // binary file or non-utf8
    };

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        if let Some(reason) = classify_secret_line(line) {
            findings.push((path.to_path_buf(), line_num, reason.to_string()));
        }
    }
}

/// Classifies a single line; shape-based so docs mentioning bare
/// pattern names (e.g. backticked `ghp_`) are not flagged, while real
/// long token shapes still are.
fn classify_secret_line(line: &str) -> Option<&'static str> {
    let priv_needle = concat!("BEGIN ", "PRIVATE ", "KEY");
    let ghp_needle = concat!("gh", "p_");
    let pat_needle = concat!("github_", "pat_");
    let akia_needle = concat!("AK", "IA");

    if line.contains(priv_needle) {
        Some("private key banner detected")
    } else if has_long_prefixed_token(line, ghp_needle) || has_long_prefixed_token(line, pat_needle)
    {
        Some("GitHub personal access token detected")
    } else if line.len() >= 20
        && let Some(pos) = line.find(akia_needle)
        && let Some(sub) = line.get(pos..pos + 20)
        && sub.chars().all(|c| c.is_ascii_alphanumeric())
    {
        Some("AWS Access Key ID detected")
    } else {
        None
    }
}

/// True when `prefix` occurs with a trailing run of >= 20 token chars,
/// so bare pattern names in docs do not match but real tokens do.
fn has_long_prefixed_token(line: &str, prefix: &str) -> bool {
    const MIN_TAIL: usize = 20;
    let mut search = line;
    while let Some(pos) = search.find(prefix) {
        let tail = &search[pos + prefix.len()..];
        let run = tail
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        if run >= MIN_TAIL {
            return true;
        }
        search = &search[pos + 1..];
    }
    false
}

fn cmd_wasm() -> ExitCode {
    println!("checking wasm32-unknown-unknown target for L0 crates...");
    let mut cmd = Command::new("cargo");
    cmd.args([
        "check",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "autocompare-evidence",
    ]);

    if let Ok(home) = std::env::var("HOME") {
        let cargo_bin = format!("{home}/.cargo/bin");
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", format!("{cargo_bin}:{path}"));
        }
    }

    let status = cmd.status();
    match status {
        Ok(s) if s.success() => {
            println!("wasm: ok");
            ExitCode::SUCCESS
        }
        Ok(s) => {
            eprintln!("wasm check failed with exit code: {s}");
            ExitCode::from(1)
        }
        Err(err) => {
            eprintln!("failed to execute cargo check: {err}");
            ExitCode::from(1)
        }
    }
}

fn cmd_check_parity() -> ExitCode {
    let round_dir = ".unlazy/v1/evidence/wave-0/round-0";
    let status = Command::new("tools/parity-harness/run-round.sh")
        .arg(round_dir)
        .status();

    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().and_then(|c| u8::try_from(c).ok()).unwrap_or(1)),
        Err(err) => {
            eprintln!("failed to run parity harness: {err}");
            ExitCode::from(1)
        }
    }
}

fn cmd_ci() -> ExitCode {
    println!("running xtask ci pipeline...");
    if cmd_layers() != ExitCode::SUCCESS {
        return ExitCode::from(1);
    }
    if cmd_secrets() != ExitCode::SUCCESS {
        return ExitCode::from(1);
    }
    if cmd_wasm() != ExitCode::SUCCESS {
        return ExitCode::from(1);
    }
    println!("ci: ok");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_layer_mapping() {
        assert_eq!(crate_layer("autocompare-evidence"), Some(0));
        assert_eq!(crate_layer("autocompare-diff"), Some(0));
        assert_eq!(crate_layer("autocompare-vfs"), Some(1));
        assert_eq!(crate_layer("autocompare-compare"), Some(2));
        assert_eq!(crate_layer("autocompare-cli"), Some(3));
        assert_eq!(crate_layer("autocompare-gui"), Some(3));
        assert_eq!(crate_layer("unknown-crate"), None);
    }

    #[test]
    fn test_extract_package_name() {
        let manifest = r#"
[package]
name = "autocompare-test"
version = "0.1.0"
"#;
        assert_eq!(
            extract_package_name(manifest),
            Some("autocompare-test".to_string())
        );
    }

    #[test]
    fn secrets_shape_fp_bare_names_ignored() {
        // docs mention bare pattern names (e.g. checks/release-w1/REPORT.md:7)
        let doc =
            "secrets (private-key/`ghp_`/`github_pat_`/`AKIA` scan, `.png` skip, khud ko skip)";
        assert_eq!(classify_secret_line(doc), None);
        assert_eq!(classify_secret_line("use `ghp_` prefix here"), None);
        assert_eq!(classify_secret_line("see `github_pat_` docs"), None);
        assert_eq!(classify_secret_line("short ghp_abc still fine"), None);
    }

    #[test]
    fn secrets_shape_boundary_lengths() {
        let tail19: String = "a".repeat(19);
        let tail20: String = "a".repeat(20);
        assert_eq!(classify_secret_line(&format!("x ghp_{tail19} y")), None);
        assert_eq!(
            classify_secret_line(&format!("x ghp_{tail20} y")),
            Some("GitHub personal access token detected")
        );
    }

    #[test]
    fn secrets_shape_real_tokens_detected() {
        let classic = "ghp_".to_string() + &"a".repeat(36);
        assert_eq!(
            classify_secret_line(&format!("token = \"{classic}\"")),
            Some("GitHub personal access token detected")
        );
        let fine = "github_pat_".to_string() + &"B".repeat(30);
        assert_eq!(
            classify_secret_line(&format!("token = \"{fine}\"")),
            Some("GitHub personal access token detected")
        );
        // canonical shapes built via concat so this file holds no literal secret
        let akia_test = concat!("AK", "IA").to_string() + &"B".repeat(16);
        assert_eq!(
            classify_secret_line(&format!("key = \"{akia_test}\"")),
            Some("AWS Access Key ID detected")
        );
        let banner = format!("-----{}-----", concat!("BEGIN ", "PRIVATE ", "KEY"));
        assert_eq!(
            classify_secret_line(&banner),
            Some("private key banner detected")
        );
    }

    #[test]
    fn test_extract_dependencies() {
        let manifest = r#"
[dependencies]
autocompare-diff = { path = "../diff" }
clap = "4"
"#;
        let deps = extract_dependencies(manifest);
        assert!(deps.contains(&"autocompare-diff".to_string()));
        assert!(deps.contains(&"clap".to_string()));
    }
}
