//! Grammar and unimportant-text rules: whitespace, casing, and comments filtering,
//! plus grammar-aware word-level intra-line diffing and Beyond Compare parity line diffing.
//!
//! Layer 0: no panics, no unwrap/expect in non-test code, no unsafe.

#![allow(dead_code)]

use crate::{InlineSpan, Op, engine};

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
}

impl CommentRule {
    /// Common C-style comments (`//` and `/* ... */`).
    pub fn c_style() -> Self {
        Self {
            line_prefix: Some("//".to_string()),
            block_start: Some("/*".to_string()),
            block_end: Some("*/".to_string()),
        }
    }

    /// Common Python/Shell-style line comments (`#`).
    pub fn hash_style() -> Self {
        Self {
            line_prefix: Some("#".to_string()),
            block_start: None,
            block_end: None,
        }
    }

    /// Common SQL-style line comments (`--` and `/* ... */`).
    pub fn sql_style() -> Self {
        Self {
            line_prefix: Some("--".to_string()),
            block_start: Some("/*".to_string()),
            block_end: Some("*/".to_string()),
        }
    }

    /// HTML / XML comments (`<!-- ... -->`).
    pub fn xml_style() -> Self {
        Self {
            line_prefix: None,
            block_start: Some("<!--".to_string()),
            block_end: Some("-->".to_string()),
        }
    }
}

/// Tracks block comment state across lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommentState {
    /// If currently inside an open block comment, stores the expected closing delimiter.
    pub active_block_end: Option<String>,
}

impl CommentState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_inside_comment(&self) -> bool {
        self.active_block_end.is_some()
    }
}

/// Grammar comparison options governing unimportant differences.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GrammarOptions {
    /// If true, case differences are considered unimportant.
    pub ignore_case: bool,
    /// Whitespace comparison rule.
    pub whitespace: WhitespaceRule,
    /// If true, differences inside comments are considered unimportant.
    pub ignore_comments: bool,
    /// Configured comment syntax rules.
    pub comment_rules: Vec<CommentRule>,
    /// If true, completely blank lines are ignored.
    pub ignore_blank_lines: bool,
}

