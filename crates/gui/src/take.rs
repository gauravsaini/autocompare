//! Three-way merge conflict Take resolution.
//!
//! Provides pure headless conflict resolution by taking Left, Right, Base, or Both.

#![forbid(unsafe_code)]

/// Choice of source when resolving a three-way merge conflict block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeChoice {
    Left,
    Right,
    Base,
    Both,
}

/// Error encountered during three-way conflict resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeError {
    ConflictNotFound,
    MalformedConflict,
}

impl std::fmt::Display for TakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictNotFound => write!(f, "conflict block not found at specified index"),
            Self::MalformedConflict => write!(f, "malformed conflict marker structure"),
        }
    }
}

impl std::error::Error for TakeError {}

/// Resolves the N-th conflict block in a merged line stream by taking Left, Right, Base, or Both.
///
/// Scans for standard merge3 markers (`<<<<<<<`, `|||||||`, `=======`, `>>>>>>>`).
/// Replaces the entire marker block with the chosen candidate lines.
pub fn resolve_take(
    lines: &[String],
    conflict_index: usize,
    choice: TakeChoice,
) -> Result<Vec<String>, TakeError> {
    let mut out = Vec::with_capacity(lines.len());
    let mut current_conflict = 0usize;
    let mut i = 0usize;

    while i < lines.len() {
        let Some(line) = lines.get(i) else {
            break;
        };

        if line.starts_with("<<<<<<<") {
            if current_conflict == conflict_index {
                let mut left_lines = Vec::new();
                let mut base_lines = Vec::new();
                let mut right_lines = Vec::new();
                let mut section = 0; // 0 = left, 1 = base, 2 = right
                let mut found_end = false;

                i = i.saturating_add(1);
                while i < lines.len() {
                    let Some(cur) = lines.get(i) else {
                        break;
                    };
                    if cur.starts_with("|||||||") {
                        section = 1;
                    } else if cur.starts_with("=======") {
                        section = 2;
                    } else if cur.starts_with(">>>>>>>") {
                        found_end = true;
                        i = i.saturating_add(1);
                        break;
                    } else {
                        match section {
                            0 => left_lines.push(cur.clone()),
                            1 => base_lines.push(cur.clone()),
                            2 => right_lines.push(cur.clone()),
                            _ => {}
                        }
                    }
                    i = i.saturating_add(1);
                }

                if !found_end {
                    return Err(TakeError::MalformedConflict);
                }

                match choice {
                    TakeChoice::Left => out.extend(left_lines),
                    TakeChoice::Right => out.extend(right_lines),
                    TakeChoice::Base => out.extend(base_lines),
                    TakeChoice::Both => {
                        out.extend(left_lines);
                        out.extend(right_lines);
                    }
                }
                current_conflict = current_conflict.saturating_add(1);
                continue;
            } else {
                current_conflict = current_conflict.saturating_add(1);
                out.push(line.clone());
                i = i.saturating_add(1);
                continue;
            }
        }

        out.push(line.clone());
        i = i.saturating_add(1);
    }

    if conflict_index >= current_conflict {
        return Err(TakeError::ConflictNotFound);
    }

    Ok(out)
}

