//! Text file loading and line comparison.
//!
//! [`load_lines`] reads raw bytes, decodes UTF-8 leniently (invalid
//! bytes become U+FFFD) and normalises line endings for display: one
//! trailing `\r` is stripped from each `\n`-separated line, and a
//! trailing `\n` never produces a phantom empty last line.
//!
//! [`compare_text_files`] builds private comparison keys per line
//! (case and whitespace normalised according to
//! [`TextOptions`](crate::TextOptions)) and runs the pure diff from
//! `autocompare-diff` over those keys. The lines it returns are
//! always the original display lines, never the keys.

use std::path::Path;

use autocompare_diff::diff_lines;

use crate::{Error, Result, TextComparison, TextOptions};

/// Loads a text file for comparison. See the module documentation.
pub(crate) fn load_lines(path: &Path) -> Result<Vec<String>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::Missing {
                path: path.display().to_string(),
            });
        }
        Err(e) => {
            return Err(Error::Io {
                path: path.display().to_string(),
                source: e,
            });
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    Ok(split_lines(&text))
}

/// Splits decoded text into display lines.
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<String> = text.split('\n').map(strip_cr).collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    lines
}

/// Strips one trailing `\r` (CRLF display normalisation).
fn strip_cr(line: &str) -> String {
    match line.strip_suffix('\r') {
        Some(stripped) => stripped.to_owned(),
        None => line.to_owned(),
    }
}

/// Compares two text files line by line. See the module documentation.
pub(crate) fn compare_text_files(
    left: &Path,
    right: &Path,
    opts: &TextOptions,
) -> Result<TextComparison> {
    let left_lines = load_lines(left)?;
    let right_lines = load_lines(right)?;
    Ok(compare_text_lines(&left_lines, &right_lines, opts))
}

/// Compares two in-memory line vectors with the same keys + diff engine
/// as [`compare_text_files`]. Wave-66V: the backend apply-edit path
/// rebuilds display lines from cached rows and re-compares without
/// touching disk.
pub fn compare_text_lines(
    left_lines: &[String],
    right_lines: &[String],
    opts: &TextOptions,
) -> TextComparison {
    let left_keys: Vec<String> = left_lines
        .iter()
        .map(|line| comparison_key(line, opts))
        .collect();
    let right_keys: Vec<String> = right_lines
        .iter()
        .map(|line| comparison_key(line, opts))
        .collect();
    let left_refs: Vec<&str> = left_keys.iter().map(|key| key.as_str()).collect();
    let right_refs: Vec<&str> = right_keys.iter().map(|key| key.as_str()).collect();
    let script = diff_lines(&left_refs, &right_refs);
    TextComparison {
        left_lines: left_lines.to_vec(),
        right_lines: right_lines.to_vec(),
        script,
    }
}

/// Builds the private comparison key for one line: whitespace runs
/// collapse to single spaces (and the line is trimmed) when
/// `ignore_space_change` is set, and case is folded when
/// `ignore_case` is set.
fn comparison_key(line: &str, opts: &TextOptions) -> String {
    let mut key = if opts.ignore_space_change {
        line.split_whitespace().collect::<Vec<&str>>().join(" ")
    } else {
        line.to_owned()
    };
    if opts.ignore_case {
        key = key.to_lowercase();
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_options_defaults_and_extensions() {
        let def = TextOptions::default();
        assert!(!def.ignore_case);
        assert!(!def.ignore_space_change);
        assert!(!def.minor_unimportant);
        assert!(def.suppressed_lines.is_empty());
        assert_eq!(def.encoding, None);
        assert_eq!(def.eol, None);

        let custom = TextOptions {
            ignore_case: true,
            ignore_space_change: false,
            minor_unimportant: true,
            suppressed_lines: vec![0, 3],
            encoding: Some("windows-1252".to_string()),
            eol: Some("crlf".to_string()),
        };
        let l = vec!["A".to_string(), "B".to_string()];
        let r = vec!["a".to_string(), "B".to_string()];
        let cmp = compare_text_lines(&l, &r, &custom);
        assert_eq!(cmp.left_lines, l);
        assert_eq!(cmp.right_lines, r);
        assert!(cmp.script.exact);
    }
}

