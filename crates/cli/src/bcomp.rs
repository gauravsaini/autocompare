//! Beyond Compare (bcomp) CLI compatibility normalizer.
//!
//! Translates legacy single-dash and slash flags (-ro, -automerge, -mergeoutput, -vcs1..4,
//! -title1..4, /ro, /automerge, etc.) and positional invocations into standard autocompare
//! subcommands and clap options.

use std::path::Path;

const KNOWN_SUBCOMMANDS: &[&str] = &[
    "text",
    "folder",
    "hex",
    "merge",
    "table",
    "sync",
    "picture",
    "folder-merge",
    "audio",
    "session",
    "help",
    "--help",
    "-h",
    "--version",
    "-V",
];

/// Normalizes bcomp-style single-dash flags and shorthand invocations.
pub fn normalize_and_resolve_args(raw_args: &[String]) -> Vec<String> {
    let normalized = normalize_single_dash_flags(raw_args);
    resolve_direct_invocation(&normalized)
}

/// Normalizes single-dash bcomp and slash flags into canonical clap options.
pub fn normalize_single_dash_flags(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());

    for arg in args {
        // BC switches are case-insensitive; lower only the flag part, preserve value case.
        // Paths and @session values stay verbatim.
        let cmp_arg: String = match arg.split_once('=') {
            Some((flag, val)) if flag.starts_with('-') || flag.starts_with('/') => {
                format!("{}={val}", flag.to_ascii_lowercase())
            }
            None if arg.starts_with('-') || arg.starts_with('/') => arg.to_ascii_lowercase(),
            _ => arg.clone(),
        };
        let orig = arg;
        let arg = &cmp_arg;
        // Read-only flags
        if arg == "-ro" || arg == "/ro" || arg == "-readonly" || arg == "/readonly" {
            out.push("--ro".to_string());
        } else if arg == "-ro1" || arg == "/ro1" || arg == "-leftreadonly" || arg == "/leftreadonly"
        {
            out.push("--ro1".to_string());
            out.push("--ro".to_string());
        } else if arg == "-ro2"
            || arg == "/ro2"
            || arg == "-rightreadonly"
            || arg == "/rightreadonly"
        {
            out.push("--ro2".to_string());
            out.push("--ro".to_string());
        } else if arg == "-centerreadonly"
            || arg == "/centerreadonly"
            || arg == "-ro3"
            || arg == "/ro3"
            || arg == "-ro4"
            || arg == "/ro4"
            || arg == "-outputreadonly"
            || arg == "/outputreadonly"
        {
            out.push("--ro".to_string());
        // Merge automation flags
        } else if arg == "-automerge" || arg == "/automerge" {
            out.push("--automerge".to_string());
        } else if arg == "-reviewconflicts" || arg == "/reviewconflicts" {
            out.push("--reviewconflicts".to_string());
        } else if arg == "-mergeoutput"
            || arg == "/mergeoutput"
            || arg == "-savetarget"
            || arg == "/savetarget"
        {
            out.push("--mergeoutput".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-mergeoutput=")
            .or_else(|| arg.strip_prefix("/mergeoutput="))
            .or_else(|| arg.strip_prefix("-savetarget="))
            .or_else(|| arg.strip_prefix("/savetarget="))
        {
            out.push(format!("--mergeoutput={val}"));
        // Ancestor / Base aliases
        } else if arg == "-ancestor"
            || arg == "/ancestor"
            || arg == "-base"
            || arg == "/base"
            || arg == "-center"
            || arg == "/center"
        {
            out.push("--vcs3".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-ancestor=")
            .or_else(|| arg.strip_prefix("/ancestor="))
            .or_else(|| arg.strip_prefix("-base="))
            .or_else(|| arg.strip_prefix("/base="))
            .or_else(|| arg.strip_prefix("-center="))
            .or_else(|| arg.strip_prefix("/center="))
        {
            out.push(format!("--vcs3={val}"));
        // VCS 1..4 inputs
        } else if arg == "-vcs1" || arg == "/vcs1" {
            out.push("--vcs1".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-vcs1=")
            .or_else(|| arg.strip_prefix("/vcs1="))
        {
            out.push(format!("--vcs1={val}"));
        } else if arg == "-vcs2" || arg == "/vcs2" {
            out.push("--vcs2".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-vcs2=")
            .or_else(|| arg.strip_prefix("/vcs2="))
        {
            out.push(format!("--vcs2={val}"));
        } else if arg == "-vcs3" || arg == "/vcs3" {
            out.push("--vcs3".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-vcs3=")
            .or_else(|| arg.strip_prefix("/vcs3="))
        {
            out.push(format!("--vcs3={val}"));
        } else if arg == "-vcs4" || arg == "/vcs4" {
            out.push("--vcs4".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-vcs4=")
            .or_else(|| arg.strip_prefix("/vcs4="))
        {
            out.push(format!("--vcs4={val}"));
        // VCS pane titles
        } else if arg == "-title1" || arg == "/title1" || arg == "-lefttitle" || arg == "/lefttitle"
        {
            out.push("--title1".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-title1=")
            .or_else(|| arg.strip_prefix("/title1="))
            .or_else(|| arg.strip_prefix("-lefttitle="))
            .or_else(|| arg.strip_prefix("/lefttitle="))
        {
            out.push(format!("--title1={val}"));
        } else if arg == "-title2"
            || arg == "/title2"
            || arg == "-righttitle"
            || arg == "/righttitle"
        {
            out.push("--title2".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-title2=")
            .or_else(|| arg.strip_prefix("/title2="))
            .or_else(|| arg.strip_prefix("-righttitle="))
            .or_else(|| arg.strip_prefix("/righttitle="))
        {
            out.push(format!("--title2={val}"));
        } else if arg == "-title3"
            || arg == "/title3"
            || arg == "-centertitle"
            || arg == "/centertitle"
            || arg == "-ancestortitle"
            || arg == "/ancestortitle"
        {
            out.push("--title3".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-title3=")
            .or_else(|| arg.strip_prefix("/title3="))
            .or_else(|| arg.strip_prefix("-centertitle="))
            .or_else(|| arg.strip_prefix("/centertitle="))
            .or_else(|| arg.strip_prefix("-ancestortitle="))
            .or_else(|| arg.strip_prefix("/ancestortitle="))
        {
            out.push(format!("--title3={val}"));
        } else if arg == "-title4"
            || arg == "/title4"
            || arg == "-outputtitle"
            || arg == "/outputtitle"
        {
            out.push("--title4".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-title4=")
            .or_else(|| arg.strip_prefix("/title4="))
            .or_else(|| arg.strip_prefix("-outputtitle="))
            .or_else(|| arg.strip_prefix("/outputtitle="))
        {
            out.push(format!("--title4={val}"));
        // Reporting
        } else if arg == "-report" || arg == "/report" {
            out.push("--report".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-report=")
            .or_else(|| arg.strip_prefix("/report="))
        {
            out.push(format!("--report={val}"));
        } else if arg == "-report-title"
            || arg == "/report-title"
            || arg == "-reporttitle"
            || arg == "/reporttitle"
        {
            out.push("--report-title".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-report-title=")
            .or_else(|| arg.strip_prefix("/report-title="))
            .or_else(|| arg.strip_prefix("-reporttitle="))
            .or_else(|| arg.strip_prefix("/reporttitle="))
        {
            out.push(format!("--report-title={val}"));
        // Sessions
        } else if arg == "-session" || arg == "/session" {
            out.push("--session".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-session=")
            .or_else(|| arg.strip_prefix("/session="))
        {
            out.push(format!("--session={val}"));
        } else if arg == "-save-session"
            || arg == "/save-session"
            || arg == "-savesession"
            || arg == "/savesession"
        {
            out.push("--save-session".to_string());
        } else if let Some(val) = arg
            .strip_prefix("-save-session=")
            .or_else(|| arg.strip_prefix("/save-session="))
            .or_else(|| arg.strip_prefix("-savesession="))
            .or_else(|| arg.strip_prefix("/savesession="))
        {
            out.push(format!("--save-session={val}"));
        } else if let Some(val) = arg.strip_prefix('@') {
            out.push(format!("--session={val}"));
        // Solo and backup flags
        } else if arg == "-solo" || arg == "/solo" {
            out.push("--solo".to_string());
        } else if arg == "-nobackups" || arg == "/nobackups" {
            out.push("--nobackups".to_string());
        } else if arg == "-closescript" || arg == "/closescript" {
            // Ignored in CLI headless mode.
            // Exit code modes
        } else if arg == "-qc"
            || arg == "/qc"
            || arg == "-quickcompare"
            || arg == "/quickcompare"
            || arg == "-bcexitcode"
            || arg == "/bcexitcode"
            || arg == "-bcexitcodes"
            || arg == "/bcexitcodes"
            || arg == "-bc"
        {
            out.push("--bc-exit-code".to_string());
        } else if arg == "-diffexitcode" || arg == "/diffexitcode" {
            out.push("--exit-code-mode=diff".to_string());
        // Shorthand comparison subcommands
        } else if arg == "-text" || arg == "/text" {
            out.push("text".to_string());
        } else if arg == "-folder" || arg == "/folder" {
            out.push("folder".to_string());
        } else if arg == "-hex" || arg == "/hex" {
            out.push("hex".to_string());
        } else if arg == "-merge" || arg == "/merge" {
            out.push("merge".to_string());
        } else if arg == "-table" || arg == "/table" {
            out.push("table".to_string());
        } else if arg == "-sync" || arg == "/sync" {
            out.push("sync".to_string());
        } else if arg == "-picture" || arg == "/picture" {
            out.push("picture".to_string());
        } else if arg == "-folder-merge"
            || arg == "/folder-merge"
            || arg == "-foldermerge"
            || arg == "/foldermerge"
        {
            out.push("folder-merge".to_string());
        } else if arg == "-audio" || arg == "/audio" {
            out.push("audio".to_string());
        } else {
            out.push(orig.clone());
        }
    }

    out
}

/// Resolves direct invocations, shorthand VCS switches, and positional arguments into subcommands.
pub fn resolve_direct_invocation(args: &[String]) -> Vec<String> {
    if args.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(args.len().saturating_add(4));
    let bin_name = args.first().cloned().unwrap_or_default();
    out.push(bin_name);

    let rest = match args.get(1..) {
        Some(s) => s,
        None => return out,
    };

    // Scan for subcommands or non-flag positional arguments.
    let mut non_flag_indices = Vec::new();
    let mut vcs_consumed_indices = Vec::new();
    let mut iter = rest.iter().enumerate().peekable();
    let mut vcs1 = None;
    let mut vcs2 = None;
    let mut vcs3 = None;
    let mut vcs4 = None;

    while let Some((idx, token)) = iter.next() {
        if token == "--vcs1" {
            vcs_consumed_indices.push(idx);
            if let Some((v_idx, val)) = iter.next() {
                vcs_consumed_indices.push(v_idx);
                vcs1 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs1=") {
            vcs_consumed_indices.push(idx);
            vcs1 = Some(val.to_string());
        } else if token == "--vcs2" {
            vcs_consumed_indices.push(idx);
            if let Some((v_idx, val)) = iter.next() {
                vcs_consumed_indices.push(v_idx);
                vcs2 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs2=") {
            vcs_consumed_indices.push(idx);
            vcs2 = Some(val.to_string());
        } else if token == "--vcs3" {
            vcs_consumed_indices.push(idx);
            if let Some((v_idx, val)) = iter.next() {
                vcs_consumed_indices.push(v_idx);
                vcs3 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs3=") {
            vcs_consumed_indices.push(idx);
            vcs3 = Some(val.to_string());
        } else if token == "--vcs4" {
            vcs_consumed_indices.push(idx);
            if let Some((v_idx, val)) = iter.next() {
                vcs_consumed_indices.push(v_idx);
                vcs4 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs4=") {
            vcs_consumed_indices.push(idx);
            vcs4 = Some(val.to_string());
        } else if token.starts_with('-') {
            // Check if this flag takes an argument that is not attached.
            if token == "--report"
                || token == "--report-title"
                || token == "--session"
                || token == "--save-session"
                || token == "--mergeoutput"
                || token == "--output"
                || token == "--width"
                || token == "--depth"
                || token == "--include"
                || token == "--exclude"
                || token == "--max-hunks"
                || token == "--delimiter"
                || token == "--direction"
                || token == "--heatmap"
                || token == "--color"
                || token == "--title1"
                || token == "--title2"
                || token == "--title3"
                || token == "--title4"
                || token == "--exit-code-mode"
                || token == "-w"
                || token == "-d"
            {
                let _ = iter.next(); // Skip value token
            }
        } else {
            non_flag_indices.push(idx);
        }
    }

    if let Some(&first_idx) = non_flag_indices.first() {
        if let Some(first_tok) = rest.get(first_idx)
            && KNOWN_SUBCOMMANDS.iter().any(|&cmd| cmd == first_tok)
        {
            // Already has a valid subcommand or help flag; return as-is.
            out.extend(rest.iter().cloned());
            return out;
        }

        let pos_count = non_flag_indices.len();

        // 4 Positional arguments: Git mergetool syntax `bcomp local remote base merged`
        if pos_count == 4 {
            let p0 = non_flag_indices
                .first()
                .and_then(|&i| rest.get(i))
                .cloned()
                .unwrap_or_default();
            let p1 = non_flag_indices
                .get(1)
                .and_then(|&i| rest.get(i))
                .cloned()
                .unwrap_or_default();
            let p2 = non_flag_indices
                .get(2)
                .and_then(|&i| rest.get(i))
                .cloned()
                .unwrap_or_default();
            let p3 = non_flag_indices
                .get(3)
                .and_then(|&i| rest.get(i))
                .cloned()
                .unwrap_or_default();

            let is_dirs = Path::new(&p0).is_dir() && Path::new(&p1).is_dir();
            let subcommand = if is_dirs { "folder-merge" } else { "merge" };

            out.push(subcommand.to_string());
            // Positional order for merge: base, left (local), right (remote)
            out.push(p2);
            out.push(p0);
            out.push(p1);
            out.push(format!("--output={p3}"));

            // Append remaining non-positional arguments
            for (idx, token) in rest.iter().enumerate() {
                if !non_flag_indices.contains(&idx) {
                    out.push(token.clone());
                }
            }
            return out;
        }

        // 2 Positional arguments with --vcs3 (ancestor/base) present -> 3-way merge
        if pos_count == 2 && vcs3.is_some() {
            let base = vcs3.unwrap_or_default();
            let left = non_flag_indices
                .first()
                .and_then(|&i| rest.get(i))
                .cloned()
                .unwrap_or_default();
            let right = non_flag_indices
                .get(1)
                .and_then(|&i| rest.get(i))
                .cloned()
                .unwrap_or_default();

            let is_dirs = Path::new(&left).is_dir() && Path::new(&right).is_dir();
            let subcommand = if is_dirs { "folder-merge" } else { "merge" };

            out.push(subcommand.to_string());
            out.push(base);
            out.push(left);
            out.push(right);

            if let Some(out_path) = vcs4 {
                out.push(format!("--output={out_path}"));
            }

            for (idx, token) in rest.iter().enumerate() {
                if !non_flag_indices.contains(&idx) && !vcs_consumed_indices.contains(&idx) {
                    out.push(token.clone());
                }
            }
            return out;
        }

        // Positional invocation without subcommand.
        let subcommand = match pos_count {
            2 => {
                let p1 = rest.get(first_idx).map(|s| s.as_str()).unwrap_or_default();
                let p2 = non_flag_indices
                    .get(1)
                    .and_then(|&i| rest.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or_default();
                if Path::new(p1).is_dir() && Path::new(p2).is_dir() {
                    "folder"
                } else {
                    "text"
                }
            }
            3 => {
                let p1 = rest.get(first_idx).map(|s| s.as_str()).unwrap_or_default();
                let p2 = non_flag_indices
                    .get(1)
                    .and_then(|&i| rest.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or_default();
                let p3 = non_flag_indices
                    .get(2)
                    .and_then(|&i| rest.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or_default();
                if Path::new(p1).is_dir() && Path::new(p2).is_dir() && Path::new(p3).is_dir() {
                    "folder-merge"
                } else {
                    "merge"
                }
            }
            1 => {
                let p1 = rest.get(first_idx).map(|s| s.as_str()).unwrap_or_default();
                if p1.ends_with(".bcss") || p1.ends_with(".json") {
                    out.push(format!("--session={p1}"));
                    for (i, token) in rest.iter().enumerate() {
                        if i != first_idx {
                            out.push(token.clone());
                        }
                    }
                    return out;
                }
                "text"
            }
            _ => "text",
        };

        // Insert subcommand before the first non-flag positional argument.
        for (i, token) in rest.iter().enumerate() {
            if i == first_idx {
                out.push(subcommand.to_string());
            }
            out.push(token.clone());
        }
        return out;
    }

    // No positional arguments, but VCS flags might be present.
    if let (Some(l), Some(r)) = (vcs1, vcs2) {
        let is_dirs = Path::new(&l).is_dir() && Path::new(&r).is_dir();
        if let Some(b) = vcs3 {
            let subcommand = if is_dirs { "folder-merge" } else { "merge" };
            out.push(subcommand.to_string());
            out.push(b);
            out.push(l);
            out.push(r);
            if let Some(o) = vcs4 {
                out.push(format!("--output={o}"));
            }
        } else {
            let subcommand = if is_dirs { "folder" } else { "text" };
            out.push(subcommand.to_string());
            out.push(l);
            out.push(r);
        }
        // Append remaining flags that were not consumed by vcs1..4
        for (idx, token) in rest.iter().enumerate() {
            if !vcs_consumed_indices.contains(&idx) {
                out.push(token.clone());
            }
        }
        return out;
    }

    out.extend(rest.iter().cloned());
    out
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
    use std::path::PathBuf;

    fn fixture_path(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/parity-harness/fixtures")
            .join(rel)
    }

    #[test]
    fn test_normalize_read_only_flags() {
        let input = vec![
            "app".to_string(),
            "-ro".to_string(),
            "/ro".to_string(),
            "-readonly".to_string(),
            "/readonly".to_string(),
            "-ro1".to_string(),
            "/ro1".to_string(),
            "-leftreadonly".to_string(),
            "/leftreadonly".to_string(),
            "-ro2".to_string(),
            "/ro2".to_string(),
            "-rightreadonly".to_string(),
            "/rightreadonly".to_string(),
            "-ro3".to_string(),
            "/ro3".to_string(),
            "-centerreadonly".to_string(),
            "/centerreadonly".to_string(),
            "-ro4".to_string(),
            "/ro4".to_string(),
            "-outputreadonly".to_string(),
            "/outputreadonly".to_string(),
        ];
        let normalized = normalize_single_dash_flags(&input);
        assert!(normalized.contains(&"--ro".to_string()));
        assert!(normalized.contains(&"--ro1".to_string()));
        assert!(normalized.contains(&"--ro2".to_string()));
    }

    #[test]
    fn test_normalize_automerge_and_mergeoutput() {
        let input = vec![
            "app".to_string(),
            "-automerge".to_string(),
            "/automerge".to_string(),
            "-reviewconflicts".to_string(),
            "/reviewconflicts".to_string(),
            "-mergeoutput=out.txt".to_string(),
            "/mergeoutput=out2.txt".to_string(),
            "-savetarget=out3.txt".to_string(),
            "/savetarget=out4.txt".to_string(),
            "-mergeoutput".to_string(),
            "out5.txt".to_string(),
        ];
        let normalized = normalize_single_dash_flags(&input);
        assert!(normalized.contains(&"--automerge".to_string()));
        assert!(normalized.contains(&"--reviewconflicts".to_string()));
        assert!(normalized.contains(&"--mergeoutput=out.txt".to_string()));
        assert!(normalized.contains(&"--mergeoutput=out2.txt".to_string()));
        assert!(normalized.contains(&"--mergeoutput=out3.txt".to_string()));
        assert!(normalized.contains(&"--mergeoutput=out4.txt".to_string()));
        assert!(normalized.contains(&"--mergeoutput".to_string()));
        assert!(normalized.contains(&"out5.txt".to_string()));
    }

    #[test]
    fn test_normalize_vcs_inputs_and_titles() {
        let input = vec![
            "app".to_string(),
            "-vcs1=left.txt".to_string(),
            "/vcs2=right.txt".to_string(),
            "-ancestor=base.txt".to_string(),
            "/base=base2.txt".to_string(),
            "-center=base3.txt".to_string(),
            "-vcs4=merged.txt".to_string(),
            "-title1=LeftPane".to_string(),
            "/lefttitle=LeftPane2".to_string(),
            "-title2=RightPane".to_string(),
            "/righttitle=RightPane2".to_string(),
            "-title3=CenterPane".to_string(),
            "-title4=OutputPane".to_string(),
            "-report=html:rep.html".to_string(),
            "-reporttitle=CustomTitle".to_string(),
            "@session.bcss".to_string(),
            "-bcexitcode".to_string(),
            "-qc".to_string(),
            "-diffexitcode".to_string(),
        ];
        let normalized = normalize_single_dash_flags(&input);
        assert!(normalized.contains(&"--vcs1=left.txt".to_string()));
        assert!(normalized.contains(&"--vcs2=right.txt".to_string()));
        assert!(normalized.contains(&"--vcs3=base.txt".to_string()));
        assert!(normalized.contains(&"--vcs3=base2.txt".to_string()));
        assert!(normalized.contains(&"--vcs3=base3.txt".to_string()));
        assert!(normalized.contains(&"--vcs4=merged.txt".to_string()));
        assert!(normalized.contains(&"--title1=LeftPane".to_string()));
        assert!(normalized.contains(&"--title1=LeftPane2".to_string()));
        assert!(normalized.contains(&"--title2=RightPane".to_string()));
        assert!(normalized.contains(&"--title2=RightPane2".to_string()));
        assert!(normalized.contains(&"--title3=CenterPane".to_string()));
        assert!(normalized.contains(&"--title4=OutputPane".to_string()));
        assert!(normalized.contains(&"--report=html:rep.html".to_string()));
        assert!(normalized.contains(&"--report-title=CustomTitle".to_string()));
        assert!(normalized.contains(&"--session=session.bcss".to_string()));
        assert!(normalized.contains(&"--bc-exit-code".to_string()));
        assert!(normalized.contains(&"--exit-code-mode=diff".to_string()));
    }

    #[test]
    fn test_normalize_subcommand_aliases() {
        let input = vec![
            "app".to_string(),
            "-text".to_string(),
            "-folder".to_string(),
            "-hex".to_string(),
            "-merge".to_string(),
            "-table".to_string(),
            "-sync".to_string(),
            "-picture".to_string(),
            "-folder-merge".to_string(),
            "-audio".to_string(),
        ];
        let normalized = normalize_single_dash_flags(&input);
        assert_eq!(
            normalized,
            vec![
                "app",
                "text",
                "folder",
                "hex",
                "merge",
                "table",
                "sync",
                "picture",
                "folder-merge",
                "audio"
            ]
        );
    }

    #[test]
    fn test_resolve_git_mergetool_four_way_files() {
        let raw = vec![
            "bcomp".to_string(),
            "local.txt".to_string(),
            "remote.txt".to_string(),
            "base.txt".to_string(),
            "merged.txt".to_string(),
            "-automerge".to_string(),
        ];
        let resolved = normalize_and_resolve_args(&raw);
        assert_eq!(resolved.first(), Some(&"bcomp".to_string()));
        assert_eq!(resolved.get(1), Some(&"merge".to_string()));
        assert_eq!(resolved.get(2), Some(&"base.txt".to_string()));
        assert_eq!(resolved.get(3), Some(&"local.txt".to_string()));
        assert_eq!(resolved.get(4), Some(&"remote.txt".to_string()));
        assert_eq!(resolved.get(5), Some(&"--output=merged.txt".to_string()));
        assert!(resolved.contains(&"--automerge".to_string()));
    }

    #[test]
    fn test_resolve_git_mergetool_four_way_dirs() {
        let fx = fixture_path("folder");
        let left_dir = fx.join("left").to_str().expect("path").to_string();
        let right_dir = fx.join("right").to_str().expect("path").to_string();
        let base_dir = fx.join("left").to_str().expect("path").to_string();
        let out_dir = fx.join("right").to_str().expect("path").to_string();

        let raw = vec![
            "bcomp".to_string(),
            left_dir.clone(),
            right_dir.clone(),
            base_dir.clone(),
            out_dir.clone(),
        ];
        let resolved = normalize_and_resolve_args(&raw);
        assert_eq!(resolved.get(1), Some(&"folder-merge".to_string()));
        assert_eq!(resolved.get(2), Some(&base_dir));
        assert_eq!(resolved.get(3), Some(&left_dir));
        assert_eq!(resolved.get(4), Some(&right_dir));
        assert_eq!(resolved.get(5), Some(&format!("--output={out_dir}")));
    }

    #[test]
    fn test_resolve_two_way_with_vcs3_ancestor() {
        let raw = vec![
            "bcomp".to_string(),
            "-ancestor=base.txt".to_string(),
            "left.txt".to_string(),
            "right.txt".to_string(),
            "-mergeoutput=out.txt".to_string(),
        ];
        let resolved = normalize_and_resolve_args(&raw);
        assert_eq!(resolved.get(1), Some(&"merge".to_string()));
        assert_eq!(resolved.get(2), Some(&"base.txt".to_string()));
        assert_eq!(resolved.get(3), Some(&"left.txt".to_string()));
        assert_eq!(resolved.get(4), Some(&"right.txt".to_string()));
        assert!(resolved.contains(&"--mergeoutput=out.txt".to_string()));
    }

    #[test]
    fn test_resolve_two_positionals_files_vs_dirs() {
        let f_left = fixture_path("t_left.txt").to_str().expect("p").to_string();
        let f_right = fixture_path("t_right.txt").to_str().expect("p").to_string();

        let raw_files = vec!["bcomp".to_string(), f_left.clone(), f_right.clone()];
        let res_files = normalize_and_resolve_args(&raw_files);
        assert_eq!(res_files.get(1), Some(&"text".to_string()));

        let d_left = fixture_path("folder/left").to_str().expect("p").to_string();
        let d_right = fixture_path("folder/right")
            .to_str()
            .expect("p")
            .to_string();
        let raw_dirs = vec!["bcomp".to_string(), d_left.clone(), d_right.clone()];
        let res_dirs = normalize_and_resolve_args(&raw_dirs);
        assert_eq!(res_dirs.get(1), Some(&"folder".to_string()));
    }

    #[test]
    fn test_resolve_three_positionals_files_vs_dirs() {
        let b = fixture_path("m_base.txt").to_str().expect("p").to_string();
        let l = fixture_path("m_left.txt").to_str().expect("p").to_string();
        let r = fixture_path("m_right.txt").to_str().expect("p").to_string();

        let raw_files = vec!["bcomp".to_string(), b.clone(), l.clone(), r.clone()];
        let res_files = normalize_and_resolve_args(&raw_files);
        assert_eq!(res_files.get(1), Some(&"merge".to_string()));

        let d_base = fixture_path("folder/left").to_str().expect("p").to_string();
        let d_left = fixture_path("folder/left").to_str().expect("p").to_string();
        let d_right = fixture_path("folder/right")
            .to_str()
            .expect("p")
            .to_string();
        let raw_dirs = vec!["bcomp".to_string(), d_base, d_left, d_right];
        let res_dirs = normalize_and_resolve_args(&raw_dirs);
        assert_eq!(res_dirs.get(1), Some(&"folder-merge".to_string()));
    }

    #[test]
    fn test_resolve_vcs_flags_space_syntax_and_output() {
        let raw = vec![
            "bcomp".to_string(),
            "-vcs1".to_string(),
            "left.txt".to_string(),
            "-vcs2".to_string(),
            "right.txt".to_string(),
            "-vcs3".to_string(),
            "base.txt".to_string(),
            "-vcs4".to_string(),
            "merged.txt".to_string(),
            "-automerge".to_string(),
        ];
        let res = normalize_and_resolve_args(&raw);
        assert_eq!(res.get(1), Some(&"merge".to_string()));
        assert_eq!(res.get(2), Some(&"base.txt".to_string()));
        assert_eq!(res.get(3), Some(&"left.txt".to_string()));
        assert_eq!(res.get(4), Some(&"right.txt".to_string()));
        assert!(res.contains(&"--output=merged.txt".to_string()));
        assert!(res.contains(&"--automerge".to_string()));
    }

    #[test]
    fn test_resolve_vcs_dirs_two_way() {
        let d_left = fixture_path("folder/left").to_str().expect("p").to_string();
        let d_right = fixture_path("folder/right")
            .to_str()
            .expect("p")
            .to_string();
        let raw = vec![
            "bcomp".to_string(),
            format!("-vcs1={d_left}"),
            format!("-vcs2={d_right}"),
        ];
        let res = normalize_and_resolve_args(&raw);
        assert_eq!(res.get(1), Some(&"folder".to_string()));
        assert_eq!(res.get(2), Some(&d_left));
        assert_eq!(res.get(3), Some(&d_right));
    }
}