impl GrammarOptions {
    /// Preset that ignores all whitespace differences and casing.
    pub fn loose() -> Self {
        Self {
            ignore_case: true,
            whitespace: WhitespaceRule::IgnoreChanges,
            ignore_comments: true,
            comment_rules: vec![CommentRule::c_style()],
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

/// Strips comments from a line while persisting multi-line comment state.
pub fn strip_comments_with_state(
    line: &str,
    rules: &[CommentRule],
    state: &mut CommentState,
) -> String {
    let mut out = String::with_capacity(line.len());
    let mut curr_boff = 0usize;

    while curr_boff < line.len() {
        if let Some(ref end_delim) = state.active_block_end {
            let rest = line.get(curr_boff..).unwrap_or("");
            if let Some(pos) = rest.find(end_delim) {
                curr_boff = curr_boff
                    .saturating_add(pos)
                    .saturating_add(end_delim.len());
                state.active_block_end = None;
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
            } else {
                return out;
            }
        } else {
            let rest = line.get(curr_boff..).unwrap_or("");
            let mut earliest_line_prefix: Option<usize> = None;
            let mut earliest_block_start: Option<(usize, &str, &str)> = None;

            for rule in rules {
                if let Some(ref prefix) = rule.line_prefix
                    && let Some(pos) = rest.find(prefix)
                {
                    earliest_line_prefix = Some(earliest_line_prefix.map_or(pos, |p| p.min(pos)));
                }
                if let (Some(bstart), Some(bend)) = (&rule.block_start, &rule.block_end)
                    && let Some(pos) = rest.find(bstart)
                {
                    match earliest_block_start {
                        Some((best_pos, _, _)) if pos < best_pos => {
                            earliest_block_start = Some((pos, bstart.as_str(), bend.as_str()));
                        }
                        None => {
                            earliest_block_start = Some((pos, bstart.as_str(), bend.as_str()));
                        }
                        _ => {}
                    }
                }
            }

            match (earliest_line_prefix, earliest_block_start) {
                (None, None) => {
                    out.push_str(rest);
                    break;
                }
                (Some(lp_pos), None) => {
                    out.push_str(rest.get(..lp_pos).unwrap_or(""));
                    break;
                }
                (None, Some((bs_pos, bstart, bend))) => {
                    out.push_str(rest.get(..bs_pos).unwrap_or(""));
                    let after_bstart = bs_pos.saturating_add(bstart.len());
                    let inside_rest = rest.get(after_bstart..).unwrap_or("");
                    if let Some(end_pos) = inside_rest.find(bend) {
                        curr_boff = curr_boff
                            .saturating_add(after_bstart)
                            .saturating_add(end_pos)
                            .saturating_add(bend.len());
                        if !out.ends_with(' ') && !out.is_empty() {
                            out.push(' ');
                        }
                    } else {
                        state.active_block_end = Some(bend.to_string());
                        return out;
                    }
                }
                (Some(lp_pos), Some((bs_pos, bstart, bend))) => {
                    if lp_pos < bs_pos {
                        out.push_str(rest.get(..lp_pos).unwrap_or(""));
                        break;
                    } else {
                        out.push_str(rest.get(..bs_pos).unwrap_or(""));
                        let after_bstart = bs_pos.saturating_add(bstart.len());
                        let inside_rest = rest.get(after_bstart..).unwrap_or("");
                        if let Some(end_pos) = inside_rest.find(bend) {
                            curr_boff = curr_boff
                                .saturating_add(after_bstart)
                                .saturating_add(end_pos)
                                .saturating_add(bend.len());
                            if !out.ends_with(' ') && !out.is_empty() {
                                out.push(' ');
                            }
                        } else {
                            state.active_block_end = Some(bend.to_string());
                            return out;
                        }
                    }
                }
            }
        }
    }

    out
}

/// Normalizes a single line according to grammar whitespace and casing rules.
pub fn normalize_line(line: &str, options: &GrammarOptions) -> String {
    let mut state = CommentState::default();
    normalize_line_with_state(line, options, &mut state)
}

/// Normalizes a single line while tracking persistent comment state across lines.
pub fn normalize_line_with_state(
    line: &str,
    options: &GrammarOptions,
    state: &mut CommentState,
) -> String {
    let mut s = if options.ignore_comments {
        strip_comments_with_state(line, &options.comment_rules, state)
    } else {
        line.to_string()
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
    let mut state = CommentState::default();
    let mut out = Vec::with_capacity(lines.len());
    for &line in lines {
        out.push(normalize_line_with_state(line, options, &mut state));
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
}

/// A parsed token with character offset range [start, end) and importance tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarToken<'a> {
    pub text: &'a str,
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

/// Tokenizes a line into words, punctuation, whitespace, and comments.
pub fn tokenize_line<'a>(line: &'a str, rules: &[CommentRule]) -> Vec<GrammarToken<'a>> {
    let mut state = CommentState::default();
    tokenize_line_with_state(line, rules, &mut state)
}

/// Tokenizes a line while preserving multi-line comment state across lines.
pub fn tokenize_line_with_state<'a>(
    line: &'a str,
    rules: &[CommentRule],
    state: &mut CommentState,
) -> Vec<GrammarToken<'a>> {
    let mut tokens = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut i = 0usize;
    let mut cpos = 0usize;

    // Check if continuing inside a block comment
    if let Some(ref end_delim) = state.active_block_end {
        if let Some(pos) = line.find(end_delim) {
            let end_byte = pos.saturating_add(end_delim.len());
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
            state.active_block_end = None;
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

    while i < chars.len() {
        let (boff, ch) = chars.get(i).copied().unwrap_or((0, ' '));
        let rest = line.get(boff..).unwrap_or("");

        // Check comment triggers at this byte position
        let mut line_comment_match = false;
        let mut block_match: Option<(&str, &str)> = None;

        for rule in rules {
            if let Some(ref prefix) = rule.line_prefix
                && rest.starts_with(prefix)
            {
                line_comment_match = true;
                break;
            }
            if let (Some(bstart), Some(bend)) = (&rule.block_start, &rule.block_end)
                && rest.starts_with(bstart)
            {
                block_match = Some((bstart.as_str(), bend.as_str()));
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

        if let Some((bstart, bend)) = block_match {
            let after_start = bstart.len();
            let inside = rest.get(after_start..).unwrap_or("");
            if let Some(end_rel) = inside.find(bend) {
                let total_bytes = after_start
                    .saturating_add(end_rel)
                    .saturating_add(bend.len());
                let target_byte = boff.saturating_add(total_bytes);
                let start_cpos = cpos;
                let mut comm_chars = 0usize;
                while i < chars.len() {
                    if let Some(&(cur_b, _)) = chars.get(i) {
                        if cur_b < target_byte {
                            comm_chars = comm_chars.saturating_add(1);
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
                state.active_block_end = Some(bend.to_string());
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
    let mut state = CommentState::default();
    let mut result = Vec::with_capacity(lines.len());
    for &line in lines {
        result.push(tokenize_line_with_state(line, rules, &mut state));
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

    let tokens_a = tokenize_line(a, &options.comment_rules);
    let tokens_b = tokenize_line(b, &options.comment_rules);

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
            | WhitespaceRule::IgnoreChanges => " ".to_string(),
            WhitespaceRule::IgnoreAll => String::new(),
        }
    } else if kind == TokenKind::Comment && options.ignore_comments {
        "__COMMENT__".to_string()
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
    all_a_comment && all_b_comment && options.ignore_comments
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
