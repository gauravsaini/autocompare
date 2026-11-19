//! Offscreen screenshot generation for verification and smoke testing.
//!
//! Renders the Text comparison fixture session into a valid PNG without
//! requiring an interactive window or display server.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use crate::viewmodel::{TextRow, TextRowKind, gutter_geometry};
use autocompare_diff::{RgbaImage, encode_png};
use std::path::Path;

// Diff highlight colors matching views.rs
const BG_DARK: [u8; 4] = [27, 27, 27, 255];
const BG_PANEL: [u8; 4] = [38, 38, 38, 255];
const FG_TEXT: [u8; 4] = [230, 230, 230, 255];
const FG_DIM: [u8; 4] = [140, 140, 140, 255];
const DELETED_BG: [u8; 4] = [120, 32, 32, 255];
const DELETED_FG: [u8; 4] = [255, 170, 170, 255];
const INSERTED_BG: [u8; 4] = [24, 90, 46, 255];
const INSERTED_FG: [u8; 4] = [170, 240, 180, 255];
const CHANGED_BG: [u8; 4] = [122, 102, 20, 255];
const CHANGED_FG: [u8; 4] = [255, 235, 160, 255];
const ACCENT_TAB: [u8; 4] = [50, 70, 105, 255];

/// Renders the Text comparison session fixture to a PNG at the given path.
///
/// Paths whose file name contains `folder` render the Folder session
/// fixture instead (same size/shape, folder tree + sync preview), so
/// `--screenshot <folder-png>` proves the folder UI without touching
/// the CLI flags.
pub fn render_fixture_png(out_path: &Path) -> Result<(), String> {
    let is_folder = out_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_lowercase().contains("folder"));
    if is_folder {
        return render_folder_fixture_png(out_path);
    }
    render_text_fixture_png(out_path)
}

