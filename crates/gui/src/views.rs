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
use crate::viewmodel::{self, FolderRow, MergeSummary, TableModel, TextRow, TextRowKind, TextRows};

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

/// Header: title, subtitle, and dark/light mode toggle with WCAG contrast verification.
pub fn paint_header(ui: &mut Ui, dark_mode: &mut bool) {
    ui.horizontal(|ui| {
        ui.heading("autocompare");
        ui.label(RichText::new("desktop client").weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if *dark_mode {
                "☀ Light Mode"
            } else {
                "🌙 Dark Mode"
            };
            if ui.button(label).clicked() {
                *dark_mode = !*dark_mode;
                ui.ctx().set_visuals(if *dark_mode {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
            }
        });
    });
    ui.label(
        RichText::new("drag & drop paths or browse via native dialogs | pure background engines")
            .small()
            .weak(),
    );
    ui.add_space(4.0);
}

/// The Home View launcher screen displaying cards for all 9 comparison session types.
pub fn paint_home(ui: &mut Ui, tab: &mut Tab) {
    let model = viewmodel::home_model();
    ui.heading("Comparison Sessions");
    ui.label(RichText::new("Select a session type or use the tabs above:").weak());
    ui.add_space(8.0);

    egui::Grid::new("home_sessions_grid")
        .num_columns(3)
        .spacing([12.0, 12.0])
        .show(ui, |ui| {
            for (i, item) in model.items.iter().enumerate() {
                ui.group(|ui| {
                    ui.set_width(260.0);
                    ui.set_height(75.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(item.title).strong().size(14.0));
                        ui.label(RichText::new(item.description).small().weak());
                        ui.add_space(4.0);
                        if ui.button(format!("Open {}", item.title)).clicked() {
                            *tab = item.tab;
                        }
                    });
                });
                if i % 3 == 2 {
                    ui.end_row();
                }
            }
        });
}

/// The ten tabs: Home plus nine comparison session types.
pub fn paint_tabs(ui: &mut Ui, tab: &mut Tab) {
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(tab, Tab::Home, "Home");
        ui.selectable_value(tab, Tab::Text, "Text");
        ui.selectable_value(tab, Tab::Folder, "Folder");
        ui.selectable_value(tab, Tab::Hex, "Hex");
        ui.selectable_value(tab, Tab::Merge, "Merge");
        ui.selectable_value(tab, Tab::Table, "Table");
        ui.selectable_value(tab, Tab::Sync, "Sync");
        ui.selectable_value(tab, Tab::Picture, "Picture");
        ui.selectable_value(tab, Tab::FolderMerge, "FolderMerge");
        ui.selectable_value(tab, Tab::Audio, "Audio");
    });
    ui.add_space(2.0);
}

/// Two path text fields with native file/folder picker buttons plus Compare.
pub fn paint_paths(
    ui: &mut Ui,
    noun: &str,
    is_dir: bool,
    left: &mut String,
    right: &mut String,
    compare: &mut bool,
) {
    ui.horizontal(|ui| {
        let half = (ui.available_width() / 2.0).max(180.0) - 75.0;
        ui.add(
            TextEdit::singleline(left)
                .desired_width(half)
                .hint_text(format!("left {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *left = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *left = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(right)
                .desired_width(half)
                .hint_text(format!("right {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *right = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *right = file.display().to_string();
            }
        }
    });
    let clicked = ui
        .horizontal(|ui| ui.button(format!("Compare {noun}s")).clicked())
        .inner;
    if clicked {
        *compare = true;
    }
    ui.add_space(4.0);
}

/// Three path text fields with native dialog pickers for three-way tabs.
pub fn paint_triple_paths(
    ui: &mut Ui,
    noun: &str,
    is_dir: bool,
    base: &mut String,
    left: &mut String,
    right: &mut String,
    compare: &mut bool,
) {
    ui.horizontal(|ui| {
        let third = (ui.available_width() / 3.0).max(130.0) - 75.0;
        ui.add(
            TextEdit::singleline(base)
                .desired_width(third)
                .hint_text(format!("base {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *base = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *base = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(left)
                .desired_width(third)
                .hint_text(format!("left {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *left = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *left = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(right)
                .desired_width(third)
                .hint_text(format!("right {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *right = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *right = file.display().to_string();
            }
        }
    });
    let clicked = ui
        .horizontal(|ui| ui.button(format!("Compare {noun}s")).clicked())
        .inner;
    if clicked {
        *compare = true;
    }
    ui.add_space(4.0);
}

/// Generic scrollable summary lines for the plan/stat tabs
/// (Hex, Sync, FolderMerge, Audio).
pub fn paint_lines(ui: &mut Ui, lines: &[String]) {
    if lines.is_empty() {
        ui.label(RichText::new("(no output)").weak());
        return;
    }
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, lines.len(), |ui, range| {
            for i in range {
                let Some(line) = lines.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                ui.label(RichText::new(line.as_str()).monospace());
            }
        });
}

/// The three-way merge output: conflict blocks highlighted, notes
/// for conflicts and truncation first, with Take resolution actions.
pub fn paint_merge(ui: &mut Ui, model: &mut MergeSummary) {
    if model.conflicted {
        paint_note(ui, "the merge has conflicts; choose an action to resolve:");
        let mut resolve_choice = None;
        ui.horizontal(|ui| {
            if ui.button("Take Left").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Left);
            }
            if ui.button("Take Right").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Right);
            }
            if ui.button("Take Base").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Base);
            }
            if ui.button("Take Both").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Both);
            }
        });
        if let Some(choice) = resolve_choice
            && let Ok(resolved) = viewmodel::resolve_take(&model.lines, 0, choice)
        {
            model.conflicted = resolved.iter().any(|l| l.starts_with("<<<<<<<"));
            model.lines = resolved;
        }
        ui.add_space(4.0);
    }
    if model.truncated {
        paint_note(ui, "showing the first lines only; the rest is cut off");
    }
    if model.lines.is_empty() {
        ui.label(RichText::new("(merged output is empty)").weak());
        return;
    }
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, model.lines.len(), |ui, range| {
            for i in range {
                let Some(line) = model.lines.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                let marker = line.starts_with("<<<<<<<")
                    || line.starts_with("=======")
                    || line.starts_with(">>>>>>>");
                let mut text = RichText::new(line.as_str()).monospace();
                if marker {
                    text = text.color(RED).strong();
                }
                ui.label(text);
            }
        });
}

