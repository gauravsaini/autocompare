//! Grammar and unimportant-text rules: whitespace, casing, line endings, comments,
//! and string literals filtering, plus grammar-aware word-level intra-line diffing
//! and Beyond Compare parity line diffing.
//!
//! Layer 0: no panics, no unwrap/expect in non-test code, no unsafe.

use crate::{InlineSpan, Op, engine};

/// Rules for handling line ending differences (CRLF vs LF vs CR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEndingRule {
    /// Compare line endings strictly (CRLF != LF).
    Compare,
    /// Ignore differences in line endings (treat CRLF, LF, and CR as identical).
    #[default]
    Ignore,
}

/// Detected line ending convention of a text buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEndingType {
    /// Unix standard (`\n`).
    #[default]
    Lf,
    /// Windows standard (`\r\n`).
    Crlf,
    /// Classic Mac standard (`\r`).
    Cr,
    /// Inconsistent mixed line endings.
    Mixed,
    /// Empty text with no line endings.
    None,
}

/// Detects the line ending convention used in the provided text.
pub fn detect_line_ending(text: &str) -> LineEndingType {
    let mut has_crlf = false;
    let mut has_lf = false;
    let mut has_cr = false;

    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes.get(i) == Some(&b'\r') {
            if bytes.get(i.saturating_add(1)) == Some(&b'\n') {
                has_crlf = true;
                i = i.saturating_add(2);
                continue;
            } else {
                has_cr = true;
            }
        } else if bytes.get(i) == Some(&b'\n') {
            has_lf = true;
        }
        i = i.saturating_add(1);
    }

    match (has_crlf, has_lf, has_cr) {
        (false, false, false) => LineEndingType::None,
        (true, false, false) => LineEndingType::Crlf,
        (false, true, false) => LineEndingType::Lf,
        (false, false, true) => LineEndingType::Cr,
        _ => LineEndingType::Mixed,
    }
}

/// Normalizes line endings in text to the target line ending convention.
pub fn normalize_line_endings(text: &str, target: LineEndingType) -> String {
    let sep = match target {
        LineEndingType::Crlf => "\r\n",
        LineEndingType::Cr => "\r",
        LineEndingType::Lf | LineEndingType::Mixed | LineEndingType::None => "\n",
    };

    let mut out = String::with_capacity(text.len());
    let mut lines = text.split('\n').peekable();

    while let Some(raw_line) = lines.next() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        out.push_str(line);
        if lines.peek().is_some() {
            out.push_str(sep);
        }
    }

    out
}

/// Rules for handling whitespace differences between texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhitespaceRule {
    /// Compare whitespace strictly and character-for-character.
    #[default]
    CompareAll,
    /// Ignore differences in leading whitespace (indentation).
    IgnoreLeading,
    /// Ignore differences in trailing whitespace.
    IgnoreTrailing,
    /// Ignore differences in both leading and trailing whitespace.
    IgnoreLeadingAndTrailing,
    /// Treat any run of whitespace as a single space, and ignore edge whitespace.
    IgnoreChanges,
    /// Collapse any contiguous sequence of whitespace characters into a single space,
    /// preserving surrounding structure.
    CollapseRuns,
    /// Ignore all whitespace characters entirely.
    IgnoreAll,
}

/// A rule describing comment syntax in a grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentRule {
    /// Prefix for a single-line comment (e.g. `//`, `#`, `--`).
    pub line_prefix: Option<String>,
    /// Opening delimiter for a multi-line block comment (e.g. `/*`).
    pub block_start: Option<String>,
    /// Closing delimiter for a multi-line block comment (e.g. `*/`).
    pub block_end: Option<String>,
    /// Whether block comments can nest inside each other (e.g. Rust, Swift).
    pub nested: bool,
}

impl CommentRule {
    /// Common C-style comments (`//` and `/* ... */`, non-nested).
    pub fn c_style() -> Self {
        Self {
            line_prefix: Some("//".to_string()),
            block_start: Some("/*".to_string()),
            block_end: Some("*/".to_string()),
            nested: false,
        }
    }

    /// Rust and Swift style comments (`//` and nested `/* ... */`).
    pub fn rust_style() -> Self {
        Self {
            line_prefix: Some("//".to_string()),
            block_start: Some("/*".to_string()),
            block_end: Some("*/".to_string()),
            nested: true,
        }
    }

    /// Common Python/Shell-style line comments (`#`).
    pub fn hash_style() -> Self {
        Self {
            line_prefix: Some("#".to_string()),
            block_start: None,
            block_end: None,
            nested: false,
        }
    }

    /// Python style comments (alias for hash style).
    pub fn python_style() -> Self {
        Self::hash_style()
    }

    /// Common SQL-style line comments (`--` and `/* ... */`).
    pub fn sql_style() -> Self {
        Self {
            line_prefix: Some("--".to_string()),
            block_start: Some("/*".to_string()),
            block_end: Some("*/".to_string()),
            nested: false,
        }
    }

    /// HTML / XML comments (`<!-- ... -->`).
    pub fn xml_style() -> Self {
        Self {
            line_prefix: None,
            block_start: Some("<!--".to_string()),
            block_end: Some("-->".to_string()),
            nested: false,
        }
    }

    /// Pascal / Delphi style comments (`{ ... }` and `(* ... *)`).
    pub fn pascal_style() -> Self {
        Self {
            line_prefix: Some("//".to_string()),
            block_start: Some("{".to_string()),
            block_end: Some("}".to_string()),
            nested: false,
        }
    }

    /// Lua style comments (`--` and `--[[ ... ]]`).
    pub fn lua_style() -> Self {
        Self {
            line_prefix: Some("--".to_string()),
            block_start: Some("--[[".to_string()),
            block_end: Some("]]".to_string()),
            nested: false,
        }
    }
}

/// A rule describing string literal syntax in a grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteralRule {
    /// Opening quote delimiter (e.g. `"` or `'` or `"""`).
    pub start: String,
    /// Closing quote delimiter (e.g. `"` or `'` or `"""`).
    pub end: String,
    /// Escape character (e.g. `\` for `\"`).
    pub escape: Option<char>,
    /// Whether this string literal can span multiple lines.
    pub multiline: bool,
}

impl StringLiteralRule {
    /// Standard double-quoted string literal (`"..."` with `\"` escape).
    pub fn double_quote() -> Self {
        Self {
            start: "\"".to_string(),
            end: "\"".to_string(),
            escape: Some('\\'),
            multiline: false,
        }
    }

    /// Standard single-quoted string literal (`'...'` with `\'` escape).
    pub fn single_quote() -> Self {
        Self {
            start: "'".to_string(),
            end: "'".to_string(),
            escape: Some('\\'),
            multiline: false,
        }
    }

    /// Python-style triple double-quoted multiline string (`"""..."""`).
    pub fn python_multiline() -> Self {
        Self {
            start: "\"\"\"".to_string(),
            end: "\"\"\"".to_string(),
            escape: Some('\\'),
            multiline: true,
        }
    }

    /// Javascript/Typescript template literal (`` `...` `` with `\`` escape, multiline).
    pub fn backtick_multiline() -> Self {
        Self {
            start: "`".to_string(),
            end: "`".to_string(),
            escape: Some('\\'),
            multiline: true,
        }
    }
}

/// Tracks block comment state across lines, including nested comments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommentState {
    /// If currently inside an open block comment, stores the expected opening delimiter.
    pub active_block_start: Option<String>,
    /// If currently inside an open block comment, stores the expected closing delimiter.
    pub active_block_end: Option<String>,
    /// Current comment nesting depth (for nested comment grammars).
    pub nesting_depth: usize,
    /// Whether the active block comment supports nesting.
    pub is_nested: bool,
}

