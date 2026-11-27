//! Declarative file format converters for structural and normalized diffing.
//!
//! Inspired by Beyond Compare's file-format conversion pipeline and declarative
//! harness configuration: users can normalize files before diffing using built-in
//! pure-Rust transforms (`canonical_json`, `normalize_eol`) or sandboxed
//! read-only stdin→stdout external converters configured in
//! `autocompare.formats.toml`.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Hard upper bound (16 MiB) on output produced by an external converter process.
pub const MAX_CONVERTER_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// Errors returned while parsing converter config or transforming file bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatConvertError {
    /// Input bytes were not valid JSON for a JSON canonicalization rule.
    InvalidJson(String),
    /// Failed to spawn or write to an external converter executable.
    ProcessSpawn {
        /// Program name that failed to start.
        program: String,
        /// Underlying OS error message.
        message: String,
    },
    /// External converter exited with a non-zero exit status.
    ProcessFailed {
        /// Program name that failed.
        program: String,
        /// Exit code, if available.
        code: Option<i32>,
        /// Captured stderr snippet.
        stderr: String,
    },
    /// External converter output exceeded the configured byte cap.
    OutputTooLarge {
        /// Configured cap in bytes.
        limit: usize,
        /// Observed byte count.
        actual: usize,
    },
    /// Declarative TOML config contained an invalid or incomplete rule.
    InvalidConfig(String),
}

impl fmt::Display for FormatConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(msg) => write!(f, "invalid JSON input: {msg}"),
            Self::ProcessSpawn { program, message } => {
                write!(f, "failed to run converter '{program}': {message}")
            }
            Self::ProcessFailed {
                program,
                code,
                stderr,
            } => write!(
                f,
                "converter '{program}' exited with status {:?}: {}",
                code,
                stderr.trim()
            ),
            Self::OutputTooLarge { limit, actual } => write!(
                f,
                "converter output exceeded {limit} byte cap (read {actual} bytes)"
            ),
            Self::InvalidConfig(msg) => write!(f, "invalid format converter config: {msg}"),
        }
    }
}

impl std::error::Error for FormatConvertError {}

/// Transformation strategy applied to matching file bytes prior to diffing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FormatTransform {
    /// Parses JSON, optionally sorts all object keys recursively, and pretty-prints.
    CanonicalJson {
        /// Sort object keys alphabetically at every nesting depth.
        sort_keys: bool,
    },
    /// Normalizes `\r\n` / `\r` line endings to `\n`, optionally trimming trailing spaces.
    NormalizeEol {
        /// Strip trailing horizontal whitespace on each line.
        strip_trailing_whitespace: bool,
    },
    /// Pipes input bytes to a read-only external executable over stdin and reads stdout.
    ///
    /// Never invokes a shell; arguments are passed directly via `argv`.
    ExternalCommand {
        /// Executable path or name on `PATH`.
        program: String,
        /// Direct argument vector (no shell interpolation).
        args: Vec<String>,
        /// Maximum stdout bytes accepted before aborting.
        max_output_bytes: usize,
    },
}

/// A single declarative format rule mapping file extensions to a [`FormatTransform`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatRule {
    /// Human-readable rule name (e.g., `"Canonical JSON"`).
    pub name: String,
    /// Lowercase file extensions without leading dots (e.g., `["json", "arb"]`).
    pub extensions: Vec<String>,
    /// Transform applied to matching files.
    pub transform: FormatTransform,
}

impl FormatRule {
    /// Returns `true` if `path` has a matching extension (case-insensitive).
    #[must_use]
    pub fn matches_path(&self, path: &str) -> bool {
        let Some(ext) = extract_extension(path) else {
            return false;
        };
        self.extensions
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(ext))
    }
}

/// Registry of declarative format conversion rules evaluated in registration order.
#[derive(Debug, Clone, Default)]
pub struct FormatConverterRegistry {
    rules: Vec<FormatRule>,
}

