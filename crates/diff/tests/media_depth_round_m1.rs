//! Media-depth round-m1: Beyond Compare parity edge cases.
//!
//! Covers the workbook RightOnly fix (a sheet present only on the
//! right must be reported, not silently dropped) and the media-tags
//! totality fix (extra_tags and container format now participate in
//! the diff instead of being ignored).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use autocompare_diff::media_tags::{MediaFormat, MediaTags, diff_media_tags};
use autocompare_diff::{Sheet, SheetDiffStatus, Table, Workbook, diff_workbooks};

fn sheet(name: &str, rows: Vec<Vec<&str>>) -> Sheet {
    Sheet {
        name: name.to_string(),
        table: Table {
            header: Vec::new(),
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(|c| c.to_string()).collect())
                .collect(),
        },
    }
}

#[test]
fn workbook_reports_right_only_sheet() {
    let left = Workbook {
        sheets: vec![sheet("Shared", vec![vec!["a"]])],
    };
    let right = Workbook {
        sheets: vec![
            sheet("Shared", vec![vec!["a"]]),
            sheet("Extra", vec![vec!["b"]]),
        ],
    };
    let diff = diff_workbooks(&left, &right);
    assert!(!diff.identical);
    assert_eq!(diff.sheet_diffs.len(), 2);
    let extra = diff
        .sheet_diffs
        .iter()
        .find(|s| s.name == "Extra")
        .expect("right-only sheet must be reported");
    assert!(matches!(extra.status, SheetDiffStatus::RightOnly));
}

#[test]
fn workbook_reports_left_only_sheet() {
    let left = Workbook {
        sheets: vec![
            sheet("Shared", vec![vec!["a"]]),
            sheet("Gone", vec![vec!["x"]]),
        ],
    };
    let right = Workbook {
        sheets: vec![sheet("Shared", vec![vec!["a"]])],
    };
    let diff = diff_workbooks(&left, &right);
    assert!(!diff.identical);
    let gone = diff
        .sheet_diffs
        .iter()
        .find(|s| s.name == "Gone")
        .expect("left-only sheet must be reported");
    assert!(matches!(gone.status, SheetDiffStatus::LeftOnly));
}

#[test]
fn media_tags_diff_covers_extra_tags_and_format() {
    let left = MediaTags {
        title: Some("Song".to_string()),
        format: MediaFormat::Mp3,
        extra_tags: vec![("ENCODER".to_string(), "LAME".to_string())],
        ..Default::default()
    };
    let right = MediaTags {
        title: Some("Song".to_string()),
        format: MediaFormat::Flac,
        extra_tags: vec![("ENCODER".to_string(), "FLAC-1.4".to_string())],
        ..Default::default()
    };
    let diff = diff_media_tags(&left, &right);
    assert!(!diff.identical);
    assert!(
        diff.differing.iter().any(|(f, _, _)| f == "Format"),
        "container format must participate, got {:?}",
        diff.differing
    );
    assert!(
        diff.differing
            .iter()
            .any(|(f, l, r)| f == "ENCODER" && l == "LAME" && r == "FLAC-1.4"),
        "extra_tags must be compared, got {:?}",
        diff.differing
    );
}

#[test]
fn media_tags_extra_one_sided() {
    let left = MediaTags {
        format: MediaFormat::Mp3,
        ..Default::default()
    };
    let right = MediaTags {
        format: MediaFormat::Mp3,
        extra_tags: vec![("BPM".to_string(), "120".to_string())],
        ..Default::default()
    };
    let diff = diff_media_tags(&left, &right);
    assert!(!diff.identical);
    assert!(
        diff.right_only
            .iter()
            .any(|(f, v)| f == "BPM" && v == "120"),
        "one-sided extra tag must be right_only, got {:?}",
        diff.right_only
    );
}
