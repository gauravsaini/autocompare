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
    /// All 3-way take sides in left-to-right order.
    pub const ALL: [Self; 3] = [Self::Left, Self::Center, Self::Right];

    /// Slice of all take sides in left-to-right order.
    pub fn sides() -> &'static [Self; 3] {
        &Self::ALL
    }

    /// Zero-based index for pane ordering: Left = 0, Center = 1, Right = 2.
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Center => 1,
            Self::Right => 2,
        }
    }

    /// Converts a zero-based index (0, 1, 2) back to a `TakeSide`.
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Left),
            1 => Some(Self::Center),
            2 => Some(Self::Right),
            _ => None,
        }
    }

    /// Maps from GUI [`TakeChoice`] to [`TakeSide`] (Both returns None).
    pub fn from_gui_choice(choice: TakeChoice) -> Option<Self> {
        match choice {
            TakeChoice::Left => Some(Self::Left),
            TakeChoice::Base => Some(Self::Center),
            TakeChoice::Right => Some(Self::Right),
            TakeChoice::Both => None,
        }
    }

    /// Maps from diff engine [`DiffTakeChoice`] to [`TakeSide`].
    pub fn from_diff_choice(choice: DiffTakeChoice) -> Option<Self> {
        match choice {
            DiffTakeChoice::Left => Some(Self::Left),
            DiffTakeChoice::Base => Some(Self::Center),
            DiffTakeChoice::Right => Some(Self::Right),
            DiffTakeChoice::BothLeftFirst | DiffTakeChoice::BothRightFirst => None,
        }
    }

    /// Returns the opposite side (Left <-> Right, Center stays Center).
    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Center => Self::Center,
            Self::Right => Self::Left,
        }
    }

    /// Short single-letter representation for gutter/badges ("L", "C", "R").
    pub fn short_code(self) -> &'static str {
        match self {
            Self::Left => "L",
            Self::Center => "C",
            Self::Right => "R",
        }
    }

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

/// Take side gap px: tight rhythm ([`crate::theme::GEO_TIGHT_GAP`] = 4.0).
pub fn take_side_gap() -> f32 {
    crate::theme::GEO_TIGHT_GAP
}

/// Take side bar button height px: hit floor ([`crate::theme::SPACE_XL`] = 24.0).
pub fn take_side_bar_h() -> f32 {
    TAKE_BUTTON_H
}