impl FormatConverterRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Creates a registry with built-in safe pure-Rust converters.
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut reg = Self::empty();
        reg.register(FormatRule {
            name: "Canonical JSON".to_string(),
            extensions: vec!["json".to_string()],
            transform: FormatTransform::CanonicalJson { sort_keys: true },
        });
        reg
    }

    /// Appends a rule to the registry.
    pub fn register(&mut self, rule: FormatRule) {
        self.rules.push(rule);
    }

    /// Returns all registered rules.
    #[must_use]
    pub fn rules(&self) -> &[FormatRule] {
        &self.rules
    }

    /// Finds the first rule matching `path`'s extension.
    #[must_use]
    pub fn find_for_path(&self, path: &str) -> Option<&FormatRule> {
        self.rules.iter().find(|r| r.matches_path(path))
    }

    /// Converts `input` using the first rule matching `path`, returning `Ok(None)`
    /// when no rule matches.
    ///
    /// # Errors
    ///
    /// Returns [`FormatConvertError`] if the matched transform fails.
    pub fn convert(&self, path: &str, input: &[u8]) -> Result<Option<Vec<u8>>, FormatConvertError> {
        let Some(rule) = self.find_for_path(path) else {
            return Ok(None);
        };
        let converted = apply_rule(rule, input)?;
        Ok(Some(converted))
    }

    /// Parses a minimal `autocompare.formats.toml` document with `[[format]]` blocks.
    ///
    /// Supported keys per `[[format]]` entry:
    /// - `name = "..."`
    /// - `extensions = ["json", "arb"]`
    /// - `kind = "canonical_json" | "normalize_eol" | "external"`
    /// - `sort_keys = true | false` (for `canonical_json`)
    /// - `strip_trailing_whitespace = true | false` (for `normalize_eol`)
    /// - `program = "..."`, `args = ["..."]`, `max_output_bytes = 1048576` (for `external`)
    ///
    /// # Errors
    ///
    /// Returns [`FormatConvertError::InvalidConfig`] on malformed syntax or missing fields.
    pub fn from_toml(toml_text: &str) -> Result<Self, FormatConvertError> {
        let mut reg = Self::empty();
        let mut current: Option<BTreeMap<String, String>> = None;

        for (line_idx, raw_line) in toml_text.lines().enumerate() {
            let line = strip_toml_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            if line == "[[format]]" {
                if let Some(block) = current.take() {
                    reg.register(build_rule_from_kv(&block)?);
                }
                current = Some(BTreeMap::new());
                continue;
            }
            let Some((key, val)) = line.split_once('=') else {
                return Err(FormatConvertError::InvalidConfig(format!(
                    "line {}: expected 'key = value'",
                    line_idx.saturating_add(1)
                )));
            };
            let Some(block) = current.as_mut() else {
                return Err(FormatConvertError::InvalidConfig(format!(
                    "line {}: key '{}' appears before [[format]] header",
                    line_idx.saturating_add(1),
                    key.trim()
                )));
            };
            block.insert(key.trim().to_string(), val.trim().to_string());
        }

        if let Some(block) = current.take() {
            reg.register(build_rule_from_kv(&block)?);
        }

        Ok(reg)
    }
}

/// Applies a single [`FormatRule`] to `input` bytes.
///
/// # Errors
///
/// Propagates JSON parse errors or external process errors.
pub fn apply_rule(rule: &FormatRule, input: &[u8]) -> Result<Vec<u8>, FormatConvertError> {
    match &rule.transform {
        FormatTransform::CanonicalJson { sort_keys } => canonicalize_json(input, *sort_keys),
        FormatTransform::NormalizeEol {
            strip_trailing_whitespace,
        } => Ok(normalize_eol(input, *strip_trailing_whitespace)),
        FormatTransform::ExternalCommand {
            program,
            args,
            max_output_bytes,
        } => run_external_converter(program, args, input, *max_output_bytes),
    }
}

fn canonicalize_json(input: &[u8], sort_keys: bool) -> Result<Vec<u8>, FormatConvertError> {
    let mut value: Value =
        serde_json::from_slice(input).map_err(|e| FormatConvertError::InvalidJson(e.to_string()))?;
    if sort_keys {
        sort_json_value(&mut value);
    }
    let mut out = serde_json::to_vec_pretty(&value)
        .map_err(|e| FormatConvertError::InvalidJson(e.to_string()))?;
    out.push(b'\n');
    Ok(out)
}

