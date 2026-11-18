//! egui painting of the view models.
//!
//! All interpretation of engine results happened in
//! [`crate::viewmodel`]; this file only turns rows into widgets. The
//! few pure helpers here (`pane_segments`, `gutter_text`,
//! `folder_columns`, `premultiplied_rgba`, `heatmap_color_image`) are
//! paint-adjacent logic and are unit-tested headlessly alongside the
//! viewmodel.
//!
//! The diff palette is fixed rather than theme-following: a deleted
//! row must read the same in light and dark mode, so its colors carry
//! their own contrast.

use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Color32, FontId, RichText, Stroke, TextEdit, TextFormat, TextureHandle, Ui,
};

use crate::app::Tab;
use crate::viewmodel::{self, FolderRow, TextRow, TextRowKind, TextRows};

// The fixed diff palette (semantic colors, own contrast).
const DELETED_FG: Color32 = Color32::from_rgb(255, 170, 170);
const DELETED_BG: Color32 = Color32::from_rgb(120, 32, 32);
const INSERTED_FG: Color32 = Color32::from_rgb(170, 240, 180);
const INSERTED_BG: Color32 = Color32::from_rgb(24, 90, 46);
const CHANGED_FG: Color32 = Color32::from_rgb(255, 235, 160);
const CHANGED_BG: Color32 = Color32::from_rgb(122, 102, 20);
/// Accent color for changed spans. egui ships no bold monospace face,
/// so the CLI's "bold" half of the bold+underline accent becomes
/// egui's own notion of strong: a brighter color, plus the underline.
const ACCENT_FG: Color32 = Color32::from_rgb(255, 255, 210);
const DIM: Color32 = Color32::from_rgb(150, 150, 150);
const NEWER_BLUE: Color32 = Color32::from_rgb(130, 170, 255);
const RED: Color32 = Color32::from_rgb(255, 100, 100);
const GREEN: Color32 = Color32::from_rgb(120, 220, 130);
const ERROR_BG: Color32 = Color32::from_rgb(60, 20, 20);

/// Header: title plus the standing hint that this client has no
/// file-dialog dependency.
pub fn paint_header(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.heading("autocompare");
        ui.label(RichText::new("desktop client").weak());
    });
    ui.label(
        RichText::new("paths are typed, not picked - no file-dialog dependency by design")
            .small()
            .weak(),
    );
    ui.add_space(4.0);
}

/// The three comparison tabs.
pub fn paint_tabs(ui: &mut Ui, tab: &mut Tab) {
    ui.horizontal(|ui| {
        ui.selectable_value(tab, Tab::Text, "Text");
        ui.selectable_value(tab, Tab::Folder, "Folder");
        ui.selectable_value(tab, Tab::Picture, "Picture");
    });
    ui.add_space(2.0);
}

/// Two path text fields plus the Compare button. Sets `compare` when
/// the button is clicked.
pub fn paint_paths(
    ui: &mut Ui,
    noun: &str,
    left: &mut String,
    right: &mut String,
    compare: &mut bool,
) {
    ui.horizontal(|ui| {
        let half = (ui.available_width() / 2.0).max(140.0) - 8.0;
        ui.add(
            TextEdit::singleline(left)
                .desired_width(half)
                .hint_text(format!("left {noun} path")),
        );
        ui.add(
            TextEdit::singleline(right)
                .desired_width(half)
                .hint_text(format!("right {noun} path")),
        );
    });
    let clicked = ui
        .horizontal(|ui| ui.button(format!("Compare {noun}s")).clicked())
        .inner;
    if clicked {
        *compare = true;
    }
    ui.add_space(4.0);
}

/// The red error panel: every engine failure ends up here, never as a
/// crash.
pub fn paint_error(ui: &mut Ui, message: &str) {
    let mut frame = egui::Frame::group(ui.style());
    frame.fill = ERROR_BG;
    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("error:")
                    .monospace()
                    .strong()
                    .color(DELETED_FG),
            );
            ui.label(RichText::new(message).color(DELETED_FG));
        });
    });
}

fn paint_note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().weak());
}

// ----------------------------------------------------------------------------
// Text view
// ----------------------------------------------------------------------------