/// Take side button padding px: side pad ([`crate::theme::SPACE_SM`] = 8.0).
pub fn take_side_button_pad() -> f32 {
    TAKE_BUTTON_PAD_X
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

    /// Returns the chosen take side for conflict `id`, or `None` if unresolved or not a single side choice.
    pub fn side_at(&self, id: usize) -> Option<TakeSide> {
        let block = self.session.conflict_by_id(id)?;
        match block {
            autocompare_diff::merge::MergeBlock::Conflict { resolution, .. } => match resolution {
                Some(DiffTakeResolution::Choice(DiffTakeChoice::Left)) => Some(TakeSide::Left),
                Some(DiffTakeResolution::Choice(DiffTakeChoice::Base)) => Some(TakeSide::Center),
                Some(DiffTakeResolution::Choice(DiffTakeChoice::Right)) => Some(TakeSide::Right),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns a list of all conflict resolution choices in order (0..conflict_count).
    pub fn resolved_sides(&self) -> Vec<Option<TakeSide>> {
        let count = self.conflict_count();
        let mut out = Vec::with_capacity(count);
        for id in 0..count {
            out.push(self.side_at(id));
        }
        out
    }

    /// Captures a read-only snapshot of this session's state.
    pub fn snapshot(&self) -> TakeSessionSnapshot {
        TakeSessionSnapshot {
            cursor: self.cursor,
            conflict_count: self.conflict_count(),
            unresolved_count: self.unresolved_count(),
            is_fully_resolved: self.is_fully_resolved(),
            resolutions: self.resolved_sides(),
            output_lines: self.output_lines(),
            output_text: self.output_text(),
        }
    }

    /// Restores conflict resolutions and cursor from a snapshot.
    pub fn restore_snapshot(&mut self, snapshot: &TakeSessionSnapshot) {
        for (id, opt_side) in snapshot.resolutions.iter().enumerate() {
            if id < self.conflict_count() {
                match opt_side {
                    Some(side) => {
                        let _ = self.take(id, *side);
                    }
                    None => {
                        let _ = self.clear(id);
                    }
                }
            }
        }
        self.cursor = snapshot.cursor;
        self.sync_buffer();
    }

    /// Writes the current resolved output text to the specified filesystem path.
    /// Returns the number of bytes written.
    pub fn save_output_to_path(&self, path: &std::path::Path) -> Result<usize, std::io::Error> {
        let text = self.output_text();
        std::fs::write(path, text.as_bytes())?;
        Ok(text.len())
    }
}

/// Read-only snapshot of a 3-way merge Take session state at a point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakeSessionSnapshot {
    pub cursor: Option<usize>,
    pub conflict_count: usize,
    pub unresolved_count: usize,
    pub is_fully_resolved: bool,
    pub resolutions: Vec<Option<TakeSide>>,
    pub output_lines: Vec<String>,
    pub output_text: String,
}

impl TakeSessionSnapshot {
    /// Number of resolved conflicts in this snapshot.
    pub fn resolved_count(&self) -> usize {
        self.conflict_count.saturating_sub(self.unresolved_count)
    }

    /// Resolution progress fraction (0.0 to 1.0).
    pub fn resolution_fraction(&self) -> f32 {
        take_resolution_fraction(self.resolved_count(), self.conflict_count)
    }

    /// Resolution progress bar fill width at `avail_w`.
    pub fn resolution_bar_width(&self, avail_w: f32) -> f32 {
        take_resolution_bar_width(avail_w, self.resolved_count(), self.conflict_count)
    }

    /// Formatted stepper counter text ("Conflict N of M", "M conflicts", etc.).
    pub fn stepper_text(&self) -> String {
        take_stepper_counter_text(self.cursor, self.conflict_count)
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

// ----------------------------------------------------------------------------
// GUTTERCOPY8: conflict marks on minimap (headless; paint owner ke paas).
// BC minimap me har unresolved conflict ek band hai — overview ruler par
// wahi fraction-math + 4px floor taaki chhote conflicts bhi dikhen aur
// click lagein. Geometry first (XS floor reuse), typo nahi, colors frozen.
// WIRING (paint): conflict rows `take_minimap_marks(rows, total, height)` se bands.
// ----------------------------------------------------------------------------

/// Conflict minimap band min height px: one XS step (overview floor parity).
pub const TAKE_MINIMAP_MARK_MIN_H: f32 = crate::theme::SPACE_XS;

/// Ek conflict row ka minimap band `(y0, y1)` px: fraction mapping +
/// min-height floor (neeche se katkar, height ke andar). `None`: khali
/// totals, OOB row, non-positive/non-finite height. Pure; headless-tested.
pub fn take_minimap_band(row: usize, total_rows: usize, height: f32) -> Option<(f32, f32)> {
    if total_rows == 0 || row >= total_rows {
        return None;
    }
    if !height.is_finite() || height <= 0.0 {
        return None;
    }
    let total_f = total_rows as f32;
    let y0 = (row as f32 / total_f) * height;
    let mut y1 = ((row.saturating_add(1)) as f32 / total_f) * height;
    if y1 < y0 + TAKE_MINIMAP_MARK_MIN_H {
        y1 = (y0 + TAKE_MINIMAP_MARK_MIN_H).min(height);
    }
    if y0.is_finite() && y1.is_finite() && y0 >= 0.0 && y1 <= height && y0 < y1 {
        Some((y0, y1))
    } else {
        None
    }
}

/// Conflict rows ke minimap bands, input order me; kharab rows skip.
/// Khali input par khali vec. Pure; headless-tested.
pub fn take_minimap_marks(rows: &[usize], total_rows: usize, height: f32) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for row in rows {
        if let Some(band) = take_minimap_band(*row, total_rows, height) {
            out.push(band);
        }
    }
    out
}

// ----------------------------------------------------------------------------
// EXQ-MERGE2: 3-way merge pane widths, row height, and take button hit-slop (headless).
// BC parity: Left / Base (Center) / Right 3-way split with SECTION_GAP dividers,
// minimum pane floor so narrow views scroll instead of crushing panes,
// TypoRole::Mono line-height row rhythm, and SPACE_XS hit-slop expansion for touch/mouse.
// Geometry first, typography second, colors frozen.
// ----------------------------------------------------------------------------

/// Merge pane minimum width px: 130.0 floor so narrow windows scroll instead of crushing panes.
pub const MERGE_PANE_MIN_W: f32 = 130.0;

/// Merge pane divider gap px: section rhythm ([`crate::theme::GEO_SECTION_GAP`] = 8.0).
pub const MERGE_SECTION_GAP: f32 = crate::theme::GEO_SECTION_GAP;

/// Merge 3-way panes minimum total width px: 3 * 130 + 2 * 8 = 406.0.
pub const MERGE_TOTAL_MIN_W: f32 = MERGE_PANE_MIN_W * 3.0 + MERGE_SECTION_GAP * 2.0;

/// Take button hit-slop per side px: one XS step ([`crate::theme::SPACE_XS`] = 4.0).
pub const TAKE_BUTTON_HIT_SLOP: f32 = crate::theme::SPACE_XS;

/// 3-way pane widths for merge view: (left, base/center, right).
/// Deducts 2x [`MERGE_SECTION_GAP`] from `avail_w`, splits into equal thirds,
/// and clamps each pane to at least [`MERGE_PANE_MIN_W`].
/// Any leftover after left and base is allocated to the right pane so total fills row.
/// Pure; headless-tested.
pub fn merge_three_pane_widths(avail_w: f32) -> (f32, f32, f32) {
    if !avail_w.is_finite() || avail_w <= 0.0 {
        return (MERGE_PANE_MIN_W, MERGE_PANE_MIN_W, MERGE_PANE_MIN_W);
    }
    let total = (avail_w - 2.0 * MERGE_SECTION_GAP).max(3.0 * MERGE_PANE_MIN_W);
    let third = total / 3.0;
    let left = third.max(MERGE_PANE_MIN_W);
    let base = third.max(MERGE_PANE_MIN_W);
    let right = (total - left - base).max(MERGE_PANE_MIN_W);
    (left, base, right)
}

/// Alias for 3-way pane widths in merge view.
pub fn three_pane_widths(avail_w: f32) -> (f32, f32, f32) {
    merge_three_pane_widths(avail_w)
}

/// Whether 3-way merge panes require horizontal scroll at `avail_w`.
/// True when `avail_w < MERGE_TOTAL_MIN_W` (406px).
/// Non-finite values return false. Pure; headless-tested.
pub fn merge_three_pane_needs_hscroll(avail_w: f32) -> bool {
    if !avail_w.is_finite() {
        return false;
    }
    avail_w < MERGE_TOTAL_MIN_W
}

/// Merge row height px: single source from TypoRole Mono line-height (15.6px).
/// Pure; headless-tested.
pub const fn merge_row_height() -> f32 {
    crate::theme::TypoRole::Mono.line_height_px()
}

/// Alias for take row height px.
pub const fn take_row_height() -> f32 {
    merge_row_height()
}

/// Take button hit-slop size px (single source: `TAKE_BUTTON_HIT_SLOP`).
pub const fn take_button_hit_slop() -> f32 {
    TAKE_BUTTON_HIT_SLOP
}

/// Take button visual rect ko expand karta hai har taraf `TAKE_BUTTON_HIT_SLOP` (4px).
/// Input visual `(x0, y0, x1, y1)` -> `(x0 - slop, y0 - slop, x1 + slop, y1 + slop)`.
/// Non-finite ya invalid rects (`x0 > x1` ya `y0 > y1`) par `None`. Pure; headless-tested.
pub fn take_button_hit_rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<(f32, f32, f32, f32)> {
    if !x0.is_finite()
        || !y0.is_finite()
        || !x1.is_finite()
        || !y1.is_finite()
        || x0 > x1
        || y0 > y1
    {
        return None;
    }
    let s = TAKE_BUTTON_HIT_SLOP;
    Some((x0 - s, y0 - s, x1 + s, y1 + s))
}

/// Take button hit test: `(px, py)` point expanded hit rect ke andar hai ya nahi.
/// Non-finite point ya invalid rect par false. Pure; headless-tested.
pub fn take_button_hit_contains(x0: f32, y0: f32, x1: f32, y1: f32, px: f32, py: f32) -> bool {
    if !px.is_finite() || !py.is_finite() {
        return false;
    }
    match take_button_hit_rect(x0, y0, x1, y1) {
        Some((hx0, hy0, hx1, hy1)) => px >= hx0 && px <= hx1 && py >= hy0 && py <= hy1,
        None => false,
    }
}

/// Take button hit height with slop px: `TAKE_BUTTON_H + 2 * TAKE_BUTTON_HIT_SLOP` (24 + 8 = 32).
pub fn take_button_hit_height_with_slop() -> f32 {
    TAKE_BUTTON_H + 2.0 * TAKE_BUTTON_HIT_SLOP
}

/// Take button hit rect for a given `TakeChoice` at position `(x, y)` (top-left).
pub fn take_choice_button_hit_rect(
    x: f32,
    y: f32,
    choice: TakeChoice,
) -> Option<(f32, f32, f32, f32)> {
    let w = take_choice_button_width(choice);
    let h = TAKE_BUTTON_H;
    take_button_hit_rect(x, y, x + w, y + h)
}

/// Take button hit rect for a given `TakeSide` at position `(x, y)` (top-left).
pub fn take_side_button_hit_rect(x: f32, y: f32, side: TakeSide) -> Option<(f32, f32, f32, f32)> {
    let w = take_side_button_width(side);
    let h = TAKE_BUTTON_H;
    take_button_hit_rect(x, y, x + w, y + h)
}

// ----------------------------------------------------------------------------
// EXQ-MERGE3: take vertical rhythm from Mono line-height + 3-way divider
// gaps via SECTION_GAP at 960px (headless). BC parity: take buttons sit
// on the Mono row rhythm (15.6px line-height) with symmetric vertical
// padding so stacked buttons never look cramped; 3-way dividers are two
// SECTION_GAP (8px) gutters, exact at 960px wide. Geometry first
// (Mono line-height + SECTION_GAP reuse, no new literals), typography
// second (Mono role single source), colors frozen.
// WIRING (paint): take button y-centering uses `take_button_vpad`;
// row pitch uses `take_button_row_pitch`; divider x-origins use
// `merge_three_divider_x(avail_w)`.
// ----------------------------------------------------------------------------

/// Take button vertical rhythm role: Mono (12 regular monospace).
/// Single source for button-to-row centering. Pure.
pub const fn take_button_rhythm_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Take button symmetric vertical pad px: half the leftover of the
/// 24px hit floor over one Mono line-height ((24 - 15.6) / 2 = 4.2).
/// Centers the label on the Mono row rhythm. Pure; headless-tested.
pub fn take_button_vpad() -> f32 {
    (TAKE_BUTTON_H - take_button_rhythm_role().line_height_px()) / 2.0
}

/// Take button stacked row pitch px: button height + one tight gap
/// (24 + 4 = 28). Guarantees stacked take buttons never collide.
/// Pure; headless-tested.
pub fn take_button_row_pitch() -> f32 {
    TAKE_BUTTON_H + crate::theme::GEO_TIGHT_GAP
}

/// 3-way divider gap px: section rhythm single source (8.0).
pub fn merge_divider_gap() -> f32 {
    MERGE_SECTION_GAP
}

/// 3-way divider leading-x origins `(x0, x1)` px at `avail_w`:
/// `x0 = left_w`, `x1 = left_w + gap + base_w` from
/// [`merge_three_pane_widths`]. Non-finite/non-positive input par the
/// min-floor layout origins. Pure; headless-tested.
pub fn merge_three_divider_x(avail_w: f32) -> (f32, f32) {
    let (left, base, _) = merge_three_pane_widths(avail_w);
    let gap = merge_divider_gap();
    (left, left + gap + base)
}

/// True when the wide 3-way row exactly fills `avail_w`:
/// `left + base + right + 2 * gap == avail_w` within 0.01.
/// Non-finite input par false. Pure; headless-tested.
pub fn merge_three_panes_fill_exact(avail_w: f32) -> bool {
    if !avail_w.is_finite() || avail_w <= 0.0 {
        return false;
    }
    let (left, base, right) = merge_three_pane_widths(avail_w);
    ((left + base + right + 2.0 * merge_divider_gap()) - avail_w).abs() < 0.01
}

// ----------------------------------------------------------------------------
// EXQ-TAKE7: tall take-side markers over 1200px canvas (headless).
// BC parity: take markers aligned with hunk rows over tall scroll, take
// hit locked over long scroll, marker rhythm tall. Geometry first
// (Mono row-height + canvas clamp + hit-slop reuse, 1200 fixture canvas
// only new const), typography second (Mono single source), colors frozen.
// WIRING (paint): marker rows `take_marker_rows(rows)` se bands; take
// button `take_tall_side_hit_rect(row, x, side)` se hit-locked rect;
// scroll me `take_marker_visible_y` / `take_tall_hit_locked_y`.
// ----------------------------------------------------------------------------

/// Tall take canvas height px: double canonical viewport height
/// ([`crate::theme::VIEWPORT_H`] 600 x 2 = 1200 fixture, edge over 600 canonical).
pub const TAKE_TALL_CANVAS_H: f32 = crate::theme::VIEWPORT_H as f32 * 2.0;

/// Tall take canvas height px (single source: [`TAKE_TALL_CANVAS_H`]).
/// Pure; headless-tested.
pub fn take_tall_canvas_h() -> f32 {
    TAKE_TALL_CANVAS_H
}

/// Marker row top y px: `row * merge_row_height()` — hunk row se aligned.
/// Pure; headless-tested.
pub fn take_marker_row_y(row: usize) -> f32 {
    row as f32 * merge_row_height()
}

/// Marker row band `(y0, y1)` px over tall canvas: row-aligned top + one
/// row height, neeche se canvas par clamp. `None`: OOB row (`y0 >= canvas`)
/// ya non-finite. Pure; headless-tested.
pub fn take_marker_row_range(row: usize) -> Option<(f32, f32)> {
    let y0 = take_marker_row_y(row);
    if !y0.is_finite() || y0 < 0.0 || y0 >= TAKE_TALL_CANVAS_H {
        return None;
    }
    let y1 = (y0 + merge_row_height()).min(TAKE_TALL_CANVAS_H);
    if y1.is_finite() && y1 > y0 {
        Some((y0, y1))
    } else {
        None
    }
}

/// Marker row bands input order me; OOB rows skip. Khali input par khali vec.
/// Pure; headless-tested.
pub fn take_marker_rows(rows: &[usize]) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for row in rows {
        if let Some(band) = take_marker_row_range(*row) {
            out.push(band);
        }
    }
    out
}

/// Marker rhythm step px: single source row height (tall par bhi wahi beat).
/// Pure; headless-tested.
pub fn take_marker_rhythm_step() -> f32 {
    merge_row_height()
}

/// True jab `y` marker rhythm par hai: `y / step` near-integer (+-0.01).
/// Non-finite ya negative par false. Pure; headless-tested.
pub fn take_marker_on_rhythm(y: f32) -> bool {
    if !y.is_finite() || y < 0.0 {
        return false;
    }
    let step = take_marker_rhythm_step();
    if !step.is_finite() || step <= 0.0 {
        return false;
    }
    let q = y / step;
    if !q.is_finite() {
        return false;
    }
    (q - q.round()).abs() < 0.01
}

/// Tall take-side button hit rect for `row` at gutter-x `x`: button marker
/// row par centered (`top = marker_y - vpad`, button row se 4.2 taller)
/// phir hit-slop expand via [`take_side_button_hit_rect`]. OOB row ya
/// non-finite `x` par `None`. Pure; headless-tested.
pub fn take_tall_side_hit_rect(row: usize, x: f32, side: TakeSide) -> Option<(f32, f32, f32, f32)> {
    let marker_y = take_marker_row_y(row);
    if !marker_y.is_finite() || marker_y < 0.0 || marker_y >= TAKE_TALL_CANVAS_H {
        return None;
    }
    if !x.is_finite() {
        return None;
    }
    let top = marker_y - take_button_vpad();
    take_side_button_hit_rect(x, top, side)
}

/// Scroll-aware marker visible y px: `marker_y - scroll_y`, viewport
/// `[0, viewport_h)` ke andar tabhi `Some`. OOB marker ya bad input par
/// `None`. Pure; headless-tested.
pub fn take_marker_visible_y(row: usize, scroll_y: f32, viewport_h: f32) -> Option<f32> {
    if !scroll_y.is_finite() || !viewport_h.is_finite() || viewport_h <= 0.0 {
        return None;
    }
    let marker_y = take_marker_row_y(row);
    if !marker_y.is_finite() || marker_y < 0.0 || marker_y >= TAKE_TALL_CANVAS_H {
        return None;
    }
    let vis = marker_y - scroll_y;
    if vis.is_finite() && vis >= 0.0 && vis < viewport_h {
        Some(vis)
    } else {
        None
    }
}

/// Tall hit lock top y px: take button top viewport me clamp taaki lambe
/// scroll par bhi hit rahe — `[0, viewport_h - hit_h]` me lock, hit height
/// [`take_button_hit_height_with_slop`] (32). OOB/invisible marker ya bad
/// input par `None`. Pure; headless-tested.
pub fn take_tall_hit_locked_y(row: usize, scroll_y: f32, viewport_h: f32) -> Option<f32> {
    let vis = take_marker_visible_y(row, scroll_y, viewport_h)?;
    let hit_h = take_button_hit_height_with_slop();
    let max_top = viewport_h - hit_h;
    if !max_top.is_finite() {
        return None;
    }
    let top = vis - take_button_vpad();
    if max_top <= 0.0 {
        Some(0.0)
    } else {
        Some(top.max(0.0).min(max_top))
    }
}

// ----------------------------------------------------------------------------
// EXQ-TAKE33: live merge take — buttons row, center alignment, badge spot (headless).
// BC parity: take bar teeno uniform buttons ek row me tight gap par; center
// (ancestor) pane divider-gap ke baad shuru hokar canvas me centered; har
// unresolved conflict par badge pane ki trailing edge se XS inset par marker
// row se aligned. Geometry first (SPACE_XS/GEO_TIGHT_GAP/GEO_SECTION_GAP +
// take_bar/merge_three reuse, no new literals), typography second (badge text
// Small status precedent single source), colors frozen.
// WIRING (paint): button x `take_buttons_row_x(i)` se; center pane
// `merge_center_pane_x(avail_w)` se; badge `take_conflict_badge_pos` se;
// `merge_center_pane_aligned(avail_w)` 1200 par true hona chahiye.
// ----------------------------------------------------------------------------

/// Take buttons row me `index` wale button ka leading-x px:
/// `index * (uniform_w + gap)`. 0/1/2 ke bahar `None`.
/// Pure; headless-tested.
pub fn take_buttons_row_x(index: usize) -> Option<f32> {
    if index > 2 {
        return None;
    }
    let pitch = take_bar_button_width() + take_bar_gap();
    if !pitch.is_finite() {
        return None;
    }
    Some(index as f32 * pitch)
}

/// 3-way center (ancestor) pane ka leading-x px: `left_w + divider_gap`
/// ([`merge_three_pane_widths`] + [`merge_divider_gap`] se, no new literal).
/// Pure; headless-tested.
pub fn merge_center_pane_x(avail_w: f32) -> f32 {
    let (left, _, _) = merge_three_pane_widths(avail_w);
    left + merge_divider_gap()
}

/// True jab center pane canvas me centered hai:
/// `center_mid == avail_w / 2` ±0.01. Non-finite/non-positive par false.
/// Pure; headless-tested.
pub fn merge_center_pane_aligned(avail_w: f32) -> bool {
    if !avail_w.is_finite() || avail_w <= 0.0 {
        return false;
    }
    let (_, base, _) = merge_three_pane_widths(avail_w);
    let mid = merge_center_pane_x(avail_w) + base / 2.0;
    (mid - avail_w / 2.0).abs() < 0.01
}

/// Conflict badge trailing inset px: one XS step (pane edge rhythm parity).
/// Pure; headless-tested.
pub fn take_conflict_badge_inset() -> f32 {
    crate::theme::SPACE_XS
}

/// Conflict badge anchor `(x, y)` px: trailing edge se XS inset
/// (`pane_x + pane_w - inset`), marker row top se aligned
/// ([`take_marker_row_y`]). `None`: OOB row (1200 canvas ke bahar),
/// non-finite/negative pane input. Pure; headless-tested.
pub fn take_conflict_badge_pos(row: usize, pane_x: f32, pane_w: f32) -> Option<(f32, f32)> {
    if !pane_x.is_finite() || !pane_w.is_finite() || pane_w <= 0.0 {
        return None;
    }
    let y = take_marker_row_y(row);
    if !y.is_finite() || y < 0.0 || y >= TAKE_TALL_CANVAS_H {
        return None;
    }
    let x = pane_x + pane_w - take_conflict_badge_inset();
    if x.is_finite() && x >= pane_x {
        Some((x, y))
    } else {
        None
    }
}

/// Conflict badge text role: Small weak proportional (status precedent —
/// [`take_counter_text_role`] wahi single source).
pub const fn take_conflict_badge_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::chrome_status()
}

/// Conflict badge text size px (single source: Small, 11.0).
pub fn take_conflict_badge_text_size() -> f32 {
    take_conflict_badge_text_role().size()
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

#[cfg(test)]
mod exq_guttercopy8_tests {
    use super::*;

    #[test]
    fn guttercopy8_conflict_band_maps_row_fraction_with_floor() {
        // Mirrors checks/gui-exq-guttercopy8/round-1/comparison/edge-padded-minimap-tall.json.
        assert_eq!(TAKE_MINIMAP_MARK_MIN_H, crate::theme::SPACE_XS);
        assert_eq!(TAKE_MINIMAP_MARK_MIN_H, 4.0);
        assert_eq!(TAKE_MINIMAP_MARK_MIN_H, crate::gutter::GUTTER_SEGMENT_MIN_H);
        // 4 rows @400px: row 1 ka band 100..200.
        assert_eq!(take_minimap_band(1, 4, 400.0), Some((100.0, 200.0)));
        // Tall file: row 500 @1000 rows/400px floor se 200..204.
        assert_eq!(take_minimap_band(500, 1000, 400.0), Some((200.0, 204.0)));
        // Bad input par None.
        assert_eq!(take_minimap_band(4, 4, 400.0), None);
        assert_eq!(take_minimap_band(0, 0, 400.0), None);
        assert_eq!(take_minimap_band(1, 4, 0.0), None);
        assert_eq!(take_minimap_band(1, 4, f32::NAN), None);
    }

    #[test]
    fn guttercopy8_conflict_marks_keep_order_and_skip_bad_rows() {
        assert_eq!(
            take_minimap_marks(&[1, 3], 4, 400.0),
            vec![(100.0, 200.0), (300.0, 400.0)]
        );
        // OOB row skip, achhi wali rahe.
        assert_eq!(take_minimap_marks(&[1, 99], 4, 400.0), vec![(100.0, 200.0)]);
        assert!(take_minimap_marks(&[], 4, 400.0).is_empty());
        assert!(take_minimap_marks(&[0], 0, 400.0).is_empty());
    }
}

#[cfg(test)]
pub mod exq_merge2_tests {
    use super::*;

    #[test]
    pub fn merge2_three_pane_widths() {
        assert_eq!(MERGE_PANE_MIN_W, 130.0);
        assert_eq!(MERGE_SECTION_GAP, crate::theme::GEO_SECTION_GAP);
        assert_eq!(MERGE_SECTION_GAP, 8.0);
        assert_eq!(MERGE_TOTAL_MIN_W, 406.0);
        assert_eq!(
            MERGE_TOTAL_MIN_W,
            MERGE_PANE_MIN_W * 3.0 + MERGE_SECTION_GAP * 2.0
        );

        // Wide 960px: equal thirds minus two section gaps (944 / 3 = 314.66666...)
        let (left, base, right) = merge_three_pane_widths(960.0);
        assert!((left - 314.666_66).abs() < 0.01);
        assert!((base - 314.666_66).abs() < 0.01);
        assert!((left + base + right + 2.0 * MERGE_SECTION_GAP - 960.0).abs() < 0.01);
        assert!(!merge_three_pane_needs_hscroll(960.0));

        // Narrow 360px: each pane clamped to 130px floor, needs hscroll
        let (n_left, n_base, n_right) = merge_three_pane_widths(360.0);
        assert_eq!(n_left, MERGE_PANE_MIN_W);
        assert_eq!(n_base, MERGE_PANE_MIN_W);
        assert_eq!(n_right, MERGE_PANE_MIN_W);
        assert!(merge_three_pane_needs_hscroll(360.0));

        // Boundary tests around 406.0
        assert!(!merge_three_pane_needs_hscroll(406.0));
        assert!(merge_three_pane_needs_hscroll(405.9));

        // Degenerate / non-finite inputs
        assert_eq!(
            merge_three_pane_widths(0.0),
            (MERGE_PANE_MIN_W, MERGE_PANE_MIN_W, MERGE_PANE_MIN_W)
        );
        assert_eq!(
            merge_three_pane_widths(-50.0),
            (MERGE_PANE_MIN_W, MERGE_PANE_MIN_W, MERGE_PANE_MIN_W)
        );
        assert_eq!(
            merge_three_pane_widths(f32::NAN),
            (MERGE_PANE_MIN_W, MERGE_PANE_MIN_W, MERGE_PANE_MIN_W)
        );
        assert!(!merge_three_pane_needs_hscroll(f32::NAN));
        assert!(!merge_three_pane_needs_hscroll(f32::INFINITY));

        // three_pane_widths alias consistency
        assert_eq!(three_pane_widths(960.0), merge_three_pane_widths(960.0));
    }

    #[test]
    pub fn merge2_row_height() {
        assert_eq!(
            merge_row_height(),
            crate::theme::TypoRole::Mono.line_height_px()
        );
        assert!((merge_row_height() - 15.6).abs() < 0.01);
        assert_eq!(take_row_height(), merge_row_height());
    }

    #[test]
    pub fn merge2_hit_slop() {
        assert_eq!(TAKE_BUTTON_HIT_SLOP, crate::theme::SPACE_XS);
        assert_eq!(TAKE_BUTTON_HIT_SLOP, 4.0);
        assert_eq!(take_button_hit_slop(), 4.0);
        assert_eq!(take_button_hit_height_with_slop(), 32.0);

        // Hit rect expansion: (10, 20, 90, 44) -> (6, 16, 94, 48)
        let visual = (10.0, 20.0, 90.0, 44.0);
        let hit = take_button_hit_rect(visual.0, visual.1, visual.2, visual.3);
        assert_eq!(hit, Some((6.0, 16.0, 94.0, 48.0)));

        // Points inside visual
        assert!(take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 50.0, 30.0));
        assert!(take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 10.0, 20.0));
        assert!(take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 90.0, 44.0));

        // Points in slop region (outside visual, inside hit)
        assert!(take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 8.0, 18.0));
        assert!(take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 92.0, 46.0));
        assert!(take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 6.0, 16.0));
        assert!(take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 94.0, 48.0));

        // Points outside slop
        assert!(!take_button_hit_contains(10.0, 20.0, 90.0, 44.0, 5.0, 16.0));
        assert!(!take_button_hit_contains(
            10.0, 20.0, 90.0, 44.0, 95.0, 48.0
        ));
        assert!(!take_button_hit_contains(
            10.0, 20.0, 90.0, 44.0, 50.0, 15.0
        ));
        assert!(!take_button_hit_contains(
            10.0, 20.0, 90.0, 44.0, 50.0, 49.0
        ));

        // Choice and side helpers
        let choice_hit = take_choice_button_hit_rect(0.0, 0.0, TakeChoice::Right);
        assert!(choice_hit.is_some());
        if let Some((cx0, cy0, cx1, cy1)) = choice_hit {
            assert_eq!(cx0, -4.0);
            assert_eq!(cy0, -4.0);
            assert_eq!(cx1, 86.0 + 4.0);
            assert_eq!(cy1, 24.0 + 4.0);
        }

        let side_hit = take_side_button_hit_rect(0.0, 0.0, TakeSide::Center);
        assert!(side_hit.is_some());
        if let Some((sx0, sy0, sx1, sy1)) = side_hit {
            assert_eq!(sx0, -4.0);
            assert_eq!(sy0, -4.0);
            assert_eq!(sx1, 93.0 + 4.0);
            assert_eq!(sy1, 24.0 + 4.0);
        }

        // Invalid rects & non-finite checks
        assert_eq!(take_button_hit_rect(10.0, 20.0, 5.0, 44.0), None);
        assert_eq!(take_button_hit_rect(10.0, 20.0, 90.0, 15.0), None);
        assert_eq!(take_button_hit_rect(f32::NAN, 20.0, 90.0, 44.0), None);
        assert!(!take_button_hit_contains(
            10.0,
            20.0,
            90.0,
            44.0,
            f32::NAN,
            30.0
        ));
    }

    #[test]
    pub fn merge2_narrow_fixture() {
        let fixture_raw =
            include_str!("../../../checks/gui-exq-merge2/comparison/merge-3way-narrow.json");
        assert!(fixture_raw.contains("\"name\":\"merge-3way-narrow\""));
        assert!(fixture_raw.contains("\"avail_w\":360.0"));
        assert!(fixture_raw.contains("\"needs_hscroll\":true"));
        assert!(fixture_raw.contains("\"pane_min_w\":130.0"));
        assert!(fixture_raw.contains("\"section_gap\":8.0"));
        assert!(fixture_raw.contains("\"total_min_w\":406.0"));
        assert!(fixture_raw.contains("\"row_height\":15.6"));
        assert!(fixture_raw.contains("\"take_button_hit_slop\":4.0"));
        assert!(fixture_raw.contains("\"wide_avail_w\":960.0"));
        assert!(fixture_raw.contains("\"wide_needs_hscroll\":false"));

        // Match headless calculations against fixture numbers
        assert_eq!(MERGE_PANE_MIN_W, 130.0);
        assert_eq!(MERGE_SECTION_GAP, 8.0);
        assert_eq!(MERGE_TOTAL_MIN_W, 406.0);
        assert!((merge_row_height() - 15.6).abs() < 0.01);
        assert_eq!(TAKE_BUTTON_HIT_SLOP, 4.0);

        // Narrow 360.0 from fixture
        assert!(merge_three_pane_needs_hscroll(360.0));
        let (l360, b360, r360) = merge_three_pane_widths(360.0);
        assert_eq!(l360, 130.0);
        assert_eq!(b360, 130.0);
        assert_eq!(r360, 130.0);

        // Wide 960.0 from fixture
        assert!(!merge_three_pane_needs_hscroll(960.0));
        let (lw, _, _) = merge_three_pane_widths(960.0);
        assert!((lw - 314.666_66).abs() < 0.001);
    }
}