fn sort_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            let old = std::mem::take(map);
            for (k, mut v) in old {
                sort_json_value(&mut v);
                sorted.insert(k, v);
            }
            for (k, v) in sorted {
                map.insert(k, v);
            }
        }
        Value::Array(items) => {
            for item in items {
                sort_json_value(item);
            }
        }
        _ => {}
    }
}

fn normalize_eol(input: &[u8], strip_trailing_whitespace: bool) -> Vec<u8> {
    let text = String::from_utf8_lossy(input);
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if !strip_trailing_whitespace {
        return normalized.into_bytes();
    }
    let had_trailing_newline = normalized.ends_with('\n');
    let mut out = normalized
        .lines()
        .map(|line| line.trim_end_matches([' ', '\t']))
        .collect::<Vec<_>>()
        .join("\n");
    if had_trailing_newline {
        out.push('\n');
    }
    out.into_bytes()
}

fn run_external_converter(
    program: &str,
    args: &[String],
    input: &[u8],
    max_output_bytes: usize,
) -> Result<Vec<u8>, FormatConvertError> {
    let cap = max_output_bytes.min(MAX_CONVERTER_OUTPUT_BYTES);
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| FormatConvertError::ProcessSpawn {
            program: program.to_string(),
            message: e.to_string(),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input)
            .map_err(|e| FormatConvertError::ProcessSpawn {
                program: program.to_string(),
                message: e.to_string(),
            })?;
    }

    let mut out = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let limit_plus_one = (cap as u64).saturating_add(1);
        stdout
            .take(limit_plus_one)
            .read_to_end(&mut out)
            .map_err(|e| FormatConvertError::ProcessSpawn {
                program: program.to_string(),
                message: e.to_string(),
            })?;
        if out.len() > cap {
            let _ = child.kill();
            let _ = child.wait();
            return Err(FormatConvertError::OutputTooLarge {
                limit: cap,
                actual: out.len(),
            });
        }
    }

    let mut stderr_buf = Vec::new();
    if let Some(stderr) = child.stderr.take() {
        let _ = stderr.take(4096).read_to_end(&mut stderr_buf);
    }

    let status = child.wait().map_err(|e| FormatConvertError::ProcessSpawn {
        program: program.to_string(),
        message: e.to_string(),
    })?;

    if !status.success() {
        return Err(FormatConvertError::ProcessFailed {
            program: program.to_string(),
            code: status.code(),
            stderr: String::from_utf8_lossy(&stderr_buf).into_owned(),
        });
    }

    Ok(out)
}

fn extract_extension(path: &str) -> Option<&str> {
    let file_name = path.rsplit(['/', '\\']).next()?;
    let dot = file_name.rfind('.')?;
    if dot == 0 {
        return None;
    }
    file_name.get(dot.saturating_add(1)..)
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => return line.get(..idx).unwrap_or(line),
            _ => {}
        }
    }
    line
}

fn parse_toml_string(raw: &str) -> Result<String, FormatConvertError> {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        let inner = trimmed
            .get(1..trimmed.len().saturating_sub(1))
            .unwrap_or("");
        Ok(inner.to_string())
    } else {
        Err(FormatConvertError::InvalidConfig(format!(
            "expected quoted string, got '{trimmed}'"
        )))
    }
}

fn parse_toml_string_array(raw: &str) -> Result<Vec<String>, FormatConvertError> {
    let trimmed = raw.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return Err(FormatConvertError::InvalidConfig(format!(
            "expected string array [...], got '{trimmed}'"
        )));
    }
    let inner = trimmed
        .get(1..trimmed.len().saturating_sub(1))
        .unwrap_or("")
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for item in inner.split(',') {
        let part = item.trim();
        if !part.is_empty() {
            out.push(parse_toml_string(part)?);
        }
    }
    Ok(out)
}

fn parse_toml_bool(raw: &str) -> Result<bool, FormatConvertError> {
    match raw.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(FormatConvertError::InvalidConfig(format!(
            "expected boolean true/false, got '{other}'"
        ))),
    }
}