/// The side-by-side text comparison: notes first, then the aligned
/// grid. Rows are virtualized (`show_rows`), so a capped 20_000-row
/// model paints only the visible ones.
pub fn paint_text(ui: &mut Ui, model: &TextRows) {
    if model.coarse {
        paint_note(
            ui,
            "inputs exceeded the edit budget; showing a coarse comparison",
        );
    }
    if model.truncated {
        paint_note(
            ui,
            &format!(
                "showing the first {} rows; the rest is cut off",
                viewmodel::MAX_ROWS
            ),
        );
    }
    if model.rows.is_empty() {
        ui.label(RichText::new("(nothing to display: both sides are empty)").weak());
        return;
    }
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let base_text = ui.style().visuals.text_color();
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, model.rows.len(), |ui, range| {
            for i in range {
                let Some(row) = model.rows.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                ui.horizontal(|ui| {
                    let (fg, bg) = kind_palette(row.kind, base_text);
                    ui.label(side_job(Side::Left, row, model, font_id.clone(), fg, bg));
                    ui.label(RichText::new(" | ").monospace().color(DIM));
                    ui.label(side_job(Side::Right, row, model, font_id.clone(), fg, bg));
                });
            }
        });
}

/// Which side of a text row a pane renders.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

/// One pane of one row as a single styled job: a dim line-number
/// gutter, the text with accent runs, and padding so every pane is
/// exactly `pane_chars` wide and the two columns stay aligned.
/// `keep_trailing_whitespace` keeps the padding (and blank sides)
/// from collapsing.
fn side_job(
    side: Side,
    row: &TextRow,
    model: &TextRows,
    font_id: FontId,
    fg: Color32,
    bg: Color32,
) -> LayoutJob {
    let (no, line, spans) = match side {
        Side::Left => (row.left_no, row.left.as_str(), row.accent_a.as_slice()),
        Side::Right => (row.right_no, row.right.as_str(), row.accent_b.as_slice()),
    };
    let mut job = LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut gutter = TextFormat::simple(font_id.clone(), DIM);
    gutter.background = bg;
    job.append(&gutter_text(no, model.gutter_chars), 0.0, gutter);
    for (segment, accented) in pane_segments(line, spans, model.pane_chars) {
        let mut fmt = TextFormat::simple(font_id.clone(), fg);
        fmt.background = bg;
        if accented {
            fmt.color = ACCENT_FG;
            fmt.underline = Stroke::new(1.0, ACCENT_FG);
        }
        job.append(&segment, 0.0, fmt);
    }
    job
}

/// (text color, row background) for one row kind. Equal rows keep the
/// theme's text color on a transparent background.
fn kind_palette(kind: TextRowKind, base_text: Color32) -> (Color32, Color32) {
    match kind {
        TextRowKind::Equal => (base_text, Color32::TRANSPARENT),
        TextRowKind::Deleted => (DELETED_FG, DELETED_BG),
        TextRowKind::Inserted => (INSERTED_FG, INSERTED_BG),
        TextRowKind::Changed => (CHANGED_FG, CHANGED_BG),
    }
}

/// The line-number gutter: right-aligned number plus one separating
/// space, or blanks when the row has no such side.
fn gutter_text(no: Option<usize>, width: usize) -> String {
    match no {
        Some(n) => format!("{n:>width$} "),
        None => " ".repeat(width.saturating_add(1)),
    }
}

/// Splits `line` into (text, accented) segments at the half-open
/// char ranges in `spans`, truncated at `pane` characters like the
/// CLI's panes, with one trailing unaccented segment padding to the
/// full pane width. Pure; unit-tested.
fn pane_segments(line: &str, spans: &[(usize, usize)], pane: usize) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    let mut current_accent = false;
    let mut visible = 0usize;
    for (idx, ch) in line.chars().enumerate() {
        if visible >= pane {
            break;
        }
        let inside = spans.iter().any(|&(start, end)| idx >= start && idx < end);
        if inside != current_accent && !current.is_empty() {
            out.push((std::mem::take(&mut current), current_accent));
        }
        current_accent = inside;
        current.push(ch);
        visible = visible.saturating_add(1);
    }
    if !current.is_empty() {
        out.push((current, current_accent));
    }
    if visible < pane {
        out.push((" ".repeat(pane - visible), false));
    }
    out
}

// ----------------------------------------------------------------------------
// Folder view
// ----------------------------------------------------------------------------

/// The folder listing: the summary line (with its truncation note)
/// above the monospace grid so it stays visible while scrolling.
/// Rows are virtualized the same way as the text view.
pub fn paint_folder(ui: &mut Ui, rows: &[FolderRow], summary: &str) {
    ui.label(RichText::new(summary).monospace().weak());
    ui.add_space(4.0);
    if rows.is_empty() {
        ui.label(RichText::new("(both folders are empty)").weak());
        return;
    }
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, rows.len(), |ui, range| {
            for i in range {
                let Some(row) = rows.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(row.glyph).monospace().color(glyph_color(row)));
                    ui.label(RichText::new(folder_columns(row)).monospace());
                });
            }
        });
}