#[cfg(test)]
mod exq_merge3_tests {
    use super::*;
    use crate::theme::{GEO_SECTION_GAP, GEO_TIGHT_GAP, TypoRole};

    #[test]
    fn merge3_take_vertical_rhythm_from_mono() {
        assert_eq!(take_button_rhythm_role(), TypoRole::Mono);
        assert!((take_button_rhythm_role().line_height_px() - 15.6).abs() < 0.01);
        assert!((take_button_vpad() - 4.2).abs() < 0.01);
        assert_eq!(
            take_button_vpad(),
            (TAKE_BUTTON_H - TypoRole::Mono.line_height_px()) / 2.0
        );
        assert!((take_button_row_pitch() - 28.0).abs() < 0.01);
        assert_eq!(take_button_row_pitch(), TAKE_BUTTON_H + GEO_TIGHT_GAP);
        assert!(take_button_row_pitch() > merge_row_height());
    }

    #[test]
    fn merge3_divider_gaps_via_section_gap_at_960() {
        assert_eq!(merge_divider_gap(), GEO_SECTION_GAP);
        assert_eq!(merge_divider_gap(), 8.0);
        assert_eq!(merge_divider_gap(), MERGE_SECTION_GAP);
        let (x0, x1) = merge_three_divider_x(960.0);
        assert!((x0 - 314.666_66).abs() < 0.01);
        assert!((x1 - 637.333_3).abs() < 0.05);
        assert!(((x1 - x0) - (314.666_66 + 8.0)).abs() < 0.05);
        assert!(merge_three_panes_fill_exact(960.0));
        assert!(!merge_three_pane_needs_hscroll(960.0));
    }

