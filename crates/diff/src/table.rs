//! Delimited (CSV-style) table parsing and cell comparison.
//!
//! Pure layer 0: no I/O, no panics. Parsing is total — malformed
//! input is preserved as parsed, never an error. The parser walks
//! `char` boundaries only, so multibyte UTF-8, NUL bytes and
//! megabyte cells pass through untouched.

use std::iter::Peekable;
use std::mem;
use std::str::Chars;

use crate::engine;
use crate::{DiffScript, EDIT_BUDGET};

/// One parsed table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    /// Header cells when the first row is treated as a header.
    pub header: Vec<String>,
    /// Data rows; rows keep their own width (uneven is preserved).
    pub rows: Vec<Vec<String>>,
}

/// Parses delimiter-separated text, RFC-4180-lite: quoted fields with
/// doubled-quote escapes; delimiters and newlines inside quotes are
/// data; `\r\n` is tolerated; a trailing newline is not a row. Never
/// errors — uneven or unterminated input is preserved as parsed, with
/// these deliberate lenient rules:
///
/// * A `"` opens a quoted field only at the very start of a field; a
///   quote appearing mid-cell (as in `ab"c`) is ordinary data.
/// * Inside a quoted field, any `"` that is not the first half of a
///   `""` escape closes the field. Characters between a closing quote
///   and the next delimiter or row break (as in `"ab"x,`) stay in the
///   cell, which then parses as `abx`.
/// * An unterminated quoted field ends at the end of input; the cell
///   keeps everything read so far.
/// * Row breaks are `\n` and `\r\n`. Inside quotes both are data, and
///   a bare `\r` anywhere else is data too.
/// * A row exists only once something starts it: a data character, a
///   delimiter or a quote. Empty text therefore parses to zero rows
///   (never one empty row), and so does text holding only blank
///   lines. A lone quoted field `""` is one row with one empty cell.
/// * A trailing row break creates no trailing empty row, and
///   trailing blank lines are dropped; a blank line between content
///   is kept as a row with a single empty cell.
/// * Row breaks win over a delimiter configured as `\n` or `\r`, and
///   the delimiter wins over quoting when it is itself `"` (quoting
///   is then disabled).
pub fn parse_delimited(text: &str, delimiter: char, first_row_is_header: bool) -> Table {
    let mut parser = Parser::new(delimiter);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        parser.step(c, &mut chars);
    }
    parser.finish();
    let rows = parser.rows;
    let (header, data) = if first_row_is_header {
        let mut iter = rows.into_iter();
        (iter.next().unwrap_or_default(), iter.collect())
    } else {
        (Vec::new(), rows)
    };
    Table { header, rows: data }
}

/// Row-level diff over whole cell rows: the same Myers engine used
/// for lines, over `Vec<String>` rows.
pub fn diff_rows(a: &[Vec<String>], b: &[Vec<String>]) -> DiffScript {
    let (ops, exact) = engine::diff_slices(a, b, Some(EDIT_BUDGET));
    DiffScript { ops, exact }
}

/// Indices of columns whose cells differ between two rows, compared
/// position by position; a cell missing on one side counts as
/// differing. Sorted, no duplicates.
pub fn diff_cells(a: &[String], b: &[String]) -> Vec<usize> {
    let len = a.len().max(b.len());
    (0..len).filter(|&i| a.get(i) != b.get(i)).collect()
}

/// Where the parser stands inside the current field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldState {
    /// At the very start of a field: a `"` here opens a quoted field.
    Start,
    /// Inside an unquoted field: a `"` is ordinary data. Also the
    /// state right after a closing quote, where extra characters are
    /// leniently appended to the cell.
    Unquoted,
    /// Inside a quoted field: delimiter, `\n` and `\r` are data, and
    /// `""` is an escaped quote.
    Quoted,
}

/// One-pass, character-level state machine over the raw text.
struct Parser {
    delimiter: char,
    state: FieldState,
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    field: String,
    /// Whether anything (a data character, a delimiter or a quote)
    /// has started the current row.
    row_started: bool,
    /// Blank lines seen since the last row. They become rows with a
    /// single empty cell each once content follows, and are dropped
    /// at the end of input (trailing blank lines make no rows).
    pending_blanks: usize,
}

impl Parser {
    fn new(delimiter: char) -> Self {
        Self {
            delimiter,
            state: FieldState::Start,
            rows: Vec::new(),
            row: Vec::new(),
            field: String::new(),
            row_started: false,
            pending_blanks: 0,
        }
    }

    /// Consumes one character, looking ahead through `chars` when the
    /// decision needs the next character (`\r\n`, `""`).
    fn step(&mut self, c: char, chars: &mut Peekable<Chars<'_>>) {
        if self.state == FieldState::Quoted {
            // Inside quotes only `"` is special: doubled it is an
            // escape, anything else closes the field leniently.
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    self.field.push('"');
                } else {
                    self.state = FieldState::Unquoted;
                }
            } else {
                self.field.push(c);
            }
            return;
        }
        // Row breaks outrank everything, including a delimiter
        // configured as '\n' or '\r'.
        if c == '\n' {
            self.end_row();
        } else if c == '\r' && chars.peek() == Some(&'\n') {
            chars.next();
            self.end_row();
        } else if c == self.delimiter {
            self.end_field();
        } else if c == '"' && self.state == FieldState::Start {
            // A quote opens a quoted field only at the very start of
            // a field (and never when the delimiter is '"').
            self.start_row();
            self.state = FieldState::Quoted;
        } else {
            self.start_row();
            self.field.push(c);
            self.state = FieldState::Unquoted;
        }
    }

    /// The current row now has content: pending blank lines become
    /// empty-cell rows first.
    fn start_row(&mut self) {
        for _ in 0..self.pending_blanks {
            self.rows.push(vec![String::new()]);
        }
        self.pending_blanks = 0;
        self.row_started = true;
    }

    /// Ends the current field at a delimiter; an empty field is a
    /// real field.
    fn end_field(&mut self) {
        self.start_row();
        self.row.push(mem::take(&mut self.field));
        self.state = FieldState::Start;
    }

    /// Ends the current row at a row break; a break over a row that
    /// never started counts as a blank line instead.
    fn end_row(&mut self) {
        if self.row_started {
            self.flush_row();
        } else {
            self.pending_blanks = self.pending_blanks.saturating_add(1);
        }
        self.state = FieldState::Start;
    }

    /// Ends input: a started row is kept, pending blank lines are
    /// dropped (a trailing break makes no trailing empty row).
    fn finish(&mut self) {
        if self.row_started {
            self.flush_row();
        }
        self.pending_blanks = 0;
    }

    /// Commits the current row: the pending field, then the row.
    fn flush_row(&mut self) {
        self.row.push(mem::take(&mut self.field));
        self.rows.push(mem::take(&mut self.row));
        self.row_started = false;
    }
}