/// Resolves all conflict blocks in a merged line stream by taking the chosen candidate.
///
/// Replaces every conflict block (`<<<<<<<` ... `>>>>>>>`) with the chosen candidate.
pub fn resolve_take_all(lines: &[String], choice: TakeChoice) -> Result<Vec<String>, TakeError> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;

    while i < lines.len() {
        let Some(line) = lines.get(i) else {
            break;
        };

        if line.starts_with("<<<<<<<") {
            let mut left_lines = Vec::new();
            let mut base_lines = Vec::new();
            let mut right_lines = Vec::new();
            let mut section = 0; // 0 = left, 1 = base, 2 = right
            let mut found_end = false;

            i = i.saturating_add(1);
            while i < lines.len() {
                let Some(cur) = lines.get(i) else {
                    break;
                };
                if cur.starts_with("|||||||") {
                    section = 1;
                } else if cur.starts_with("=======") {
                    section = 2;
                } else if cur.starts_with(">>>>>>>") {
                    found_end = true;
                    i = i.saturating_add(1);
                    break;
                } else {
                    match section {
                        0 => left_lines.push(cur.clone()),
                        1 => base_lines.push(cur.clone()),
                        2 => right_lines.push(cur.clone()),
                        _ => {}
                    }
                }
                i = i.saturating_add(1);
            }

            if !found_end {
                return Err(TakeError::MalformedConflict);
            }

            match choice {
                TakeChoice::Left => out.extend(left_lines),
                TakeChoice::Right => out.extend(right_lines),
                TakeChoice::Base => out.extend(base_lines),
                TakeChoice::Both => {
                    out.extend(left_lines);
                    out.extend(right_lines);
                }
            }
            continue;
        }

        out.push(line.clone());
        i = i.saturating_add(1);
    }

    Ok(out)
}

/// Counts conflict blocks in a merged line stream.
pub fn conflict_count(lines: &[String]) -> usize {
    let mut count = 0usize;
    for line in lines {
        if line.starts_with("<<<<<<<") {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Returns true if the line starts with any of the standard merge3 conflict markers.
pub fn is_conflict_marker(line: &str) -> bool {
    line.starts_with("<<<<<<<")
        || line.starts_with("|||||||")
        || line.starts_with("=======")
        || line.starts_with(">>>>>>>")
}

// ----------------------------------------------------------------------------
// Take affordance geometry + conflict block height (headless).
// Geometry first: per-choice button widths + block heights; typography
// second: Mono line-height se block stack. Colors untouched.
// ----------------------------------------------------------------------------

/// Take button height px: 24 hit floor (row rhythm parity).
pub const TAKE_BUTTON_H: f32 = crate::theme::SPACE_XL;
/// Take button side pad px: do taraf section rhythm.
pub const TAKE_BUTTON_PAD_X: f32 = crate::theme::SPACE_SM;
/// Take label advance px/char: body 13 estimate (CTA precedent parity).
pub const TAKE_LABEL_ADVANCE_PX: f32 = 7.0;

/// Take button label (paint parity): hamesha "Take {Left|Right|Base|Both}".
pub fn take_choice_label(choice: TakeChoice) -> &'static str {
    match choice {
        TakeChoice::Left => "Take Left",
        TakeChoice::Right => "Take Right",
        TakeChoice::Base => "Take Base",
        TakeChoice::Both => "Take Both",
    }
}

/// Ek Take button ki width px: label chars x advance + do taraf pad.
/// Sabse lamba "Take Right" (10 chars) = 86. Pure; headless-tested.
pub fn take_choice_button_width(choice: TakeChoice) -> f32 {
    take_choice_label(choice).chars().count() as f32 * TAKE_LABEL_ADVANCE_PX
        + TAKE_BUTTON_PAD_X * 2.0
}

/// Conflict block ki height px: `line_count` Mono lines ka stack
/// (`TypoRole::Mono.line_height_px` = 15.6). Khali block par 0.
/// Pure; headless-tested.
pub fn take_conflict_block_height_px(line_count: usize) -> f32 {
    line_count as f32 * crate::theme::TypoRole::Mono.line_height_px()
}

// ----------------------------------------------------------------------------
// Thin Take wiring over autocompare-diff (headless, no egui, no I/O).
// BC parity: Left / Center (=ancestor/base) / Right. Stepper + editable
// output buffer + save-to-output text live here; model lives in diff.
// ----------------------------------------------------------------------------

use autocompare_diff::merge::{
    MergeSession as DiffMergeSession, TakeChoice as DiffTakeChoice,
    TakeResolution as DiffTakeResolution, TextBuffer as DiffTextBuffer,
};
use autocompare_diff::text_diff::DiffAlgorithm;

/// BC-parity take side: Left / Center (ancestor) / Right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeSide {
    Left,
    Center,
    Right,
}

/// Label for a take side (paint parity): "Take Left/Center/Right".
pub fn take_side_label(side: TakeSide) -> &'static str {
    match side {
        TakeSide::Left => "Take Left",
        TakeSide::Center => "Take Center",
        TakeSide::Right => "Take Right",
    }
}