    #[test]
    fn merge3_wide_fixture() {
        let raw = include_str!("../../../checks/gui-exq-merge3/comparison/merge-wide-960.json");
        assert!(raw.contains("\"name\":\"merge-wide-960\""));
        assert!(raw.contains("\"avail_w\":960.0"));
        assert!(raw.contains("\"section_gap\":8.0"));
        assert!(raw.contains("\"total_min_w\":406.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"take_button_vpad\":4.2"));
        assert!(raw.contains("\"take_button_pitch\":28.0"));
        assert!(raw.contains("\"copy_button_sym_pad\":4.0"));
        assert!((take_button_vpad() - 4.2).abs() < 0.01);
        assert!((merge_row_height() - 15.6).abs() < 0.01);
        let (l, b, r) = merge_three_pane_widths(960.0);
        assert!((l - 314.666_66).abs() < 0.01);
        assert!((b - 314.666_66).abs() < 0.01);
        assert!((r - 314.666_66).abs() < 0.01);
        assert!(merge_three_panes_fill_exact(960.0));
    }
}

#[cfg(test)]
mod exq_take7_tests {
    use super::*;

    #[test]
    fn take7_tall_marker_rows_aligned_with_hunk_rows() {
        assert_eq!(take_tall_canvas_h(), TAKE_TALL_CANVAS_H);
        assert_eq!(TAKE_TALL_CANVAS_H, 1200.0);
        assert_eq!(take_marker_row_y(0), 0.0);
        assert!((take_marker_row_y(10) - 156.0).abs() < 0.05);
        assert!((take_marker_row_y(76) - 1185.6).abs() < 0.5);
        // Band = one row stack, tall end canvas par clamp.
        let band0 = take_marker_row_range(0).unwrap_or_default();
        assert!((band0.0 - 0.0).abs() < 0.01);
        assert!((band0.1 - merge_row_height()).abs() < 0.01);
        let band76 = take_marker_row_range(76).unwrap_or_default();
        assert!((band76.0 - 1185.6).abs() < 0.5);
        assert!((band76.1 - 1200.0).abs() < 0.01);
        // Canvas ke bahar row par None.
        assert_eq!(take_marker_row_range(77), None);
        assert_eq!(take_marker_row_range(usize::MAX), None);
        // Order kept, OOB skip.
        let bands = take_marker_rows(&[0, 10, 77, 76]);
        assert_eq!(bands.len(), 3);
        let first = bands.first().copied().unwrap_or_default();
        assert!((first.0 - 0.0).abs() < 0.01);
        assert!(take_marker_rows(&[]).is_empty());
    }

