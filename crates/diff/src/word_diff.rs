//! Word-level inline spans for one pair of lines.
//!
//! Each line is split into whitespace-delimited tokens that remember
//! their character range; the same Myers machinery used for lines
//! diffs the token texts; and Delete/Insert runs map back to one
//! [`InlineSpan`] per change, covering the first token's start through
//! the last token's end on each side. A run that exists on only one
//! side gets a zero-length range on the other side (start == end),
//! which renderers treat as "nothing to highlight".

use crate::{InlineSpan, Op, engine};

/// One whitespace-delimited token with its half-open character range.
struct Token<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

/// Character offset of token `i`, or the end anchor when the token
/// does not exist.
fn tok_start(toks: &[Token<'_>], i: usize) -> usize {
    match toks.get(i) {
        Some(t) => t.start,
        None => end_anchor(toks),
    }
}

/// Character offset one past token `i`, or the end anchor when the
/// token does not exist.
fn tok_end(toks: &[Token<'_>], i: usize) -> usize {
    match toks.get(i) {
        Some(t) => t.end,
        None => end_anchor(toks),
    }
}

/// The position "adjacent to" token index `i`: the start of the token
/// at `i` when it exists, the end of the last token otherwise, and 0
/// for a side with no tokens at all.
fn anchor(toks: &[Token<'_>], i: usize) -> usize {
    match toks.get(i) {
        Some(t) => t.start,
        None => end_anchor(toks),
    }
}

fn end_anchor(toks: &[Token<'_>]) -> usize {
    match toks.last() {
        Some(t) => t.end,
        None => 0,
    }
}

/// Splits a line into tokens on whitespace, recording each token's
/// half-open character range. Whitespace between tokens belongs to no
/// span.
fn tokenize(line: &str) -> Vec<Token<'_>> {
    let mut out: Vec<Token<'_>> = Vec::new();
    let mut cpos = 0usize; // current character index
    let mut in_token = false;
    let mut bstart = 0usize; // byte offset where the token began
    let mut cstart = 0usize; // character index where the token began
    for (boff, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if in_token {
                out.push(Token {
                    text: line.get(bstart..boff).unwrap_or(""),
                    start: cstart,
                    end: cpos,
                });
                in_token = false;
            }
        } else if !in_token {
            in_token = true;
            bstart = boff;
            cstart = cpos;
        }
        cpos += 1;
    }
    if in_token {
        out.push(Token {
            text: line.get(bstart..line.len()).unwrap_or(""),
            start: cstart,
            end: cpos,
        });
    }
    out
}

/// Changed character spans for one pair of lines. Empty when the lines
/// are equal.
pub(crate) fn inline_diff(a: &str, b: &str) -> Vec<InlineSpan> {
    if a == b {
        return Vec::new();
    }
    let ta = tokenize(a);
    let tb = tokenize(b);
    let sa: Vec<&str> = ta.iter().map(|t| t.text).collect();
    let sb: Vec<&str> = tb.iter().map(|t| t.text).collect();
    let (ops, _exact) = engine::diff_slices(&sa, &sb, Some(crate::EDIT_BUDGET));
    let mut spans: Vec<InlineSpan> = Vec::new();
    let (mut ai, mut bi) = (0usize, 0usize);
    let mut idx = 0usize;
    while let Some(op) = ops.get(idx) {
        match op {
            Op::Equal { len } => {
                ai = ai.saturating_add(*len);
                bi = bi.saturating_add(*len);
            }
            Op::Delete { len } => {
                let last_a = ai.saturating_add(*len).saturating_sub(1);
                // A replace block is a Delete directly followed by an
                // Insert (canonical order).
                match ops.get(idx.saturating_add(1)) {
                    Some(Op::Insert { len: ins }) => {
                        let last_b = bi.saturating_add(*ins).saturating_sub(1);
                        spans.push(InlineSpan {
                            a_start: tok_start(&ta, ai),
                            a_end: tok_end(&ta, last_a),
                            b_start: tok_start(&tb, bi),
                            b_end: tok_end(&tb, last_b),
                        });
                        ai = ai.saturating_add(*len);
                        bi = bi.saturating_add(*ins);
                        idx += 1;
                    }
                    _ => {
                        let at = anchor(&tb, bi);
                        spans.push(InlineSpan {
                            a_start: tok_start(&ta, ai),
                            a_end: tok_end(&ta, last_a),
                            b_start: at,
                            b_end: at,
                        });
                        ai = ai.saturating_add(*len);
                    }
                }
            }
            Op::Insert { len } => {
                let last_b = bi.saturating_add(*len).saturating_sub(1);
                let at = anchor(&ta, ai);
                spans.push(InlineSpan {
                    a_start: at,
                    a_end: at,
                    b_start: tok_start(&tb, bi),
                    b_end: tok_end(&tb, last_b),
                });
                bi = bi.saturating_add(*len);
            }
        }
        idx += 1;
    }
    spans
}

/// Character-level changed spans between two lines.
pub(crate) fn char_diff(a: &str, b: &str) -> Vec<InlineSpan> {
    if a == b {
        return Vec::new();
    }
    let ca: Vec<char> = a.chars().collect();
    let cb: Vec<char> = b.chars().collect();
    let (ops, _) = engine::diff_slices(&ca, &cb, Some(crate::EDIT_BUDGET));

    let mut spans = Vec::new();
    let (mut ai, mut bi) = (0usize, 0usize);
    let mut idx = 0usize;

    while let Some(op) = ops.get(idx) {
        match op {
            Op::Equal { len } => {
                ai = ai.saturating_add(*len);
                bi = bi.saturating_add(*len);
            }
            Op::Delete { len } => {
                let a_start = ai;
                let a_end = ai.saturating_add(*len);
                match ops.get(idx.saturating_add(1)) {
                    Some(Op::Insert { len: ins }) => {
                        let b_start = bi;
                        let b_end = bi.saturating_add(*ins);
                        spans.push(InlineSpan {
                            a_start,
                            a_end,
                            b_start,
                            b_end,
                        });
                        ai = a_end;
                        bi = b_end;
                        idx += 1;
                    }
                    _ => {
                        spans.push(InlineSpan {
                            a_start,
                            a_end,
                            b_start: bi,
                            b_end: bi,
                        });
                        ai = a_end;
                    }
                }
            }
            Op::Insert { len } => {
                let b_start = bi;
                let b_end = bi.saturating_add(*len);
                spans.push(InlineSpan {
                    a_start: ai,
                    a_end: ai,
                    b_start,
                    b_end,
                });
                bi = b_end;
            }
        }
        idx += 1;
    }

    spans
}

/// Hierarchical word-and-character inline diffing (Beyond Compare parity).
///
/// Computes word-level spans first. For replaced words that share substantial
/// character structure (e.g. prefix/suffix like `foo_old_bar` vs `foo_new_bar`),
/// character-level diffing is applied inside the word to isolate the exact
/// changed characters rather than highlighting the entire token.
pub(crate) fn word_char_diff(a: &str, b: &str) -> Vec<InlineSpan> {
    if a == b {
        return Vec::new();
    }
    let word_spans = inline_diff(a, b);
    let mut refined = Vec::new();
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    for span in word_spans {
        let a_len = span.a_end.saturating_sub(span.a_start);
        let b_len = span.b_end.saturating_sub(span.b_start);

        if a_len == 0 || b_len == 0 || (a_len < 3 && b_len < 3) {
            refined.push(span);
            continue;
        }

        let sub_a: String = a_chars
            .get(span.a_start..span.a_end)
            .unwrap_or(&[])
            .iter()
            .collect();
        let sub_b: String = b_chars
            .get(span.b_start..span.b_end)
            .unwrap_or(&[])
            .iter()
            .collect();

        let sub_spans = char_diff(&sub_a, &sub_b);
        let total_sub_a_diff: usize = sub_spans
            .iter()
            .map(|s| s.a_end.saturating_sub(s.a_start))
            .sum();
        let total_sub_b_diff: usize = sub_spans
            .iter()
            .map(|s| s.b_end.saturating_sub(s.b_start))
            .sum();

        // Refine only when both sides keep the majority of their characters;
        // wholly rewritten words (e.g. `cat` vs `elephant`) keep one coarse span.
        if !sub_spans.is_empty()
            && total_sub_a_diff.saturating_mul(2) < a_len
            && total_sub_b_diff.saturating_mul(2) < b_len
        {
            for sub in sub_spans {
                refined.push(InlineSpan {
                    a_start: span.a_start.saturating_add(sub.a_start),
                    a_end: span.a_start.saturating_add(sub.a_end),
                    b_start: span.b_start.saturating_add(sub.b_start),
                    b_end: span.b_start.saturating_add(sub.b_end),
                });
            }
        } else {
            refined.push(span);
        }
    }

    refined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_diff() {
        let spans = char_diff("abcde", "abXde");
        assert_eq!(
            spans,
            vec![InlineSpan {
                a_start: 2,
                a_end: 3,
                b_start: 2,
                b_end: 3,
            }]
        );
    }

    #[test]
    fn test_word_char_diff_refinement() {
        // "my_variable_old" vs "my_variable_new" shares prefix "my_variable_"
        let spans = word_char_diff("let my_variable_old = 1;", "let my_variable_new = 1;");
        assert_eq!(
            spans,
            vec![InlineSpan {
                a_start: 16,
                a_end: 19,
                b_start: 16,
                b_end: 19,
            }]
        );

        // Completely different words retain coarse span
        let spans_coarse = word_char_diff("cat", "elephant");
        assert_eq!(
            spans_coarse,
            vec![InlineSpan {
                a_start: 0,
                a_end: 3,
                b_start: 0,
                b_end: 8,
            }]
        );
    }
}
