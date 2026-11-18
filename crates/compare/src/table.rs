//! Delimited table comparison over the filesystem.
//!
//! Files are read whole (quoted fields may span lines), decoded
//! leniently (invalid bytes become replacement characters) and parsed
//! by `autocompare_diff::parse_delimited` with the caller's delimiter
//! and header choice. The row script comes from the same engine used
//! for lines; cell-level highlighting stays the CLI's job, driven by
//! the public `diff_cells`.

use std::path::Path;

use autocompare_diff::{Table, diff_rows, parse_delimited};

use crate::{Error, Result, TableComparison, TableOptions};

/// Reads and parses one delimited file: bytes in, lenient UTF-8
/// decode, total parse. `NotFound` becomes [`Error::Missing`],
/// anything else becomes [`Error::Io`].
fn load_table(path: &Path, opts: &TableOptions) -> Result<Table> {
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
    Ok(parse_delimited(&text, opts.delimiter, opts.header))
}

/// Compares two delimited files. See the crate documentation.
pub(crate) fn compare_table_files(
    left: &Path,
    right: &Path,
    opts: &TableOptions,
) -> Result<TableComparison> {
    let left_table = load_table(left, opts)?;
    let right_table = load_table(right, opts)?;
    let script = diff_rows(&left_table.rows, &right_table.rows);
    Ok(TableComparison {
        header_l: left_table.header,
        header_r: right_table.header,
        rows_l: left_table.rows,
        rows_r: right_table.rows,
        script,
    })
}