impl TakeSide {
    /// Maps to this crate's marker-level [`TakeChoice`] (Center == Base).
    pub fn to_gui_choice(self) -> TakeChoice {
        match self {
            Self::Left => TakeChoice::Left,
            Self::Center => TakeChoice::Base,
            Self::Right => TakeChoice::Right,
        }
    }

    /// Maps to the diff engine's [`DiffTakeChoice`] (Center == Base).
    pub fn to_diff_choice(self) -> DiffTakeChoice {
        match self {
            Self::Left => DiffTakeChoice::Left,
            Self::Center => DiffTakeChoice::Base,
            Self::Right => DiffTakeChoice::Right,
        }
    }
}

/// Joins output lines into save-to-output text (BC parity): `"\n"` join
/// plus trailing newline when non-empty, empty stays empty. Pure.
pub fn join_take_output(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(line);
    }
    out.push('\n');
    out
}

/// Thin 3-way Take session: owns the diff [`DiffMergeSession`], an
/// editable output [`DiffTextBuffer`], and a conflict stepper cursor.
/// Paint calls `take_*` / `step_*`, edits via `buffer_mut`, saves via
/// [`TakeSession::output_text`]. No egui, no I/O, never panics.
pub struct TakeSession {
    session: DiffMergeSession,
    buffer: DiffTextBuffer,
    cursor: Option<usize>,
}

impl TakeSession {
    /// Builds a 3-way session (Myers) and syncs the editable buffer.
    pub fn new(base: &[&str], left: &[&str], right: &[&str]) -> Self {
        Self::new_with_algorithm(base, left, right, DiffAlgorithm::Myers)
    }

    /// Builds a 3-way session with explicit diff algorithm.
    pub fn new_with_algorithm(
        base: &[&str],
        left: &[&str],
        right: &[&str],
        algorithm: DiffAlgorithm,
    ) -> Self {
        let session = DiffMergeSession::new_with_algorithm(base, left, right, algorithm);
        let buffer = DiffTextBuffer::from_lines(session.render_lines());
        let cursor = session.first_unresolved();
        Self {
            session,
            buffer,
            cursor,
        }
    }

    /// Total conflict blocks.
    pub fn conflict_count(&self) -> usize {
        self.session.conflict_count()
    }

    /// Unresolved conflict blocks.
    pub fn unresolved_count(&self) -> usize {
        self.session.unresolved_count()
    }

    /// True when every conflict has a take decision.
    pub fn is_fully_resolved(&self) -> bool {
        self.session.is_fully_resolved()
    }

    /// Stepper cursor (current conflict id, if any).
    pub fn current(&self) -> Option<usize> {
        self.cursor
    }

    /// Steps to the first unresolved conflict.
    pub fn step_to_first(&mut self) -> Option<usize> {
        self.cursor = self.session.first_unresolved();
        self.cursor
    }

    /// Steps to the next unresolved conflict after the cursor.
    pub fn step_to_next(&mut self) -> Option<usize> {
        let from = self.cursor.unwrap_or(usize::MAX);
        // `next_unresolved` needs strictly-greater id; at end wrap to first.
        let next = if self.cursor.is_some() {
            self.session.next_unresolved(from)
        } else {
            self.session.first_unresolved()
        };
        self.cursor = next.or_else(|| self.session.first_unresolved());
        self.cursor
    }