/// The glyph column color: same/dim, different and left-only/red,
/// newer/blue, right-only/green, like the CLI's coloring.
fn glyph_color(row: &FolderRow) -> Color32 {
    match row.glyph {
        '=' => DIM,
        '!' | 'L' => RED,
        '>' | '<' => NEWER_BLUE,
        'R' => GREEN,
        _ => DIM,
    }
}

/// The columns after the glyph, shaped like the CLI's row: sizes,
/// times, then the path with the directory mark.
fn folder_columns(row: &FolderRow) -> String {
    let ls = row.left_size.as_deref().unwrap_or("-");
    let rs = row.right_size.as_deref().unwrap_or("-");
    let lt = row.left_time.as_deref().unwrap_or("-");
    let rt = row.right_time.as_deref().unwrap_or("-");
    let dir_mark = if row.is_dir { "/" } else { "" };
    let rel_path = row.rel_path.as_str();
    format!(" {ls:>10} {rs:>10}  {lt:>16} {rt:>16}  {rel_path}{dir_mark}")
}

// ----------------------------------------------------------------------------
// Picture view
// ----------------------------------------------------------------------------

/// The picture summary lines, then the heatmap texture at its
/// natural size (shrunk to the panel width when needed).
pub fn paint_picture(ui: &mut Ui, lines: &[String], heatmap: Option<&TextureHandle>) {
    for line in lines {
        ui.label(RichText::new(line.as_str()).monospace());
    }
    match heatmap {
        Some(handle) => {
            ui.add_space(6.0);
            ui.add(
                egui::Image::new((handle.id(), handle.size_vec2())).max_width(ui.available_width()),
            );
        }
        None => paint_note(ui, "heatmap unavailable for these inputs"),
    }
}

/// Straight-alpha RGBA bytes into an egui [`ColorImage`]: egui
/// textures expect premultiplied alpha, so every channel is scaled by
/// its alpha with saturating integer math. `None` when the bytes are
/// empty or disagree with the declared dimensions - those cases have
/// nothing to display and must not reach `ColorImage::new`, whose
/// size debug-assert would otherwise fire.
pub fn heatmap_color_image(bytes: &[u8], width: u32, height: u32) -> Option<egui::ColorImage> {
    let w = usize::try_from(width).ok()?;
    let h = usize::try_from(height).ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    let expected = w.checked_mul(h)?.checked_mul(4)?;
    if bytes.len() != expected {
        return None;
    }
    let pixels: Vec<Color32> = premultiplied_rgba(bytes)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| Color32::from_rgba_premultiplied(r, g, b, a))
        .collect();
    if pixels.len() != w.checked_mul(h)? {
        return None;
    }
    Some(egui::ColorImage::new([w, h], pixels))
}

/// Premultiplied copy of straight-alpha RGBA bytes: each color
/// channel scaled by alpha, saturating per channel. Empty for input
/// whose length is not a multiple of four. Pure; unit-tested.
fn premultiplied_rgba(bytes: &[u8]) -> Vec<u8> {
    if !bytes.len().is_multiple_of(4) {
        return Vec::new();
    }
    let (chunks, _empty_remainder) = bytes.as_chunks::<4>();
    let mut out = Vec::with_capacity(bytes.len());
    for &[r, g, b, a] in chunks {
        out.extend_from_slice(&[premul(r, a), premul(g, a), premul(b, a), a]);
    }
    out
}