fn build_rule_from_kv(kv: &BTreeMap<String, String>) -> Result<FormatRule, FormatConvertError> {
    let name = kv
        .get("name")
        .ok_or_else(|| FormatConvertError::InvalidConfig("missing 'name'".to_string()))
        .and_then(|v| parse_toml_string(v))?;

    let extensions = kv
        .get("extensions")
        .ok_or_else(|| FormatConvertError::InvalidConfig("missing 'extensions'".to_string()))
        .and_then(|v| parse_toml_string_array(v))?
        .into_iter()
        .map(|s| s.trim_start_matches('.').to_ascii_lowercase())
        .collect::<Vec<_>>();

    let kind = kv
        .get("kind")
        .ok_or_else(|| FormatConvertError::InvalidConfig("missing 'kind'".to_string()))
        .and_then(|v| parse_toml_string(v))?;

    let transform = match kind.as_str() {
        "canonical_json" => {
            let sort_keys = match kv.get("sort_keys") {
                Some(v) => parse_toml_bool(v)?,
                None => true,
            };
            FormatTransform::CanonicalJson { sort_keys }
        }
        "normalize_eol" => {
            let strip = match kv.get("strip_trailing_whitespace") {
                Some(v) => parse_toml_bool(v)?,
                None => false,
            };
            FormatTransform::NormalizeEol {
                strip_trailing_whitespace: strip,
            }
        }
        "external" => {
            let program = kv
                .get("program")
                .ok_or_else(|| {
                    FormatConvertError::InvalidConfig(
                        "external converter requires 'program'".to_string(),
                    )
                })
                .and_then(|v| parse_toml_string(v))?;
            let args = match kv.get("args") {
                Some(v) => parse_toml_string_array(v)?,
                None => Vec::new(),
            };
            let max_output_bytes = match kv.get("max_output_bytes") {
                Some(v) => v.trim().parse::<usize>().map_err(|_| {
                    FormatConvertError::InvalidConfig(format!("invalid max_output_bytes: '{v}'"))
                })?,
                None => MAX_CONVERTER_OUTPUT_BYTES,
            };
            FormatTransform::ExternalCommand {
                program,
                args,
                max_output_bytes,
            }
        }
        other => {
            return Err(FormatConvertError::InvalidConfig(format!(
                "unsupported converter kind '{other}'"
            )));
        }
    };

    Ok(FormatRule {
        name,
        extensions,
        transform,
    })
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
    fn canonical_json_sorts_nested_keys_deterministically() {
        let reg = FormatConverterRegistry::with_builtins();
        let left = br#"{"z": 1, "a": {"d": true, "b": [2, 1]}}"#;
        let right = br#"{"a": {"b": [2, 1], "d": true}, "z": 1}"#;

        let out_l = reg
            .convert("config/app.json", left)
            .expect("convert left")
            .expect("matched json");
        let out_r = reg
            .convert("config/app.json", right)
            .expect("convert right")
            .expect("matched json");

        assert_eq!(out_l, out_r);
        assert!(reg.convert("README.md", b"hello").expect("ok").is_none());
    }

    #[test]
    fn parses_declarative_formats_toml_and_normalizes_eol() {
        let toml = r#"
            # User-defined format rules
            [[format]]
            name = "Sorted JSON"
            extensions = ["json", "arb"]
            kind = "canonical_json"
            sort_keys = true

            [[format]]
            name = "Clean Text"
            extensions = ["txt", "cfg"]
            kind = "normalize_eol"
            strip_trailing_whitespace = true
        "#;

        let reg = FormatConverterRegistry::from_toml(toml).expect("parse toml");
        assert_eq!(reg.rules().len(), 2);

        let cleaned = reg
            .convert("notes.cfg", b"alpha   \r\nbeta\t\r\n")
            .expect("convert cfg")
            .expect("matched cfg");
        assert_eq!(String::from_utf8_lossy(&cleaned), "alpha\nbeta\n");
    }

    #[test]
    fn rejects_invalid_json_cleanly() {
        let reg = FormatConverterRegistry::with_builtins();
        let err = reg.convert("broken.json", b"{not valid}").unwrap_err();
        assert!(matches!(err, FormatConvertError::InvalidJson(_)));
    }
}
