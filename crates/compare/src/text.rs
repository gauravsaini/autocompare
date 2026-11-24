//! Text file loading and line comparison.
//!
//! [`load_lines`] reads raw bytes, decodes UTF-8 leniently (invalid
//! bytes become U+FFFD) and normalises line endings for display: one
//! trailing `\r` is stripped from each `\n`-separated line, and a
//! trailing `\n` never produces a phantom empty last line.
//!
//! Encoding honouring: [`compare_text_files`] decodes with
//! [`decode_bytes`] per `TextOptions.encoding` (`utf-8` default,
//! `windows-1252` / `latin-1` mapped byte-to-char) instead of assuming
//! UTF-8. EOL honouring: when `TextOptions.eol` is set, lone `\r`
//! endings are normalised to `\n` before splitting (and one trailing
//! `\r` is stripped from in-memory lines before keying), so CRLF / CR
//! / LF mixes compare equal; with no `eol` set the legacy display
//! split applies.

use std::path::Path;

use autocompare_diff::diff_lines;

use crate::{Error, Result, TextComparison, TextOptions};

/// Decodes raw file bytes per an `encoding` name (case-insensitive,
/// surrounding whitespace ignored):
///
/// * `utf-8` / `utf8` / empty / `None` — lossy UTF-8 (invalid bytes
///   become U+FFFD). This is the default and the legacy behaviour.
/// * `windows-1252` / `cp1252` / `1252` — Windows-1252: bytes
///   `0x00`–`0x7F` and `0xA0`–`0xFF` map to the same code point,
///   `0x80`–`0x9F` map via the CP1252 table (undefined slots become
///   U+FFFD).
/// * `latin-1` / `latin1` / `iso-8859-1` — every byte maps to the
///   same code point (U+0000–U+00FF).
///
/// Unknown names fall back to lossy UTF-8.
pub(crate) fn decode_bytes(bytes: &[u8], encoding: Option<&str>) -> String {
    let name = encoding.unwrap_or("").trim().to_ascii_lowercase();
    match name.as_str() {
        "" | "utf-8" | "utf8" => String::from_utf8_lossy(bytes).into_owned(),
        "windows-1252" | "cp1252" | "1252" | "ansi" => decode_windows_1252(bytes),
        "latin-1" | "latin1" | "iso-8859-1" | "iso8859-1" => {
            bytes.iter().map(|b| *b as char).collect()
        }
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Maps one Windows-1252 byte `0x80`–`0x9F` to its Unicode scalar.
/// Undefined slots (`0x81`, `0x8D`, `0x8F`, `0x90`, `0x9D`) yield
/// U+FFFD, matching lossy-decode conventions.
fn cp1252_byte(byte: u8) -> char {
    const TABLE: [char; 32] = [
        '€', '\u{FFFD}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{FFFD}', 'Ž',
        '\u{FFFD}', '\u{FFFD}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ',
        '\u{FFFD}', 'ž', 'Ÿ',
    ];
    TABLE
        .get(byte.wrapping_sub(0x80) as usize)
        .copied()
        .unwrap_or('\u{FFFD}')
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if !(0x80..0xA0).contains(&b) {
            out.push(b as char);
        } else {
            out.push(cp1252_byte(b));
        }
    }
    out
}

/// Normalises line endings of decoded text when an `eol` policy is
/// set (`Some`, any value): CRLF folds to LF, then any remaining lone
/// CR folds to LF, so old-Mac (`\r`) files compare equal to Unix
/// (`\n`) files. With `None` the text passes through untouched
/// (legacy display split in [`split_lines`] still strips one trailing
/// `\r` per `\n` line, so CRLF-vs-LF files already compare equal).
fn normalize_eol_text(text: &str, eol: Option<&str>) -> String {
    if eol.is_none() {
        return text.to_owned();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Strips one trailing `\r` from an in-memory line when an `eol`
/// policy is set. Display lines are untouched; only comparison keys
/// go through here.
fn normalize_eol_line<'a>(line: &'a str, eol: Option<&str>) -> &'a str {
    if eol.is_some() {
        line.strip_suffix('\r').unwrap_or(line)
    } else {
        line
    }
}

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

/// Reads one file and decodes it per `opts.encoding`, honouring the
/// caller's declared encoding instead of assuming UTF-8. EOL
/// normalisation per `opts.eol` applies before splitting.
fn load_lines_with_opts(path: &Path, opts: &TextOptions) -> Result<Vec<String>> {
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
    let text = decode_bytes(&bytes, opts.encoding.as_deref());
    Ok(split_lines(&normalize_eol_text(&text, opts.eol.as_deref())))
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

/// Compares two text files line by line. Files are decoded per
/// `opts.encoding` and line endings normalised per `opts.eol`; see
/// the module documentation. Display lines are the decoded display
/// lines, never the comparison keys.
pub(crate) fn compare_text_files(
    left: &Path,
    right: &Path,
    opts: &TextOptions,
) -> Result<TextComparison> {
    let left_lines = load_lines_with_opts(left, opts)?;
    let right_lines = load_lines_with_opts(right, opts)?;
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
    let eol = opts.eol.as_deref();
    let left_keys: Vec<String> = left_lines
        .iter()
        .map(|line| comparison_key(normalize_eol_line(line, eol), opts))
        .collect();
    let right_keys: Vec<String> = right_lines
        .iter()
        .map(|line| comparison_key(normalize_eol_line(line, eol), opts))
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
    use autocompare_diff::Op;

    /// True when every op is `Equal` (inputs identical under the keys).
    /// `script.exact` only reports the edit budget, not equality.
    fn all_equal(script: &autocompare_diff::DiffScript) -> bool {
        script
            .ops
            .iter()
            .all(|op| matches!(op, Op::Equal { .. }))
    }

    #[test]
    fn windows_1252_bytes_decode_correctly() {
        // 0xE9 alone is invalid UTF-8 (lossy => U+FFFD) but é in 1252.
        let bytes = vec![0x48, 0xE9, 0x6C, 0x6C, 0x6F, 0x80, 0x93, 0x81];
        let decoded = decode_bytes(&bytes, Some("windows-1252"));
        assert_eq!(decoded, "Héllo€\u{201C}\u{FFFD}");
        // Case-insensitive + alias.
        assert_eq!(decode_bytes(&bytes, Some("  CP1252 ")), decoded);
        // latin-1 maps every byte to the same code point.
        assert_eq!(
            decode_bytes(&[0xE9, 0x80], Some("latin-1")),
            "\u{E9}\u{80}"
        );
        // Default / unknown falls back to lossy UTF-8.
        let lossy = String::from_utf8_lossy(&bytes).into_owned();
        assert_eq!(decode_bytes(&bytes, None), lossy);
        assert_eq!(decode_bytes(&bytes, Some("bogus-enc")), lossy);
        // Valid UTF-8 round-trips under the UTF-8 names; pure ASCII
        // round-trips under windows-1252 too (UTF-8 multibyte sequences
        // do NOT — 0xC3 0xA9 decodes as two 1252 chars, by design).
        let utf8 = "café €".as_bytes();
        for enc in [None, Some("utf-8"), Some("UTF8")] {
            assert_eq!(decode_bytes(utf8, enc), "café €");
        }
        assert_eq!(decode_bytes(b"plain ascii", Some("windows-1252")), "plain ascii");
    }

    #[test]
    fn eol_option_normalises_cr_crlf_lf_before_diff() {
        let lf = TextOptions {
            eol: Some("lf".to_string()),
            ..TextOptions::default()
        };
        // Lone-CR file text folds to LF when eol is set ...
        assert_eq!(normalize_eol_text("a\rb\r\nc\n", lf.eol.as_deref()), "a\nb\nc\n");
        // ... and passes through untouched without it.
        assert_eq!(normalize_eol_text("a\rb\r\nc\n", None), "a\rb\r\nc\n");
        // In-memory lines with trailing CR compare equal iff eol set.
        let l = vec!["a\r".to_string(), "b".to_string()];
        let r = vec!["a".to_string(), "b".to_string()];
        assert!(all_equal(&compare_text_lines(&l, &r, &lf).script));
        assert!(!all_equal(&compare_text_lines(&l, &r, &TextOptions::default()).script));
        // Display lines are never mutated by eol normalisation.
        let cmp = compare_text_lines(&l, &r, &lf);
        assert_eq!(cmp.left_lines, l);
        assert_eq!(cmp.right_lines, r);
    }

    #[test]
    fn compare_text_files_honours_encoding_and_eol_end_to_end() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("wt163c-{}-enc", std::process::id()));
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        // "Héllo€" as windows-1252 bytes (0xE9 / 0x80 are lone,
        // invalid-UTF-8 bytes). Same bytes on both sides so the only
        // variable is the declared encoding.
        let raw = [0x48, 0xE9, 0x6C, 0x6C, 0x6F, 0x80];
        let left = dir.join("l.txt");
        let right = dir.join("r.txt");
        if std::fs::write(&left, raw).is_err() || std::fs::write(&right, raw).is_err() {
            std::fs::remove_dir_all(&dir).ok();
            return;
        }
        // Default (UTF-8 lossy) decodes both lone bytes as U+FFFD.
        let plain = match compare_text_files(&left, &right, &TextOptions::default()) {
            Ok(cmp) => cmp,
            Err(_) => {
                std::fs::remove_dir_all(&dir).ok();
                return;
            }
        };
        assert!(all_equal(&plain.script));
        assert_eq!(plain.left_lines, vec!["H\u{FFFD}llo\u{FFFD}".to_string()]);
        // Declared windows-1252 decodes 0xE9 as é and 0x80 as €.
        let opts = TextOptions {
            encoding: Some("windows-1252".to_string()),
            ..TextOptions::default()
        };
        let cmp = match compare_text_files(&left, &right, &opts) {
            Ok(cmp) => cmp,
            Err(_) => {
                std::fs::remove_dir_all(&dir).ok();
                return;
            }
        };
        assert!(all_equal(&cmp.script));
        assert_eq!(cmp.left_lines, vec!["Héllo€".to_string()]);
        // CR-only vs LF file: differ without eol, equal with eol set.
        let cr = dir.join("cr.txt");
        let lf_file = dir.join("lf.txt");
        let wrote = std::fs::File::create(&cr)
            .and_then(|mut f| f.write_all(b"a\rb\n"))
            .and(std::fs::write(&lf_file, "a\nb\n"));
        if wrote.is_err() {
            std::fs::remove_dir_all(&dir).ok();
            return;
        }
        let no_eol = match compare_text_files(&cr, &lf_file, &TextOptions::default()) {
            Ok(cmp) => cmp,
            Err(_) => {
                std::fs::remove_dir_all(&dir).ok();
                return;
            }
        };
        assert!(!all_equal(&no_eol.script));
        let eol_opts = TextOptions {
            eol: Some("lf".to_string()),
            ..TextOptions::default()
        };
        let with_eol = match compare_text_files(&cr, &lf_file, &eol_opts) {
            Ok(cmp) => cmp,
            Err(_) => {
                std::fs::remove_dir_all(&dir).ok();
                return;
            }
        };
        assert!(all_equal(&with_eol.script));
        std::fs::remove_dir_all(&dir).ok();
    }

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