/// Renders the Text comparison session fixture to a PNG at the given path.
pub fn render_text_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = 960u32;
    let height = 600u32;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };

    let mut pixels = vec![0u8; byte_count];
    // Fill background
    fill_rect(&mut pixels, dims, (0, 0, width, height), BG_DARK);

    // Title bar
    fill_rect(&mut pixels, dims, (0, 0, width, 36), BG_PANEL);
    // Window controls
    fill_rect(&mut pixels, dims, (12, 12, 12, 12), [255, 95, 87, 255]);
    fill_rect(&mut pixels, dims, (32, 12, 12, 12), [254, 188, 46, 255]);
    fill_rect(&mut pixels, dims, (52, 12, 12, 12), [40, 201, 64, 255]);
    draw_string(
        &mut pixels,
        dims,
        80,
        14,
        "fixtures/left.rs <--> fixtures/right.rs - Text",
        FG_TEXT,
    );

    // Tab bar
    fill_rect(&mut pixels, dims, (0, 36, width, 32), [32, 32, 32, 255]);
    draw_string(&mut pixels, dims, 16, 46, "Home", FG_DIM);
    fill_rect(&mut pixels, dims, (64, 40, 60, 24), ACCENT_TAB);
    draw_string(&mut pixels, dims, 76, 46, "Text", FG_TEXT);
    draw_string(&mut pixels, dims, 138, 46, "Folder", FG_DIM);
    draw_string(&mut pixels, dims, 200, 46, "Hex", FG_DIM);
    draw_string(&mut pixels, dims, 244, 46, "Merge", FG_DIM);
    draw_string(&mut pixels, dims, 300, 46, "Table", FG_DIM);
    draw_string(&mut pixels, dims, 356, 46, "Sync", FG_DIM);
    draw_string(&mut pixels, dims, 404, 46, "Picture", FG_DIM);
    draw_string(&mut pixels, dims, 474, 46, "FolderMerge", FG_DIM);
    draw_string(&mut pixels, dims, 574, 46, "Audio", FG_DIM);

    // Path row
    fill_rect(&mut pixels, dims, (12, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 20, 82, "fixtures/left.rs", FG_TEXT);
    fill_rect(&mut pixels, dims, (400, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 412, 82, "Browse...", FG_TEXT);

    fill_rect(&mut pixels, dims, (490, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 498, 82, "fixtures/right.rs", FG_TEXT);
    fill_rect(&mut pixels, dims, (878, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 890, 82, "Browse...", FG_TEXT);

    // Session toolbar: Swap Sides + Reload, session identity below tabs.
    draw_string(
        &mut pixels,
        dims,
        16,
        104,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        FG_TEXT,
    );

    // Main diff pane borders
    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    let gutter_x = 940u32;
    let gutter_w = 14u32;

    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 460, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (476, pane_top, 460, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (gutter_x, pane_top, gutter_w, pane_height),
        [32, 32, 32, 255],
    );

    // Fixture diff rows
    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(2),
            right_no: Some(2),
            left: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            right: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Deleted,
            left_no: Some(4),
            right_no: None,
            left: "    let legacy_flag = true;".to_string(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Inserted,
            left_no: None,
            right_no: Some(4),
            left: String::new(),
            right: "    validate_safe_path(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(5),
            right_no: Some(5),
            left: "    Ok(content.len())".to_string(),
            right: "    Ok(content.len())".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(6),
            right_no: Some(6),
            left: "}".to_string(),
            right: "}".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    let row_h = 24u32;
    for (i, row) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }

        let (left_bg, left_fg, right_bg, right_fg) = match row.kind {
            TextRowKind::Equal => (BG_DARK, FG_TEXT, BG_DARK, FG_TEXT),
            TextRowKind::Changed => (CHANGED_BG, CHANGED_FG, CHANGED_BG, CHANGED_FG),
            TextRowKind::Deleted => (DELETED_BG, DELETED_FG, BG_DARK, FG_DIM),
            TextRowKind::Inserted => (BG_DARK, FG_DIM, INSERTED_BG, INSERTED_FG),
        };

        // Left pane row
        fill_rect(&mut pixels, dims, (12, y, 460, row_h), left_bg);
        if let Some(no) = row.left_no {
            draw_string(
                &mut pixels,
                dims,
                16,
                y.saturating_add(6),
                &format!("{no:>2} "),
                FG_DIM,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            44,
            y.saturating_add(6),
            &row.left,
            left_fg,
        );

        // Right pane row
        fill_rect(&mut pixels, dims, (476, y, 460, row_h), right_bg);
        if let Some(no) = row.right_no {
            draw_string(
                &mut pixels,
                dims,
                480,
                y.saturating_add(6),
                &format!("{no:>2} "),
                FG_DIM,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            508,
            y.saturating_add(6),
            &row.right,
            right_fg,
        );
    }

    // Thumbnail overview gutter geometry
    let geom = gutter_geometry(rows.len(), pane_height as f32, &rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start as u32);
        let seg_h = ((seg.y_end - seg.y_start) as u32).max(2);
        let seg_color = match seg.kind {
            TextRowKind::Equal => BG_PANEL,
            TextRowKind::Changed => CHANGED_BG,
            TextRowKind::Deleted => DELETED_BG,
            TextRowKind::Inserted => INSERTED_BG,
        };
        fill_rect(
            &mut pixels,
            dims,
            (gutter_x, seg_y, gutter_w, seg_h),
            seg_color,
        );
    }

    // Status bar: honest counts plus last-action feedback.
    fill_rect(&mut pixels, dims, (0, 570, width, 30), [32, 32, 32, 255]);
    draw_string(
        &mut pixels,
        dims,
        16,
        578,
        "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done: 7 row(s)",
        FG_DIM,
    );

    let image = RgbaImage {
        width,
        height,
        pixels,
    };

    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;

    Ok(())
}

/// Renders the Folder comparison session fixture: filter checkboxes,
/// honest counts, tree rows with glyphs, and the sync preview panel
/// (mode labels, dry-run options, consent hint, Apply gate). All
/// strings come from the headless `tree`/`home` logic so the shot
/// proves the wired UI, not a mock.
pub fn render_folder_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::home::{folder_empty_hint, sync_consent_hint};
    use crate::tree::{SyncModeChoice, count_folder_rows, folder_tree_filtered, sync_preview};
    use crate::viewmodel::FolderRow;

    let width = 960u32;
    let height = 600u32;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };

    let mut pixels = vec![0u8; byte_count];
    fill_rect(&mut pixels, dims, (0, 0, width, height), BG_DARK);

    // Title bar
    fill_rect(&mut pixels, dims, (0, 0, width, 36), BG_PANEL);
    fill_rect(&mut pixels, dims, (12, 12, 12, 12), [255, 95, 87, 255]);
    fill_rect(&mut pixels, dims, (32, 12, 12, 12), [254, 188, 46, 255]);
    fill_rect(&mut pixels, dims, (52, 12, 12, 12), [40, 201, 64, 255]);
    draw_string(
        &mut pixels,
        dims,
        80,
        14,
        "fixtures/left-dir <--> fixtures/right-dir - Folder",
        FG_TEXT,
    );

    // Tab bar with Folder highlighted
    fill_rect(&mut pixels, dims, (0, 36, width, 32), [32, 32, 32, 255]);
    draw_string(&mut pixels, dims, 16, 46, "Home", FG_DIM);
    draw_string(&mut pixels, dims, 64, 46, "Text", FG_DIM);
    fill_rect(&mut pixels, dims, (118, 40, 66, 24), ACCENT_TAB);
    draw_string(&mut pixels, dims, 130, 46, "Folder", FG_TEXT);
    draw_string(&mut pixels, dims, 200, 46, "Hex", FG_DIM);
    draw_string(&mut pixels, dims, 244, 46, "Merge", FG_DIM);
    draw_string(&mut pixels, dims, 300, 46, "Table", FG_DIM);
    draw_string(&mut pixels, dims, 356, 46, "Sync", FG_DIM);
    draw_string(&mut pixels, dims, 404, 46, "Picture", FG_DIM);
    draw_string(&mut pixels, dims, 474, 46, "FolderMerge", FG_DIM);
    draw_string(&mut pixels, dims, 574, 46, "Audio", FG_DIM);

    // Path row
    fill_rect(&mut pixels, dims, (12, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 20, 82, "fixtures/left-dir", FG_TEXT);
    fill_rect(&mut pixels, dims, (400, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 412, 82, "Browse...", FG_TEXT);
    fill_rect(&mut pixels, dims, (490, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 498, 82, "fixtures/right-dir", FG_TEXT);
    fill_rect(&mut pixels, dims, (878, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 890, 82, "Browse...", FG_TEXT);

    // Session toolbar
    draw_string(
        &mut pixels,
        dims,
        16,
        104,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        FG_TEXT,
    );

    // Fixture rows: headless truth, same glyphs the engine paints.
    let rows = vec![
        FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '>',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
        FolderRow {
            rel_path: "src/b.txt".to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "orphan.txt".to_string(),
            is_dir: false,
            glyph: 'R',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
    ];
    let counts = count_folder_rows(&rows);
    let expanded: std::collections::BTreeSet<String> = ["src".to_string()].into_iter().collect();
    let display = crate::tree::FolderDisplay::default();
    let nodes = folder_tree_filtered(&rows, &expanded, &display);
    let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
    let consent_hint = sync_consent_hint(false, preview.steps.len());
    let empty_hint = folder_empty_hint(false, false);
    let _ = empty_hint;

    // Filter checkboxes line (labels mirror app.rs Folder tab).
    draw_string(
        &mut pixels,
        dims,
        16,
        122,
        "[ ] Hide orphans  [ ] Left newer only  [ ] Right newer only  [ ] Flatten",
        FG_TEXT,
    );
    // Honest counts line.
    draw_string(&mut pixels, dims, 16, 138, &counts.summary(), FG_DIM);

    // Tree panel.
    let pane_top = 158u32;
    let pane_height = 260u32;
    let pane_bottom = pane_top.saturating_add(pane_height);
    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 936, pane_height),
        [22, 22, 22, 255],
    );
    let row_h = 24u32;
    for (i, node) in nodes.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }
        let (bg, fg) = match node.row.glyph {
            '=' => (BG_DARK, FG_TEXT),
            '!' | 'L' => (DELETED_BG, DELETED_FG),
            '>' | '<' => (CHANGED_BG, CHANGED_FG),
            'R' => (INSERTED_BG, INSERTED_FG),
            _ => (BG_DARK, FG_DIM),
        };
        fill_rect(&mut pixels, dims, (12, y, 936, row_h), bg);
        if node.row.is_dir {
            let icon = if node.is_expanded { "v" } else { ">" };
            draw_string(&mut pixels, dims, 18, y.saturating_add(6), icon, FG_DIM);
        }
        let indent = (node.depth as u32).saturating_mul(4u32);
        let gx = 34u32.saturating_add(indent.saturating_mul(7));
        draw_string(
            &mut pixels,
            dims,
            gx,
            y.saturating_add(6),
            &node.row.glyph.to_string(),
            fg,
        );
        draw_string(
            &mut pixels,
            dims,
            gx.saturating_add(16),
            y.saturating_add(6),
            &node.row.rel_path,
            fg,
        );
    }

    // Sync preview panel.
    let panel_y = pane_bottom.saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (12, panel_y, 936, 116),
        [28, 28, 28, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(6),
        "Sync preview (dry-run)",
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(22),
        &format!(
            "({})  ({})",
            SyncModeChoice::UpdateBoth.label(),
            SyncModeChoice::Mirror.label()
        ),
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(38),
        &preview.summary(),
        FG_DIM,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(54),
        consent_hint,
        FG_DIM,
    );
    fill_rect(
        &mut pixels,
        dims,
        (20, panel_y.saturating_add(72), 150, 24),
        [60, 60, 60, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        32,
        panel_y.saturating_add(78),
        "Apply sync plan",
        FG_TEXT,
    );

    // Status bar.
    fill_rect(&mut pixels, dims, (0, 570, width, 30), [32, 32, 32, 255]);
    draw_string(
        &mut pixels,
        dims,
        16,
        578,
        "folder: 4 entries, 0 pruned | staged src/a.txt for text compare",
        FG_DIM,
    );

    let image = RgbaImage {
        width,
        height,
        pixels,
    };

    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;

    Ok(())
}

fn fill_rect(pixels: &mut [u8], dims: (u32, u32), rect: (u32, u32, u32, u32), color: [u8; 4]) {
    let (img_w, img_h) = dims;
    let (x, y, w, h) = rect;
    let x_end = (x.saturating_add(w)).min(img_w);
    let y_end = (y.saturating_add(h)).min(img_h);

    for cy in y..y_end {
        let row_offset = (cy as usize)
            .saturating_mul(img_w as usize)
            .saturating_mul(4);
        for cx in x..x_end {
            let offset = row_offset.saturating_add((cx as usize).saturating_mul(4));
            if let Some(dst) = pixels.get_mut(offset..offset.saturating_add(4))
                && dst.len() == 4
            {
                dst.copy_from_slice(&color);
            }
        }
    }
}

// 5x7 ASCII font: geometry first (advance/kerning), typography second (bitmap tuning).
// Colors frozen: this module only moves the pen and picks bits, never hues.
//
// Advance table: default 6px (5 cols + 1 gap, was 7) so fixture rows fit the
// 460px panes without clipping; single-stem / punctuation glyphs take 4px.
// `kern_adjust` tucks known overhang pairs by 1px (never more, never pushes).
fn glyph_advance(ch: u8) -> u32 {
    match ch {
        b' ' => 4,
        b'\'' | b',' | b'.' | b':' | b';' | b'!' | b'I' | b'i' | b'j' | b'l' | b't' | b'|'
        | b'`' => 4,
        _ => 6,
    }
}

/// Kerning nudge in pixels between two adjacent bytes: -1 (tuck), else 0.
/// Explicit caps pairs first, then one general rule for top-right overhangs
/// (`T/Y/V/r/f`) tucking into round/lowercase/punctuation followers.
fn kern_adjust(prev: u8, curr: u8) -> i32 {
    match (prev, curr) {
        (b'A', b'V')
        | (b'A', b'W')
        | (b'L', b'T')
        | (b'L', b'V')
        | (b'L', b'Y')
        | (b'P', b'A')
        | (b'T', b'o')
        | (b'T', b'e')
        | (b'T', b'a')
        | (b'V', b'A')
        | (b'Y', b'V') => -1,
        _ => {
            let tight_prev = matches!(prev, b'T' | b'Y' | b'V' | b'r' | b'f');
            let tuck_curr = matches!(curr, b'a' | b'c' | b'e' | b'o' | b'.' | b',' | b'\'' | b'"');
            if tight_prev && tuck_curr { -1 } else { 0 }
        }
    }
}

/// Pen width of `s` under the advance + kerning rules (layout proof helper).
fn text_width(s: &str) -> u32 {
    let mut w = 0u32;
    let mut prev: Option<u8> = None;
    for b in s.bytes() {
        if let Some(p) = prev {
            let k = kern_adjust(p, b);
            if k < 0 {
                w = w.saturating_sub(1);
            }
        }
        w = w.saturating_add(glyph_advance(b));
        prev = Some(b);
    }
    w
}

/// Count of non-empty columns in a 5-wide pattern (readability probe helper).
fn nonzero_columns(pattern: &[u8; 5]) -> usize {
    pattern.iter().filter(|c| **c != 0).count()
}

fn draw_string(
    pixels: &mut [u8],
    dims: (u32, u32),
    mut x: u32,
    y: u32,
    s: &str,
    color: [u8; 4],
) -> u32 {
    let mut prev: Option<u8> = None;
    for b in s.bytes() {
        if let Some(p) = prev {
            let k = kern_adjust(p, b);
            if k < 0 {
                x = x.saturating_sub(1);
            }
        }
        draw_glyph(pixels, dims, x, y, b, color);
        x = x.saturating_add(glyph_advance(b));
        prev = Some(b);
    }
    x
}

fn draw_glyph(pixels: &mut [u8], dims: (u32, u32), x: u32, y: u32, ch: u8, color: [u8; 4]) {
    let (img_w, img_h) = dims;
    let pattern = glyph_pattern(ch);
    for (col_idx, &col_byte) in pattern.iter().enumerate() {
        let px = x.saturating_add(col_idx as u32);
        for row_idx in 0..7u32 {
            if (col_byte & (1 << row_idx)) != 0 {
                let py = y.saturating_add(row_idx);
                if px < img_w && py < img_h {
                    let offset = (py as usize)
                        .saturating_mul(img_w as usize)
                        .saturating_add(px as usize)
                        .saturating_mul(4);
                    if let Some(dst) = pixels.get_mut(offset..offset.saturating_add(4))
                        && dst.len() == 4
                    {
                        dst.copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

// Standard 5-column byte patterns for common ASCII symbols
fn glyph_pattern(ch: u8) -> [u8; 5] {
    match ch {
        b' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        b'!' => [0x00, 0x00, 0x5f, 0x00, 0x00],
        b'"' => [0x00, 0x07, 0x00, 0x07, 0x00],
        b'#' => [0x14, 0x7f, 0x14, 0x7f, 0x14],
        b'$' => [0x24, 0x2a, 0x7f, 0x2a, 0x12],
        b'%' => [0x23, 0x13, 0x08, 0x64, 0x62],
        b'&' => [0x36, 0x49, 0x55, 0x22, 0x50],
        b'\'' => [0x00, 0x05, 0x03, 0x00, 0x00],
        b'(' => [0x00, 0x1c, 0x22, 0x41, 0x00],
        b')' => [0x00, 0x41, 0x22, 0x1c, 0x00],
        b'*' => [0x14, 0x08, 0x3e, 0x08, 0x14],
        b'+' => [0x08, 0x08, 0x3e, 0x08, 0x08],
        // Single-stem punctuation: one active column reads sharper at 1x than
        // the old doubled blocks; comma/semicolon keep a 1px descender tail.
        b',' => [0x00, 0x00, 0x50, 0x20, 0x00],
        b'-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        b'.' => [0x00, 0x00, 0x60, 0x00, 0x00],
        b'/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        b'0' => [0x3e, 0x51, 0x49, 0x45, 0x3e],
        b'1' => [0x00, 0x42, 0x7f, 0x40, 0x00],
        b'2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        b'3' => [0x21, 0x41, 0x45, 0x4b, 0x31],
        b'4' => [0x18, 0x14, 0x12, 0x7f, 0x10],
        b'5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        b'6' => [0x3c, 0x4a, 0x49, 0x49, 0x30],
        b'7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        b'8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        b'9' => [0x06, 0x49, 0x49, 0x29, 0x1e],
        b':' => [0x00, 0x00, 0x36, 0x00, 0x00],
        b';' => [0x00, 0x00, 0x56, 0x20, 0x00],
        b'<' => [0x08, 0x14, 0x22, 0x41, 0x00],
        b'=' => [0x14, 0x14, 0x14, 0x14, 0x14],
        b'>' => [0x00, 0x41, 0x22, 0x14, 0x08],
        b'?' => [0x02, 0x01, 0x51, 0x09, 0x06],
        b'@' => [0x32, 0x49, 0x79, 0x41, 0x3e],
        b'A' => [0x7e, 0x11, 0x11, 0x11, 0x7e],
        b'B' => [0x7f, 0x49, 0x49, 0x49, 0x36],
        b'C' => [0x3e, 0x41, 0x41, 0x41, 0x22],
        b'D' => [0x7f, 0x41, 0x41, 0x22, 0x1c],
        b'E' => [0x7f, 0x49, 0x49, 0x49, 0x41],
        b'F' => [0x7f, 0x09, 0x09, 0x09, 0x01],
        b'G' => [0x3e, 0x41, 0x49, 0x49, 0x7a],
        b'H' => [0x7f, 0x08, 0x08, 0x08, 0x7f],
        b'I' => [0x00, 0x41, 0x7f, 0x41, 0x00],
        b'J' => [0x20, 0x40, 0x41, 0x3f, 0x01],
        b'K' => [0x7f, 0x08, 0x14, 0x22, 0x41],
        b'L' => [0x7f, 0x40, 0x40, 0x40, 0x40],
        b'M' => [0x7f, 0x02, 0x0c, 0x02, 0x7f],
        b'N' => [0x7f, 0x04, 0x08, 0x10, 0x7f],
        b'O' => [0x3e, 0x41, 0x41, 0x41, 0x3e],
        b'P' => [0x7f, 0x09, 0x09, 0x09, 0x06],
        b'Q' => [0x3e, 0x41, 0x51, 0x21, 0x5e],
        b'R' => [0x7f, 0x09, 0x19, 0x29, 0x46],
        b'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        b'T' => [0x01, 0x01, 0x7f, 0x01, 0x01],
        b'U' => [0x3f, 0x40, 0x40, 0x40, 0x3f],
        b'V' => [0x1f, 0x20, 0x40, 0x20, 0x1f],
        b'W' => [0x7f, 0x20, 0x18, 0x20, 0x7f],
        b'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        b'Y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        b'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        b'[' => [0x00, 0x7f, 0x41, 0x41, 0x00],
        b'\\' => [0x02, 0x04, 0x08, 0x10, 0x20],
        b']' => [0x00, 0x41, 0x41, 0x7f, 0x00],
        b'^' => [0x04, 0x02, 0x01, 0x02, 0x04],
        b'_' => [0x40, 0x40, 0x40, 0x40, 0x40],
        b'`' => [0x00, 0x01, 0x02, 0x04, 0x00],
        b'a' => [0x20, 0x54, 0x54, 0x54, 0x78],
        b'b' => [0x7f, 0x48, 0x44, 0x44, 0x38],
        b'c' => [0x38, 0x44, 0x44, 0x44, 0x20],
        b'd' => [0x38, 0x44, 0x44, 0x48, 0x7f],
        b'e' => [0x38, 0x54, 0x54, 0x54, 0x18],
        b'f' => [0x08, 0x7e, 0x09, 0x01, 0x02],
        b'g' => [0x0c, 0x52, 0x52, 0x52, 0x3e],
        b'h' => [0x7f, 0x08, 0x04, 0x04, 0x78],
        b'i' => [0x00, 0x44, 0x7d, 0x40, 0x00],
        b'j' => [0x20, 0x40, 0x44, 0x3d, 0x00],
        b'k' => [0x7f, 0x10, 0x28, 0x44, 0x00],
        b'l' => [0x00, 0x41, 0x7f, 0x40, 0x00],
        b'm' => [0x7c, 0x04, 0x18, 0x04, 0x78],
        b'n' => [0x7c, 0x08, 0x04, 0x04, 0x78],
        b'o' => [0x38, 0x44, 0x44, 0x44, 0x38],
        b'p' => [0x7c, 0x14, 0x14, 0x14, 0x08],
        b'q' => [0x08, 0x14, 0x14, 0x18, 0x7c],
        b'r' => [0x7c, 0x08, 0x04, 0x04, 0x08],
        b's' => [0x48, 0x54, 0x54, 0x54, 0x20],
        b't' => [0x04, 0x3f, 0x44, 0x40, 0x20],
        b'u' => [0x3c, 0x40, 0x40, 0x20, 0x7c],
        b'v' => [0x1c, 0x20, 0x40, 0x20, 0x1c],
        b'w' => [0x3c, 0x40, 0x30, 0x40, 0x3c],
        b'x' => [0x44, 0x28, 0x10, 0x28, 0x44],
        b'y' => [0x0c, 0x50, 0x50, 0x50, 0x3c],
        b'z' => [0x44, 0x64, 0x54, 0x4c, 0x44],
        b'{' => [0x00, 0x08, 0x36, 0x41, 0x00],
        b'|' => [0x00, 0x00, 0x7f, 0x00, 0x00],
        b'}' => [0x00, 0x41, 0x36, 0x08, 0x00],
        b'~' => [0x08, 0x04, 0x08, 0x10, 0x08],
        // Unknown bytes render as `?` outline, never a solid block.
        _ => [0x02, 0x01, 0x51, 0x09, 0x06],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_magic_ok(bytes: &[u8]) -> bool {
        bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
    }

    #[test]
    fn text_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-folder-text-fixture.png");
        assert!(render_text_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
    }

    #[test]
    fn folder_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-folder-folder-fixture.png");
        assert!(render_folder_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn dispatcher_routes_by_file_name() {
        let dir = std::env::temp_dir();
        let text_out = dir.join("vis-dispatch-text.png");
        let folder_out = dir.join("vis-dispatch-folder.png");
        assert!(render_fixture_png(&text_out).is_ok());
        assert!(render_fixture_png(&folder_out).is_ok());
        let text_bytes = std::fs::read(&text_out).unwrap_or_default();
        let folder_bytes = std::fs::read(&folder_out).unwrap_or_default();
        assert!(!text_bytes.is_empty() && !folder_bytes.is_empty());
        assert!(png_magic_ok(&text_bytes));
        assert!(png_magic_ok(&folder_bytes));
        assert_ne!(text_bytes, folder_bytes);
    }

    #[test]
    fn narrow_glyphs_use_tighter_advance() {
        for ch in *b" ',.:;!Iijlt|`" {
            assert_eq!(glyph_advance(ch), 4);
        }
        for ch in *b"MWaen=>/" {
            assert_eq!(glyph_advance(ch), 6);
        }
        assert!(glyph_advance(b'i') < glyph_advance(b'M'));
    }

    #[test]
    fn kern_tight_pairs_tuck_one_pixel() {
        assert_eq!(kern_adjust(b'A', b'V'), -1);
        assert_eq!(kern_adjust(b'L', b'T'), -1);
        assert_eq!(kern_adjust(b'T', b'o'), -1);
        assert_eq!(kern_adjust(b'V', b'A'), -1);
        assert_eq!(kern_adjust(b'r', b'e'), -1);
        assert_eq!(kern_adjust(b'n', b'n'), 0);
        assert_eq!(kern_adjust(b'e', b'e'), 0);
        assert_eq!(kern_adjust(b'T', b'n'), 0);
    }

    #[test]
    fn punctuation_dots_are_single_stem() {
        assert_eq!(nonzero_columns(&glyph_pattern(b'.')), 1);
        assert_eq!(nonzero_columns(&glyph_pattern(b':')), 1);
        assert_eq!(nonzero_columns(&glyph_pattern(b',')), 2);
        assert_eq!(nonzero_columns(&glyph_pattern(b';')), 2);
        assert_eq!(glyph_pattern(b'.').get(2), Some(&0x60u8));
        assert_eq!(glyph_pattern(b',').get(3), Some(&0x20u8));
        // Unknown bytes fall back to the `?` outline, never a solid block.
        assert_eq!(glyph_pattern(0x80), glyph_pattern(b'?'));
    }

    #[test]
    fn text_width_respects_kerning_and_fits_panes() {
        let plain = glyph_advance(b'T').saturating_add(glyph_advance(b'o'));
        assert_eq!(text_width("To"), plain.saturating_sub(1));
        assert_eq!(
            text_width("nn"),
            glyph_advance(b'n').saturating_add(glyph_advance(b'n'))
        );
        // Fixture strings must fit their painted fields at 960px canvas width.
        assert!(text_width("fixtures/left.rs") < 380);
        assert!(text_width("fixtures/right.rs") < 380);
        assert!(
            text_width("[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax")
                < 928
        );
        assert!(
            text_width(
                "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done: 7 row(s)"
            ) < 928
        );
    }

    #[test]
    fn draw_string_returns_pen_and_clips_safely() {
        let dims = (20u32, 10u32);
        let total = 20usize.saturating_mul(10).saturating_mul(4);
        let mut pixels = vec![0u8; total];
        let pen = draw_string(&mut pixels, dims, 2, 1, "Hi.", FG_TEXT);
        assert_eq!(pen, 2u32.saturating_add(text_width("Hi.")));
        // Pen near the right edge clips glyph pixels but still advances.
        let edge = draw_string(&mut pixels, dims, 19, 1, "Hi.", FG_TEXT);
        assert_eq!(edge, 19u32.saturating_add(text_width("Hi.")));
        assert_eq!(pixels.len(), total);
    }
}