impl CommentState {
    /// Creates a fresh inactive comment state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if currently inside an unclosed block comment.
    pub fn is_inside_comment(&self) -> bool {
        self.nesting_depth > 0 || self.active_block_end.is_some()
    }
}

/// Tracks multiline string literal state across lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringLiteralState {
    /// If currently inside a multiline string literal, stores the expected closing delimiter.
    pub active_end_delim: Option<String>,
    /// Escape character for the active string literal.
    pub escape_char: Option<char>,
}

impl StringLiteralState {
    /// Creates a fresh inactive string literal state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if currently inside an unclosed string literal.
    pub fn is_inside_string(&self) -> bool {
        self.active_end_delim.is_some()
    }
}

/// Combined parser state across lines for comments and string literals.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParseState {
    /// Comment tracking state.
    pub comment: CommentState,
    /// String literal tracking state.
    pub string: StringLiteralState,
}

impl ParseState {
    /// Creates a fresh parse state.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Grammar comparison options governing unimportant differences.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GrammarOptions {
    /// If true, case differences are considered unimportant.
    pub ignore_case: bool,
    /// Whitespace comparison rule.
    pub whitespace: WhitespaceRule,
    /// Line ending comparison rule.
    pub line_ending: LineEndingRule,
    /// If true, differences inside comments are considered unimportant.
    pub ignore_comments: bool,
    /// Configured comment syntax rules.
    pub comment_rules: Vec<CommentRule>,
    /// Configured string literal syntax rules.
    pub string_rules: Vec<StringLiteralRule>,
    /// If true, string literal differences are considered unimportant.
    pub ignore_string_contents: bool,
    /// If true, completely blank lines are ignored.
    pub ignore_blank_lines: bool,
}

impl GrammarOptions {
    /// Preset that ignores all whitespace differences, line endings, comments, and casing.
    pub fn loose() -> Self {
        Self {
            ignore_case: true,
            whitespace: WhitespaceRule::IgnoreChanges,
            line_ending: LineEndingRule::Ignore,
            ignore_comments: true,
            comment_rules: vec![CommentRule::rust_style()],
            string_rules: vec![
                StringLiteralRule::double_quote(),
                StringLiteralRule::single_quote(),
            ],
            ignore_string_contents: false,
            ignore_blank_lines: true,
        }
    }
}

/// Classification of a difference between two lines or spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Importance {
    /// The texts are identical.
    Identical,
    /// The texts differ only in unimportant aspects (whitespace, case, comments, blank lines).
    UnimportantDifference,
    /// The texts contain meaningful/important semantic differences.
    ImportantDifference,
}

/// Checks whether a line consists purely of whitespace or is empty.
pub fn is_blank_line(line: &str) -> bool {
    line.chars().all(|c| c.is_whitespace())
}

/// Strips comments (both line comments and block comments) from a single line.
pub fn strip_comments(line: &str, rules: &[CommentRule]) -> String {
    let mut state = CommentState::default();
    strip_comments_with_state(line, rules, &mut state)
}

/// Strips comments from a line while persisting multi-line comment state (including nested comments).
pub fn strip_comments_with_state(
    line: &str,
    rules: &[CommentRule],
    state: &mut CommentState,
) -> String {
    let mut parse_state = ParseState {
        comment: state.clone(),
        string: StringLiteralState::default(),
    };
    let out = strip_comments_and_strings_with_state(line, rules, &[], &mut parse_state, false);
    *state = parse_state.comment;
    out
}