    #[test]
    fn take7_tall_hit_lock_over_long_scroll() {
        // Scroll 600 / viewport 600 par row 0 upar nikalta hai, row 39 dikhta hai.
        assert_eq!(take_marker_visible_y(0, 600.0, 600.0), None);
        let vis39 = take_marker_visible_y(39, 600.0, 600.0).unwrap_or(-1.0);
        assert!((vis39 - 8.4).abs() < 0.5);
        // Row 39 ka button lock bina clamp (4.2 < 568).
        let lock39 = take_tall_hit_locked_y(39, 600.0, 600.0).unwrap_or(-1.0);
        assert!((lock39 - 4.2).abs() < 0.5);
        // Row 76 neeche: raw top 581.4 viewport-hit 568 par lock.
        let vis76 = take_marker_visible_y(76, 600.0, 600.0).unwrap_or(-1.0);
        assert!((vis76 - 585.6).abs() < 0.5);
        let lock76 = take_tall_hit_locked_y(76, 600.0, 600.0).unwrap_or(-1.0);
        assert!(
            (lock76 - (600.0 - take_button_hit_height_with_slop())).abs() < 0.5,
            "hit lock clamps to viewport bottom"
        );
        // Invisible marker ya bad input par None.
        assert_eq!(take_tall_hit_locked_y(0, 600.0, 600.0), None);
        assert_eq!(take_tall_hit_locked_y(39, 600.0, 0.0), None);
        assert_eq!(take_tall_hit_locked_y(39, f32::NAN, 600.0), None);
        assert_eq!(take_marker_visible_y(39, 600.0, f32::INFINITY), None);
    }