    /// Steps to the previous unresolved conflict before the cursor.
    pub fn step_to_prev(&mut self) -> Option<usize> {
        let next = match self.cursor {
            Some(from) => self
                .session
                .prev_unresolved(from)
                .or_else(|| self.session.first_unresolved()),
            None => self.session.first_unresolved(),
        };
        self.cursor = next;
        self.cursor
    }

    /// Takes `side` for conflict `id` and re-syncs the output buffer.
    pub fn take(&mut self, id: usize, side: TakeSide) -> bool {
        let ok = self
            .session
            .resolve_conflict(id, DiffTakeResolution::Choice(side.to_diff_choice()));
        if ok {
            self.cursor = Some(id);
            self.sync_buffer();
        }
        ok
    }

    /// Takes Left for conflict `id`.
    pub fn take_left(&mut self, id: usize) -> bool {
        self.take(id, TakeSide::Left)
    }

    /// Takes Center (ancestor/base) for conflict `id`.
    pub fn take_center(&mut self, id: usize) -> bool {
        self.take(id, TakeSide::Center)
    }

    /// Takes Right for conflict `id`.
    pub fn take_right(&mut self, id: usize) -> bool {
        self.take(id, TakeSide::Right)
    }

    /// Takes `side` for every conflict and re-syncs the buffer.
    pub fn take_all(&mut self, side: TakeSide) {
        match side {
            TakeSide::Left => self.session.take_all_left(),
            TakeSide::Center => self.session.take_all_center(),
            TakeSide::Right => self.session.take_all_right(),
        }
        self.sync_buffer();
    }

    /// Clears one conflict back to unresolved and re-syncs the buffer.
    pub fn clear(&mut self, id: usize) -> bool {
        let ok = self.session.clear_resolution(id);
        if ok {
            self.sync_buffer();
        }
        ok
    }

    /// Rendered output lines (markers for unresolved conflicts).
    pub fn output_lines(&self) -> Vec<String> {
        self.session.render_lines()
    }

    /// Save-to-output text (BC parity trailing newline).
    pub fn output_text(&self) -> String {
        join_take_output(&self.output_lines())
    }

    /// Editable output buffer (synced after every take/clear).
    pub fn buffer(&self) -> &DiffTextBuffer {
        &self.buffer
    }

    /// Mutable editable output buffer (typing, undo/redo live here).
    pub fn buffer_mut(&mut self) -> &mut DiffTextBuffer {
        &mut self.buffer
    }

    /// Re-syncs the editable buffer from the session render.
    pub fn sync_buffer(&mut self) {
        self.buffer = DiffTextBuffer::from_lines(self.session.render_lines());
    }
}

// ----------------------------------------------------------------------------
// CopyTake6 take bar + conflict stepper (headless; paint owner ke paas).
// BC parity: Take Left/Center/Right uniform bar, conflict stepper
// (prev/next + honest counter), narrow par h-scroll. Geometry first
// (uniform 93px buttons + 4px gaps, scale reuse), typography second
// (bar text Body button precedent, counter Small status precedent).
// Colors frozen. WIRING (paint): take bar me `take_bar_button_width`
// uniform teeno buttons + `take_bar_gap`; stepper row me
// `take_stepper_button_h` Prev/Next + `take_stepper_counter_text`;
// `take_bar_needs_hscroll(avail_w)` par bar scroll me.
// ----------------------------------------------------------------------------

/// Ek Take-side button ki width px: paint label chars x advance + do
/// taraf pad. "Take Center" sabse lamba (11 chars) = 93.
/// Note: [`take_side_label`] se (paint parity), [`take_choice_label`] se
/// nahi — Center paint me ancestor/base nahi, "Center" hai.
/// Pure; headless-tested.
pub fn take_side_button_width(side: TakeSide) -> f32 {
    take_side_label(side).chars().count() as f32 * TAKE_LABEL_ADVANCE_PX + TAKE_BUTTON_PAD_X * 2.0
}

