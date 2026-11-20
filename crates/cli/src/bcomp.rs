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

fn normalize_single_dash_flags(args: &[String]) -> Vec<String> {
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
        } else if arg == "-centerreadonly" || arg == "/centerreadonly" {
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

fn resolve_direct_invocation(args: &[String]) -> Vec<String> {
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
    let mut iter = rest.iter().enumerate().peekable();
    let mut vcs1 = None;
    let mut vcs2 = None;
    let mut vcs3 = None;
    let mut vcs4 = None;

    while let Some((idx, token)) = iter.next() {
        if token == "--vcs1" {
            if let Some((_, val)) = iter.next() {
                vcs1 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs1=") {
            vcs1 = Some(val.to_string());
        } else if token == "--vcs2" {
            if let Some((_, val)) = iter.next() {
                vcs2 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs2=") {
            vcs2 = Some(val.to_string());
        } else if token == "--vcs3" {
            if let Some((_, val)) = iter.next() {
                vcs3 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs3=") {
            vcs3 = Some(val.to_string());
        } else if token == "--vcs4" {
            if let Some((_, val)) = iter.next() {
                vcs4 = Some(val.clone());
            }
        } else if let Some(val) = token.strip_prefix("--vcs4=") {
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
                if !non_flag_indices.contains(&idx) && !token.starts_with("--vcs3") {
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
        if let Some(b) = vcs3 {
            out.push("merge".to_string());
            out.push(b);
            out.push(l);
            out.push(r);
            if let Some(o) = vcs4 {
                out.push(format!("--output={o}"));
            }
        } else {
            out.push("text".to_string());
            out.push(l);
            out.push(r);
        }
        // Append remaining flags that aren't vcs1..4
        for token in rest {
            if !token.starts_with("--vcs") {
                out.push(token.clone());
            }
        }
        return out;
    }

    out.extend(rest.iter().cloned());
    out
}