    #[test]
    fn take7_tall_side_hit_and_marker_rhythm() {
        assert!((take_marker_rhythm_step() - 15.6).abs() < 0.01);
        assert_eq!(take_marker_rhythm_step(), merge_row_height());
        // Har tall marker band rhythm par baithta hai.
        for row in [0usize, 10, 39, 76] {
            assert!(
                take_marker_on_rhythm(take_marker_row_y(row)),
                "row {row} off rhythm"
            );
        }
        assert!(take_marker_on_rhythm(0.0));
        assert!(!take_marker_on_rhythm(7.0));
        assert!(!take_marker_on_rhythm(-1.0));
        assert!(!take_marker_on_rhythm(f32::NAN));
        // Center button hit rect row 10 par: x slop -4..97, top 156-4.2-4.
        let hit = take_tall_side_hit_rect(10, 0.0, TakeSide::Center).unwrap_or_default();
        assert!((hit.0 - -4.0).abs() < 0.01);
        assert!((hit.2 - 97.0).abs() < 0.01);
        assert!((hit.1 - (156.0 - take_button_vpad() - TAKE_BUTTON_HIT_SLOP)).abs() < 0.5);
        assert!((hit.3 - hit.1 - take_button_hit_height_with_slop()).abs() < 0.01);
        // OOB / non-finite par None.
        assert_eq!(take_tall_side_hit_rect(77, 0.0, TakeSide::Left), None);
        assert_eq!(take_tall_side_hit_rect(10, f32::NAN, TakeSide::Right), None);
    }

    #[test]
    fn take7_tall_fixture() {
        let raw = include_str!("../../../checks/gui-exq-take7/comparison/take-tall-1200.json");
        assert!(raw.contains("\"name\":\"take-tall-1200\""));
        assert!(raw.contains("\"canvas_h\":1200.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"rhythm_step\":15.6"));
        assert!(raw.contains("\"take_button_h\":24.0"));
        assert!(raw.contains("\"take_hit_h_slop\":32.0"));
        assert!(raw.contains("\"scroll_y\":600.0"));
        assert!(raw.contains("\"viewport_h\":600.0"));
        // Fixture numbers headless recompute se match.
        assert!((take_tall_canvas_h() - 1200.0).abs() < 0.01);
        assert!((take_marker_row_y(76) - 1185.6).abs() < 0.5);
        let lock76 = take_tall_hit_locked_y(76, 600.0, 600.0).unwrap_or(-1.0);
        assert!((lock76 - 568.0).abs() < 0.5);
    }
}

#[cfg(test)]
mod exq_take33_tests {
    use super::*;