/// Take bar uniform button width px: teeno sides ek baseline — sabse
/// chaude Center (93) sabko cover karta hai taaki bar aligned rahe.
/// Pure; headless-tested.
pub fn take_bar_button_width() -> f32 {
    take_side_button_width(TakeSide::Left)
        .max(take_side_button_width(TakeSide::Center))
        .max(take_side_button_width(TakeSide::Right))
}

/// Take bar gap px: tight rhythm (4).
pub fn take_bar_gap() -> f32 {
    crate::theme::GEO_TIGHT_GAP
}

/// Take bar total width px: 3 x uniform + 2 x gap = 287.
/// Non-finite par 0 (paint bar chhupata hai, kabhi NaN layout nahi).
pub fn take_bar_total_width() -> f32 {
    let total = take_bar_button_width() * 3.0 + take_bar_gap() * 2.0;
    if total.is_finite() { total } else { 0.0 }
}

/// Take bar ko h-scroll chahiye ya nahi: avail < total par true.
/// Non-finite avail par false. Pure; headless-tested.
pub fn take_bar_needs_hscroll(avail_w: f32) -> bool {
    if !avail_w.is_finite() {
        return false;
    }
    avail_w < take_bar_total_width()
}

/// Conflict stepper button height px: take hit floor parity (24).
pub fn take_stepper_button_h() -> f32 {
    TAKE_BUTTON_H
}

/// Stepper gap px: tight rhythm (4, prev/counter/next beat).
pub fn take_stepper_gap() -> f32 {
    crate::theme::GEO_TIGHT_GAP
}

/// Stepper counter text: total 0 par "No conflicts"; cursor par 1-based
/// "Conflict N of M"; cursor na ho par honest "{M} conflicts".
/// `saturating_add` se 1-based display kabhi wrap nahi. Pure.
pub fn take_stepper_counter_text(current: Option<usize>, total: usize) -> String {
    if total == 0 {
        return String::from("No conflicts");
    }
    match current {
        Some(id) => format!("Conflict {} of {}", id.saturating_add(1), total),
        None => format!("{total} conflicts"),
    }
}

/// Take bar button text role: Body default (button precedent — HomeButton
/// Body 13 scale; take label advance bhi wahi estimate hai).
pub const fn take_action_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Take bar button text size px (single source: Body, 13.0).
pub fn take_action_text_size() -> f32 {
    take_action_text_role().size()
}

/// Stepper counter text role: Small weak proportional (status precedent).
pub const fn take_counter_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::chrome_status()
}