/// Internal helper to strip comments and optionally string literals with full parser state.
fn strip_comments_and_strings_with_state(
    line: &str,
    comment_rules: &[CommentRule],
    string_rules: &[StringLiteralRule],
    state: &mut ParseState,
    ignore_strings: bool,
) -> String {
    let mut out = String::with_capacity(line.len());
    let mut curr_boff = 0usize;

    while curr_boff < line.len() {
        // 1. If currently inside a multiline comment:
        if state.comment.is_inside_comment() {
            let bend = state.comment.active_block_end.clone().unwrap_or_default();
            let bstart = state.comment.active_block_start.clone().unwrap_or_default();
            let is_nested = state.comment.is_nested;
            let rest = line.get(curr_boff..).unwrap_or("");

            let next_end = if !bend.is_empty() {
                rest.find(&bend)
            } else {
                None
            };
            let next_start = if is_nested && !bstart.is_empty() {
                rest.find(&bstart)
            } else {
                None
            };

            match (next_start, next_end) {
                (Some(s_pos), Some(e_pos)) if s_pos < e_pos => {
                    state.comment.nesting_depth = state.comment.nesting_depth.saturating_add(1);
                    curr_boff = curr_boff.saturating_add(s_pos).saturating_add(bstart.len());
                }
                (_, Some(e_pos)) => {
                    if is_nested && state.comment.nesting_depth > 1 {
                        state.comment.nesting_depth = state.comment.nesting_depth.saturating_sub(1);
                    } else {
                        state.comment.nesting_depth = 0;
                        state.comment.active_block_start = None;
                        state.comment.active_block_end = None;
                        state.comment.is_nested = false;
                    }
                    curr_boff = curr_boff.saturating_add(e_pos).saturating_add(bend.len());
                    if !out.ends_with(' ') && !out.is_empty() {
                        out.push(' ');
                    }
                }
                (Some(s_pos), None) => {
                    state.comment.nesting_depth = state.comment.nesting_depth.saturating_add(1);
                    curr_boff = curr_boff.saturating_add(s_pos).saturating_add(bstart.len());
                }
                (None, None) => {
                    return out;
                }
            }
            continue;
        }

        // 2. If currently inside a multiline string:
        if state.string.is_inside_string() {
            let send = state.string.active_end_delim.clone().unwrap_or_default();
            let esc = state.string.escape_char;
            let rest = line.get(curr_boff..).unwrap_or("");
            let mut found_end: Option<usize> = None;
            let mut byte_idx = 0usize;
            let mut is_escaped = false;

            while byte_idx < rest.len() {
                let sub = rest.get(byte_idx..).unwrap_or("");
                if let Some(ec) = esc
                    && sub.starts_with(ec)
                {
                    is_escaped = !is_escaped;
                    byte_idx = byte_idx.saturating_add(ec.len_utf8());
                    continue;
                }

                if !is_escaped && sub.starts_with(&send) {
                    found_end = Some(byte_idx);
                    break;
                }
                is_escaped = false;
                let ch_len = sub.chars().next().map_or(1, |c| c.len_utf8());
                byte_idx = byte_idx.saturating_add(ch_len);
            }

            if let Some(pos) = found_end {
                let end_boff = curr_boff.saturating_add(pos).saturating_add(send.len());
                if ignore_strings {
                    out.push_str("\"\"");
                } else {
                    out.push_str(line.get(curr_boff..end_boff).unwrap_or(""));
                }
                curr_boff = end_boff;
                state.string.active_end_delim = None;
                state.string.escape_char = None;
            } else {
                if !ignore_strings {
                    out.push_str(rest);
                }
                return out;
            }
            continue;
        }

        // 3. Normal scanning: look for strings or comments starting at or after curr_boff
        let rest = line.get(curr_boff..).unwrap_or("");
        if rest.is_empty() {
            break;
        }

        // Check if string literal opens first
        let mut string_candidate: Option<(usize, &StringLiteralRule)> = None;
        for srule in string_rules {
            if let Some(pos) = rest.find(&srule.start) {
                match string_candidate {
                    Some((best_pos, _)) if pos < best_pos => {
                        string_candidate = Some((pos, srule));
                    }
                    None => {
                        string_candidate = Some((pos, srule));
                    }
                    _ => {}
                }
            }
        }

        // Check if comment opens
        let mut line_comment_pos: Option<usize> = None;
        let mut block_comment_candidate: Option<(usize, &CommentRule)> = None;

        for crule in comment_rules {
            if let Some(ref prefix) = crule.line_prefix
                && let Some(pos) = rest.find(prefix)
            {
                line_comment_pos = Some(line_comment_pos.map_or(pos, |p| p.min(pos)));
            }
            if let (Some(bstart), Some(_)) = (&crule.block_start, &crule.block_end)
                && let Some(pos) = rest.find(bstart)
            {
                match block_comment_candidate {
                    Some((best_pos, _)) if pos < best_pos => {
                        block_comment_candidate = Some((pos, crule));
                    }
                    None => {
                        block_comment_candidate = Some((pos, crule));
                    }
                    _ => {}
                }
            }
        }

        // Determine which syntax construct appears earliest
        let s_pos = string_candidate.map(|(p, _)| p);
        let lc_pos = line_comment_pos;
        let bc_pos = block_comment_candidate.map(|(p, _)| p);

        let min_pos = s_pos.into_iter().chain(lc_pos).chain(bc_pos).min();

        match min_pos {
            None => {
                out.push_str(rest);
                break;
            }
            Some(pos) => {
                // If a string literal starts earliest:
                if s_pos == Some(pos)
                    && let Some((_, srule)) = string_candidate
                {
                    out.push_str(rest.get(..pos).unwrap_or(""));
                    let after_start = pos.saturating_add(srule.start.len());
                    let inside = rest.get(after_start..).unwrap_or("");
                    let mut found_close: Option<usize> = None;
                    let mut bidx = 0usize;
                    let mut is_escaped = false;

                    while bidx < inside.len() {
                        let sub = inside.get(bidx..).unwrap_or("");
                        if let Some(ec) = srule.escape
                            && sub.starts_with(ec)
                        {
                            is_escaped = !is_escaped;
                            bidx = bidx.saturating_add(ec.len_utf8());
                            continue;
                        }

                        if !is_escaped && sub.starts_with(&srule.end) {
                            found_close = Some(bidx);
                            break;
                        }
                        is_escaped = false;
                        let ch_len = sub.chars().next().map_or(1, |c| c.len_utf8());
                        bidx = bidx.saturating_add(ch_len);
                    }

                    if let Some(close_offset) = found_close {
                        let total_len = after_start
                            .saturating_add(close_offset)
                            .saturating_add(srule.end.len());
                        if ignore_strings {
                            out.push_str("\"\"");
                        } else {
                            out.push_str(rest.get(pos..total_len).unwrap_or(""));
                        }
                        curr_boff = curr_boff.saturating_add(total_len);
                    } else if srule.multiline {
                        state.string.active_end_delim = Some(srule.end.clone());
                        state.string.escape_char = srule.escape;
                        if !ignore_strings {
                            out.push_str(rest.get(pos..).unwrap_or(""));
                        }
                        return out;
                    } else {
                        // Unclosed single-line string literal: copy remainder
                        out.push_str(rest.get(pos..).unwrap_or(""));
                        break;
                    }
                } else if lc_pos == Some(pos) {
                    // Line comment appears earliest: truncate line
                    out.push_str(rest.get(..pos).unwrap_or(""));
                    break;
                } else if let Some((_, crule)) = block_comment_candidate {
                    // Block comment appears earliest
                    out.push_str(rest.get(..pos).unwrap_or(""));
                    let bstart = crule.block_start.as_deref().unwrap_or("");
                    let bend = crule.block_end.as_deref().unwrap_or("");
                    let after_bstart = pos.saturating_add(bstart.len());
                    let inside = rest.get(after_bstart..).unwrap_or("");

                    if crule.nested {
                        state.comment.active_block_start = Some(bstart.to_string());
                        state.comment.active_block_end = Some(bend.to_string());
                        state.comment.nesting_depth = 1;
                        state.comment.is_nested = true;
                        curr_boff = curr_boff.saturating_add(after_bstart);
                    } else if let Some(end_pos) = inside.find(bend) {
                        curr_boff = curr_boff
                            .saturating_add(after_bstart)
                            .saturating_add(end_pos)
                            .saturating_add(bend.len());
                        if !out.ends_with(' ') && !out.is_empty() {
                            out.push(' ');
                        }
                    } else {
                        state.comment.active_block_start = Some(bstart.to_string());
                        state.comment.active_block_end = Some(bend.to_string());
                        state.comment.nesting_depth = 1;
                        state.comment.is_nested = false;
                        return out;
                    }
                }
            }
        }
    }

    out
}

/// Normalizes a single line according to grammar whitespace, line endings, and casing rules.
pub fn normalize_line(line: &str, options: &GrammarOptions) -> String {
    let mut state = ParseState::default();
    normalize_line_with_parse_state(line, options, &mut state)
}

/// Normalizes a single line while tracking persistent comment state across lines.
pub fn normalize_line_with_state(
    line: &str,
    options: &GrammarOptions,
    state: &mut CommentState,
) -> String {
    let mut parse_state = ParseState {
        comment: state.clone(),
        string: StringLiteralState::default(),
    };
    let res = normalize_line_with_parse_state(line, options, &mut parse_state);
    *state = parse_state.comment;
    res
}

/// Normalizes a single line using complete persistent parsing state (comments + strings).
pub fn normalize_line_with_parse_state(
    line: &str,
    options: &GrammarOptions,
    state: &mut ParseState,
) -> String {
    // Strip trailing line ending characters if ignore_line_endings is active
    let clean_line = if options.line_ending == LineEndingRule::Ignore {
        line.strip_suffix("\r\n")
            .or_else(|| line.strip_suffix('\r'))
            .or_else(|| line.strip_suffix('\n'))
            .unwrap_or(line)
    } else {
        line
    };

    let mut s = if options.ignore_comments || options.ignore_string_contents {
        strip_comments_and_strings_with_state(
            clean_line,
            &options.comment_rules,
            &options.string_rules,
            state,
            options.ignore_string_contents,
        )
    } else {
        clean_line.to_string()
    };

    if options.ignore_blank_lines && is_blank_line(&s) {
        return String::new();
    }

    match options.whitespace {
        WhitespaceRule::CompareAll => {}
        WhitespaceRule::IgnoreLeading => {
            s = s.trim_start().to_string();
        }
        WhitespaceRule::IgnoreTrailing => {
            s = s.trim_end().to_string();
        }
        WhitespaceRule::IgnoreLeadingAndTrailing => {
            s = s.trim().to_string();
        }
        WhitespaceRule::IgnoreChanges => {
            let mut normalized = String::with_capacity(s.len());
            let mut in_space = false;
            for ch in s.trim().chars() {
                if ch.is_whitespace() {
                    if !in_space {
                        normalized.push(' ');
                        in_space = true;
                    }
                } else {
                    normalized.push(ch);
                    in_space = false;
                }
            }
            s = normalized;
        }
        WhitespaceRule::CollapseRuns => {
            let mut normalized = String::with_capacity(s.len());
            let mut in_space = false;
            for ch in s.chars() {
                if ch.is_whitespace() {
                    if !in_space {
                        normalized.push(' ');
                        in_space = true;
                    }
                } else {
                    normalized.push(ch);
                    in_space = false;
                }
            }
            s = normalized;
        }
        WhitespaceRule::IgnoreAll => {
            s = s.chars().filter(|c| !c.is_whitespace()).collect();
        }
    }

    if options.ignore_case {
        s = s.to_lowercase();
    }

    s
}

