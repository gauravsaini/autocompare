//! Beyond Compare (bcomp) CLI compatibility normalizer.
//!
//! Translates legacy single-dash flags (-ro, -automerge, -mergeoutput, -vcs1..4)
//! and positional invocations into standard autocompare subcommands.

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
        if arg == "-ro" || arg == "-ro1" || arg == "-ro2" {
            out.push("--ro".to_string());
        } else if arg == "-automerge" {
            out.push("--automerge".to_string());
        } else if arg == "-mergeoutput" {
            out.push("--mergeoutput".to_string());
        } else if let Some(val) = arg.strip_prefix("-mergeoutput=") {
            out.push(format!("--mergeoutput={val}"));
        } else if arg == "-vcs1" {
            out.push("--vcs1".to_string());
        } else if let Some(val) = arg.strip_prefix("-vcs1=") {
            out.push(format!("--vcs1={val}"));
        } else if arg == "-vcs2" {
            out.push("--vcs2".to_string());
        } else if let Some(val) = arg.strip_prefix("-vcs2=") {
            out.push(format!("--vcs2={val}"));
        } else if arg == "-vcs3" {
            out.push("--vcs3".to_string());
        } else if let Some(val) = arg.strip_prefix("-vcs3=") {
            out.push(format!("--vcs3={val}"));
        } else if arg == "-vcs4" {
            out.push("--vcs4".to_string());
        } else if let Some(val) = arg.strip_prefix("-vcs4=") {
            out.push(format!("--vcs4={val}"));
        } else if arg == "-report" {
            out.push("--report".to_string());
        } else if let Some(val) = arg.strip_prefix("-report=") {
            out.push(format!("--report={val}"));
        } else if arg == "-session" {
            out.push("--session".to_string());
        } else if let Some(val) = arg.strip_prefix("-session=") {
            out.push(format!("--session={val}"));
        } else if arg == "-save-session" {
            out.push("--save-session".to_string());
        } else if let Some(val) = arg.strip_prefix("-save-session=") {
            out.push(format!("--save-session={val}"));
        } else {
            out.push(arg.clone());
        }
    }

    out
}

fn resolve_direct_invocation(args: &[String]) -> Vec<String> {
    if args.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(args.len().saturating_add(2));
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
            {
                let _ = iter.next(); // Skip value
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

        // Positional invocation without subcommand.
        let pos_count = non_flag_indices.len();
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
            3.. => "merge",
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