/// Stepper counter text size px (single source: Small, 11.0).
pub fn take_counter_text_size() -> f32 {
    take_counter_text_role().size()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_take_left_replaces_conflict_with_left_lines() {
        let lines = vec![
            "clean header".to_string(),
            "<<<<<<< left".to_string(),
            "left content".to_string(),
            "||||||| base".to_string(),
            "base content".to_string(),
            "=======".to_string(),
            "right content".to_string(),
            ">>>>>>> right".to_string(),
            "clean footer".to_string(),
        ];
        let resolved = resolve_take(&lines, 0, TakeChoice::Left).unwrap_or_default();
        assert_eq!(
            resolved,
            vec![
                "clean header".to_string(),
                "left content".to_string(),
                "clean footer".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_take_right_and_both() {
        let lines = vec![
            "<<<<<<<".to_string(),
            "L1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>>".to_string(),
        ];
        let right = resolve_take(&lines, 0, TakeChoice::Right).unwrap_or_default();
        assert_eq!(right, vec!["R1".to_string()]);

        let both = resolve_take(&lines, 0, TakeChoice::Both).unwrap_or_default();
        assert_eq!(both, vec!["L1".to_string(), "R1".to_string()]);
    }

    #[test]
    fn resolve_take_all_resolves_every_conflict_block() {
        let lines = vec![
            "h1".to_string(),
            "<<<<<<<".to_string(),
            "L1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>>".to_string(),
            "mid".to_string(),
            "<<<<<<<".to_string(),
            "L2".to_string(),
            "=======".to_string(),
            "R2".to_string(),
            ">>>>>>>".to_string(),
            "end".to_string(),
        ];
        assert_eq!(conflict_count(&lines), 2);
        let res = resolve_take_all(&lines, TakeChoice::Right).unwrap_or_default();
        assert_eq!(
            res,
            vec![
                "h1".to_string(),
                "R1".to_string(),
                "mid".to_string(),
                "R2".to_string(),
                "end".to_string(),
            ]
        );
        assert_eq!(conflict_count(&res), 0);
    }

    #[test]
    fn is_conflict_marker_detects_all_four_markers() {
        assert!(is_conflict_marker("<<<<<<< left"));
        assert!(is_conflict_marker("||||||| base"));
        assert!(is_conflict_marker("======="));
        assert!(is_conflict_marker(">>>>>>> right"));
        assert!(!is_conflict_marker("normal line"));
    }

    #[test]
    fn take_buttons_share_hit_height_and_fit_labels() {
        assert_eq!(TAKE_BUTTON_H, 24.0);
        assert_eq!(take_choice_button_width(TakeChoice::Right), 86.0);
        assert_eq!(take_choice_button_width(TakeChoice::Left), 79.0);
        assert_eq!(take_choice_button_width(TakeChoice::Base), 79.0);
        assert_eq!(take_choice_button_width(TakeChoice::Both), 79.0);
        // Sab buttons ek baseline: sabse chauda Right sabko cover karta hai.
        for choice in [
            TakeChoice::Left,
            TakeChoice::Right,
            TakeChoice::Base,
            TakeChoice::Both,
        ] {
            assert!(
                take_choice_button_width(choice) <= take_choice_button_width(TakeChoice::Right)
            );
            assert!(take_choice_label(choice).chars().count() >= 9);
        }
    }

    #[test]
    fn take_conflict_block_height_stacks_mono_lines() {
        assert_eq!(take_conflict_block_height_px(0), 0.0);
        let one = take_conflict_block_height_px(1);
        assert!((one - 15.6).abs() < 0.01);
        assert!((take_conflict_block_height_px(4) - one * 4.0).abs() < 0.01);
    }

    #[test]
    fn take_side_maps_center_to_base_and_labels() {
        assert_eq!(take_side_label(TakeSide::Left), "Take Left");
        assert_eq!(take_side_label(TakeSide::Center), "Take Center");
        assert_eq!(take_side_label(TakeSide::Right), "Take Right");
        assert_eq!(TakeSide::Center.to_gui_choice(), TakeChoice::Base);
        assert_eq!(TakeSide::Center.to_diff_choice(), DiffTakeChoice::Base);
    }

    #[test]
    fn take_session_conflict_stepper_and_output() {
        let base = ["common", "base_val", "tail"];
        let left = ["common", "left_val", "tail"];
        let right = ["common", "right_val", "tail"];
        let mut sess = TakeSession::new(&base, &left, &right);
        assert_eq!(sess.conflict_count(), 1);
        assert_eq!(sess.unresolved_count(), 1);
        assert_eq!(sess.current(), Some(0));
        assert!(sess.take_center(0));
        assert_eq!(sess.output_lines(), vec!["common", "base_val", "tail"]);
        assert_eq!(sess.output_text(), "common\nbase_val\ntail\n");
        assert!(sess.is_fully_resolved());
        // Editable buffer syncs and supports undo.
        assert_eq!(sess.buffer().lines(), &["common", "base_val", "tail"]);
        sess.buffer_mut().replace_line(1, "edited".to_string());
        assert_eq!(sess.buffer().line(1), Some("edited"));
        assert!(sess.buffer_mut().undo());
        assert_eq!(sess.buffer().line(1), Some("base_val"));
    }

    #[test]
    fn take_session_stepper_next_prev_and_take_all() {
        let base = ["a", "b"];
        let left = ["a_l", "b_l"];
        let right = ["a_r", "b_r"];
        let mut sess = TakeSession::new(&base, &left, &right);
        assert_eq!(sess.conflict_count(), 1);
        assert_eq!(sess.step_to_first(), Some(0));
        assert_eq!(sess.step_to_next(), Some(0));
        assert_eq!(sess.step_to_prev(), Some(0));
        sess.take_all(TakeSide::Right);
        assert!(sess.is_fully_resolved());
        assert_eq!(sess.output_lines(), vec!["a_r", "b_r"]);
        assert!(sess.clear(0));
        assert_eq!(sess.unresolved_count(), 1);
    }

    #[test]
    fn join_take_output_trailing_newline_parity() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(join_take_output(&empty), String::new());
        assert_eq!(
            join_take_output(&["x".to_string(), "y".to_string()]),
            "x\ny\n"
        );
    }

    #[test]
    fn copytake6_take_bar_uniform_and_total() {
        // Paint labels: Left 9 chars = 79, Center 11 = 93, Right 10 = 86.
        assert_eq!(take_side_button_width(TakeSide::Left), 79.0);
        assert_eq!(take_side_button_width(TakeSide::Center), 93.0);
        assert_eq!(take_side_button_width(TakeSide::Right), 86.0);
        assert_eq!(take_bar_button_width(), 93.0);
        assert_eq!(take_bar_gap(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(take_bar_total_width(), 287.0);
        assert_eq!(
            take_bar_total_width(),
            take_bar_button_width() * 3.0 + take_bar_gap() * 2.0
        );
    }

    #[test]
    fn copytake6_take_bar_hscroll_only_when_narrow() {
        // Mirrors checks/gui-exq-copytake6/comparison/copytake-narrow-240.json.
        assert!(take_bar_needs_hscroll(240.0));
        assert!(!take_bar_needs_hscroll(700.0));
        assert!(!take_bar_needs_hscroll(take_bar_total_width()));
        assert!(!take_bar_needs_hscroll(f32::NAN));
        assert!(!take_bar_needs_hscroll(f32::INFINITY));
    }

    #[test]
    fn copytake6_stepper_geometry_and_honest_counter() {
        assert_eq!(take_stepper_button_h(), TAKE_BUTTON_H);
        assert_eq!(take_stepper_gap(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(take_stepper_counter_text(None, 0), "No conflicts");
        assert_eq!(take_stepper_counter_text(Some(0), 2), "Conflict 1 of 2");
        assert_eq!(take_stepper_counter_text(Some(1), 2), "Conflict 2 of 2");
        assert_eq!(take_stepper_counter_text(None, 2), "2 conflicts");
        // Saturating 1-based display: usize::MAX par bhi wrap nahi.
        assert_eq!(
            take_stepper_counter_text(Some(usize::MAX), usize::MAX),
            format!("Conflict {} of {}", usize::MAX, usize::MAX)
        );
    }

    #[test]
    fn copytake6_take_roles_reuse_body_and_small() {
        assert_eq!(take_action_text_role(), crate::theme::TypoRole::Body);
        assert_eq!(take_action_text_size(), 13.0);
        assert_eq!(
            take_counter_text_role(),
            crate::theme::TypoRole::chrome_status()
        );
        assert_eq!(take_counter_text_size(), 11.0);
    }

    #[test]
    fn copytake6_stepper_counter_follows_session_cursor() {
        let base = ["common", "base_val", "tail"];
        let left = ["common", "left_val", "tail"];
        let right = ["common", "right_val", "tail"];
        let mut sess = TakeSession::new(&base, &left, &right);
        assert_eq!(sess.conflict_count(), 1);
        assert_eq!(
            take_stepper_counter_text(sess.current(), sess.conflict_count()),
            "Conflict 1 of 1"
        );
        assert!(sess.take_left(0));
        assert_eq!(sess.current(), Some(0));
        assert_eq!(
            take_stepper_counter_text(sess.current(), sess.conflict_count()),
            "Conflict 1 of 1"
        );
    }
}