/// Normalizes a sequence of lines using grammar rules and persistent comment state.
pub fn normalize_lines(lines: &[&str], options: &GrammarOptions) -> Vec<String> {
    let mut state = ParseState::default();
    let mut out = Vec::with_capacity(lines.len());
    for &line in lines {
        out.push(normalize_line_with_parse_state(line, options, &mut state));
    }
    out
}

/// Assesses the difference importance between two lines.
pub fn compare_lines_importance(a: &str, b: &str, options: &GrammarOptions) -> Importance {
    if a == b {
        return Importance::Identical;
    }

    if options.ignore_blank_lines && is_blank_line(a) && is_blank_line(b) {
        return Importance::UnimportantDifference;
    }

    let norm_a = normalize_line(a, options);
    let norm_b = normalize_line(b, options);

    if norm_a == norm_b {
        Importance::UnimportantDifference
    } else {
        Importance::ImportantDifference
    }
}

// ----------------------------------------------------------------------------
// Word-level intra-line diff with grammar awareness
// ----------------------------------------------------------------------------

/// Kind of token extracted during line analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Whitespace,
    Punctuation,
    Comment,
    StringLiteral,
}

/// A parsed token with character offset range [start, end) and importance tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarToken<'a> {
    pub text: &'a str,
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

/// Tokenizes a line into words, punctuation, whitespace, comments, and strings.
pub fn tokenize_line<'a>(line: &'a str, rules: &[CommentRule]) -> Vec<GrammarToken<'a>> {
    let mut state = ParseState::default();
    tokenize_line_with_parse_state(line, rules, &[], &mut state)
}

/// Tokenizes a line while preserving multi-line comment state across lines.
pub fn tokenize_line_with_state<'a>(
    line: &'a str,
    rules: &[CommentRule],
    state: &mut CommentState,
) -> Vec<GrammarToken<'a>> {
    let mut parse_state = ParseState {
        comment: state.clone(),
        string: StringLiteralState::default(),
    };
    let toks = tokenize_line_with_parse_state(line, rules, &[], &mut parse_state);
    *state = parse_state.comment;
    toks
}