/// One channel scaled by alpha: `c * a / 255`, saturating.
fn premul(channel: u8, alpha: u8) -> u8 {
    let scaled = u16::from(channel).saturating_mul(u16::from(alpha)) / 255;
    u8::try_from(scaled).unwrap_or(255)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_line_pads_to_the_pane_width() {
        assert_eq!(
            pane_segments("ab", &[], 4),
            vec![("ab".to_string(), false), ("  ".to_string(), false)]
        );
    }

    #[test]
    fn accent_span_splits_the_pane_into_three_segments() {
        assert_eq!(
            pane_segments("abc", &[(1, 2)], 3),
            vec![
                ("a".to_string(), false),
                ("b".to_string(), true),
                ("c".to_string(), false),
            ]
        );
    }

    #[test]
    fn accents_at_line_start_and_truncation_are_handled() {
        assert_eq!(
            pane_segments("abc", &[(0, 2)], 4),
            vec![
                ("ab".to_string(), true),
                ("c".to_string(), false),
                (" ".to_string(), false),
            ]
        );
        // Longer than the pane: truncated, like the CLI's panes.
        assert_eq!(
            pane_segments("abcdef", &[(5, 6)], 3),
            vec![("abc".to_string(), false)]
        );
    }

    #[test]
    fn empty_line_is_only_padding() {
        assert_eq!(
            pane_segments("", &[(0, 1)], 2),
            vec![("  ".to_string(), false)]
        );
    }

    #[test]
    fn zero_length_spans_accent_nothing() {
        assert_eq!(
            pane_segments("ab", &[(1, 1)], 2),
            vec![("ab".to_string(), false)]
        );
    }

    #[test]
    fn gutter_text_right_aligns_numbers_and_blanks_sides() {
        assert_eq!(gutter_text(Some(12), 4), "  12 ".to_string());
        assert_eq!(gutter_text(None, 4), "     ".to_string());
        assert_eq!(gutter_text(Some(123456), 4), "123456 ".to_string());
    }

    #[test]
    fn folder_columns_match_the_cli_shape() {
        let row = FolderRow {
            rel_path: "a/b.txt".to_string(),
            is_dir: false,
            glyph: '!',
            left_size: Some("10".to_string()),
            right_size: None,
            left_time: Some("1970-01-01 00:00".to_string()),
            right_time: None,
            changed: true,
        };
        assert_eq!(
            folder_columns(&row),
            "         10          -  1970-01-01 00:00                -  a/b.txt"
        );
        let dir = FolderRow {
            rel_path: "a".to_string(),
            is_dir: true,
            ..row
        };
        assert!(folder_columns(&dir).ends_with("a/"));
    }

    #[test]
    fn premultiplied_rgba_is_saturating_per_channel() {
        // Opaque passes through unchanged.
        assert_eq!(
            premultiplied_rgba(&[255, 10, 20, 255]),
            vec![255, 10, 20, 255]
        );
        // 255*128/255 = 128, 10*128/255 = 5, 20*128/255 = 10.
        assert_eq!(
            premultiplied_rgba(&[255, 10, 20, 128]),
            vec![128, 5, 10, 128]
        );
        // Transparent collapses to zero.
        assert_eq!(premultiplied_rgba(&[200, 200, 200, 0]), vec![0, 0, 0, 0]);
        // Ragged input models nothing; so does empty.
        assert!(premultiplied_rgba(&[1, 2, 3]).is_empty());
        assert!(premultiplied_rgba(&[]).is_empty());
    }

    #[test]
    fn heatmap_image_builds_only_from_consistent_bytes() {
        let image = heatmap_color_image(&[255, 0, 0, 255], 1, 1).unwrap_or_default();
        let [w, h] = image.size;
        assert_eq!((w, h), (1, 1));
        assert_eq!(
            image.pixels,
            vec![Color32::from_rgba_premultiplied(255, 0, 0, 255)]
        );
        // Bytes disagreeing with the declared size model nothing.
        assert!(heatmap_color_image(&[255, 0, 0, 255], 2, 1).is_none());
        // Empty dimensions or empty bytes model nothing.
        assert!(heatmap_color_image(&[], 0, 0).is_none());
        assert!(heatmap_color_image(&[], 1, 1).is_none());
    }

    #[test]
    fn side_job_lays_out_gutter_text_and_padding_in_order() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(1),
            right_no: None,
            left: "ab".to_string(),
            right: String::new(),
            accent_a: vec![(1, 2)],
            accent_b: Vec::new(),
        };
        let model = TextRows {
            rows: vec![row.clone()],
            truncated: false,
            coarse: false,
            pane_chars: 4,
            gutter_chars: 4,
        };
        let job = side_job(
            Side::Left,
            &row,
            &model,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        // Gutter "   1 ", text "ab" with "b" accented, then pane
        // padding: the job text is exactly gutter + padded line.
        assert_eq!(job.text, "   1 ab  ");
        assert!(job.keep_trailing_whitespace);
        // Sections: gutter, plain "a", accented "b", padding.
        assert_eq!(job.sections.len(), 4);
        // The blank right side is all spaces: gutter (5) plus the
        // padded empty pane (4).
        let blank = side_job(
            Side::Right,
            &row,
            &model,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert_eq!(blank.text, "         ");
    }
}
