//! Grammar and unimportant-text rules: whitespace, casing, and comments filtering,
//! plus grammar-aware word-level intra-line diffing.
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
    /// The texts differ only in unimportant aspects (whitespace, case, comments).
    UnimportantDifference,
    /// The texts contain meaningful/important semantic differences.
    ImportantDifference,
}

/// Normalizes a single line according to grammar whitespace and casing rules.
pub fn normalize_line(line: &str, options: &GrammarOptions) -> String {
    let mut s = line.to_string();

    if options.ignore_comments {
        s = strip_line_comments(&s, &options.comment_rules);
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

/// Removes line comment suffixes from a line.
fn strip_line_comments(line: &str, rules: &[CommentRule]) -> String {
    let mut cut_idx = line.len();
    for rule in rules {
        if let Some(prefix) = &rule.line_prefix
            && let Some(pos) = line.find(prefix)
            && pos < cut_idx
        {
            cut_idx = pos;
        }
    }
    line.get(..cut_idx).unwrap_or(line).to_string()
}

/// Assesses the difference importance between two lines.
pub fn compare_lines_importance(a: &str, b: &str, options: &GrammarOptions) -> Importance {
    if a == b {
        return Importance::Identical;
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
    let mut tokens = Vec::new();
    let mut cpos = 0usize;

    // Check if line contains a line comment
    let mut comment_byte_idx = None;
    for rule in rules {
        if let Some(prefix) = &rule.line_prefix
            && let Some(pos) = line.find(prefix)
        {
            comment_byte_idx = Some(comment_byte_idx.map_or(pos, |curr: usize| curr.min(pos)));
        }
    }

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let (boff, ch) = chars.get(i).copied().unwrap_or((0, ' '));

        // If at or past comment prefix, rest of line is a comment token
        if let Some(c_byte) = comment_byte_idx
            && boff >= c_byte
        {
            let rest_text = line.get(boff..).unwrap_or("");
            let rest_chars = chars.len().saturating_sub(i);
            tokens.push(GrammarToken {
                text: rest_text,
                start: cpos,
                end: cpos.saturating_add(rest_chars),
                kind: TokenKind::Comment,
            });
            break;
        }

        if ch.is_whitespace() {
            let start_cpos = cpos;
            let start_boff = boff;
            while i < chars.len() {
                let (next_boff, next_ch) = chars.get(i).copied().unwrap_or((0, ' '));
                if let Some(c_byte) = comment_byte_idx
                    && next_boff >= c_byte
                {
                    break;
                }
                if next_ch.is_whitespace() {
                    cpos += 1;
                    i += 1;
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
                let (next_boff, next_ch) = chars.get(i).copied().unwrap_or((0, ' '));
                if let Some(c_byte) = comment_byte_idx
                    && next_boff >= c_byte
                {
                    break;
                }
                if next_ch.is_alphanumeric() || next_ch == '_' {
                    cpos += 1;
                    i += 1;
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
            // Punctuation / symbol (single char or cluster)
            tokens.push(GrammarToken {
                text: line
                    .get(boff..boff.saturating_add(ch.len_utf8()))
                    .unwrap_or(""),
                start: cpos,
                end: cpos.saturating_add(1),
                kind: TokenKind::Punctuation,
            });
            cpos += 1;
            i += 1;
        }
    }

    tokens
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
                // Tokens are equal under normalization.
                // Check if they differed in raw text (unimportant difference).
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
                        idx += 1;
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
        idx += 1;
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
    // If all tokens on both sides are whitespace and whitespace differences are ignored:
    let all_a_whitespace =
        tokens_a.is_empty() || tokens_a.iter().all(|t| t.kind == TokenKind::Whitespace);
    let all_b_whitespace =
        tokens_b.is_empty() || tokens_b.iter().all(|t| t.kind == TokenKind::Whitespace);
    if all_a_whitespace && all_b_whitespace && options.whitespace != WhitespaceRule::CompareAll {
        return true;
    }

    // If all tokens are comments and comments are ignored:
    let all_a_comment = tokens_a
        .iter()
        .all(|t| t.kind == TokenKind::Comment || t.kind == TokenKind::Whitespace);
    let all_b_comment = tokens_b
        .iter()
        .all(|t| t.kind == TokenKind::Comment || t.kind == TokenKind::Whitespace);
    all_a_comment && all_b_comment && options.ignore_comments
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
    fn test_grammar_inline_diff_tags_unimportant() {
        let opt = GrammarOptions {
            ignore_case: true,
            whitespace: WhitespaceRule::IgnoreChanges,
            ..Default::default()
        };
        let spans = grammar_inline_diff("FooBar = 123", "foobar = 456", &opt);
        assert_eq!(spans.len(), 2);
        // "FooBar" vs "foobar" is unimportant because ignore_case is on
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