/// Tokenizes a line with complete parser state (comments + string literals).
pub fn tokenize_line_with_parse_state<'a>(
    line: &'a str,
    comment_rules: &[CommentRule],
    string_rules: &[StringLiteralRule],
    state: &mut ParseState,
) -> Vec<GrammarToken<'a>> {
    let mut tokens = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut i = 0usize;
    let mut cpos = 0usize;

    // Check if continuing inside a block comment
    if state.comment.is_inside_comment() {
        let bend = state.comment.active_block_end.clone().unwrap_or_default();
        if !bend.is_empty()
            && let Some(pos) = line.find(&bend)
        {
            let end_byte = pos.saturating_add(bend.len());
            let mut close_chars = 0usize;
            while i < chars.len() {
                if let Some(&(boff, _)) = chars.get(i) {
                    if boff < end_byte {
                        close_chars = close_chars.saturating_add(1);
                        i = i.saturating_add(1);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            tokens.push(GrammarToken {
                text: line.get(..end_byte).unwrap_or(""),
                start: cpos,
                end: cpos.saturating_add(close_chars),
                kind: TokenKind::Comment,
            });
            cpos = cpos.saturating_add(close_chars);
            state.comment.nesting_depth = 0;
            state.comment.active_block_start = None;
            state.comment.active_block_end = None;
            state.comment.is_nested = false;
        } else {
            let rest_chars = chars.len().saturating_sub(i);
            tokens.push(GrammarToken {
                text: line,
                start: cpos,
                end: cpos.saturating_add(rest_chars),
                kind: TokenKind::Comment,
            });
            return tokens;
        }
    }

    // Check if continuing inside a multiline string
    if state.string.is_inside_string() {
        let send = state.string.active_end_delim.clone().unwrap_or_default();
        if !send.is_empty()
            && let Some(pos) = line.find(&send)
        {
            let end_byte = pos.saturating_add(send.len());
            let mut close_chars = 0usize;
            while i < chars.len() {
                if let Some(&(boff, _)) = chars.get(i) {
                    if boff < end_byte {
                        close_chars = close_chars.saturating_add(1);
                        i = i.saturating_add(1);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            tokens.push(GrammarToken {
                text: line.get(..end_byte).unwrap_or(""),
                start: cpos,
                end: cpos.saturating_add(close_chars),
                kind: TokenKind::StringLiteral,
            });
            cpos = cpos.saturating_add(close_chars);
            state.string.active_end_delim = None;
            state.string.escape_char = None;
        } else {
            let rest_chars = chars.len().saturating_sub(i);
            tokens.push(GrammarToken {
                text: line,
                start: cpos,
                end: cpos.saturating_add(rest_chars),
                kind: TokenKind::StringLiteral,
            });
            return tokens;
        }
    }

    while i < chars.len() {
        let (boff, ch) = chars.get(i).copied().unwrap_or((0, ' '));
        let rest = line.get(boff..).unwrap_or("");

        // Check string literals
        let mut string_match: Option<&StringLiteralRule> = None;
        for srule in string_rules {
            if rest.starts_with(&srule.start) {
                string_match = Some(srule);
                break;
            }
        }

        if let Some(srule) = string_match {
            let after_start = srule.start.len();
            let inside = rest.get(after_start..).unwrap_or("");
            let mut found_close: Option<usize> = None;
            let mut bidx = 0usize;
            let mut is_escaped = false;

            while bidx < inside.len() {
                let sub = inside.get(bidx..).unwrap_or("");
                if let Some(ec) = srule.escape
                    && sub.starts_with(ec)
                {
                    is_escaped = !is_escaped;
                    bidx = bidx.saturating_add(ec.len_utf8());
                    continue;
                }
                if !is_escaped && sub.starts_with(&srule.end) {
                    found_close = Some(bidx);
                    break;
                }
                is_escaped = false;
                let ch_len = sub.chars().next().map_or(1, |c| c.len_utf8());
                bidx = bidx.saturating_add(ch_len);
            }

            if let Some(close_offset) = found_close {
                let total_bytes = after_start
                    .saturating_add(close_offset)
                    .saturating_add(srule.end.len());
                let target_byte = boff.saturating_add(total_bytes);
                let start_cpos = cpos;
                let mut str_chars = 0usize;
                while i < chars.len() {
                    if let Some(&(cur_b, _)) = chars.get(i) {
                        if cur_b < target_byte {
                            str_chars = str_chars.saturating_add(1);
                            cpos = cpos.saturating_add(1);
                            i = i.saturating_add(1);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                tokens.push(GrammarToken {
                    text: line.get(boff..target_byte).unwrap_or(""),
                    start: start_cpos,
                    end: cpos,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            } else if srule.multiline {
                state.string.active_end_delim = Some(srule.end.clone());
                state.string.escape_char = srule.escape;
                let rest_chars = chars.len().saturating_sub(i);
                tokens.push(GrammarToken {
                    text: rest,
                    start: cpos,
                    end: cpos.saturating_add(rest_chars),
                    kind: TokenKind::StringLiteral,
                });
                break;
            }
        }

        // Check comment triggers at this byte position
        let mut line_comment_match = false;
        let mut block_match: Option<(&str, &str, bool)> = None;

        for rule in comment_rules {
            if let Some(ref prefix) = rule.line_prefix
                && rest.starts_with(prefix)
            {
                line_comment_match = true;
                break;
            }
            if let (Some(bstart), Some(bend)) = (&rule.block_start, &rule.block_end)
                && rest.starts_with(bstart)
            {
                block_match = Some((bstart.as_str(), bend.as_str(), rule.nested));
                break;
            }
        }

        if line_comment_match {
            let rest_chars = chars.len().saturating_sub(i);
            tokens.push(GrammarToken {
                text: rest,
                start: cpos,
                end: cpos.saturating_add(rest_chars),
                kind: TokenKind::Comment,
            });
            break;
        }

        if let Some((bstart, bend, is_nested)) = block_match {
            let after_start = bstart.len();
            let inside = rest.get(after_start..).unwrap_or("");
            if !is_nested && let Some(end_rel) = inside.find(bend) {
                let total_bytes = after_start
                    .saturating_add(end_rel)
                    .saturating_add(bend.len());
                let target_byte = boff.saturating_add(total_bytes);
                let start_cpos = cpos;
                while i < chars.len() {
                    if let Some(&(cur_b, _)) = chars.get(i) {
                        if cur_b < target_byte {
                            cpos = cpos.saturating_add(1);
                            i = i.saturating_add(1);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                tokens.push(GrammarToken {
                    text: line.get(boff..target_byte).unwrap_or(""),
                    start: start_cpos,
                    end: cpos,
                    kind: TokenKind::Comment,
                });
                continue;
            } else {
                state.comment.active_block_start = Some(bstart.to_string());
                state.comment.active_block_end = Some(bend.to_string());
                state.comment.nesting_depth = 1;
                state.comment.is_nested = is_nested;
                let rest_chars = chars.len().saturating_sub(i);
                tokens.push(GrammarToken {
                    text: rest,
                    start: cpos,
                    end: cpos.saturating_add(rest_chars),
                    kind: TokenKind::Comment,
                });
                break;
            }
        }

        if ch.is_whitespace() {
            let start_cpos = cpos;
            let start_boff = boff;
            while i < chars.len() {
                let (_, next_ch) = chars.get(i).copied().unwrap_or((0, ' '));
                if next_ch.is_whitespace() {
                    cpos = cpos.saturating_add(1);
                    i = i.saturating_add(1);
                } else {
                    break;
                }
            }
            let end_boff = chars.get(i).map(|p| p.0).unwrap_or(line.len());
            tokens.push(GrammarToken {
                text: line.get(start_boff..end_boff).unwrap_or(""),
                start: start_cpos,
                end: cpos,
                kind: TokenKind::Whitespace,
            });
        } else if ch.is_alphanumeric() || ch == '_' {
            let start_cpos = cpos;
            let start_boff = boff;
            while i < chars.len() {
                let (_, next_ch) = chars.get(i).copied().unwrap_or((0, ' '));
                if next_ch.is_alphanumeric() || next_ch == '_' {
                    cpos = cpos.saturating_add(1);
                    i = i.saturating_add(1);
                } else {
                    break;
                }
            }
            let end_boff = chars.get(i).map(|p| p.0).unwrap_or(line.len());
            tokens.push(GrammarToken {
                text: line.get(start_boff..end_boff).unwrap_or(""),
                start: start_cpos,
                end: cpos,
                kind: TokenKind::Word,
            });
        } else {
            tokens.push(GrammarToken {
                text: line
                    .get(boff..boff.saturating_add(ch.len_utf8()))
                    .unwrap_or(""),
                start: cpos,
                end: cpos.saturating_add(1),
                kind: TokenKind::Punctuation,
            });
            cpos = cpos.saturating_add(1);
            i = i.saturating_add(1);
        }
    }

    tokens
}

/// Tokenizes a multi-line document while preserving comment state across lines.
pub fn tokenize_lines<'a>(lines: &[&'a str], rules: &[CommentRule]) -> Vec<Vec<GrammarToken<'a>>> {
    let mut state = ParseState::default();
    let mut result = Vec::with_capacity(lines.len());
    for &line in lines {
        result.push(tokenize_line_with_parse_state(line, rules, &[], &mut state));
    }
    result
}

/// Character span of intra-line differences with an importance classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarInlineSpan {
    /// Left line character range start.
    pub a_start: usize,
    /// Left line character range end.
    pub a_end: usize,
    /// Right line character range start.
    pub b_start: usize,
    /// Right line character range end.
    pub b_end: usize,
    /// True if the difference is unimportant per grammar rules (e.g. whitespace, case, comment).
    pub is_unimportant: bool,
}

impl GrammarInlineSpan {
    /// Converts this span into a standard [`InlineSpan`].
    pub fn to_inline_span(&self) -> InlineSpan {
        InlineSpan {
            a_start: self.a_start,
            a_end: self.a_end,
            b_start: self.b_start,
            b_end: self.b_end,
        }
    }
}

/// Computes grammar-aware word-level intra-line differences.
pub fn grammar_inline_diff(a: &str, b: &str, options: &GrammarOptions) -> Vec<GrammarInlineSpan> {
    if a == b {
        return Vec::new();
    }

    let mut parse_a = ParseState::default();
    let mut parse_b = ParseState::default();
    let tokens_a = tokenize_line_with_parse_state(
        a,
        &options.comment_rules,
        &options.string_rules,
        &mut parse_a,
    );
    let tokens_b = tokenize_line_with_parse_state(
        b,
        &options.comment_rules,
        &options.string_rules,
        &mut parse_b,
    );

    let norm_texts_a: Vec<String> = tokens_a
        .iter()
        .map(|t| normalize_token_text(t.text, t.kind, options))
        .collect();
    let norm_texts_b: Vec<String> = tokens_b
        .iter()
        .map(|t| normalize_token_text(t.text, t.kind, options))
        .collect();

    let slice_a: Vec<&str> = norm_texts_a.iter().map(|s| s.as_str()).collect();
    let slice_b: Vec<&str> = norm_texts_b.iter().map(|s| s.as_str()).collect();

    let (ops, _) = engine::diff_slices(&slice_a, &slice_b, Some(crate::EDIT_BUDGET));

    let mut spans = Vec::new();
    let mut ai = 0usize;
    let mut bi = 0usize;
    let mut idx = 0usize;

    while let Some(op) = ops.get(idx) {
        match op {
            Op::Equal { len } => {
                for offset in 0..*len {
                    let curr_a = ai.saturating_add(offset);
                    let curr_b = bi.saturating_add(offset);
                    if let (Some(ta), Some(tb)) = (tokens_a.get(curr_a), tokens_b.get(curr_b))
                        && ta.text != tb.text
                    {
                        spans.push(GrammarInlineSpan {
                            a_start: ta.start,
                            a_end: ta.end,
                            b_start: tb.start,
                            b_end: tb.end,
                            is_unimportant: true,
                        });
                    }
                }
                ai = ai.saturating_add(*len);
                bi = bi.saturating_add(*len);
            }
            Op::Delete { len } => {
                let last_a = ai.saturating_add(*len).saturating_sub(1);
                match ops.get(idx.saturating_add(1)) {
                    Some(Op::Insert { len: ins }) => {
                        let last_b = bi.saturating_add(*ins).saturating_sub(1);
                        let a_start = tokens_a.get(ai).map(|t| t.start).unwrap_or(0);
                        let a_end = tokens_a.get(last_a).map(|t| t.end).unwrap_or(a_start);
                        let b_start = tokens_b.get(bi).map(|t| t.start).unwrap_or(0);
                        let b_end = tokens_b.get(last_b).map(|t| t.end).unwrap_or(b_start);

                        let is_unimportant = is_span_unimportant(
                            tokens_a.get(ai..=last_a).unwrap_or(&[]),
                            tokens_b.get(bi..=last_b).unwrap_or(&[]),
                            options,
                        );

                        spans.push(GrammarInlineSpan {
                            a_start,
                            a_end,
                            b_start,
                            b_end,
                            is_unimportant,
                        });
                        ai = ai.saturating_add(*len);
                        bi = bi.saturating_add(*ins);
                        idx = idx.saturating_add(1);
                    }
                    _ => {
                        let a_start = tokens_a.get(ai).map(|t| t.start).unwrap_or(0);
                        let a_end = tokens_a.get(last_a).map(|t| t.end).unwrap_or(a_start);
                        let b_pos = tokens_b
                            .get(bi)
                            .map(|t| t.start)
                            .unwrap_or(tokens_b.last().map(|t| t.end).unwrap_or(0));

                        let is_unimportant = is_span_unimportant(
                            tokens_a.get(ai..=last_a).unwrap_or(&[]),
                            &[],
                            options,
                        );

                        spans.push(GrammarInlineSpan {
                            a_start,
                            a_end,
                            b_start: b_pos,
                            b_end: b_pos,
                            is_unimportant,
                        });
                        ai = ai.saturating_add(*len);
                    }
                }
            }
            Op::Insert { len } => {
                let last_b = bi.saturating_add(*len).saturating_sub(1);
                let b_start = tokens_b.get(bi).map(|t| t.start).unwrap_or(0);
                let b_end = tokens_b.get(last_b).map(|t| t.end).unwrap_or(b_start);
                let a_pos = tokens_a
                    .get(ai)
                    .map(|t| t.start)
                    .unwrap_or(tokens_a.last().map(|t| t.end).unwrap_or(0));

                let is_unimportant =
                    is_span_unimportant(&[], tokens_b.get(bi..=last_b).unwrap_or(&[]), options);

                spans.push(GrammarInlineSpan {
                    a_start: a_pos,
                    a_end: a_pos,
                    b_start,
                    b_end,
                    is_unimportant,
                });
                bi = bi.saturating_add(*len);
            }
        }
        idx = idx.saturating_add(1);
    }

    spans
}

fn normalize_token_text(text: &str, kind: TokenKind, options: &GrammarOptions) -> String {
    if kind == TokenKind::Whitespace {
        match options.whitespace {
            WhitespaceRule::CompareAll => text.to_string(),
            WhitespaceRule::IgnoreLeading
            | WhitespaceRule::IgnoreTrailing
            | WhitespaceRule::IgnoreLeadingAndTrailing
            | WhitespaceRule::IgnoreChanges
            | WhitespaceRule::CollapseRuns => " ".to_string(),
            WhitespaceRule::IgnoreAll => String::new(),
        }
    } else if kind == TokenKind::Comment && options.ignore_comments {
        "__COMMENT__".to_string()
    } else if kind == TokenKind::StringLiteral && options.ignore_string_contents {
        "__STRING__".to_string()
    } else if options.ignore_case {
        text.to_lowercase()
    } else {
        text.to_string()
    }
}

fn is_span_unimportant(
    tokens_a: &[GrammarToken<'_>],
    tokens_b: &[GrammarToken<'_>],
    options: &GrammarOptions,
) -> bool {
    let all_a_whitespace =
        tokens_a.is_empty() || tokens_a.iter().all(|t| t.kind == TokenKind::Whitespace);
    let all_b_whitespace =
        tokens_b.is_empty() || tokens_b.iter().all(|t| t.kind == TokenKind::Whitespace);
    if all_a_whitespace && all_b_whitespace && options.whitespace != WhitespaceRule::CompareAll {
        return true;
    }

    let all_a_comment = tokens_a
        .iter()
        .all(|t| t.kind == TokenKind::Comment || t.kind == TokenKind::Whitespace);
    let all_b_comment = tokens_b
        .iter()
        .all(|t| t.kind == TokenKind::Comment || t.kind == TokenKind::Whitespace);
    if all_a_comment && all_b_comment && options.ignore_comments {
        return true;
    }

    let all_a_str = tokens_a
        .iter()
        .all(|t| t.kind == TokenKind::StringLiteral || t.kind == TokenKind::Whitespace);
    let all_b_str = tokens_b
        .iter()
        .all(|t| t.kind == TokenKind::StringLiteral || t.kind == TokenKind::Whitespace);
    all_a_str && all_b_str && options.ignore_string_contents
}

// ----------------------------------------------------------------------------
// Line-level diff with grammar awareness and Beyond Compare display filters
// ----------------------------------------------------------------------------

/// Display filter for line differences (Beyond Compare parity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayFilter {
    /// Show all lines (identical, unimportant diffs, important diffs).
    #[default]
    All,
    /// Show all differences (both important and unimportant).
    Diffs,
    /// Show only important differences (ignoring unimportant differences).
    DiffsImportantOnly,
    /// Show same/matching lines (identical and unimportant differences).
    Same,
}

/// A line-level diff entry tagged with its grammar importance classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarLineDiff {
    /// The edit operation (Equal, Delete, or Insert).
    pub op: Op,
    /// Importance classification of this line change.
    pub importance: Importance,
    /// Line content from the left input, if present.
    pub left_line: Option<String>,
    /// Line content from the right input, if present.
    pub right_line: Option<String>,
}

impl GrammarLineDiff {
    pub fn is_identical(&self) -> bool {
        self.importance == Importance::Identical
    }

    pub fn is_unimportant(&self) -> bool {
        self.importance == Importance::UnimportantDifference
    }

    pub fn is_important(&self) -> bool {
        self.importance == Importance::ImportantDifference
    }
}

/// Diff script produced by grammar-aware line comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarDiffScript {
    /// Detailed line-level differences.
    pub lines: Vec<GrammarLineDiff>,
    /// Whether the script is exact (did not hit edit budget fallback).
    pub exact: bool,
    /// Count of lines that are completely identical.
    pub identical_count: usize,
    /// Count of lines that differ only in unimportant grammar aspects.
    pub unimportant_count: usize,
    /// Count of lines that contain important differences.
    pub important_count: usize,
}

impl GrammarDiffScript {
    /// Filters diff lines according to the specified Beyond Compare display filter.
    pub fn filter(&self, filter: DisplayFilter) -> Vec<&GrammarLineDiff> {
        self.lines
            .iter()
            .filter(|line| match filter {
                DisplayFilter::All => true,
                DisplayFilter::Diffs => line.importance != Importance::Identical,
                DisplayFilter::DiffsImportantOnly => {
                    line.importance == Importance::ImportantDifference
                }
                DisplayFilter::Same => line.importance != Importance::ImportantDifference,
            })
            .collect()
    }

    /// Returns true if any important differences exist between the inputs.
    pub fn has_important_differences(&self) -> bool {
        self.important_count > 0
    }

    /// Returns true if any differences (important or unimportant) exist between the inputs.
    pub fn has_differences(&self) -> bool {
        self.unimportant_count > 0 || self.important_count > 0
    }
}

/// Compares two sequences of lines using grammar rules and unimportant text filtering.
///
/// Lines are aligned using their normalized forms (accounting for whitespace, comments,
/// case, and blank lines), and each aligned line or inserted/deleted line is tagged with
/// its [`Importance`].
pub fn diff_lines_with_grammar(
    a: &[&str],
    b: &[&str],
    options: &GrammarOptions,
) -> GrammarDiffScript {
    let norm_a = normalize_lines(a, options);
    let norm_b = normalize_lines(b, options);

    let norm_a_refs: Vec<&str> = norm_a.iter().map(|s| s.as_str()).collect();
    let norm_b_refs: Vec<&str> = norm_b.iter().map(|s| s.as_str()).collect();

    let (ops, exact) = engine::diff_slices(&norm_a_refs, &norm_b_refs, Some(crate::EDIT_BUDGET));

    let mut lines = Vec::new();
    let mut ai = 0usize;
    let mut bi = 0usize;
    let mut identical_count = 0usize;
    let mut unimportant_count = 0usize;
    let mut important_count = 0usize;
    let mut idx = 0usize;

    while let Some(op) = ops.get(idx) {
        match op {
            Op::Equal { len } => {
                for offset in 0..*len {
                    let a_curr = ai.saturating_add(offset);
                    let b_curr = bi.saturating_add(offset);
                    let left = a.get(a_curr).copied();
                    let right = b.get(b_curr).copied();

                    let importance = match (left, right) {
                        (Some(l), Some(r)) if l == r => {
                            identical_count = identical_count.saturating_add(1);
                            Importance::Identical
                        }
                        (Some(_), Some(_)) => {
                            unimportant_count = unimportant_count.saturating_add(1);
                            Importance::UnimportantDifference
                        }
                        _ => {
                            important_count = important_count.saturating_add(1);
                            Importance::ImportantDifference
                        }
                    };

                    lines.push(GrammarLineDiff {
                        op: Op::Equal { len: 1 },
                        importance,
                        left_line: left.map(|s| s.to_string()),
                        right_line: right.map(|s| s.to_string()),
                    });
                }
                ai = ai.saturating_add(*len);
                bi = bi.saturating_add(*len);
            }
            Op::Delete { len } => match ops.get(idx.saturating_add(1)) {
                Some(Op::Insert { len: ins }) => {
                    let min_len = (*len).min(*ins);
                    for offset in 0..min_len {
                        let a_curr = ai.saturating_add(offset);
                        let b_curr = bi.saturating_add(offset);
                        let left = a.get(a_curr).copied();
                        let right = b.get(b_curr).copied();

                        let importance = match (left, right) {
                            (Some(l), Some(r)) => compare_lines_importance(l, r, options),
                            _ => Importance::ImportantDifference,
                        };

                        match importance {
                            Importance::Identical => {
                                identical_count = identical_count.saturating_add(1);
                            }
                            Importance::UnimportantDifference => {
                                unimportant_count = unimportant_count.saturating_add(1);
                            }
                            Importance::ImportantDifference => {
                                important_count = important_count.saturating_add(1);
                            }
                        }

                        lines.push(GrammarLineDiff {
                            op: Op::Equal { len: 1 },
                            importance,
                            left_line: left.map(|s| s.to_string()),
                            right_line: right.map(|s| s.to_string()),
                        });
                    }

                    if *len > min_len {
                        for offset in min_len..*len {
                            let a_curr = ai.saturating_add(offset);
                            let left = a.get(a_curr).copied();
                            let is_unimp =
                                options.ignore_blank_lines && left.is_some_and(is_blank_line);
                            let importance = if is_unimp {
                                unimportant_count = unimportant_count.saturating_add(1);
                                Importance::UnimportantDifference
                            } else {
                                important_count = important_count.saturating_add(1);
                                Importance::ImportantDifference
                            };

                            lines.push(GrammarLineDiff {
                                op: Op::Delete { len: 1 },
                                importance,
                                left_line: left.map(|s| s.to_string()),
                                right_line: None,
                            });
                        }
                    } else if *ins > min_len {
                        for offset in min_len..*ins {
                            let b_curr = bi.saturating_add(offset);
                            let right = b.get(b_curr).copied();
                            let is_unimp =
                                options.ignore_blank_lines && right.is_some_and(is_blank_line);
                            let importance = if is_unimp {
                                unimportant_count = unimportant_count.saturating_add(1);
                                Importance::UnimportantDifference
                            } else {
                                important_count = important_count.saturating_add(1);
                                Importance::ImportantDifference
                            };

                            lines.push(GrammarLineDiff {
                                op: Op::Insert { len: 1 },
                                importance,
                                left_line: None,
                                right_line: right.map(|s| s.to_string()),
                            });
                        }
                    }

                    ai = ai.saturating_add(*len);
                    bi = bi.saturating_add(*ins);
                    idx = idx.saturating_add(1);
                }
                _ => {
                    for offset in 0..*len {
                        let a_curr = ai.saturating_add(offset);
                        let left = a.get(a_curr).copied();
                        let is_unimp =
                            options.ignore_blank_lines && left.is_some_and(is_blank_line);
                        let importance = if is_unimp {
                            unimportant_count = unimportant_count.saturating_add(1);
                            Importance::UnimportantDifference
                        } else {
                            important_count = important_count.saturating_add(1);
                            Importance::ImportantDifference
                        };

                        lines.push(GrammarLineDiff {
                            op: Op::Delete { len: 1 },
                            importance,
                            left_line: left.map(|s| s.to_string()),
                            right_line: None,
                        });
                    }
                    ai = ai.saturating_add(*len);
                }
            },
            Op::Insert { len } => {
                for offset in 0..*len {
                    let b_curr = bi.saturating_add(offset);
                    let right = b.get(b_curr).copied();
                    let is_unimp = options.ignore_blank_lines && right.is_some_and(is_blank_line);
                    let importance = if is_unimp {
                        unimportant_count = unimportant_count.saturating_add(1);
                        Importance::UnimportantDifference
                    } else {
                        important_count = important_count.saturating_add(1);
                        Importance::ImportantDifference
                    };

                    lines.push(GrammarLineDiff {
                        op: Op::Insert { len: 1 },
                        importance,
                        left_line: None,
                        right_line: right.map(|s| s.to_string()),
                    });
                }
                bi = bi.saturating_add(*len);
            }
        }
        idx = idx.saturating_add(1);
    }

    GrammarDiffScript {
        lines,
        exact,
        identical_count,
        unimportant_count,
        important_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_rules() {
        let opt_trim = GrammarOptions {
            whitespace: WhitespaceRule::IgnoreLeadingAndTrailing,
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("  hello  ", "hello", &opt_trim),
            Importance::UnimportantDifference
        );
        assert_eq!(
            compare_lines_importance("hello", "hello", &opt_trim),
            Importance::Identical
        );
        assert_eq!(
            compare_lines_importance("hello", "world", &opt_trim),
            Importance::ImportantDifference
        );

        let opt_collapse = GrammarOptions {
            whitespace: WhitespaceRule::CollapseRuns,
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("a    b    c", "a b c", &opt_collapse),
            Importance::UnimportantDifference
        );
    }

    #[test]
    fn test_line_endings_detection_and_normalization() {
        assert_eq!(detect_line_ending("foo\nbar\n"), LineEndingType::Lf);
        assert_eq!(detect_line_ending("foo\r\nbar\r\n"), LineEndingType::Crlf);
        assert_eq!(detect_line_ending("foo\rbar\r"), LineEndingType::Cr);
        assert_eq!(detect_line_ending("foo\r\nbar\n"), LineEndingType::Mixed);
        assert_eq!(detect_line_ending("foobar"), LineEndingType::None);

        let crlf_text = "line1\r\nline2\r\n";
        let normalized = normalize_line_endings(crlf_text, LineEndingType::Lf);
        assert_eq!(normalized, "line1\nline2\n");

        let opt_le = GrammarOptions {
            line_ending: LineEndingRule::Ignore,
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("test line\r\n", "test line\n", &opt_le),
            Importance::UnimportantDifference
        );
    }

    #[test]
    fn test_case_insensitivity() {
        let opt_case = GrammarOptions {
            ignore_case: true,
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("HELLO WORLD", "hello world", &opt_case),
            Importance::UnimportantDifference
        );
    }

    #[test]
    fn test_line_comment_stripping() {
        let opt_comm = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::c_style()],
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("let x = 1; // initial", "let x = 1; // updated", &opt_comm),
            Importance::UnimportantDifference
        );
        assert_eq!(
            compare_lines_importance("let x = 1;", "let x = 2; // updated", &opt_comm),
            Importance::ImportantDifference
        );
    }

    #[test]
    fn test_nested_comments() {
        let opt_nested = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::rust_style()],
            ..Default::default()
        };
        let mut state = CommentState::new();
        let res1 = strip_comments_with_state(
            "/* outer /* inner */ still comment */ code();",
            &opt_nested.comment_rules,
            &mut state,
        );
        assert_eq!(res1.trim(), "code();");
        assert!(!state.is_inside_comment());
    }

    #[test]
    fn test_string_literal_comment_protection() {
        let opt_str = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::c_style()],
            string_rules: vec![StringLiteralRule::double_quote()],
            ..Default::default()
        };
        // Comment delimiter inside string literal should NOT be stripped as a comment
        let norm = normalize_line("let s = \"// not a comment\";", &opt_str);
        assert_eq!(norm, "let s = \"// not a comment\";");
    }

    #[test]
    fn test_block_comment_intra_line() {
        let opt = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::c_style()],
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance(
                "let x = /* old comment */ 42;",
                "let x = /* new comment */ 42;",
                &opt
            ),
            Importance::UnimportantDifference
        );
        assert_eq!(
            compare_lines_importance("/* full comment */", "/* different comment */", &opt),
            Importance::UnimportantDifference
        );
    }

    #[test]
    fn test_xml_block_comment() {
        let opt = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::xml_style()],
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance(
                "<config><!-- comment a --></config>",
                "<config><!-- comment b --></config>",
                &opt
            ),
            Importance::UnimportantDifference
        );
    }

    #[test]
    fn test_multi_line_comment_normalization() {
        let opt = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::c_style()],
            ..Default::default()
        };
        let lines_a = [
            "/* start of comment",
            " middle text",
            " end of comment */",
            "let x = 10;",
        ];
        let lines_b = [
            "/* entirely different",
            " commentary lines",
            " */",
            "let x = 10;",
        ];

        let norm_a = normalize_lines(&lines_a, &opt);
        let norm_b = normalize_lines(&lines_b, &opt);

        assert_eq!(norm_a.get(3), norm_b.get(3));
        assert_eq!(norm_a.get(1).map(|s| s.trim()), Some(""));
        assert_eq!(norm_b.get(1).map(|s| s.trim()), Some(""));
    }

    #[test]
    fn test_ignore_blank_lines() {
        let opt = GrammarOptions {
            ignore_blank_lines: true,
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("   ", "", &opt),
            Importance::UnimportantDifference
        );
        assert_eq!(
            compare_lines_importance("", "", &opt),
            Importance::Identical
        );
    }

    #[test]
    fn test_diff_lines_with_grammar_identical() {
        let opt = GrammarOptions::default();
        let a = ["alpha", "beta", "gamma"];
        let b = ["alpha", "beta", "gamma"];

        let script = diff_lines_with_grammar(&a, &b, &opt);
        assert!(script.exact);
        assert_eq!(script.identical_count, 3);
        assert_eq!(script.unimportant_count, 0);
        assert_eq!(script.important_count, 0);
        assert!(!script.has_differences());
        assert!(!script.has_important_differences());
    }

    #[test]
    fn test_diff_lines_with_grammar_unimportant_and_filters() {
        let opt = GrammarOptions {
            ignore_case: true,
            whitespace: WhitespaceRule::IgnoreChanges,
            ignore_comments: true,
            comment_rules: vec![CommentRule::c_style()],
            ignore_blank_lines: true,
            ..Default::default()
        };

        let a = ["FOO = 1;", "   ", "// comment A", "bar = 2;"];
        let b = ["foo = 1;", "", "// comment B", "bar = 999;"];

        let script = diff_lines_with_grammar(&a, &b, &opt);
        assert!(script.has_differences());
        assert!(script.has_important_differences());
        assert_eq!(script.important_count, 1); // bar = 2 vs bar = 999
        assert_eq!(script.unimportant_count, 3); // case, blank, comment

        // Beyond Compare display filters
        let all = script.filter(DisplayFilter::All);
        assert_eq!(all.len(), 4);

        let diffs = script.filter(DisplayFilter::Diffs);
        assert_eq!(diffs.len(), 4); // all 4 differ (3 unimportant, 1 important)

        let important_only = script.filter(DisplayFilter::DiffsImportantOnly);
        assert_eq!(important_only.len(), 1);
        assert_eq!(
            important_only.first().and_then(|l| l.left_line.as_deref()),
            Some("bar = 2;")
        );

        let same = script.filter(DisplayFilter::Same);
        assert_eq!(same.len(), 3); // 3 match under grammar rules
    }

    #[test]
    fn test_diff_lines_with_grammar_blank_line_insertion() {
        let opt = GrammarOptions {
            ignore_blank_lines: true,
            ..Default::default()
        };

        let a = ["line 1", "line 2"];
        let b = ["line 1", "", "   ", "line 2"];

        let script = diff_lines_with_grammar(&a, &b, &opt);
        assert!(!script.has_important_differences());
        assert_eq!(script.unimportant_count, 2); // 2 blank lines inserted
        assert_eq!(script.identical_count, 2);
    }

    #[test]
    fn test_grammar_inline_diff_tags_unimportant() {
        let opt = GrammarOptions {
            ignore_case: true,
            whitespace: WhitespaceRule::IgnoreChanges,
            ..Default::default()
        };
        let spans = grammar_inline_diff("FooBar = 123", "foobar = 456", &opt);
        assert_eq!(spans.len(), 2);
        assert!(spans.first().is_some_and(|s| s.is_unimportant));
        assert!(spans.get(1).is_some_and(|s| !s.is_unimportant));
    }

    #[test]
    fn test_ignore_all_whitespace() {
        let opt = GrammarOptions {
            whitespace: WhitespaceRule::IgnoreAll,
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("a  b c", "abc", &opt),
            Importance::UnimportantDifference
        );
    }

    #[test]
    fn test_comment_hash_and_sql_style() {
        let opt_hash = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::hash_style()],
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("x = 1 # test", "x = 1 # other", &opt_hash),
            Importance::UnimportantDifference
        );

        let opt_sql = GrammarOptions {
            ignore_comments: true,
            comment_rules: vec![CommentRule::sql_style()],
            ..Default::default()
        };
        assert_eq!(
            compare_lines_importance("SELECT 1; -- query", "SELECT 1; -- different", &opt_sql),
            Importance::UnimportantDifference
        );
    }

    #[test]
    fn test_loose_preset() {
        let opt = GrammarOptions::loose();
        assert_eq!(
            compare_lines_importance("  FN foo() // bar", "fn foo() // baz", &opt),
            Importance::UnimportantDifference
        );
    }
}