    #[test]
    fn take33_buttons_row_reuses_space_tokens() {
        assert_eq!(take_bar_gap(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(take_bar_gap(), crate::theme::SPACE_XS);
        assert_eq!(take_buttons_row_x(0), Some(0.0));
        assert!((take_buttons_row_x(1).unwrap_or(-1.0) - 97.0).abs() < 0.01);
        assert!((take_buttons_row_x(2).unwrap_or(-1.0) - 194.0).abs() < 0.01);
        assert_eq!(take_buttons_row_x(3), None);
        // Row pitch single source: uniform 93 + tight 4.
        let x1 = take_buttons_row_x(1).unwrap_or(-1.0);
        assert!((x1 - (take_bar_button_width() + take_bar_gap())).abs() < 0.01);
        assert!((take_buttons_row_x(2).unwrap_or(-1.0) - 2.0 * x1).abs() < 0.01);
    }

    #[test]
    fn take33_center_pane_alignment() {
        // 700 edge: equal thirds, center starts after left + gap.
        let (l700, b700, r700) = merge_three_pane_widths(700.0);
        assert!((l700 - 228.0).abs() < 0.01);
        assert!((b700 - 228.0).abs() < 0.01);
        assert!((r700 - 228.0).abs() < 0.01);
        assert!((merge_center_pane_x(700.0) - 236.0).abs() < 0.01);
        assert!(merge_center_pane_aligned(700.0));
        // 1200 wide: equal thirds, centered, exact fill, no scroll.
        let (l1200, b1200, _) = merge_three_pane_widths(1200.0);
        assert!((l1200 - 394.666_66).abs() < 0.01);
        assert!((b1200 - 394.666_66).abs() < 0.01);
        assert!((merge_center_pane_x(1200.0) - 402.666_66).abs() < 0.05);
        assert!(merge_center_pane_aligned(1200.0));
        assert!(merge_three_panes_fill_exact(1200.0));
        assert!(!merge_three_pane_needs_hscroll(1200.0));
        assert!(!take_bar_needs_hscroll(1200.0));
        assert!(!merge_center_pane_aligned(0.0));
        assert!(!merge_center_pane_aligned(f32::NAN));
    }

    #[test]
    fn take33_conflict_badge_placement() {
        assert_eq!(take_conflict_badge_inset(), crate::theme::SPACE_XS);
        assert_eq!(take_conflict_badge_inset(), 4.0);
        // Badge row 10 par pane trailing se XS inset, marker row se aligned.
        let pos = take_conflict_badge_pos(10, 236.0, 228.0).unwrap_or((-1.0, -1.0));
        assert!((pos.0 - 460.0).abs() < 0.01);
        assert!((pos.1 - take_marker_row_y(10)).abs() < 0.01);
        // Badge pane ke andar hai.
        assert!(pos.0 >= 236.0 && pos.0 <= 236.0 + 228.0);
        // Bad input par None.
        assert_eq!(take_conflict_badge_pos(77, 236.0, 228.0), None);
        assert_eq!(take_conflict_badge_pos(10, f32::NAN, 228.0), None);
        assert_eq!(take_conflict_badge_pos(10, 236.0, 0.0), None);
    }

    #[test]
    fn take33_badge_role_reuses_typo_only() {
        assert_eq!(
            take_conflict_badge_text_role(),
            crate::theme::TypoRole::chrome_status()
        );
        assert_eq!(
            take_conflict_badge_text_role(),
            crate::theme::TypoRole::Small
        );
        assert_eq!(take_conflict_badge_text_size(), 11.0);
        assert_eq!(take_conflict_badge_text_size(), take_counter_text_size());
    }

    #[test]
    fn take33_narrow700_fixture() {
        let raw =
            include_str!("../../../checks/gui-exq-take33/round-1/comparison/take-narrow-700.json");
        assert!(raw.contains("\"name\":\"take-narrow-700\""));
        assert!(raw.contains("\"avail_w\":700.0"));
        assert!(raw.contains("\"needs_hscroll_700\":false"));
        assert!(raw.contains("\"take_bar_hscroll_700\":false"));
        assert!(raw.contains("\"take_bar_total\":287.0"));
        assert!(raw.contains("\"pane_w_700\":228.0"));
        assert!(raw.contains("\"center_x_700\":236.0"));
        assert!(raw.contains("\"wide_avail_w\":1200.0"));
        assert!(raw.contains("\"needs_hscroll_1200\":false"));
        // Fixture numbers headless recompute se match.
        assert!(!merge_three_pane_needs_hscroll(700.0));
        assert!(!take_bar_needs_hscroll(700.0));
        let (l, _, _) = merge_three_pane_widths(700.0);
        assert!((l - 228.0).abs() < 0.01);
        assert!((merge_center_pane_x(700.0) - 236.0).abs() < 0.01);
        assert_eq!(take_bar_total_width(), 287.0);
        assert!((take_buttons_row_x(1).unwrap_or(-1.0) - 97.0).abs() < 0.01);
    }
}

// ----------------------------------------------------------------------------
// EXQ-TAKE44: live merge take resolution geometry (headless).
// BC parity: live resolved output wahi Mono row rhythm par baithta hai;
// resolution progress avail width ka fraction hai; narrow par panes floor
// par scroll karte hain. Geometry first (take_marker_row_y /
// take_conflict_block_height_px / merge_three_pane_widths reuse, no new
// literals), typography second (counter Small single source), colors frozen.
// WIRING (paint): resolved rows `take_resolved_row_y` se y; live output
// height `take_live_output_height_px` se; progress fill
// `take_resolution_bar_width(avail_w, resolved, total)` se.
// ----------------------------------------------------------------------------

/// Live resolved row top y px: marker rhythm single source ([`take_marker_row_y`]).
/// Pure; headless-tested.
pub fn take_resolved_row_y(row: usize) -> f32 {
    take_marker_row_y(row)
}

/// Live resolved output height px: Mono stack single source
/// ([`take_conflict_block_height_px`]). Pure; headless-tested.
pub fn take_live_output_height_px(line_count: usize) -> f32 {
    take_conflict_block_height_px(line_count)
}

/// Live resolution fraction 0..1: total 0 par 1.0 (kuch resolve nahi karna);
/// resolved >= total par 1.0. Pure; headless-tested.
pub fn take_resolution_fraction(resolved: usize, total: usize) -> f32 {
    if total == 0 {
        return 1.0;
    }
    if resolved >= total {
        return 1.0;
    }
    resolved as f32 / total as f32
}

/// Live resolution bar fill width px: `avail_w * fraction`; non-finite ya
/// non-positive avail par 0.0. Pure; headless-tested.
pub fn take_resolution_bar_width(avail_w: f32, resolved: usize, total: usize) -> f32 {
    if !avail_w.is_finite() || avail_w <= 0.0 {
        return 0.0;
    }
    let frac = take_resolution_fraction(resolved, total);
    if !frac.is_finite() {
        return 0.0;
    }
    avail_w * frac
}

/// Live resolution text role: Small status precedent
/// ([`take_counter_text_role`] single source). Pure.
pub const fn take_resolution_text_role() -> crate::theme::TypoRole {
    take_counter_text_role()
}

/// Live resolution text size px (single source: Small, 11.0). Pure.
pub fn take_resolution_text_size() -> f32 {
    take_resolution_text_role().size()
}

#[cfg(test)]
mod exq_take44_tests {
    use super::*;

    #[test]
    fn take_exq_take44_live_take_geometry() {
        // Resolved rows marker rhythm se, koi naya literal nahi.
        assert_eq!(take_resolved_row_y(0), take_marker_row_y(0));
        assert!((take_resolved_row_y(10) - 156.0).abs() < 0.05);
        assert!((take_resolved_row_y(10) - take_marker_row_y(10)).abs() < 0.0001);
        assert!((take_resolved_row_y(76) - 1185.6).abs() < 0.5);
        // Live output height wahi Mono stack hai.
        assert_eq!(
            take_live_output_height_px(0),
            take_conflict_block_height_px(0)
        );
        assert!((take_live_output_height_px(1) - 15.6).abs() < 0.01);
        assert!((take_live_output_height_px(4) - 62.4).abs() < 0.05);
        assert_eq!(
            take_live_output_height_px(4),
            take_conflict_block_height_px(4)
        );
        // Resolution fraction + bar fill.
        assert_eq!(take_resolution_fraction(0, 0), 1.0);
        assert!((take_resolution_fraction(0, 2) - 0.0).abs() < 0.0001);
        assert!((take_resolution_fraction(1, 2) - 0.5).abs() < 0.0001);
        assert_eq!(take_resolution_fraction(2, 2), 1.0);
        assert_eq!(take_resolution_fraction(5, 2), 1.0);
        assert!((take_resolution_bar_width(960.0, 1, 2) - 480.0).abs() < 0.01);
        assert_eq!(take_resolution_bar_width(960.0, 0, 2), 0.0);
        assert!((take_resolution_bar_width(960.0, 2, 2) - 960.0).abs() < 0.01);
        assert_eq!(take_resolution_bar_width(f32::NAN, 1, 2), 0.0);
        assert_eq!(take_resolution_bar_width(0.0, 1, 2), 0.0);
        assert_eq!(take_resolution_bar_width(-10.0, 1, 2), 0.0);
        // Typo reuse: counter Small single source.
        assert_eq!(take_resolution_text_role(), take_counter_text_role());
        assert_eq!(take_resolution_text_role(), crate::theme::TypoRole::Small);
        assert_eq!(take_resolution_text_size(), 11.0);
        assert_eq!(take_resolution_text_size(), take_counter_text_size());
        // Fixture recompute: wide 960 live + narrow 360 edge.
        let raw =
            include_str!("../../../checks/gui-exq-take44/round-1/comparison/take-live-960.json");
        assert!(raw.contains("\"name\":\"take-live-960\""));
        assert!(raw.contains("\"wide_avail_w\":960.0"));
        assert!(raw.contains("\"pane_w_960\":314.666656"));
        assert!(raw.contains("\"center_x_960\":322.666656"));
        assert!(raw.contains("\"take_bar_total\":287.0"));
        assert!(raw.contains("\"row_height\":15.6"));
        assert!(raw.contains("\"live_output_h_4\":62.4"));
        assert!(raw.contains("\"resolved_row_y_10\":156.0"));
        assert!(raw.contains("\"resolution_half_960\":480.0"));
        assert!(raw.contains("\"narrow_avail_w\":360.0"));
        assert!(raw.contains("\"narrow_needs_hscroll\":true"));
        assert!(raw.contains("\"wide_needs_hscroll\":false"));
        let (l960, _, _) = merge_three_pane_widths(960.0);
        assert!((l960 - 314.666_66).abs() < 0.01);
        assert!((merge_center_pane_x(960.0) - 322.666_66).abs() < 0.05);
        assert_eq!(take_bar_total_width(), 287.0);
        assert!(!merge_three_pane_needs_hscroll(960.0));
        let (nl, _, _) = merge_three_pane_widths(360.0);
        assert_eq!(nl, 130.0);
        assert!(merge_three_pane_needs_hscroll(360.0));
    }
}

#[cfg(test)]
mod exq_take70_tests {
    use super::*;

    #[test]
    fn take70_geometry_tokens_are_value_identical() {
        // Buttons / gaps / slop: single-source SPACE/GEO tokens, hit math untouched.
        assert_eq!(TAKE_BUTTON_H, crate::theme::SPACE_XL);
        assert_eq!(TAKE_BUTTON_H, 24.0);
        assert_eq!(TAKE_BUTTON_PAD_X, crate::theme::SPACE_SM);
        assert_eq!(TAKE_BUTTON_PAD_X, 8.0);
        assert_eq!(MERGE_SECTION_GAP, crate::theme::GEO_SECTION_GAP);
        assert_eq!(MERGE_SECTION_GAP, 8.0);
        assert_eq!(TAKE_BUTTON_HIT_SLOP, crate::theme::SPACE_XS);
        assert_eq!(TAKE_MINIMAP_MARK_MIN_H, crate::theme::SPACE_XS);
        assert_eq!(take_bar_gap(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(take_stepper_gap(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(merge_divider_gap(), crate::theme::GEO_SECTION_GAP);
        assert_eq!(take_conflict_badge_inset(), crate::theme::SPACE_XS);
        // Row rhythm: Mono line-height single source.
        assert_eq!(
            merge_row_height(),
            crate::theme::TypoRole::Mono.line_height_px()
        );
        assert_eq!(take_row_height(), merge_row_height());
        assert!((merge_row_height() - 15.6).abs() < 0.01);
        // Derived widths: token math, behavior-identical.
        assert_eq!(
            take_button_row_pitch(),
            TAKE_BUTTON_H + crate::theme::GEO_TIGHT_GAP
        );
        assert_eq!(take_button_row_pitch(), 28.0);
        assert_eq!(take_button_hit_height_with_slop(), 32.0);
        assert_eq!(
            take_button_hit_height_with_slop(),
            TAKE_BUTTON_H + 2.0 * crate::theme::SPACE_XS
        );
        assert_eq!(
            MERGE_TOTAL_MIN_W,
            MERGE_PANE_MIN_W * 3.0 + MERGE_SECTION_GAP * 2.0
        );
        assert_eq!(MERGE_TOTAL_MIN_W, 406.0);
        // Tall canvas: viewport token x2, value-identical 1200.
        assert_eq!(TAKE_TALL_CANVAS_H, crate::theme::VIEWPORT_H as f32 * 2.0);
        assert_eq!(TAKE_TALL_CANVAS_H, 1200.0);
        assert_eq!(take_tall_canvas_h(), TAKE_TALL_CANVAS_H);
    }

    #[test]
    fn take70_remaining_literals_have_no_identical_token() {
        // 130.0 pane floor: no 130-valued SPACE/GEO/CHROME token exists (rg proof in gate).
        assert_eq!(MERGE_PANE_MIN_W, 130.0);
        // 7.0 label advance: no 7-valued typo token exists (rg proof in gate).
        assert_eq!(TAKE_LABEL_ADVANCE_PX, 7.0);
        // Hit behavior unchanged spot-checks (same numbers as merge2/copytake6 gates).
        assert_eq!(take_choice_button_width(TakeChoice::Right), 86.0);
        assert_eq!(take_side_button_width(TakeSide::Center), 93.0);
        assert_eq!(take_bar_total_width(), 287.0);
    }
}

#[cfg(test)]
mod exq_agyxswarm80_take_tests {
    use super::*;

    #[test]
    fn take_side_handling_indices_opposites_and_conversions() {
        assert_eq!(
            TakeSide::ALL,
            [TakeSide::Left, TakeSide::Center, TakeSide::Right]
        );
        assert_eq!(
            TakeSide::sides(),
            &[TakeSide::Left, TakeSide::Center, TakeSide::Right]
        );

        assert_eq!(TakeSide::Left.index(), 0);
        assert_eq!(TakeSide::Center.index(), 1);
        assert_eq!(TakeSide::Right.index(), 2);

        assert_eq!(TakeSide::from_index(0), Some(TakeSide::Left));
        assert_eq!(TakeSide::from_index(1), Some(TakeSide::Center));
        assert_eq!(TakeSide::from_index(2), Some(TakeSide::Right));
        assert_eq!(TakeSide::from_index(3), None);

        assert_eq!(TakeSide::Left.opposite(), TakeSide::Right);
        assert_eq!(TakeSide::Right.opposite(), TakeSide::Left);
        assert_eq!(TakeSide::Center.opposite(), TakeSide::Center);

        assert_eq!(TakeSide::Left.short_code(), "L");
        assert_eq!(TakeSide::Center.short_code(), "C");
        assert_eq!(TakeSide::Right.short_code(), "R");

        assert_eq!(
            TakeSide::from_gui_choice(TakeChoice::Left),
            Some(TakeSide::Left)
        );
        assert_eq!(
            TakeSide::from_gui_choice(TakeChoice::Base),
            Some(TakeSide::Center)
        );
        assert_eq!(
            TakeSide::from_gui_choice(TakeChoice::Right),
            Some(TakeSide::Right)
        );
        assert_eq!(TakeSide::from_gui_choice(TakeChoice::Both), None);

        assert_eq!(
            TakeSide::from_diff_choice(DiffTakeChoice::Left),
            Some(TakeSide::Left)
        );
        assert_eq!(
            TakeSide::from_diff_choice(DiffTakeChoice::Base),
            Some(TakeSide::Center)
        );
        assert_eq!(
            TakeSide::from_diff_choice(DiffTakeChoice::Right),
            Some(TakeSide::Right)
        );
        assert_eq!(
            TakeSide::from_diff_choice(DiffTakeChoice::BothLeftFirst),
            None
        );
        assert_eq!(
            TakeSide::from_diff_choice(DiffTakeChoice::BothRightFirst),
            None
        );

        assert_eq!(take_side_gap(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(take_side_gap(), 4.0);
        assert_eq!(take_side_bar_h(), TAKE_BUTTON_H);
        assert_eq!(take_side_bar_h(), 24.0);
        assert_eq!(take_side_button_pad(), TAKE_BUTTON_PAD_X);
        assert_eq!(take_side_button_pad(), 8.0);
    }

    #[test]
    fn take_snapshot_and_side_inspection() {
        let base = ["common_header", "base1", "mid", "base2", "tail"];
        let left = ["common_header", "left1", "mid", "left2", "tail"];
        let right = ["common_header", "right1", "mid", "right2", "tail"];
        let mut sess = TakeSession::new(&base, &left, &right);

        assert_eq!(sess.conflict_count(), 2);
        assert_eq!(sess.unresolved_count(), 2);
        assert_eq!(sess.side_at(0), None);
        assert_eq!(sess.side_at(1), None);
        assert_eq!(sess.side_at(2), None);
        assert_eq!(sess.resolved_sides(), vec![None, None]);

        assert!(sess.take_left(0));
        assert_eq!(sess.side_at(0), Some(TakeSide::Left));
        assert_eq!(sess.side_at(1), None);
        assert_eq!(sess.unresolved_count(), 1);

        assert!(sess.take_right(1));
        assert_eq!(sess.side_at(1), Some(TakeSide::Right));
        assert!(sess.is_fully_resolved());
        assert_eq!(
            sess.resolved_sides(),
            vec![Some(TakeSide::Left), Some(TakeSide::Right)]
        );

        let snap = sess.snapshot();
        assert_eq!(snap.conflict_count, 2);
        assert_eq!(snap.unresolved_count, 0);
        assert_eq!(snap.resolved_count(), 2);
        assert!(snap.is_fully_resolved);
        assert!((snap.resolution_fraction() - 1.0).abs() < 0.001);
        assert!((snap.resolution_bar_width(960.0) - 960.0).abs() < 0.01);
        assert_eq!(snap.stepper_text(), "Conflict 2 of 2");
        assert_eq!(
            snap.output_lines,
            vec!["common_header", "left1", "mid", "right2", "tail"]
        );

        assert!(sess.clear(0));
        assert!(sess.take_center(1));
        assert_eq!(sess.side_at(0), None);
        assert_eq!(sess.side_at(1), Some(TakeSide::Center));

        sess.restore_snapshot(&snap);
        assert_eq!(sess.side_at(0), Some(TakeSide::Left));
        assert_eq!(sess.side_at(1), Some(TakeSide::Right));
        assert!(sess.is_fully_resolved());
        assert_eq!(sess.output_lines(), snap.output_lines);
    }

    #[test]
    fn take_snapshot_tmp_only_output_write() {
        let base = ["head", "base_val", "foot"];
        let left = ["head", "chosen_left", "foot"];
        let right = ["head", "right_val", "foot"];
        let mut sess = TakeSession::new(&base, &left, &right);
        assert!(sess.take_left(0));

        let tmp_path =
            std::env::temp_dir().join(format!("exq-take-test-{}.txt", std::process::id()));
        let write_res = sess.save_output_to_path(&tmp_path);
        assert!(write_res.is_ok());
        let bytes_written = write_res.unwrap_or(0);
        assert!(bytes_written > 0);

        let read_back = std::fs::read_to_string(&tmp_path).unwrap_or_default();
        assert_eq!(read_back, sess.output_text());
        assert_eq!(read_back, "head\nchosen_left\nfoot\n");

        let _ = std::fs::remove_file(&tmp_path);
    }
}