/// The table comparison rows: changed rows highlighted, changed cells
/// already marked `[old => new]` by the view model.
pub fn paint_table(ui: &mut Ui, model: &TableModel) {
    if model.truncated {
        paint_note(ui, "showing the first rows only; the rest is cut off");
    }
    if model.rows.is_empty() {
        ui.label(RichText::new("(both tables are empty)").weak());
        return;
    }
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, model.rows.len(), |ui, range| {
            for i in range {
                let Some(row) = model.rows.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                let mut glyph = RichText::new(row.glyph.to_string()).monospace();
                glyph = match row.glyph {
                    '=' => glyph.color(DIM),
                    '!' | 'L' => glyph.color(RED),
                    'R' => glyph.color(GREEN),
                    _ => glyph.color(DIM),
                };
                let mut body = RichText::new(format!("{} | {}", row.label, row.text)).monospace();
                if row.changed {
                    body = body.color(CHANGED_FG);
                }
                ui.horizontal(|ui| {
                    ui.label(glyph);
                    ui.label(body);
                });
            }
        });
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
/// grid with live copy buttons and thumbnail gutter geometry.
pub fn paint_text(ui: &mut Ui, model: &mut TextRows) {
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
    let mut copy_action = None;

    ui.horizontal(|ui| {
        // Main diff scroll area
        ui.vertical(|ui| {
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

                            if row.kind != TextRowKind::Equal {
                                if ui
                                    .small_button("→")
                                    .on_hover_text("Copy Left to Right")
                                    .clicked()
                                {
                                    copy_action = Some((i, viewmodel::CopyDirection::LeftToRight));
                                }
                                if ui
                                    .small_button("←")
                                    .on_hover_text("Copy Right to Left")
                                    .clicked()
                                {
                                    copy_action = Some((i, viewmodel::CopyDirection::RightToLeft));
                                }
                            } else {
                                ui.label(RichText::new(" | ").monospace().color(DIM));
                            }

                            ui.label(side_job(Side::Right, row, model, font_id.clone(), fg, bg));
                        });
                    }
                });
        });

        // Thumbnail overview gutter on the right
        let avail_h = ui.available_height().max(100.0);
        let geom = viewmodel::gutter_geometry(model.rows.len(), avail_h, &model.rows);
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(12.0, avail_h), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_rgb(32, 32, 32));

        for seg in geom.segments {
            let seg_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y + seg.y_start),
                egui::pos2(rect.max.x, (rect.min.y + seg.y_end).min(rect.max.y)),
            );
            let color = match seg.kind {
                TextRowKind::Equal => Color32::TRANSPARENT,
                TextRowKind::Deleted => DELETED_BG,
                TextRowKind::Inserted => INSERTED_BG,
                TextRowKind::Changed => CHANGED_BG,
            };
            ui.painter().rect_filled(seg_rect, 0.0, color);
        }
    });

    if let Some((idx, dir)) = copy_action {
        let _ = viewmodel::apply_copy(model, idx, dir);
    }
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
/// above the monospace tree grid with expand/collapse drill-down.
/// Rows are virtualized the same way as the text view.
pub fn paint_folder(
    ui: &mut Ui,
    rows: &[FolderRow],
    summary: &str,
    expanded: &mut std::collections::BTreeSet<String>,
) {
    ui.label(RichText::new(summary).monospace().weak());
    ui.add_space(4.0);
    if rows.is_empty() {
        ui.label(RichText::new("(both folders are empty)").weak());
        return;
    }
    let tree_nodes = viewmodel::folder_tree(rows, expanded);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let mut toggle_dir = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, tree_nodes.len(), |ui, range| {
            for i in range {
                let Some(node) = tree_nodes.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                ui.horizontal(|ui| {
                    if node.depth > 0 {
                        ui.add_space((node.depth as f32) * 14.0);
                    }
                    if node.row.is_dir && node.has_children {
                        let icon = if node.is_expanded { "▼" } else { "▶" };
                        if ui.small_button(icon).clicked() {
                            toggle_dir = Some(node.row.rel_path.clone());
                        }
                    } else if node.row.is_dir {
                        ui.label(RichText::new("  ").monospace());
                    }
                    ui.label(
                        RichText::new(node.row.glyph)
                            .monospace()
                            .color(glyph_color(&node.row)),
                    );
                    ui.label(RichText::new(folder_columns(&node.row)).monospace());
                });
            }
        });

    if let Some(dir) = toggle_dir {
        if expanded.contains(&dir) {
            expanded.remove(&dir);
        } else {
            expanded.insert(dir);
        }
    }
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
